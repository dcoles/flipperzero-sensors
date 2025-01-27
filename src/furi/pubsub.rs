use core::ffi::c_void;
use core::marker::PhantomData;
use core::pin::Pin;
use core::ptr::{self, NonNull};

use flipperzero_sys as sys;

/// A PubSub handle.
#[repr(transparent)]
pub struct PubSub<T> {
    raw: NonNull<sys::FuriPubSub>,
    phantom: PhantomData<T>,
}

impl<T> PubSub<T> {
    pub fn alloc() -> Self {
        // SAFETY: `furi_pubsub_alloc` never fails, throwing a `furi_check` on OOM

        Self {
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_pubsub_alloc().cast()) },
            phantom: PhantomData,
        }
    }

    /// Create a `PubSub` from raw [`sys::FuriPubSub`] pointer.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null pointer to a valid `FuriPubSub` and must not be
    /// freed or invalidated while the returned handle is in scope.
    pub unsafe fn from_raw<'a>(raw: *mut sys::FuriPubSub) -> &'a PubSub<T> {
        unsafe { &*(&ptr::NonNull::new_unchecked(raw) as *const ptr::NonNull<sys::FuriPubSub> as *const PubSub<T>) }
    }

    /// Get raw `FuriPubSub` pointer.
    ///
    /// This pointer must not be `free`'d or referenced after this struct is dropped.
    pub fn as_ptr(&self) -> *mut sys::FuriPubSub {
        self.raw.as_ptr()
    }

    /// Subscribe to PubSub
    //pub fn subscribe<C: Callback<T>>(&self, mut callback: Pin<&mut C>) -> Subscription<'_, T> {
    pub fn subscribe<C: Fn(&T)>(&self, mut callback: Pin<&mut C>) -> Subscription<'_, T> {
        let ptr = unsafe { Pin::into_inner_unchecked(callback.as_mut()) } as *mut C;
        let subscription = unsafe {
            ptr::NonNull::new_unchecked(sys::furi_pubsub_subscribe(self.as_ptr(), Some(pubsub_callback::<T, C>), ptr.cast()))
        };

        Subscription {
            pubsub: self,
            raw: subscription,
        }
    }
}

impl<T> Drop for PubSub<T> {
    fn drop(&mut self) {
        unsafe { sys::furi_pubsub_free(self.raw.as_ptr()) }
    }
}

/// PubSub subscription.
///
/// The subscription remains active until this type is dropped.
pub struct Subscription<'a, T> {
    pubsub: &'a PubSub<T>,
    raw: ptr::NonNull<sys::FuriPubSubSubscription>,
}

impl<'a, T> Drop for Subscription<'a, T> {
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
