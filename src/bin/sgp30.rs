//! Flipper Zero App for
//! [Sensirion SPG30](https://sensirion.com/products/catalog/SGP30)

#![no_main]
#![no_std]

// Required for panic handler
extern crate flipperzero_rt;

use core::ffi::{c_void, CStr};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use core::{mem, ptr};

use flipperzero::furi::sync::Mutex;
use flipperzero::furi::{self, thread};
use flipperzero::gpio::i2c;
use flipperzero::{format, println};
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;
use sys::furi::Status;

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const SGP30_ADDR: i2c::DeviceAddress = i2c::DeviceAddress::new(0x58);
const SGP30_IAQ_INIT: [u8; 2] = [0x20, 0x03];
const SGP30_MEASURE_IAQ: [u8; 2] = [0x20, 0x08];
const SGP30_SET_ABS_HUMIDITY: [u8; 2] = [0x20, 0x61];
const SGP30_GET_INCEPTIVE_BASELINE: [u8; 2] = [0x20, 0xB3];
const SGP30_SET_INCEPTIVE_BASELINE: [u8; 2] = [0x20, 0x77];
const SGP30_GET_SERIAL: [u8; 2] = [0x36, 0x82];

// GUI record
const RECORD_GUI: &CStr = c"gui";
const FULLSCREEN: sys::GuiLayer = sys::GuiLayer_GuiLayerFullscreen;

static CURRENT_SAMPLE: AtomicU32 = AtomicU32::new(0);
static STATE: Mutex<State> = Mutex::new(State::new());

#[derive(Debug, Clone)]
struct State {
    co2_eq: u16,
    tvoc: u16,
}

impl State {
    pub const fn new() -> Self {
        State {
            co2_eq: 0,
            tvoc: 0,
        }
    }

    pub const fn is_zero(&self) -> bool {
        self.co2_eq == 0 && self.tvoc == 0
    }
}

// Define the FAP Manifest for this application
manifest!(
    name = "SGP30 Gas Sensor",
    app_version = 1,
    has_icon = true,
    // See https://github.com/flipperzero-rs/flipperzero/blob/v0.7.2/docs/icons.md for icon format
    icon = "../rustacean-10x10.icon",
);

// Define the entry function
entry!(main);

/// Read u16 (big endian)
fn read_u16_be(data: &[u8]) -> u16 {
    (data[0] as u16) << 8 | data[1] as u16
}

/// View draw handler.
///
/// # Safety
///
/// This must only be called from a valid draw handler.
pub unsafe extern "C" fn draw_callback(canvas: *mut sys::Canvas, _context: *mut c_void) {
    let state = STATE.lock();
    if state.is_zero() {
        let msg = format!("Warming up...");
        let width: i32 = sys::canvas_width(canvas).try_into().unwrap();
        let height: i32 = sys::canvas_height(canvas).try_into().unwrap();
        sys::canvas_draw_str_aligned(
            canvas,
            width / 2,
            height / 2,
            sys::Align_AlignCenter,
            sys::Align_AlignCenter,
            msg.as_c_str().as_ptr(),
        );
        return;
    }

    let lines = [
        format!("SGP30 Gas Sensor"),
        format!("CO2eq: {} ppm", state.co2_eq),
        format!("TVOC: {} ppb", state.tvoc),
    ];

    for (n, line) in lines.iter().enumerate() {
        sys::canvas_draw_str(canvas, 0, (n + 1) as i32 * 10, line.as_c_str().as_ptr());
    }
}

/// Input callback.
unsafe extern "C" fn app_input_callback(input_event: *mut sys::InputEvent, ctx: *mut c_void) {
    let event_queue = ctx as *mut sys::FuriMessageQueue;
    sys::furi_message_queue_put(event_queue, input_event as *mut c_void, 0);
}

// Entry point
fn main(_args: Option<&CStr>) -> i32 {
    let mut bus = i2c::Bus::EXTERNAL.acquire();

    unsafe {
        let event_queue = sys::furi_message_queue_alloc(8, mem::size_of::<sys::InputEvent>() as u32);

        // GUI Setup
        let view_port = sys::view_port_alloc();
        sys::view_port_draw_callback_set(view_port, Some(draw_callback), ptr::null_mut());
        sys::view_port_input_callback_set(
            view_port,
            Some(app_input_callback),
            event_queue as *mut c_void,
        );

        let gui = sys::furi_record_open(RECORD_GUI.as_ptr()) as *mut sys::Gui;
        sys::gui_add_view_port(gui, view_port, FULLSCREEN);

        let device = SGP30_ADDR;
        let mut running = init_sgp30(device, &mut bus);

        let mut event: MaybeUninit<sys::InputEvent> = MaybeUninit::uninit();
        while running {
            if !Status::from(sys::furi_message_queue_get(
                event_queue,
                event.as_mut_ptr().cast(),
                POLL_INTERVAL.as_millis() as u32,
            ))
            .is_err()
            {
                let event = event.assume_init();
                if event.type_ == sys::InputType_InputTypePress && event.key == sys::InputKey_InputKeyBack {
                    running = false;
                    continue;
                }
            }

            // This must be called once per second for the sensor's dynamic callibration
            read_sgp30(device, &mut bus, view_port);
        }

        // GUI Cleanup
        sys::view_port_enabled_set(view_port, false);
        sys::gui_remove_view_port(gui, view_port);
        sys::furi_record_close(RECORD_GUI.as_ptr());
        sys::view_port_free(view_port);
    }

    0
}

