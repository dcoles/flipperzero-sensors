use core::ffi::CStr;
use core::ptr;

use flipperzero_sys as sys;

/// A kind of record that can be opened.
///
/// # Safety
///
/// Implementing type must be a C-compatible structure that can be zeroed.
pub unsafe trait RawRecord {
    const NAME: &CStr;
}

/// Reference to a Record.
///
/// This prevents the record being destroyed until all instances are dropped.
#[repr(transparent)]
pub struct Record<T: RawRecord> {
    raw: ptr::NonNull<T>,
}

impl<T: RawRecord> Record<T> {
    /// Open record handle.
    ///
    /// This function will block if the associated record is not yet ready.
    pub fn open() -> Self {
        Self {
            // SAFETY: `furi_record_open` blocks until the record is initialized with a valid value.
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_record_open(T::NAME.as_ptr()).cast()) },
        }
    }

    /// Name associated with this record.
    pub fn name() -> &'static CStr {
        T::NAME
    }

    /// Returns the record data as a raw pointer.
    pub const fn as_ptr(&self) -> *mut T {
        self.raw.as_ptr()
    }

    /// Extract record.
    pub const fn as_record(&self) -> &Record<T> {
        unsafe { &*(self as *const Self as *const Record<T>) }
    }
}

impl<T: RawRecord> Clone for Record<T> {
    fn clone(&self) -> Self {
        // Just open a new record matching this one.
        Self::open()
    }
}

impl<T: RawRecord> Drop for Record<T> {
    fn drop(&mut self) {
        unsafe {
            // decrement the holders count
            sys::furi_record_close(T::NAME.as_ptr());
        }
    }
}
