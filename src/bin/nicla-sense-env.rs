//! Flipper Zero App for
//! [Adruino Nicla Sense Env](https://docs.arduino.cc/hardware/nicla-sense-env/)

#![no_main]
#![no_std]

use core::ffi::{c_double, c_int, c_void, CStr};
use core::mem;
use core::pin::pin;
use core::sync::atomic::{AtomicU32, Ordering};
use flipperzero::furi::sync::Mutex;
use flipperzero::gpio::i2c;
use flipperzero::{format, println};
use flipperzero::furi::time::Duration as FuriDuration;
use flipperzero_rt::{entry, manifest};

use flipperzero_sys as sys;

use shared::furi::hal::power::Power;
use shared::furi::record::Record;
use shared::gui::{Gui, ViewDispatcher, ViewId, View};
use shared::nicla_sense_env::{NiclaSenseEnv, IndoorSensorMode, OutdoorSensorMode};
use shared::storage::{Storage, StorageEvent};

static SAMPLE_COUNT: AtomicU32 = AtomicU32::new(0);
static VALUES: Mutex<Measurement> = Mutex::new(Measurement::new());

struct Measurement {
    temperature: f32,
    humidity: f32,
    epa_eqa: u16,
    fast_eqa: u16,
    o3: f32,
    no2: f32,
    eco2: f32,
    tvoc: f32,
    c2h6o: f32,
    relative_iaq: f32,
    current: f32,
    battery_percentage: f32,
}

impl Measurement {
    pub const fn new() -> Self {
        Measurement {
            temperature: 0.0,
            humidity: 0.0,
            epa_eqa: 0,
            fast_eqa: 0,
            o3: 0.0,
            no2: 0.0,
            eco2: 0.0,
            tvoc: 0.0,
            c2h6o: 0.0,
            relative_iaq: 0.0,
            current: 0.0,
            battery_percentage: 0.0,
        }
    }
}

manifest!(
    name = "Nicla Sense Env",
    app_version = 1,
    has_icon = true,
    // See https://github.com/flipperzero-rs/flipperzero/blob/v0.7.2/docs/icons.md for icon format
    icon = "../rustacean-10x10.icon",
);

// Define the entry function
entry!(main);

macro_rules! printf {
    ($fmt:expr) => {
        ::flipperzero_sys::__wrap_printf((($fmt) as &CStr).as_ptr());
    };
    ($fmt:expr, $($arg:expr),+) => {
        ::flipperzero_sys::__wrap_printf((($fmt) as &CStr).as_ptr(), $($arg),+);
    }
}

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

/// View draw handler.
/// Screen is 128x64 px
unsafe extern "C" fn draw_callback(canvas: *mut sys::Canvas, _context: *mut c_void) {
    let values = VALUES.lock();

    let lines = [
        format!("ARDUINO Nikla Sense ENV"),
        sprintf!(c"%0.1f degC, Humid: %0.1f%%", values.temperature as c_double, values.humidity as c_double),
        sprintf!(c"O3: %0.0f ppb, NO2: %0.0f ppb", values.o3 as c_double, values.no2 as c_double),
        sprintf!(c"eCO2: %0.0f ppm, TVOC: %0.0f", values.eco2 as c_double, values.tvoc as c_double),
        sprintf!(c"IAQ: %0.0f%%, C2H6O: %0.0f", values.relative_iaq as c_double, values.c2h6o as c_double),
        sprintf!(c"draw: %0.0f mA, battery: %0.0f%%", (-values.current * 1000.0) as c_double, values.battery_percentage as c_double),
    ];

    sys::canvas_set_font(canvas, sys::FontSecondary);
    for (n, line) in lines.iter().enumerate() {
        sys::canvas_draw_str(canvas, 0, (n + 1) as i32 * 9, line.as_c_str().as_ptr());
    }

    let samples_count = SAMPLE_COUNT.load(Ordering::Acquire);
    let spinner = match samples_count % 4 {
        0 => c"|",
        1 => c"/",
        2 => c"-",
        3 => c"\\",
        _ => unreachable!(),
    };

    sys::canvas_draw_str_aligned(
        canvas,
        122,
        10,
        sys::AlignCenter,
        sys::AlignBottom,
        spinner.as_ptr(),
    );
}

