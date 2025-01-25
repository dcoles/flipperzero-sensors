mod view_dispatcher;
mod view;
mod view_port;

use core::ffi::c_void;

use flipperzero_sys as sys;

use crate::furi::record::{Record, RecordType};

pub use view_port::ViewPort;
pub use view_dispatcher::{ViewDispatcher, ViewId};
pub use view::View;

pub struct Gui;

unsafe impl RecordType for Gui {
    const NAME: &core::ffi::CStr = c"gui";
    type CType = sys::Gui;
}

impl Record<Gui> {
    /// Add `view_port` to view_port tree.
    pub fn add_view_port(&self, view_port: &ViewPort, layer: sys::GuiLayer) {
        unsafe { sys::gui_add_view_port(self.as_ptr(), view_port.as_ptr(), layer) }
    }

    /// Remove `view_port` to view_port tree.
    pub fn remove_view_port(&self, view_port: &ViewPort) {
        unsafe { sys::gui_remove_view_port(self.as_ptr(), view_port.as_ptr()) }
    }

    // TODO: Move this onto `ViewPort` type?
    /// Send ViewPort to the front.
    ///
    /// Places selected ViewPort to the top of the drawing stack.
    pub fn view_port_send_to_front(&self, view_port: &ViewPort) {
        unsafe { sys::gui_view_port_send_to_front(self.as_ptr(), view_port.as_ptr()) }
    }

    // TODO: Add support for `gui_view_port_send_to_back`

    /// Add gui canvas commit callback
    ///
    /// This callback will be called upon Canvas commit Callback dispatched from GUI
    /// thread and is time critical
    pub unsafe fn add_framebuffer_callback(&self, callback: sys::GuiCanvasCommitCallback, context: *mut c_void) {
        unsafe { sys::gui_add_framebuffer_callback(self.as_ptr(), callback, context) }
    }

    /// Remove gui canvas commit callback
    pub unsafe fn remove_framebuffer_callback(&self, callback: sys::GuiCanvasCommitCallback, context: *mut c_void) {
        unsafe { sys::gui_remove_framebuffer_callback(self.as_ptr(), callback, context) }
    }

    /// Get gui canvas frame buffer size in bytes.
    pub fn get_framebuffer_size(&self) -> usize {
        unsafe { sys::gui_get_framebuffer_size(self.as_ptr()) }
    }

    /// When lockdown mode is enabled, only GuiLayerDesktop is shown.
    /// This feature prevents services from showing sensitive information when flipper is locked.
    pub fn set_lockdown(&self, lockdown: bool) {
        unsafe { sys::gui_set_lockdown(self.as_ptr(), lockdown) }
    }

    /// Acquire Direct Draw lock and get Canvas instance
    ///
    /// This method return Canvas instance for use in monopoly mode. Direct draw lock
    /// disables input and draw call dispatch functions in GUI service. No other
    /// applications or services will be able to draw until `direct_draw_release`
    /// call.
    pub unsafe fn direct_draw_aquire(&self) -> *mut sys::Canvas {
        unsafe { sys::gui_direct_draw_acquire(self.as_ptr()) }
    }

    /// Release Direct Draw Lock
    ///
    /// Release Direct Draw Lock, enables Input and Draw call processing. Canvas
    /// acquired in `direct_draw_acquire` will become invalid after this call.
    pub unsafe fn direct_draw_release(&self) {
        unsafe { sys::gui_direct_draw_release(self.as_ptr()) }
    }
}
