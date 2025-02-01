use core::{ffi::c_void, ptr};

use flipperzero::furi::time::FuriDuration;
use flipperzero_sys as sys;

use crate::furi::record::Record;

use super::{Gui, View};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewId(pub u32);

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

    pub const fn as_ptr(&self) -> *mut sys::ViewDispatcher {
        self.raw.as_ptr()
    }

    pub unsafe fn set_event_callback_context(&self, context: *mut c_void) {
        unsafe { sys::view_dispatcher_set_event_callback_context(self.as_ptr(), context) }
    }

    pub unsafe fn set_tick_event_callback(&self, callback: Option<unsafe extern "C" fn(*mut c_void)>, tick_period: FuriDuration) {
        unsafe { sys::view_dispatcher_set_tick_event_callback(self.as_ptr(), callback, tick_period.as_ticks()) }
    }

    pub fn add_view(&self, view_id: ViewId, view: &View) {
        unsafe { sys::view_dispatcher_add_view(self.as_ptr(), view_id.0, view.as_ptr()) }
    }

    pub fn switch_to_view(&self, view_id: ViewId) {
        unsafe { sys::view_dispatcher_switch_to_view(self.as_ptr(), view_id.0) }
    }

    pub fn remove_view(&self, view_id: ViewId) {
        unsafe { sys::view_dispatcher_remove_view(self.as_ptr(), view_id.0) }
    }

    pub fn attach_to_gui(&self, gui: &Record<Gui>, type_: sys::ViewDispatcherType) {
        unsafe { sys::view_dispatcher_attach_to_gui(self.as_ptr(), gui.as_ptr(), type_) }
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
