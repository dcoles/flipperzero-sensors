use core::borrow::Borrow;
use core::ffi::c_void;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::{self, NonNull};

use flipperzero_sys as sys;

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

#[repr(transparent)]
pub struct PubSub<T> {
    raw: NonNull<sys::FuriPubSub>,
    phantom: PhantomData<T>,
}

impl<T> PubSub<T> {
    /// Create a `PubSub` from raw [`sys::FuriPubSub`] pointer.
    ///
    /// # Safety
    ///
    /// `raw` must be a non-null pointer to a valid `FuriPubSub` and must not be
    /// freed or invalidated while the returned handle is in scope.
    pub unsafe fn from_raw(raw: *mut sys::FuriPubSub) -> PubSub<T> {
        PubSub {
            raw: ptr::NonNull::new_unchecked(raw),
            phantom: PhantomData,
        }
    }

    /// Get raw `FuriPubSub` pointer.
    ///
    /// This pointer must not be `free`'d or referenced after this struct is dropped.
    pub fn as_ptr(&self) -> *mut sys::FuriPubSub {
        self.raw.as_ptr()
    }

    /// Subscribe to PubSub
    pub fn subscribe<C: Callback<T>>(&self, mut callback: Pin<&mut C>) -> Subscription<'_, T> {
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


#[repr(transparent)]
pub struct OwnedPubSub<T> {
    raw: NonNull<sys::FuriPubSub>,
    phantom: PhantomData<T>,
}

impl<T> OwnedPubSub<T> {
    pub fn alloc() -> Self {
        // SAFETY: `furi_pubsub_alloc` never fails, throwing a `furi_check` on OOM

        Self {
            raw: unsafe { ptr::NonNull::new_unchecked(sys::furi_pubsub_alloc().cast()) },
            phantom: PhantomData,
        }
    }

    pub fn as_pubsub(&self) -> &PubSub<T> {
        unsafe { &*(self as *const Self as *const PubSub<T>) }
    }
}

impl<T> Drop for OwnedPubSub<T> {
    fn drop(&mut self) {
        unsafe { sys::furi_pubsub_free(self.raw.as_ptr()) }
    }
}

impl<T> AsRef<PubSub<T>> for OwnedPubSub<T> {
    fn as_ref(&self) -> &PubSub<T> {
        self.as_pubsub()
    }
}

impl<T> Borrow<PubSub<T>> for OwnedPubSub<T> {
    fn borrow(&self) -> &PubSub<T> {
        self.as_pubsub()
    }
}

pub trait Callback<T> {
    fn on_callback(&self, data: &T);
}

impl<T, F: Fn(&T)> Callback<T> for F {
    fn on_callback(&self, data: &T) {
        (self)(data)
    }
}

extern "C" fn pubsub_callback<T, C: Callback<T>>(message: *const c_void, context: *mut c_void) {
    let message: *const T = message.cast();
    let callback: *mut C = context.cast();

    unsafe { (*callback).on_callback(&*message) };
}
