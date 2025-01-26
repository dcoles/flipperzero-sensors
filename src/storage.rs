use core::ffi::CStr;
use core::ptr;

use flipperzero_sys as sys;

use crate::furi::pubsub::PubSub;
use crate::furi::record::{Record, RecordType};


#[repr(transparent)]
pub struct Storage;

unsafe impl RecordType for Storage {
    const NAME: &CStr = c"storage";
    type CType = sys::Storage;
}

impl Record<Storage> {
    pub fn get_pubsub(&self) -> PubSub<StorageEvent> {
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
