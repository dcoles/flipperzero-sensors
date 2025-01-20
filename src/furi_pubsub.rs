use core::ffi::c_void;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::pin::Pin;
use core::ptr::{self, NonNull};

use flipperzero_sys as sys;

pub struct Subscription {
    raw: ptr::NonNull<sys::FuriPubSubSubscription>,
}

#[repr(transparent)]
pub struct OwnedPubSub<'a, T> {
    inner: PubSub<'a, T>,
}

impl<T> OwnedPubSub<'_, T> {
    pub fn alloc() -> Self {
        // SAFETY: `furi_pubsub_alloc` never fails, throwing a `furi_check` on OOM

        Self {
            inner: unsafe { PubSub::from_raw(ptr::NonNull::new_unchecked(sys::furi_pubsub_alloc().cast())) }
        }
    }
}

impl<T> Drop for OwnedPubSub<'_, T> {
    fn drop(&mut self) {
        unsafe {
            sys::furi_pubsub_free(self.inner.as_ptr() );
        }
    }
}

impl<'a, T> Deref for OwnedPubSub<'a, T> {
    type Target = PubSub<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for OwnedPubSub<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[repr(transparent)]
pub struct PubSub<'a, T> {
    raw: NonNull<sys::FuriPubSub>,
    phantom: PhantomData<(&'a sys::FuriPubSub, T)>,
}

impl<'a, T> PubSub<'a, T> {
    /// Create a `PubSub` from raw [`sys::FuriPubSub`] pointer.
    ///
    /// # Safety
    /// 
    /// `raw` must be a pointer to a valid `FuriPubSub` and must not be
    /// invalidated while the returned struct is still in-scope.
    pub unsafe fn from_raw(raw: NonNull<sys::FuriPubSub>) -> PubSub<'a, T> {
        PubSub {
            raw,
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
    pub fn subscribe<F: FnMut(&T)>(&self, mut callback: Pin<&mut Callback<T, F>>) -> Subscription {
        let ptr = unsafe { Pin::into_inner_unchecked(callback.as_mut()) } as *mut Callback<T, F>;
        let subscription = unsafe {
            ptr::NonNull::new_unchecked(sys::furi_pubsub_subscribe(self.as_ptr(), Some(pubsub_callback::<T, F>), ptr.cast()))
        };
        
        Subscription {
            raw: subscription,
        }
    }

    pub fn unsubscribe(&self, subscription: Subscription) {
        unsafe {
            sys::furi_pubsub_unsubscribe(self.as_ptr(), subscription.raw.as_ptr());
        }
    }
}


pub struct Callback<T, F: FnMut(&T)> {
    callback: F,
    phantom: PhantomData<T>,
}

impl<T, F: FnMut(&T)> Callback<T, F> {
    pub fn new(callback: F) -> Self {
        Self {
            callback,
            phantom: PhantomData,
        }
    }
}

extern "C" fn pubsub_callback<T, F: FnMut(&T)>(message: *const c_void, context: *mut c_void) {
    let message: *const T = message.cast();
    let callback: *mut Callback<T, F> = context.cast();

    unsafe { ((*callback).callback)(&*message) };
}
