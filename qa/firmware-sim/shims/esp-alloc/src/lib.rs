//! Host stand-in for the capability heap: external allocations come from the
//! registered region with the vendor's capacity limit and first-fit behaviour.
use core::alloc::Layout;
use std::sync::Mutex;

#[derive(Clone, Copy)]
pub enum MemoryCapability {
    External,
    Internal,
}

#[derive(Clone, Copy)]
pub struct Capabilities;

impl From<MemoryCapability> for Capabilities {
    fn from(_: MemoryCapability) -> Self {
        Capabilities
    }
}

#[derive(Clone, Copy)]
struct Span {
    start: usize,
    len: usize,
}

pub struct HeapRegion(Span);

impl HeapRegion {
    /// # Safety
    /// The region must stay valid and unused elsewhere, as for the vendor heap.
    pub unsafe fn new(start: *mut u8, len: usize, _caps: Capabilities) -> Self {
        Self(Span { start: start as usize, len })
    }
}

pub struct Heap {
    free: Mutex<Vec<Span>>,
}

pub static HEAP: Heap = Heap { free: Mutex::new(Vec::new()) };

impl Heap {
    pub fn add_region(&self, region: HeapRegion) {
        self.free.lock().expect("heap poisoned").push(region.0);
    }

    pub fn free_caps(&self, _caps: Capabilities) -> usize {
        self.free.lock().expect("heap poisoned").iter().map(|span| span.len).sum()
    }

    /// # Safety
    /// Same contract as `GlobalAlloc::alloc`.
    pub unsafe fn alloc_caps(&self, _caps: Capabilities, layout: Layout) -> *mut u8 {
        let mut free = self.free.lock().expect("heap poisoned");
        let found = free.iter().enumerate().find_map(|(index, span)| {
            let start = span.start.checked_next_multiple_of(layout.align())?;
            let padding = start - span.start;
            (span.len >= padding.checked_add(layout.size())?).then_some((index, start, padding))
        });
        let Some((index, start, padding)) = found else {
            return core::ptr::null_mut();
        };
        let span = free.remove(index);
        let tail = Span { start: start + layout.size(), len: span.len - padding - layout.size() };
        if tail.len > 0 {
            free.insert(index, tail);
        }
        if padding > 0 {
            free.insert(index, Span { start: span.start, len: padding });
        }
        start as *mut u8
    }

    /// # Safety
    /// Same contract as `GlobalAlloc::dealloc`.
    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut free = self.free.lock().expect("heap poisoned");
        free.push(Span { start: ptr as usize, len: layout.size() });
        free.sort_by_key(|span| span.start);
        let mut merged: Vec<Span> = Vec::with_capacity(free.len());
        for span in free.drain(..) {
            match merged.last_mut() {
                Some(last) if last.start + last.len == span.start => last.len += span.len,
                _ => merged.push(span),
            }
        }
        *free = merged;
    }
}
