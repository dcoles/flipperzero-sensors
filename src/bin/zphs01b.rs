//! Flipper Zero App for
//! [Winsen ZPHS01B Multi-in-One Sensor Module](https://www.winsen-sensor.com/product/zphs01b.html)

#![no_main]
#![no_std]

// Required for panic handler
extern crate flipperzero_rt;

use core::ffi::{c_void, CStr};
use core::ops::Not;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use core::ptr;

use flipperzero::furi::message_queue::MessageQueue;
use flipperzero::furi::sync::Mutex;
use flipperzero::furi::serial::SerialHandle;
use flipperzero::notification::{NotificationService, NotificationMessage, NotificationSequence};
use flipperzero::{error, format, furi, notification_sequence, println};
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

const POLL_INTERVAL: Duration = Duration::from_millis(500);
const CHANNEL: sys::FuriHalSerialId = sys::FuriHalSerialId_FuriHalSerialIdLpuart;
const BAUD: u32 = 9600;

const CMD_FETCH: [u8; 9] = [0xFF, 0x01, 0x86, 0x00, 0x00, 0x00, 0x00, 0x00, 0x79];
const RESPONSE_SIZE: usize = 26;
const START_BIT: u8 = 0xFF;

// GUI record
const RECORD_GUI: &CStr = c"gui";
const FULLSCREEN: sys::GuiLayer = sys::GuiLayer_GuiLayerFullscreen;

static SAMPLE_COUNT: AtomicU32 = AtomicU32::new(0);
static VALUES: Mutex<Measurement> = Mutex::new(Measurement::new());

// Define the FAP Manifest for this application
manifest!(
    name = "Winsen ZPHS01B Gas Sensor",
    app_version = 1,
    has_icon = true,
    // See https://github.com/flipperzero-rs/flipperzero/blob/v0.7.2/docs/icons.md for icon format
    icon = "../rustacean-10x10.icon",
);

// Define the entry function
entry!(main);

fn read_u16_be(data: &[u8]) -> u16 {
    (data[0] as u16) << 8 | data[1] as u16
}

fn calculate_checksum(data: &[u8]) -> u8 {
    data.into_iter()
        .fold(0u8, |acc, &x| acc.wrapping_add(x))
        .not()
        .wrapping_add(1)
}

macro_rules! printf {
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

    let (pm_1, pm_2_5, pm_10) = values.pm();
    let voc = match values.voc() {
        VOCLevel::Clean => c"clean",
        VOCLevel::Light => c"light",
        VOCLevel::Moderate => c"moderate",
        VOCLevel::Severe => c"severe",
    };
    let lines = [
        format!("Winsen ZPHS01B Gas Sensor"),
        printf!(
            c"PM (1, 2.5, 10): (%u, %u, %u) ugm3",
            pm_1 as u32,
            pm_2_5 as u32,
            pm_10 as u32
        ),
        printf!(c"CO2: %u ppm; VOC: %s", values.co2() as u32, voc),
        printf!(
            c"Temp: %.1f degC; Humid: %u %%",
            values.temperature(),
            values.relative_humidity() as u32
        ),
        printf!(c"CH2O: %.3f mgm3; CO: %.1f ppm", values.ch2o(), values.co()),
        printf!(c"O3: %.2f ppm; NO2: %.2f ppm", values.o3(), values.no2()),
    ];

    sys::canvas_set_font(canvas, sys::Font_FontSecondary);
    for (n, line) in lines.iter().enumerate() {
        sys::canvas_draw_str(canvas, 0, (n + 1) as i32 * 10, line.as_c_str().as_ptr());
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
        sys::Align_AlignCenter,
        sys::Align_AlignBottom,
        spinner.as_ptr(),
    );
}

unsafe extern "C" fn app_input_callback(input_event: *mut sys::InputEvent, ctx: *mut c_void) {
    let event_queue: &MessageQueue<sys::InputEvent> = &*ctx.cast();
    event_queue.put(*input_event, furi::time::Duration::ZERO).unwrap();
}

