use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::marker::{PhantomData, PhantomPinned};
use core::pin::Pin;
use core::ptr;

use flipperzero_sys as sys;

use super::record::{RawRecord, Record};

pub unsafe trait RawPubSub {
    type Event;

    unsafe fn get(this: *mut Self) -> *mut sys::FuriPubSub;
}


#[repr(transparent)]
pub struct PubSub<T> {
    raw: UnsafeCell<sys::FuriPubSub>,
    _marker: PhantomData<(T, PhantomPinned)>
}

impl<T> PubSub<T> {
    /// Create a `PubSub` from raw [`sys::FuriPubSub`] pointer.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null pointer to a valid `FuriPubSub` and must not be
    /// freed or invalidated while the returned handle is in scope.
    pub unsafe fn from_raw<'a>(raw: *mut sys::FuriPubSub) -> &'a Self {
        unsafe { &*(raw.cast()) }
    }

    /// Get raw `FuriPubSub` pointer.
    ///
    /// This pointer must not be `free`'d or referenced after this struct is dropped.
    pub fn as_ptr(&self) -> *mut sys::FuriPubSub {
        self.raw.get()
    }

    /// Subscribe to PubSub
    //pub fn subscribe<C: Callback<T>>(&self, mut callback: Pin<&mut C>) -> Subscription<'_, T> {
    pub fn subscribe<C: Fn(&T)>(&self, callback: Pin<&mut C>) -> Subscription<'_, T> {
        Subscription::new(self, callback)
    }
}

impl<R: RawRecord + RawPubSub> Record<R> {
    /// Get PubSub handle.
    pub fn pubsub(&self) -> &'_ PubSub<R::Event> {
        unsafe { PubSub::from_raw(RawPubSub::get(self.as_ptr())) }
    }
}

/// PubSub subscription.
///
/// The subscription remains active until this type is dropped.
pub struct Subscription<'a, T> {
    pubsub: &'a PubSub<T>,
    raw: ptr::NonNull<sys::FuriPubSubSubscription>,
}

impl<'a, T> Subscription<'a, T> {
    fn new<C: Fn(&T)>(pubsub: &'a PubSub<T>, mut callback: Pin<&mut C>) -> Self {
        let context = unsafe { ptr::from_mut(Pin::into_inner_unchecked(callback.as_mut())) };

        Self {
            pubsub,
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_pubsub_subscribe(pubsub.as_ptr(), Some(pubsub_callback::<T, C>), context.cast())) }
        }
    }
}

impl<T> Drop for Subscription<'_, T> {
    fn drop(&mut self) {
        unsafe {
            sys::furi_pubsub_unsubscribe(self.pubsub.as_ptr(), self.raw.as_ptr());
        }
    }
}

/// Callback.
pub trait Callback<T> {
    fn on_callback(&self, data: &T);
}

impl<T, F: Fn(&T)> Callback<T> for F {
    fn on_callback(&self, data: &T) {
        (self)(data)
    }
}

extern "C" fn pubsub_callback<T, C: Fn(&T)>(message: *const c_void, context: *mut c_void) {
    let message: *const T = message.cast();
    let callback: *mut C = context.cast();

    unsafe { (*callback).on_callback(&*message) };
}
