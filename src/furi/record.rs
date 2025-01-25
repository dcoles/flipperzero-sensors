use core::borrow::Borrow;
use core::ffi::CStr;
use core::ops::Deref;
use core::ptr;

use flipperzero_sys as sys;


/// A kind of record that can be opened.
///
/// # Safety
///
/// `CType` must be a C-compatible structure that can be zeroed.
pub unsafe trait RecordType {
    const NAME: &CStr;
    type CType;
}

#[repr(transparent)]
pub struct Record<T: RecordType> {
    raw: ptr::NonNull<T::CType>,
}

impl<T: RecordType> Record<T> {
    /// Name associated with this record.
    pub fn name() -> &'static CStr {
        T::NAME
    }

    /// Returns the record data as a raw pointer.
    pub const fn as_ptr(&self) -> *mut T::CType {
        self.raw.as_ptr()
    }
}

/// Low-level wrapper of a record handle.
#[repr(transparent)]
pub struct RecordHandle<T: RecordType> {
    raw: ptr::NonNull<T::CType>,
}

impl<T: RecordType> RecordHandle<T> {
    /// Open record handle.
    ///
    /// This function will block if the associated record is not yet ready.
    pub fn open() -> Self {
        // SAFETY: `furi_record_open` blocks until the record is initialized with a valid value.

        Self {
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_record_open(T::NAME.as_ptr()).cast()) },
        }
    }

    /// Extract record.
    pub const fn as_record(&self) -> &Record<T> {
        unsafe { &*(self as *const Self as *const Record<T>) }
    }
}

impl<T: RecordType> AsRef<Record<T>> for RecordHandle<T> {
    fn as_ref(&self) -> &Record<T> {
        self.as_record()
    }
}

impl<T: RecordType> Borrow<Record<T>> for RecordHandle<T> {
    fn borrow(&self) -> &Record<T> {
        self.as_record()
    }
}

impl<T: RecordType> Deref for RecordHandle<T> {
    type Target = Record<T>;

    fn deref(&self) -> &Self::Target {
        self.as_record()
    }
}

impl<T: RecordType> Drop for RecordHandle<T> {
    fn drop(&mut self) {
        unsafe {
            // decrement the holders count
            sys::furi_record_close(T::NAME.as_ptr());
        }
    }
}