// Entry point
fn main(_args: Option<&CStr>) -> i32 {
    let mut notification_service = NotificationService::open();
    let event_queue: MessageQueue<sys::InputEvent> = MessageQueue::new(8);

    // GUI Setup
    let view_port;
    let gui;
    unsafe {
        view_port = sys::view_port_alloc();
        sys::view_port_draw_callback_set(view_port, Some(draw_callback), ptr::null_mut());
        sys::view_port_input_callback_set(
            view_port,
            Some(app_input_callback),
            &event_queue as *const MessageQueue<sys::InputEvent> as *mut _,
        );

        gui = sys::furi::UnsafeRecord::<sys::Gui>::open(RECORD_GUI);
        sys::gui_add_view_port(gui.as_ptr(), view_port, FULLSCREEN);
    }

    // UART setup
    let serial_handle = SerialHandle::acquire(CHANNEL).unwrap();
    serial_handle.init(BAUD);

    let mut buffer: heapless::Vec<u8, RESPONSE_SIZE> = heapless::Vec::new();
    let mut serial = serial_handle.async_receiver(move |data| {
        if buffer.is_empty() && data[0] != START_BIT {
            return;
        }

        let to_read = data.len().min(buffer.capacity() - buffer.len());
        buffer.extend_from_slice(&data[..to_read]).unwrap();

        if buffer.len() < RESPONSE_SIZE {
            return;
        }

        let mut values = VALUES.lock();
        let last_aqi = values.air_quality_index();

        match Measurement::try_from(&buffer[..]) {
            Err(_) => {
                buffer.clear();
                return;
            },
            Ok(v) => {
                *values = v;
                buffer.clear();
            }
        }

        println!("PM 1: {} μg/m³", values.pm_1);
        println!("PM 2.5: {} μg/m³", values.pm_2_5);
        println!("PM 10: {} μg/m³", values.pm_10);
        println!("CO₂: {} ppm (ideally below 1000 ppm)", values.co2);
        println!("VOC: {} (0 to 3)", values.voc);
        println!("Temperature: {}.{} °C", values.temp / 10 - 50, values.temp % 10);
        println!("Humidity: {} %", values.relative_humidity);
        println!(
            "Formaldehyde (CH₂O): {}.{}{}{} mg/m³ (ideally below 0.1 mg/m³)",
            values.ch2o / 1000,
            (values.ch2o / 100) % 10,
            (values.ch2o / 10) % 10,
            values.ch2o % 10
        );
        println!(
            "Carbon Monoxide (CO): {}.{} ppm (ideally below 9 ppm)",
            values.co / 10,
            values.co % 10
        );
        println!(
            "Ozone (O₃): {}.{}{} ppm (ideally below 0.08 ppm)",
            values.o3 / 100,
            (values.o3 / 10) % 10,
            values.o3 % 10
        );
        println!(
            "Nitrogen Dioxide (NO₂): {}.{}{} ppm (ideally below 0.05 ppm)",
            values.no2 / 100,
            (values.no2 / 10) % 10,
            values.no2 % 10
        );
        println!("");

        let aqi = values.air_quality_index();
        if aqi != last_aqi {
            notification_service.notify(match aqi {
                AirQualityIndex::Good => &NOTIFICATION_GOOD,
                AirQualityIndex::Moderate => &NOTIFICATION_MODERATE,
                AirQualityIndex::Sensitive => &NOTIFICATION_SENSITIVE,
                AirQualityIndex::Unhealthy => &NOTIFICATION_UNHEALTHY,
                AirQualityIndex::VeryUnhealthy => &NOTIFICATION_VERY_UNHEALTHY,
                AirQualityIndex::Hazardous => &NOTIFICATION_HAZARDOUS,
            });
        }

        SAMPLE_COUNT.fetch_add(1, Ordering::AcqRel);

        unsafe {
            sys::view_port_update(view_port);
        }
    });

    println!("Starting serial reader...");
    serial.start();
    
    loop {
        match event_queue.get(POLL_INTERVAL.try_into().unwrap()) {
            Err(err) => {
                if err != furi::Error::TimedOut {
                    panic!("event_queue get failed: {err}");
                }
            },
            Ok(event) => match (event.type_, event.key) {
                (sys::InputType_InputTypePress, sys::InputKey_InputKeyBack) => break,
                _ => continue,
            },
        }
        
        println!("Sending FETCH...");
        serial_handle.tx(&CMD_FETCH);
    }

    serial.stop();

    // GUI Cleanup
    unsafe {   
        sys::view_port_enabled_set(view_port, false);
        sys::gui_remove_view_port(gui.as_ptr(), view_port);
        sys::view_port_free(view_port);
    }
    
    0
}

