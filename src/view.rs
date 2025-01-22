use core::{ffi::c_void, ptr};

use flipperzero_sys as sys;


#[repr(transparent)]
pub struct View {
    raw: ptr::NonNull<sys::View>,
}

impl View {
    pub fn new() -> Self {
        Self {
            // SAFETY: `view_alloc` never returns NULL (throws `furi_check` on error)
            raw: unsafe { ptr::NonNull::new_unchecked(sys::view_alloc()) }
        }
    }

    pub fn as_ptr(&self) -> *mut sys::View {
        self.raw.as_ptr()
    }

    pub fn set_context(&self, context: *mut c_void) {
        unsafe { sys::view_set_context(self.as_ptr(), context) }
    }

    pub fn set_draw_callback(&self, callback: Option<unsafe extern "C" fn(*mut sys::Canvas, *mut c_void)>) {
        unsafe { sys::view_set_draw_callback(self.as_ptr(), callback) }
    }
    
    pub fn set_previous_callback(&self, callback: Option<unsafe extern "C" fn(*mut c_void) -> u32>) {
        unsafe { sys::view_set_previous_callback(self.as_ptr(), callback) }
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new()
    }
}