//! Flipper Zero App for
//! [Adruino Nicla Sense Env](https://docs.arduino.cc/hardware/nicla-sense-env/)

#![no_main]
#![no_std]

use core::ffi::{c_double, c_int, c_void, CStr};
use core::mem;
use core::pin::pin;
use core::sync::atomic::{AtomicU32, Ordering};

use flipperzero::furi::sync::Mutex;
use flipperzero::gpio::i2c::{self, DeviceAddress};
use flipperzero::{format, furi, println};
use flipperzero_rt::{entry, manifest};

use flipperzero_sys as sys;

use shared::furi_hal_power::{Power, PowerEvent};
use shared::furi_pubsub::Callback;
use shared::furi_record::Record;

const RECORD_GUI: &CStr = c"gui";

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
    
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[allow(unused)]
enum OutdoorSensorMode {
    Off = 0,
    Cleaning = 1,
    #[default]
    OutdoorAirQuality = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[allow(unused)]
enum IndoorSensorMode {
    Off = 0,
    Cleaning = 1,
    #[default]
    IndoorAirQuality = 2,
    LowPowerIndoorAirQuality = 3,
    PublicBuildingAirQuality = 4,
    Sulfur = 5,
}
    

struct NiclaSenseEnv<'a> {
    bus: &'a mut i2c::BusHandle,
    device: DeviceAddress,
}

#[allow(unused)]
impl<'a> NiclaSenseEnv<'a> {
    const DEFAULT_DEVICE_ADDRESS: u8 = 0x21;

    /// Status Register
    /// - bit 0: Temp/Humidity Enable (1 bit)
    /// - bit 1..4: Indoor Mode (3 bits)
    /// - bit 4..6: Outdoor Mode (2 bits)
    /// - bit 6: Deep Sleep
    /// - bit 7: Reset
    const STATUS_REGISTER: u8 = 0x00;
    const SLAVE_ADDRESS_REGISTER: u8 = 0x01;
    const CONTROL_REGISTER: u8 = 0x02;
    const ORANGE_LED_REGISTER: u8 = 0x03;
    const RGB_RED_REGISTER: u8 = 0x04;
    const RGB_BLUE_REGISTER: u8 = 0x05;
    const RGB_GREEN_REGISTER: u8 = 0x06;
    const RGB_INTENSITY_REGISTER: u8 = 0x07;
    const UART_CONTROL_REGISTER: u8 = 0x08;
    const SOFTWARE_REVISION_REGISTER: u8 = 0x0C; // u8
    const PRODUCT_ID_REGISTER: u8 = 0x0D; // u8
    const SERIAL_NUMBER_REGISTER: u8 = 0x0E; // [u8; 6]
    const SAMPLE_COUNTER_REGISTER: u8 = 0x14; // u32
    const TEMPERATURE_REGISTER: u8 = 0x18; // f32
    const HUMIDITY_REGISTER: u8 = 0x1C; // f32
    const ZMOD4510_STATUS_REGISTER: u8 = 0x23; // u8
    const ZMOD4510_SAMPLE_COUNTER_REGISTER: u8 = 0x24; // u32
    const ZMOD4510_EPA_AQI_REGISTER: u8 = 0x28; // u16
    const ZMOD4510_FAST_AQI_REGISTER: u8 = 0x2A; // u16
    const ZMOD4510_O3_REGISTER: u8 = 0x2C; // f32
    const ZMOD4510_NO2_REGISTER: u8 = 0x30; // f32
    const ZMOD4510_RMOX_REGISTER: u8 = 0x34; // [f32; 13]
    const ZMOD4410_STATUS_REGISTER: u8 = 0x6B; // u8
    const ZMOD4410_SAMPLE_COUNTER_REGISTER: u8 = 0x24; // u32
    const ZMOD4410_IAQ_REGISTER: u8 = 0x70; // f32
    const ZMOD4410_TVOC_REGISTER: u8 = 0x74; // f32
    const ZMOD4410_ECO2_REGISTER: u8 = 0x78; // f32
    const ZMOD4410_REL_IAQ_REGISTER: u8 = 0x7C; // f32
    const ZMOD4410_ETOH_REGISTER: u8 = 0x80; // f32
    const ZMOD4410_RMOX_REGISTER: u8 = 0x84; // f32
    const ZMOD4410_RCDA_REGISTER: u8 = 0xB8; // f32
    const ZMOD4410_RHTR_REGISTER: u8 = 0xC4; // f32
    const ZMOD4410_TEMP_REGISTER: u8 = 0xC8; // f32
    const ZMOD4410_INTENSITY_REGISTER: u8 = 0xCC; // f32
    const ZMOD4410_ODOR_CLASS_REGISTER: u8 = 0xD0; // u8


