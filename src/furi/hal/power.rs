use core::ffi::CStr;
use core::mem;

use flipperzero_sys as sys;

use crate::furi::pubsub::RawPubSub;
use crate::furi::record::{Record, RawRecord};

pub type Power = sys::Power;

unsafe impl RawRecord for Power {
    const NAME: &CStr = c"power";
}

unsafe impl RawPubSub for Power {
    type Event = sys::PowerEvent;

    unsafe fn get(this: *mut Self) -> *mut sys::FuriPubSub {
        unsafe { sys::power_get_pubsub(this) }
    }
}

impl Record<Power> {
    /// Power off device.
    pub fn power_off(&self) {
        unsafe {
            sys::power_off(self.as_ptr())
        }
    }

    /// Reboot device.
    pub fn reboot(&self, mode: sys::PowerBootMode) {
        unsafe {
            sys::power_reboot(self.as_ptr(), mode)
        }
    }

    /// Get power info.
    pub fn get_info(&self) -> sys::PowerInfo {
        unsafe {
            let mut power_info = mem::zeroed();
            sys::power_get_info(self.as_ptr(), &raw mut power_info);

            power_info
        }
    }

    /// Check battery health.
    pub fn is_battery_healthy(&self) -> bool {
        unsafe {
            sys::power_is_battery_healthy(self.as_ptr())
        }
    }

    /// Enable or disable battery low level notification message.
    pub fn enable_low_battery_level_notification(&self, enable: bool) {
        unsafe {
            sys::power_enable_low_battery_level_notification(self.as_ptr(), enable);
        }
    }
}

#[repr(C)]
pub struct PowerEvent {
    pub type_: PowerEventType,
    pub data: PowerEventData,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum PowerEventType {
    StopCharging,
    StartCharging,
    FullyCharged,
    BatteryLevelChanged,
}

#[repr(C)]
pub union PowerEventData {
    pub battery_level: u8,
}
