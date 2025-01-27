mod view_dispatcher;
mod view;
mod view_port;

use core::convert::Infallible;
use core::ffi::c_void;
use core::ptr;

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics_core::prelude::{Dimensions, DrawTarget, Point};
use embedded_graphics_core::primitives::Rectangle;
use embedded_graphics_core::Pixel;
use flipperzero_sys as sys;

use crate::furi::record::{Record, RawRecord};

pub use view_port::ViewPort;
pub use view_dispatcher::{ViewDispatcher, ViewId};
pub use view::View;

pub type Gui = sys::Gui;

unsafe impl RawRecord for Gui {
    const NAME: &core::ffi::CStr = c"gui";
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
    pub unsafe fn direct_draw_aquire(&self) -> Canvas {
        Canvas {
            raw: unsafe { ptr::NonNull::new_unchecked(sys::gui_direct_draw_acquire(self.as_ptr())) }
        }
    }

    /// Release Direct Draw Lock
    ///
    /// Release Direct Draw Lock, enables Input and Draw call processing. Canvas
    /// acquired in `direct_draw_acquire` will become invalid after this call.
    pub unsafe fn direct_draw_release(&self) {
        unsafe { sys::gui_direct_draw_release(self.as_ptr()) }
    }


}

pub struct Canvas {
    raw: ptr::NonNull<sys::Canvas>,
}

impl Canvas {
    pub fn as_ptr(&self) -> *mut sys::Canvas {
        self.raw.as_ptr()
    }

    pub fn get_size(&self) -> (usize, usize) {
        unsafe { (sys::canvas_width(self.as_ptr()), sys::canvas_height(self.as_ptr())) }
    }

    pub fn clear(&self) {
        unsafe { sys::canvas_clear(self.as_ptr()) }
    }

    pub fn commit(&self) {
        unsafe { sys::canvas_commit(self.as_ptr()) }
    }
}

impl Dimensions for Canvas {
    fn bounding_box(&self) -> Rectangle {
        let (width, height) = self.get_size();

        Rectangle {
            top_left: (0, 0).into(),
            size: (width as u32, height as u32).into(),
        }
    }
}

impl DrawTarget for Canvas {
    type Color = BinaryColor;

    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let (width, height) = self.get_size();
        let (width, height) = (width as i32, height as i32);

        unsafe {
            for Pixel(Point { x, y }, color) in pixels.into_iter() {
                if (0..=width).contains(&x) && (0..=height).contains(&y) {
                    sys::canvas_set_color(self.as_ptr(), map_color(color));
                    sys::canvas_draw_dot(self.as_ptr(), x, y);
                }
            }
        }

        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        // Clamp rectangle coordinates to visible display area
        let area = area.intersection(&self.bounding_box());

        // Do not draw if the intersection size is zero.
        if area.bottom_right().is_none() {
            return Ok(());
        }

        unsafe {
            sys::canvas_set_color(self.as_ptr(), map_color(color));
            sys::canvas_draw_box(self.as_ptr(), area.top_left.x, area.top_left.y, area.size.width as usize, area.size.height as usize);
        }

        Ok(())
    }
}

fn map_color(color: BinaryColor) -> sys::Color {
    if color.is_on() {
        sys::ColorBlack
    } else {
        sys::ColorWhite
    }
}
