use flipperzero_sys as sys;

use crate::furi_record::{Record, RecordType};

pub struct Gui;

unsafe impl RecordType for Gui {
    const NAME: &core::ffi::CStr = c"gui";
    type CType = sys::Gui;
}

impl Record<Gui> {
    pub fn set_lockdown(&self, lockdown: bool) {
        unsafe { sys::gui_set_lockdown(self.as_ptr(), lockdown) }
    }
}