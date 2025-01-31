//! Flipper Zero App demoing use of PubSub API.

#![no_main]
#![no_std]

use core::ffi::{c_int, CStr};
use core::time::Duration;
use core::pin::pin;

use flipperzero::{furi::thread::sleep, println};
use flipperzero_rt::{entry, manifest};
use flipperzero_sys::{self as sys, Storage};

use shared::furi::record::Record;

manifest!(
    name = "Storage PubSub",
    app_version = 1,
    has_icon = true,
    // See https://github.com/flipperzero-rs/flipperzero/blob/v0.7.2/docs/icons.md for icon format
    icon = "../rustacean-10x10.icon",
);

entry!(main);
fn main(_args: Option<&CStr>) -> i32 {
    // Storage Setup
    let storage = Record::<Storage>::open();
    let callback = |event: &sys::StorageEvent| {
        println!("StorageEvent: {:?}", event.type_.0 as c_int);
    };
    let callback = pin!(callback);
    let _subscription = storage.pubsub().subscribe(callback);

    loop {
        sleep(Duration::from_secs(1));
    }

    0
}
