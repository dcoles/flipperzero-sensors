use core::{ffi::c_void, ptr};

use flipperzero::furi::time::Duration as FuriDuration;
use flipperzero_sys as sys;

#[repr(transparent)]
pub struct ViewDispatcher {
    raw: ptr::NonNull<sys::ViewDispatcher>,
}

impl ViewDispatcher {
    pub fn new() -> Self {
        // SAFETY: Alloc can never return NULL (raises `furi_check` on allocation error)
        let raw = unsafe { ptr::NonNull::new_unchecked(sys::view_dispatcher_alloc()) };

        Self {
            raw
        }
    }

    pub fn as_ptr(&self) -> *mut sys::ViewDispatcher {
        self.raw.as_ptr()
    }

    pub fn set_event_callback_context(&self, context: *mut c_void) {
        unsafe { sys::view_dispatcher_set_event_callback_context(self.as_ptr(), context) }
    }

    pub fn set_tick_event_callback(&self, callback: Option<unsafe extern "C" fn(*mut c_void)>, tick_period: FuriDuration) {
        unsafe { sys::view_dispatcher_set_tick_event_callback(self.as_ptr(), callback, tick_period.as_ticks()) }
    }

    pub fn add_view(&self, view_id: u32, view: *mut sys::View) {
        unsafe { sys::view_dispatcher_add_view(self.as_ptr(), view_id, view) }
    }
    
    pub fn switch_to_view(&self, view_id: u32) {
        unsafe { sys::view_dispatcher_switch_to_view(self.as_ptr(), view_id) }
    }

    pub fn remove_view(&self, view_id: u32) {
        unsafe { sys::view_dispatcher_remove_view(self.as_ptr(), view_id) }
    }

    pub fn attach_to_gui(&self, gui: *mut sys::Gui, type_: sys::ViewDispatcherType) {
        unsafe { sys::view_dispatcher_attach_to_gui(self.as_ptr(), gui, type_) }
    }

    pub fn run(&self) {
        unsafe { sys::view_dispatcher_run(self.as_ptr()) }
    }
}

impl Default for ViewDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ViewDispatcher {
    fn drop(&mut self) {
        unsafe {
            sys::view_dispatcher_free(self.as_ptr());
        }
    }
}