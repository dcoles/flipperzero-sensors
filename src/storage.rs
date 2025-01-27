use core::ffi::CStr;

use flipperzero_sys as sys;

use crate::furi::pubsub::PubSub;
use crate::furi::record::{Record, RawRecord};

pub type Storage = sys::Storage;

unsafe impl RawRecord for Storage {
    const NAME: &CStr = c"storage";
}

impl Record<Storage> {
    pub fn pubsub(&self) -> &PubSub<StorageEvent> {
        unsafe { PubSub::from_raw(sys::storage_get_pubsub(self.as_ptr())) }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct StorageEvent {
    pub type_: StorageEventType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub enum StorageEventType {
    StorageEventTypeCardMount = 0, // SD card was mounted.
    StorageEventTypeCardUnmount, // SD card was unmounted.
    StorageEventTypeCardMountError, // An error occurred during mounting of an SD card.
    StorageEventTypeFileClose, // A file was closed.
    StorageEventTypeDirClose, // A directory was closed.
}
