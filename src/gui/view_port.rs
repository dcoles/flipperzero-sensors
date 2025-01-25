use core::{ffi::c_void, ptr};

use flipperzero_sys as sys;

#[repr(transparent)]
pub struct ViewPort {
    raw: ptr::NonNull<sys::ViewPort>,
}

impl ViewPort {
    pub fn new() -> Self {
        Self {
            // SAFETY: `view_port_alloc` never returns NULL (will `furi_check` on error).
            raw: unsafe { ptr::NonNull::new_unchecked(sys::view_port_alloc()) }
        }
    }

    /// Get pointer to raw [`sys::ViewPort`].
    pub const fn as_ptr(&self) -> *mut sys::ViewPort {
        self.raw.as_ptr()
    }

    /// Set ViewPort width.
    ///
    /// Will be used to limit canvas drawing area and autolayout feature.
    pub fn set_width(&self, width: u8) {
        unsafe { sys::view_port_set_width(self.as_ptr(), width) }
    }

    /// Get ViewPort width.
    pub fn get_width(&self) -> u8 {
        unsafe { sys::view_port_get_width(self.as_ptr()) }
    }

    /// Set ViewPort height.
    ///
    /// Will be used to limit canvas drawing area and autolayout feature.
    pub fn set_height(&self, height: u8) {
        unsafe { sys::view_port_set_height(self.as_ptr(), height) }
    }

    /// Get ViewPort height.
    pub fn get_height(&self) -> u8 {
        unsafe { sys::view_port_get_height(self.as_ptr()) }
    }

    /// Enable or disable ViewPort rendering.
    pub fn enabled(&self, enabled: bool) {
        unsafe { sys::view_port_enabled_set(self.as_ptr(), enabled) }
    }

    /// Check if ViewPort rendering is enabled.
    pub fn is_enabled(&self) -> bool {
        unsafe { sys::view_port_is_enabled(self.as_ptr()) }
    }

    /// Set draw callback.
    pub unsafe fn set_draw_callback(&self, callback: sys::ViewPortDrawCallback, context: *mut c_void) {
        unsafe { sys::view_port_draw_callback_set(self.as_ptr(), callback, context) }
    }

    /// Set input callback.
    pub unsafe fn set_input_callback(&self, callback: sys::ViewPortInputCallback, context: *mut c_void) {
        unsafe { sys::view_port_input_callback_set(self.as_ptr(), callback, context) }
    }

    /// Emit update signal to GUI system.
    ///
    /// Rendering will happen asyncronously after GUI system process signal.
    pub fn update(&self) {
        unsafe { sys::view_port_update(self.as_ptr()) }
    }

    /// Set ViewPort orientation.
    pub fn set_orientation(&self, orientation: sys::ViewPortOrientation) {
        unsafe { sys::view_port_set_orientation(self.as_ptr(), orientation) }
    }

    /// Get ViewPort orientation.
    pub fn get_orientation(&self) -> sys::ViewPortOrientation {
        unsafe { sys::view_port_get_orientation(self.as_ptr()) }
    }
}

impl Default for ViewPort {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ViewPort {
    fn drop(&mut self) {
        // SAFETY: Pointer is valid `ViewPort` and non-NULL.
        unsafe { sys::view_port_free(self.as_ptr()) }
    }
}
