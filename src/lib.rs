#![no_std]

pub mod furi;
pub mod gui;
pub mod nicla_sense_env;
pub mod storage;

#[macro_export]
macro_rules! printf {
    ($fmt:expr) => {
        ::flipperzero_sys::__wrap_printf((($fmt) as &CStr).as_ptr());
    };
    ($fmt:expr, $($arg:expr),+) => {
        ::flipperzero_sys::__wrap_printf((($fmt) as &CStr).as_ptr(), $($arg),+);
    }
}

#[macro_export]
macro_rules! sprintf {
    ($fmt:expr) => {
        ::flipperzero::furi::string::FuriString::from($fmt)
    };
    ($fmt:expr, $($arg:expr),+) => {
        {
            let mut s = ::flipperzero::furi::string::FuriString::new();
            ::flipperzero_sys::furi_string_printf(s.as_mut_ptr(), ($fmt as &CStr).as_ptr(), $($arg),+);

            s
        }
    }
}