struct MainView<'a> {
    power: *mut sys::Power,
    device: *mut NiclaSenseEnv<'a>,
}

unsafe extern "C" fn tick_callback(ctx: *mut c_void) {
    let context: &mut MainView = &mut *(ctx.cast());
    let device = &mut *context.device;

    let mut power_info = mem::zeroed();
    sys::power_get_info(context.power, &raw mut power_info);
    let battery_percentage = 100.0 * (power_info.capacity_remaining as f32 / power_info.capacity_full as f32);

    let measurement =
    if device.is_ready() {
        Measurement {
            temperature: device.temperature(),
            humidity: device.humidity(),
            epa_eqa: device.outdoor_epa_aqi(),
            fast_eqa: device.outdoor_fast_aqi(),
            o3: device.outdoor_o3(),
            no2: device.outdoor_no2(),
            eco2: device.indoor_estimated_co2(),
            tvoc: device.indoor_total_voc(),
            c2h6o: device.indoor_ethanol(),
            relative_iaq: device.indoor_relative_iqa(),
            current: power_info.current_gauge,
            battery_percentage,
        }
    } else {
        Measurement {
            temperature: 0.0,
            humidity: 0.0,
            epa_eqa: 0,
            fast_eqa: 0,
            o3: 0.0,
            no2: 0.0,
            eco2: 0.0,
            tvoc: 0.0,
            c2h6o: 0.0,
            relative_iaq: 0.0,
            current: power_info.current_gauge,
            battery_percentage,
        }
    };

    *VALUES.lock() = measurement;
    SAMPLE_COUNT.fetch_add(1, Ordering::AcqRel);
}

unsafe extern "C" fn back(ctx: *mut c_void) -> u32 {
    sys::view_dispatcher_stop(ctx.cast());

    0
}

// Entry point
fn main(_args: Option<&CStr>) -> i32 {
    let mut bus = i2c::Bus::EXTERNAL.acquire();
    let mut device = NiclaSenseEnv::with_default_addr(&mut bus);

    // Storage Setup
    let storage = Record::<Storage>::open();
    let callback = |event: &StorageEvent| {
        println!("StorageEvent: {:?}", event.type_ as c_int);
    };
    let callback = pin!(callback);
    let _subscription = storage.pubsub().subscribe(callback);

    // Power Setup
    let power= Record::<Power>::open();

    let mut context = MainView {
        power: power.as_ptr(),
        device: &raw mut device,
    };

    // GUI Setup
    let gui = Record::<Gui>::open();

    let view_dispatcher = ViewDispatcher::new();
    unsafe {
        view_dispatcher.set_event_callback_context(&raw mut context as *mut _);
        view_dispatcher.set_tick_event_callback(Some(tick_callback), FuriDuration::from_millis(500));
    }
    view_dispatcher.attach_to_gui(&gui, sys::ViewDispatcherTypeFullscreen);

    const MAIN_VIEW: ViewId = ViewId(0);
    let view = View::new();
    unsafe {
        view.set_context(view_dispatcher.as_ptr().cast());
        view.set_draw_callback(Some(draw_callback));
        view.set_previous_callback(Some(back));
    }

    view_dispatcher.add_view(MAIN_VIEW, &view);
    view_dispatcher.switch_to_view(MAIN_VIEW);

    println!("Setting outdoor sensor mode");
    if !device.set_outdoor_sensor_mode(OutdoorSensorMode::OutdoorAirQuality) {
        println!("failed to set outdoor sensor mode");
    }

    println!("Setting indoor sensor mode");
    if !device.set_indoor_sensor_mode(IndoorSensorMode::IndoorAirQuality) {
        println!("failed to set indoor sensor mode");
    }

    device.set_orange_led(1 << 7); // Enable sensor error warning
    device.set_rgb_intensity(0);

    view_dispatcher.run();

    view_dispatcher.remove_view(MAIN_VIEW);

    0
}
