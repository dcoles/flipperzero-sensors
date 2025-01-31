use core::ffi::CStr;

use flipperzero_sys as sys;

use crate::furi::pubsub::RawPubSub;
use crate::furi::record::RawRecord;

pub type Storage = sys::Storage;

unsafe impl RawRecord for Storage {
    const NAME: &CStr = c"storage";
}

unsafe impl RawPubSub for Storage {
    type Event = sys::StorageEvent;

    unsafe fn get(this: *mut Self) -> *mut sys::FuriPubSub {
        unsafe { sys::storage_get_pubsub(this) }
    }
}