#[derive(Debug, Default)]
struct Measurement {
    pm_1: u16,
    pm_2_5: u16,
    pm_10: u16,
    co2: u16,
    voc: u8,
    temp: u16,
    relative_humidity: u16,
    ch2o: u16,
    co: u16,
    o3: u16,
    no2: u16,
}

impl Measurement {
    const fn new() -> Self {
        Measurement {
            pm_1: 0,
            pm_2_5: 0,
            pm_10: 0,
            co2: 0,
            voc: 0,
            temp: 0,
            relative_humidity: 0,
            ch2o: 0,
            co: 0,
            o3: 0,
            no2: 0,
        }
    }

    /// Overall air quality.
    /// Based on the EPA AQI (8 hours)
    fn air_quality_index(&self) -> AirQualityIndex {
        let o3 = self.o3();
        let (_pm_1, pm_2_5, pm_10) = self.pm();
        let co = self.co();
        let no2 = self.no2();

        // Hazardous (Maroon)
        if o3 > 0.200 || pm_2_5 > 250 || pm_10 > 424 || co > 30.0 || no2 > 1.249 {
            return AirQualityIndex::Hazardous;
        }

        // Very Unhealthy (Purple)
        if o3 > 0.105 || pm_2_5 > 150 || pm_10 > 354 || co > 15.4 || no2 > 0.649 {
            return AirQualityIndex::VeryUnhealthy;
        }

        // Unhealthy (Red)
        if o3 > 0.085 || pm_2_5 > 55 || pm_10 > 254 || co > 12.4 || no2 > 0.360 {
            return AirQualityIndex::Unhealthy;
        }

        // Unhealthy for Sensitive Groups (Orange)
        if o3 > 0.070 || pm_2_5 > 35 || pm_10 > 154 || co > 9.4 || no2 > 0.100 {
            return AirQualityIndex::Sensitive;
        }

        // Moderate (Yellow)
        if o3 > 0.054 || pm_2_5 > 12 || pm_10 > 54 || co > 4.4 || no2 > 0.053 {
            return AirQualityIndex::Moderate;
        }

        // Good (Green)
        AirQualityIndex::Good
    }

    /// Particulate matter (μg/m) for PM 1, PM 2.5 and PM 10.
    fn pm(&self) -> (u16, u16, u16) {
        (self.pm_1, self.pm_2_5, self.pm_10)
    }

    /// Carbon Dioxide (ppm CO₂).
    /// Ideally should be below 1000 ppm.
    fn co2(&self) -> u16 {
        self.co2
    }

    /// Volitile Organic Compounds.
    fn voc(&self) -> VOCLevel {
        match self.voc {
            0 => VOCLevel::Clean,
            1 => VOCLevel::Light,
            2 => VOCLevel::Moderate,
            3 => VOCLevel::Severe,
            _ => panic!("unexpected VOC value"),
        }
    }

    /// Temperature (°C) accurate to 1 decimal place.
    fn temperature(&self) -> f64 {
        0.10 * self.temp as f64 - 50.0
    }

    /// Relative Humidity (%).
    fn relative_humidity(&self) -> u16 {
        self.relative_humidity
    }

    /// Formaldehyde (mg/m³ CH₂O) accurate to 3 decimal places.
    /// Ideally should be below 0.1 mg/m³.
    fn ch2o(&self) -> f64 {
        0.001 * self.ch2o as f64
    }

    /// Carbon Monoxide (ppm CO) accurate to 1 decimal place.
    /// Ideally should be below 9 ppm.
    fn co(&self) -> f64 {
        0.1 * self.co as f64
    }

    /// Ozone (ppm O₃) accurate to 2 decimal places.
    /// Ideally should be below 0.08 ppm.
    fn o3(&self) -> f64 {
        0.01 * self.o3 as f64
    }

