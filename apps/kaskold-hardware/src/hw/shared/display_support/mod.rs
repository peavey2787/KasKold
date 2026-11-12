// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use embedded_graphics::{
    pixelcolor::{raw::RawU16, Rgb565},
    prelude::*,
    primitives::Rectangle,
};

pub struct TeeDisplay<D> {
    pub(crate) real: D,
    shadow_active: bool,
}

impl<D> TeeDisplay<D> {
    pub fn new(real: D) -> Self {
        Self { real, shadow_active: false }
    }

    pub fn enable_shadow(&mut self) {
        self.shadow_active = crate::hw::screenshot::with_framebuffer_mut(|_| ()).is_some();
    }
}

use crate::hw::screenshot::{H, W};

/// The panel area the shadow framebuffer mirrors.
const SCREEN: Rectangle =
    Rectangle::new(Point::zero(), Size::new(shared_signer::narrow!(u32, W), shared_signer::narrow!(u32, H)));

fn write_shadow_pixel(framebuffer: &mut [u8], point: Point, color: Rgb565) {
    let (Ok(x), Ok(y)) = (usize::try_from(point.x), usize::try_from(point.y)) else {
        return;
    };
    if x >= W || y >= H {
        return;
    }
    let index = (y * W + x) * 2;
    framebuffer[index..index + 2].copy_from_slice(&RawU16::from(color).into_inner().to_be_bytes());
}

impl<D> DrawTarget for TeeDisplay<D>
where
    D: DrawTarget<Color = Rgb565> + OriginDimensions,
{
    type Color = Rgb565;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let pixels: alloc::vec::Vec<Pixel<Rgb565>> = pixels.into_iter().collect();
        if self.shadow_active {
            let _ = crate::hw::screenshot::with_framebuffer_mut(|framebuffer| {
                for &Pixel(point, color) in &pixels {
                    write_shadow_pixel(framebuffer, point, color);
                }
            });
        }
        self.real.draw_iter(pixels)
    }

    fn fill_contiguous<I>(&mut self, area: &Rectangle, colors: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Self::Color>,
    {
        let colors: alloc::vec::Vec<Rgb565> = colors.into_iter().collect();
        if self.shadow_active {
            let _ = crate::hw::screenshot::with_framebuffer_mut(|framebuffer| {
                for (point, &color) in area.points().zip(&colors) {
                    write_shadow_pixel(framebuffer, point, color);
                }
            });
        }
        self.real.fill_contiguous(area, colors)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        if self.shadow_active {
            let _ = crate::hw::screenshot::with_framebuffer_mut(|framebuffer| {
                for point in area.intersection(&SCREEN).points() {
                    write_shadow_pixel(framebuffer, point, color);
                }
            });
        }
        self.real.fill_solid(area, color)
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        if self.shadow_active {
            let bytes = RawU16::from(color).into_inner().to_be_bytes();
            let _ = crate::hw::screenshot::with_framebuffer_mut(|framebuffer| {
                for pixel in framebuffer.chunks_exact_mut(2) {
                    pixel.copy_from_slice(&bytes);
                }
            });
        }
        self.real.clear(color)
    }
}

impl<D> OriginDimensions for TeeDisplay<D>
where
    D: OriginDimensions,
{
    fn size(&self) -> Size {
        self.real.size()
    }
}