    const I2C_TIMEOUT_MS: u64 = 1000;

    const BAUD_RATE_MAP: [u32; 8] = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200];

    pub fn new(bus: &'a mut i2c::BusHandle, device: i2c::DeviceAddress) -> Self {
        NiclaSenseEnv { bus, device }
    }

    pub fn with_default_addr(bus: &'a mut i2c::BusHandle) -> Self {
        NiclaSenseEnv { bus, device: i2c::DeviceAddress::new(Self::DEFAULT_DEVICE_ADDRESS) }
    }

    pub fn is_ready(&mut self) -> bool {
        let timeout = furi::time::Duration::from_millis(Self::I2C_TIMEOUT_MS);

        self.bus.is_device_ready(self.device, timeout)
    }
    
    pub fn software_revision(&mut self) -> u8 {
        self.read_u8(Self::SOFTWARE_REVISION_REGISTER).unwrap()
    }
    
    pub fn product_id(&mut self) -> u8 {
        self.read_u8(Self::PRODUCT_ID_REGISTER).unwrap()
    }
    
    pub fn serial_number(&mut self) -> [u8; 6] {
        let mut buf = [0u8; 6];
        self.read_exact(Self::SERIAL_NUMBER_REGISTER, &mut buf).ok();

        buf
    }
    
    pub fn reset(&mut self) {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();
        self.write_u8(Self::CONTROL_REGISTER, status | (1 << 7)).ok();
    }
    
    pub fn deep_sleep(&mut self) {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();
        self.write_u8(Self::CONTROL_REGISTER, status | (1 << 6)).ok();
    }

    // TODO: restore_factory_settings

    pub fn uart_baud_rate(&mut self) -> u32 {
        let index = self.read_u8(Self::UART_CONTROL_REGISTER).unwrap() & 0x07;

        Self::BAUD_RATE_MAP[index as usize]
    }

    // TODO: set_baud_rate

    pub fn set_orange_led(&mut self, value: u8) {
        self.write_u8(Self::ORANGE_LED_REGISTER, value).unwrap()
    }
    
    pub fn set_rgb_colour(&mut self, red: u8, green: u8, blue: u8) {
        self.write_u8(Self::RGB_RED_REGISTER, red).unwrap();
        self.write_u8(Self::RGB_GREEN_REGISTER, green).unwrap();
        self.write_u8(Self::RGB_BLUE_REGISTER, blue).unwrap();
    }

    pub fn set_rgb_intensity(&mut self, value: u8) {
        self.write_u8(Self::RGB_INTENSITY_REGISTER, value).unwrap()
    }

    /// Get the temperature in degrees Celsius.
    pub fn temperature(&mut self) -> f32 {
        self.read_f32(Self::TEMPERATURE_REGISTER).unwrap()
    }
    
    /// Get the relative humidity level (0-100%).
    pub fn humidity(&mut self) -> f32 {
        self.read_f32(Self::HUMIDITY_REGISTER).unwrap()
    }

    /// Get the mode of the outdoor sensor.
    ///
    /// - 0: Mode to turn off the sensor and reduce power consumption.
    /// - 1: Cleaning mode to perform a thermal cleaning cycle of the MOx element.
    /// - 2: Mode to measure outdoor air quality.
    pub fn outdoor_sensor_mode(&mut self) -> OutdoorSensorMode {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();
        let mode = (status >> 4) & 3;

        match mode {
            0 => OutdoorSensorMode::Off,
            1 => OutdoorSensorMode::Cleaning,
            2 => OutdoorSensorMode::OutdoorAirQuality,
            _ => panic!("invalid state"),
        }
    }

    pub fn set_outdoor_sensor_mode(&mut self, mode: OutdoorSensorMode) -> bool {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();

        let new_status = (status & !(3 << 4)) | ((mode as u8) << 4);
        self.write_u8(Self::STATUS_REGISTER, new_status).is_ok()
    }

    /// Retrieves the EPA air quality index. Range is 0 to 500.
    ///
    /// The" EPA AQI" is strictly following the EPA standard and is based on 
    /// the 1-hour or 8-hour average of the O3 concentrations (concentration dependent).
    pub fn outdoor_epa_aqi(&mut self) -> u16 {
        self.read_u16(Self::ZMOD4510_EPA_AQI_REGISTER).unwrap()
    }
    
    /// Get the fast air quality index. Range is 0 to 500.
    ///
    /// As the standard averaging leads to a very slow response, especially during testing and evaluation, 
    /// "Fast AQI" provides quicker results with a 1-minute averaging.
    pub fn outdoor_fast_aqi(&mut self) -> u16 {
        self.read_u16(Self::ZMOD4510_FAST_AQI_REGISTER).unwrap()
    }

    /// Get the Ozone (O₃) value in ppb.
    pub fn outdoor_o3(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4510_O3_REGISTER).unwrap()
    }
    
    /// Get the Nitrogen Dioxide (NO₂) value in ppb.
    pub fn outdoor_no2(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4510_NO2_REGISTER).unwrap()
    }
    
    pub fn outdoor_rmox(&mut self) -> [f32; 13] {
        let mut buf = [0u8; 4 * 13];
        self.read_exact(Self::ZMOD4510_RMOX_REGISTER, &mut buf).unwrap();

        let mut values = [0.0; 13];
        for (n, b) in buf.windows(4).enumerate() {
            values[n] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }

        values
    }
    
    /// Get the mode of the indoor sensor.
    ///
    /// - 0: Mode to turn off the sensor and reduce power consumption.
    /// - 1: Cleaning mode to perform a thermal cleaning cycle of the MOx element.
    /// - 2: Mode to measure indoor air quality.
    /// - 3: Low power indoor air quality mode with lower accuracy.
    /// - 4: Public Building Air Quality mode.
    /// - 5: Mode to detect sulfur odor.
    pub fn indoor_sensor_mode(&mut self) -> IndoorSensorMode {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();
        let mode = (status >> 1) & 7;

        use IndoorSensorMode::*;
        match mode {
            0 => Off,
            1 => Cleaning,
            2 => IndoorAirQuality,
            3 => LowPowerIndoorAirQuality,
            4 => PublicBuildingAirQuality,
            5 => Sulfur,
            _ => panic!("invalid state"),
        }
    }

    pub fn set_indoor_sensor_mode(&mut self, mode: IndoorSensorMode) -> bool {
        let status = self.read_u8(Self::STATUS_REGISTER).unwrap();
        let new_status = (status & !(7 << 1)) | ((mode as u8) << 1);

        self.write_u8(Self::STATUS_REGISTER, new_status).is_ok()
    }

    /// Get the indoor air quality. The common rage is 0.0 to ~5.0.
    pub fn indoor_iqa(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_IAQ_REGISTER).unwrap()
    }
    
    /// Get the total volitile organic compounds in mg/m³.
    pub fn indoor_total_voc(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_TVOC_REGISTER).unwrap()
    }
    
    /// Get the estimated Carbon Dioxide (CO₂) in ppm.
    pub fn indoor_estimated_co2(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_ECO2_REGISTER).unwrap()
    }
    
    /// Get the relative indoor air quality in percent (0% to 100%).
    pub fn indoor_relative_iqa(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_REL_IAQ_REGISTER).unwrap()
    }
    
    /// Get the ethanol value.
    pub fn indoor_ethanol(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_ETOH_REGISTER).unwrap()
    }
    
    pub fn indoor_rmox(&mut self) -> [f32; 13] {
        let mut buf = [0u8; 4 * 13];
        self.read_exact(Self::ZMOD4410_RMOX_REGISTER, &mut buf).unwrap();

        let mut values = [0.0; 13];
        for (n, b) in buf.windows(4).enumerate() {
            values[n] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }

        values
    }
    
    pub fn indoor_rcda(&mut self) -> [f32; 3] {
        let mut buf = [0u8; 4 * 3];
        self.read_exact(Self::ZMOD4410_RCDA_REGISTER, &mut buf).unwrap();

        let mut values = [0.0; 3];
        for (n, b) in buf.windows(4).enumerate() {
            values[n] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }

        values
    }
    
    pub fn indoor_rhtr(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_RHTR_REGISTER).unwrap()
    }
    
    pub fn indoor_temp(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_TEMP_REGISTER).unwrap()
    }
    
    /// Get the odor intensity.
    pub fn indoor_odor_intensity(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_INTENSITY_REGISTER).unwrap()
    }
    
    /// Get the odor class
    pub fn indoor_odor_class(&mut self) -> u8 {
        self.read_u8(Self::ZMOD4410_ODOR_CLASS_REGISTER).unwrap()
    }

    fn write_u8(&mut self, reg_addr: u8, data: u8) -> Result<(), i2c::Error> {
        self.bus.write_u8(self.device, reg_addr, data, furi::time::Duration::from_millis(Self::I2C_TIMEOUT_MS))
    }
    
    fn read_u8(&mut self, reg_addr: u8) -> Result<u8, i2c::Error> {
        self.bus.read_u8(self.device, reg_addr, furi::time::Duration::from_millis(Self::I2C_TIMEOUT_MS))
    }
    
    fn read_u16(&mut self, reg_addr: u8) -> Result<u16, i2c::Error> {
        let mut buf = [0u8; 2];
        self.read_exact(reg_addr, &mut buf)?;

        Ok(u16::from_le_bytes(buf))
    }
    
    fn read_u32(&mut self, reg_addr: u8) -> Result<u32, i2c::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(reg_addr, &mut buf)?;

        Ok(u32::from_le_bytes(buf))
    }

    fn read_f32(&mut self, reg_addr: u8) -> Result<f32, i2c::Error> {
        let mut buf = [0u8; 4];
        self.read_exact(reg_addr, &mut buf)?;

        Ok(f32::from_le_bytes(buf))
    }

    fn read_exact(&mut self, reg_addr: u8, buf: &mut [u8]) -> Result<(), i2c::Error> {
        self.bus.read_exact(self.device, reg_addr, buf, furi::time::Duration::from_millis(Self::I2C_TIMEOUT_MS))
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
        sprintf!(c"current draw: %0.3f", values.current as c_double),

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

struct View<'a> {
    power: *mut sys::Power,
    device: *mut NiclaSenseEnv<'a>,
}