    /// Nitrogen Dioxide (ppm NO₂) accurate to 2 decimal places.
    fn no2(&self) -> f64 {
        0.01 * self.no2 as f64
    }
}

impl TryFrom<&[u8]> for Measurement {
    type Error = ();

    fn try_from(buffer: &[u8]) -> Result<Self, Self::Error> {
        if buffer.len() != RESPONSE_SIZE {
            error!("Invalid response size");
            return Err(());
        }

        let checksum = calculate_checksum(&buffer[1..25]);
        if checksum != buffer[25] {
            error!("Bad checksum! 0x{:X} != 0x{:X}", checksum, buffer[25]);
            return Err(());
        }

        Ok(Measurement {
            pm_1: read_u16_be(&buffer[2..4]),
            pm_2_5: read_u16_be(&buffer[4..6]),
            pm_10: read_u16_be(&buffer[6..8]),
            co2: read_u16_be(&buffer[8..10]),
            voc: buffer[10],
            temp: read_u16_be(&buffer[11..13]),
            relative_humidity: read_u16_be(&buffer[13..15]),
            ch2o: read_u16_be(&buffer[15..17]),
            co: read_u16_be(&buffer[17..19]),
            o3: read_u16_be(&buffer[19..21]),
            no2: read_u16_be(&buffer[21..23]),
        })
    }
}

/// VOC levels reported by ZP01-MP503.
#[derive(Debug, Clone, Copy)]
enum VOCLevel {
    Clean,
    Light,
    Moderate,
    Severe,
}

/// Air Quality Index (see https://www.airnow.gov/aqi/aqi-basics/)
/// 0 to 50: Good (Green)
/// 51 to 100: Moderate (Yellow)
/// 101 to 150: Unhealthy for Sensitive Groups (Orange)
/// 151 to 200: Unhealthy (Red)
/// 201 to 300: Very Unhealthy (Purple)
/// 301 to 500: Hazardous (Maroon)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AirQualityIndex {
    Good,
    Moderate,
    Sensitive,
    Unhealthy,
    VeryUnhealthy,
    Hazardous,
}

const NOTIFICATION_GOOD: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(0),
    NotificationMessage::led_green(228),
    NotificationMessage::led_blue(0),
    NotificationMessage::do_not_reset(),
]);

const NOTIFICATION_MODERATE: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(255),
    NotificationMessage::led_green(255),
    NotificationMessage::led_blue(0),
    NotificationMessage::do_not_reset(),
]);

const NOTIFICATION_SENSITIVE: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(255),
    NotificationMessage::led_green(126),
    NotificationMessage::led_blue(0),
    NotificationMessage::do_not_reset(),
]);

const NOTIFICATION_UNHEALTHY: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(255),
    NotificationMessage::led_green(0),
    NotificationMessage::led_blue(0),
    NotificationMessage::do_not_reset(),
]);

const NOTIFICATION_VERY_UNHEALTHY: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(143),
    NotificationMessage::led_green(63),
    NotificationMessage::led_blue(151),
    NotificationMessage::do_not_reset(),
]);

const NOTIFICATION_HAZARDOUS: NotificationSequence = notification_sequence!([
    NotificationMessage::led_red(126),
    NotificationMessage::led_green(0),
    NotificationMessage::led_blue(35),
    NotificationMessage::do_not_reset(),
]);


#[cfg(test)]
mod tests {
    use super::*;

    /// Test data
    /// PM1.0 = 101 ug/m3
    /// PM2.5 = 54 ug/m3
    /// PM10 = 150 ug/m3
    /// CO2 = 410 ppm
    /// VOC = 0
    /// Temp = 25.5 degC
    /// Humidity = 40% RH
    /// CH2O = 0.040 mg/m3
    /// O3 = 0.32 ppm
    /// NO2 = 0.80 ppm 
    const TEST_DATA: [u8; RESPONSE_SIZE] = [0xFF, 0x86, 0x00, 0x65, 0x00, 0x36, 0x00, 0x96, 0x01, 0x9A, 0x00, 0x02, 0xFD, 0x00, 0x28, 0x00, 0x28, 0x00, 0x05, 0x00, 0x20, 0x00, 0x50, 0x00, 0x00, 0xEA];

    #[test]
    fn test_measurement_decode() {

    }
}