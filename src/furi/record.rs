use core::cell::UnsafeCell;
use core::ffi::CStr;
use core::marker::PhantomData;
use core::ops::Deref;
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
    raw: UnsafeCell<T>,
}

impl<T: RawRecord> Record<T> {
    /// Name associated with this record.
    pub fn name() -> &'static CStr {
        T::NAME
    }

    pub unsafe fn from_raw<'a>(raw: *mut T) -> &'a Self {
        unsafe { &*(raw.cast()) }
    }

    /// Open record handle.
    ///
    /// This function will block if the associated record is not yet ready.
    pub fn open() -> OpenRecord<T> {
        OpenRecord::new()
    }

    /// Returns the record data as a raw pointer.
    pub const fn as_ptr(&self) -> *mut T {
        self.raw.get()
    }
}

pub struct OpenRecord<T: RawRecord> {
    raw: ptr::NonNull<T>,
    _marker: PhantomData<T>,
}

impl<T: RawRecord> OpenRecord<T> {
    /// Open record handle.
    ///
    /// This function will block if the associated record is not yet ready.
    fn new() -> Self {
        Self {
            // SAFETY: `furi_record_open` blocks until the record is initialized with a valid value.
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_record_open(T::NAME.as_ptr()).cast()) },
            _marker: PhantomData,
        }
    }

    /// Extract record.
    pub const fn as_record(&self) -> &'_ Record<T> {
        unsafe { &*(self.raw.as_ptr().cast()) }
    }
}

impl<T: RawRecord> Clone for OpenRecord<T> {
    fn clone(&self) -> Self {
        // Just open a new record matching this one.
        Self::new()
    }
}

impl <T: RawRecord> Deref for OpenRecord<T> {
    type Target = Record<T>;

    fn deref(&self) -> &Self::Target {
        self.as_record()
    }
}

impl<T: RawRecord> Drop for OpenRecord<T> {
    fn drop(&mut self) {
        unsafe {
            // decrement the holders count
            sys::furi_record_close(T::NAME.as_ptr());
        }
    }
}