unsafe extern "C" fn tick_callback(ctx: *mut c_void) {
    let context: &mut View = &mut *(ctx.cast());
    let device = &mut *context.device;

    let mut power_info = mem::zeroed();
    sys::power_get_info(context.power, &raw mut power_info);
    
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
    
    let power: Record<Power> = Record::open();
    let pubsub = power.get_pubsub();

    let callback = Callback::new(|event: &PowerEvent| {
        println!("PowerEvent: {:?}", event.type_ as c_int);
    });
    let callback = pin!(callback);
    
    let _subscription = pubsub.subscribe(callback);

    let mut context = View {
        power: power.as_ptr(),
        device: &raw mut device,
    };

    // GUI Setup
    let gui;
    let view_dispatcher;
    let view;
    unsafe {
        gui = sys::furi::UnsafeRecord::<sys::Gui>::open(RECORD_GUI);
        
        view_dispatcher = sys::view_dispatcher_alloc();
        sys::view_dispatcher_set_event_callback_context(view_dispatcher, &raw mut context as *mut _);
        sys::view_dispatcher_set_tick_event_callback(view_dispatcher, Some(tick_callback), 500);
        sys::view_dispatcher_attach_to_gui(view_dispatcher, gui.as_ptr(), sys::ViewDispatcherType_ViewDispatcherTypeFullscreen);

        view = sys::view_alloc();
        sys::view_set_context(view, view_dispatcher.cast());
        sys::view_set_draw_callback(view, Some(draw_callback));
        
        sys::view_set_previous_callback(view, Some(back));
        sys::view_dispatcher_add_view(view_dispatcher, 0, view);
        sys::view_dispatcher_switch_to_view(view_dispatcher, 0);
    }

    println!("Setting outdoor sensor mode");
    if !device.set_outdoor_sensor_mode(OutdoorSensorMode::OutdoorAirQuality) {
        println!("failed to set outdoor sensor mode");
    }

    println!("Setting indoor sensor mode");
    if !device.set_indoor_sensor_mode(IndoorSensorMode::LowPowerIndoorAirQuality) {
        println!("failed to set indoor sensor mode");
    }

    device.set_orange_led(0);
    device.set_rgb_intensity(0);
    
    unsafe {
        sys::view_dispatcher_run(view_dispatcher);
    }

    // GUI Cleanup
    unsafe {   
        sys::view_dispatcher_remove_view(view_dispatcher, 0);
        sys::view_free(view);
        sys::view_dispatcher_free(view_dispatcher);
    }

    0
}