fn sgp30_get_serial_id(
    device: i2c::DeviceAddress,
    bus: &mut i2c::BusHandle,
) -> Result<[u8; 6], i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);
    let mut buffer = [0u8; 9];

    bus.trx(device, &SGP30_GET_SERIAL, &mut buffer, timeout)?;

    Ok([
        buffer[0], buffer[1], buffer[3], buffer[4], buffer[6], buffer[7],
    ])
}

fn sgp30_iaq_init(device: i2c::DeviceAddress, bus: &mut i2c::BusHandle) -> Result<(), i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);
    bus.tx(device, &SGP30_IAQ_INIT, timeout)?;
    thread::sleep(Duration::from_millis(12));

    Ok(())
}

fn sgp30_set_abs_humidity(
    device: i2c::DeviceAddress,
    bus: &mut i2c::BusHandle,
    value: &[u8; 2],
) -> Result<(), i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);
    let crc = maxim_crc8(value, 0xFF);

    let cmd = [
        SGP30_SET_ABS_HUMIDITY[0],
        SGP30_SET_ABS_HUMIDITY[1],
        value[0],
        value[1],
        crc,
    ];
    bus.tx(device, &cmd, timeout)?;
    thread::sleep(Duration::from_millis(12));

    Ok(())
}

fn sgp30_get_tvoc_inceptive_baseline(
    device: i2c::DeviceAddress,
    bus: &mut i2c::BusHandle,
) -> Result<[u8; 2], i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);

    bus.tx(device, &SGP30_GET_INCEPTIVE_BASELINE, timeout)?;
    thread::sleep(Duration::from_millis(10));

    let mut buffer = [0u8; 3];
    bus.rx(device, &mut buffer, timeout)?;

    let cmd = [
        SGP30_SET_INCEPTIVE_BASELINE[0],
        SGP30_SET_INCEPTIVE_BASELINE[1],
        buffer[0],
        buffer[1],
        buffer[2],
    ];
    bus.tx(device, &cmd, timeout)?;
    thread::sleep(Duration::from_millis(10));

    Ok([buffer[0], buffer[1]])
}

fn sgp30_set_tvoc_baseline(
    device: i2c::DeviceAddress,
    bus: &mut i2c::BusHandle,
    value: &[u8; 2],
) -> Result<(), i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);
    let crc = maxim_crc8(value, 0xFF);

    let cmd = [
        SGP30_SET_INCEPTIVE_BASELINE[0],
        SGP30_SET_INCEPTIVE_BASELINE[1],
        value[0],
        value[1],
        crc,
    ];
    bus.tx(device, &cmd, timeout)?;
    thread::sleep(Duration::from_millis(10));

    Ok(())
}

fn sgp30_measure_iaq(
    device: i2c::DeviceAddress,
    bus: &mut i2c::BusHandle,
) -> Result<(u16, u16), i2c::Error> {
    let timeout = furi::time::Duration::from_millis(100);

    bus.tx(device, &SGP30_MEASURE_IAQ, timeout)?;
    thread::sleep(Duration::from_millis(12));

    let mut buffer = [0u8; 6];
    bus.rx(device, &mut buffer, timeout)?;

    let co2_eq = read_u16_be(&buffer[0..2]);
    let tvoc = read_u16_be(&buffer[3..5]);

    Ok((co2_eq, tvoc))
}

fn maxim_crc8(data: &[u8], crc_init: u8) -> u8 {
    let data_size = data
        .len()
        .try_into()
        .expect("maxim_crc8 data buffer too large");

    unsafe { sys::maxim_crc8(data.as_ptr(), data_size, crc_init) }
}

fn init_sgp30(device: i2c::DeviceAddress, bus: &mut i2c::BusHandle) -> bool {
    let timeout = furi::time::Duration::from_millis(100);
    if !bus.is_device_ready(device, timeout) {
        println!("ERROR: device not ready");
        return false;
    }

    if let Ok(serial_id) = sgp30_get_serial_id(device, bus) {
        println!(
            "Serial (hex): {:x} {:x} {:x} {:x} {:x} {:x}",
            serial_id[0], serial_id[1], serial_id[2], serial_id[3], serial_id[4], serial_id[5]
        );
    } else {
        println!("ERROR: GET_SERIAL_ID failed");
        return false;
    };

    if sgp30_iaq_init(device, bus).is_err() {
        println!("ERROR: IAQ_INIT failed");
        return false;
    }

    let abs_humidity: [u8; 2] = [11, (8 * 0xFF / 10) as u8]; // abs humidity = 11.8 g/m³
    if sgp30_set_abs_humidity(device, bus, &abs_humidity).is_err() {
        println!("ERROR: SET_ABS_HUMIDITY failed");
        return false;
    }

    let Ok(tvoc_baseline) = sgp30_get_tvoc_inceptive_baseline(device, bus) else {
        println!("ERROR: GET_TVOC_INCEPTIVE_BASELINE failed");
        return false;
    };

    if sgp30_set_tvoc_baseline(device, bus, &tvoc_baseline).is_err() {
        println!("ERROR: SET_TVOC_BASELINE failed");
        return false;
    }

    true
}

fn read_sgp30(device: i2c::DeviceAddress, bus: &mut i2c::BusHandle, view_port: *mut sys::ViewPort) {
    let Ok((co2_eq, tvoc)) = sgp30_measure_iaq(device, bus) else {
        println!("ERROR: MEASURE_IAQ failed");
        return;
    };

    if CURRENT_SAMPLE.load(Ordering::SeqCst) > 15 {
        let mut state = STATE.lock();
        state.co2_eq = co2_eq;
        state.tvoc = tvoc;

        println!("CO₂eq: {} ppm; TVOC: {} (ppb)", co2_eq, tvoc);
        unsafe { sys::view_port_update(view_port) };
    }

    CURRENT_SAMPLE.fetch_add(1, Ordering::SeqCst);
}
