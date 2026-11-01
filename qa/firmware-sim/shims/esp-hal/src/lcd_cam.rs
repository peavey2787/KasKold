use crate::peripherals::LCD_CAM;

pub mod cam;

pub struct LcdCam<'d> {
    pub cam: cam::Cam<'d>,
}

impl<'d> LcdCam<'d> {
    pub fn new(_peripheral: LCD_CAM<'d>) -> Self {
        Self { cam: cam::Cam::new() }
    }
}
