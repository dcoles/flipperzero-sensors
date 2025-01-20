use core::ffi::CStr;
use core::ptr::NonNull;

use flipperzero_sys as sys;


pub trait RecordType {
    const NAME: &CStr;
    type CType;
}

/// Low-level wrapper of a record handle.
#[repr(transparent)]
pub struct Record<T: RecordType> {
    raw: NonNull<T::CType>,
}

impl<T: RecordType> Record<T> {
    /// Opens a record.
    pub fn open() -> Self {
        Self {
            raw: unsafe { NonNull::new_unchecked(sys::furi_record_open(T::NAME.as_ptr()).cast()) }
        }
    }

    /// Returns the record data as a raw pointer.
    pub fn as_ptr(&self) -> *mut T::CType {
        self.raw.as_ptr()
    }
}

impl<T: RecordType> Drop for Record<T> {
    fn drop(&mut self) {
        unsafe {
            // decrement the holders count
            sys::furi_record_close(T::NAME.as_ptr());
        }
    }
}