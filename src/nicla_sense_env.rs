use flipperzero::furi::time::FuriDuration;
use flipperzero::gpio::i2c;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum OutdoorSensorMode {
    Off = 0,
    Cleaning = 1,
    #[default]
    OutdoorAirQuality = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum IndoorSensorMode {
    Off = 0,
    Cleaning = 1,
    #[default]
    IndoorAirQuality = 2,
    IndoorAirQualityLowPower = 3,
    PublicBuildingAirQuality = 4,
    Sulfur = 5,
}


pub struct NiclaSenseEnv<'a> {
    bus: &'a mut i2c::BusHandle,
    device: i2c::DeviceAddress,
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
    /// Board Slave Address Register (valid immediately after writing)
    const SLAVE_ADDRESS_REGISTER: u8 = 0x01;
    /// Board Control Register
    const CONTROL_REGISTER: u8 = 0x02;
    /// Orange LED control
    const ORANGE_LED_REGISTER: u8 = 0x03;
    /// RGB Red LED control
    const RGB_RED_REGISTER: u8 = 0x04;
    /// RGB Blue LED control
    const RGB_BLUE_REGISTER: u8 = 0x05;
    /// RGB Green LED control
    const RGB_GREEN_REGISTER: u8 = 0x06;
    /// RGB Intensity
    const RGB_INTENSITY_REGISTER: u8 = 0x07;
    /// Board UART Control Register
    const UART_CONTROL_REGISTER: u8 = 0x08;
    /// CSV Delimiter character (ASCII)
    const CSV_DELIMITER: u8 = 0x09; // ASCII code
    /// Board SW Revision
    const SOFTWARE_REVISION_REGISTER: u8 = 0x0C; // u8
    /// Product ID (currently: 0x01)
    const PRODUCT_ID_REGISTER: u8 = 0x0D; // u8 (currently 0x01)
    /// Serial Number (6x uint8, ZMOD4410 tracking number)
    const SERIAL_NUMBER_REGISTER: u8 = 0x0E; // [u8; 6] ZMOD4410 tracking number
    /// HS4001 sample counter
    const SAMPLE_COUNTER_REGISTER: u8 = 0x14; // u32
    /// HS4001 Temperature (degC)
    const TEMPERATURE_REGISTER: u8 = 0x18; // f32
    /// HS4001 Humidity (%RH)
    const HUMIDITY_REGISTER: u8 = 0x1C; // f32
    /// ZMOD4510 status
    const ZMOD4510_STATUS_REGISTER: u8 = 0x23; // u8
    /// ZMOD4510 sample counter
    const ZMOD4510_SAMPLE_COUNTER_REGISTER: u8 = 0x24; // u32
    /// ZMOD4510 EPA AQI
    const ZMOD4510_EPA_AQI_REGISTER: u8 = 0x28; // u16
    /// ZMOD4510 Fast AQI
    const ZMOD4510_FAST_AQI_REGISTER: u8 = 0x2A; // u16
    /// ZMOD4510 O3 (ppb)
    const ZMOD4510_O3_REGISTER: u8 = 0x2C; // f32
    /// ZMOD4510 NO2 (ppb)
    const ZMOD4510_NO2_REGISTER: u8 = 0x30; // f32
    /// ZMOD4510 Rmox[0] ... Rmox[12] (Ohm)
    const ZMOD4510_RMOX_REGISTER: u8 = 0x34; // [f32; 13]
    /// ZMOD4410 status
    const ZMOD4410_STATUS_REGISTER: u8 = 0x6B; // u8
    /// ZMOD4410 sample counter
    const ZMOD4410_SAMPLE_COUNTER_REGISTER: u8 = 0x24; // u32
    /// ZMOD4410 IAQ
    const ZMOD4410_IAQ_REGISTER: u8 = 0x70; // f32
    /// ZMOD4410 TVOC (mg/m3)
    const ZMOD4410_TVOC_REGISTER: u8 = 0x74; // f32
    /// ZMOD4410 eCO2 (ppm)
    const ZMOD4410_ECO2_REGISTER: u8 = 0x78; // f32
    /// ZMOD4410 Rel IAQ
    const ZMOD4410_REL_IAQ_REGISTER: u8 = 0x7C; // f32
    /// ZMOD4410 EtOH
    const ZMOD4410_ETOH_REGISTER: u8 = 0x80; // f32
    /// ZMOD4410 Rmox[0] ... Rmox[12] (Ohm)
    const ZMOD4410_RMOX_REGISTER: u8 = 0x84; // f32
    /// ZMOD4410 Rcda[0] ... Rcda[2]
    const ZMOD4410_RCDA_REGISTER: u8 = 0xB8; // f32
    /// ZMOD4410 Rhtr (heater resistance at room temperature)
    const ZMOD4410_RHTR_REGISTER: u8 = 0xC4; // f32
    /// ZMOD4410 Temp (temperature in degC used during ambient compensation)
    const ZMOD4410_TEMP_REGISTER: u8 = 0xC8; // f32
    /// ZMOD4410 Intensity (odor intensity)
    const ZMOD4410_INTENSITY_REGISTER: u8 = 0xCC; // f32
    /// ZMOD4410 Odor class (1 = sulfur odor, 0 = others)
    const ZMOD4410_ODOR_CLASS_REGISTER: u8 = 0xD0; // u8
    /// Persist settings
    const DEFAULTS_REGISTER: u8 = 0xD4;

    const I2C_TIMEOUT_MS: u64 = 1000;

    const BAUD_RATE_MAP: [u32; 8] = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200];

    pub fn new(bus: &'a mut i2c::BusHandle, device: i2c::DeviceAddress) -> Self {
        NiclaSenseEnv { bus, device }
    }

    pub fn with_default_addr(bus: &'a mut i2c::BusHandle) -> Self {
        NiclaSenseEnv { bus, device: i2c::DeviceAddress::new(Self::DEFAULT_DEVICE_ADDRESS) }
    }

    pub fn is_ready(&mut self) -> bool {
        let timeout = FuriDuration::from_millis(Self::I2C_TIMEOUT_MS);

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

    /// Set Orange LED register
    /// - bits 0 to 5: Brightness
    /// - bit 6: ???
    /// - bit 7: If set, LED will blink on sensor error independent of brightness setting.
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
    ///
    /// A value of -300.0 indicates that the temperature sensor is not ready.
    pub fn temperature(&mut self) -> f32 {
        /// 0x00 0x00 0x96 0xc3 = Not ready (-300.0)
        self.read_f32(Self::TEMPERATURE_REGISTER).unwrap()
    }

    /// Get the relative humidity level (0-100%RH).
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
    /// the 1-hour or 8-hour average of the ozone concentrations (concentration dependent).
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

    /// Get the Ozone (O₃) concentration in ppb.
    pub fn outdoor_o3(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4510_O3_REGISTER).unwrap()
    }

    /// Get the Nitrogen Dioxide (NO₂) concentration in ppb.
    pub fn outdoor_no2(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4510_NO2_REGISTER).unwrap()
    }

    /// MOx resistance.
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
            3 => IndoorAirQualityLowPower,
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

    /// Get the indoor air quality in range 0 to 5 where 0 is the best air quality and 5 is the worst.
    pub fn indoor_iqa(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_IAQ_REGISTER).unwrap()
    }

    /// Get the total volitile organic compounds in mg/m³.
    pub fn indoor_total_voc(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_TVOC_REGISTER).unwrap()
    }

    /// Get the estimated Carbon Dioxide (CO₂) concentration in ppm.
    pub fn indoor_estimated_co2(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_ECO2_REGISTER).unwrap()
    }

    /// Get the relative indoor air quality index (0 to 500) over a 24 hour period.
    /// Available in IAQ, ULP and PBAQ modes.
    ///
    /// - Below 100: Improvement in air quality
    /// - 100: No change in air quality
    /// - Over 100: Degregation in air qualiuty
    pub fn indoor_relative_iqa(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_REL_IAQ_REGISTER).unwrap()
    }

    /// Get the ethanol (EthOH) concentration in ppm.
    pub fn indoor_ethanol(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_ETOH_REGISTER).unwrap()
    }

    /// MOx resistances.
    pub fn indoor_rmox(&mut self) -> [f32; 13] {
        let mut buf = [0u8; 4 * 13];
        self.read_exact(Self::ZMOD4410_RMOX_REGISTER, &mut buf).unwrap();

        let mut values = [0.0; 13];
        for (n, b) in buf.windows(4).enumerate() {
            values[n] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }

        values
    }

    /// log10 of CDA resistances.
    pub fn indoor_rcda(&mut self) -> [f32; 3] {
        let mut buf = [0u8; 4 * 3];
        self.read_exact(Self::ZMOD4410_RCDA_REGISTER, &mut buf).unwrap();

        let mut values = [0.0; 3];
        for (n, b) in buf.windows(4).enumerate() {
            values[n] = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        }

        values
    }

    /// Heater resistance.
    pub fn indoor_rhtr(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_RHTR_REGISTER).unwrap()
    }

    /// Ambient temperature (degC).
    pub fn indoor_temp(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_TEMP_REGISTER).unwrap()
    }

    /// Get the odor intensity.
    /// Only for Sulphur Odor mode.
    pub fn indoor_odor_intensity(&mut self) -> f32 {
        self.read_f32(Self::ZMOD4410_INTENSITY_REGISTER).unwrap()
    }

    /// Get the odor class.
    /// Only for Sulphur Odor mode.
    ///
    /// - 1: "sulphur" (sulfur-based)
    /// - 0: "acceptable" (organic-based)
    pub fn indoor_odor_class(&mut self) -> u8 {
        self.read_u8(Self::ZMOD4410_ODOR_CLASS_REGISTER).unwrap()
    }

    /// Writes the current configuration to flash memory.
    /// Stores board register 0x00 ... 0x0B in flash to be default after reset
    /// This affects the following properties:
    /// - UART baud rate
    /// - UART CSV output enabled
    /// - CSV delimiter
    /// - UART Debugging enabled
    /// - I2C Device address
    /// - Indoor air quality sensor mode
    /// - Outdoor air quality sensor mode
    /// - Temperature sensor enabled
    /// - Orange LED brightness
    /// - Orange LED error status enabled
    /// - RGB LED brightness
    /// - RGB LED color
    pub fn persist_settings(&mut self) -> u8 {
        todo!()
    }

    fn write_u8(&mut self, reg_addr: u8, data: u8) -> Result<(), i2c::Error> {
        self.bus.write_u8(self.device, reg_addr, data, FuriDuration::from_millis(Self::I2C_TIMEOUT_MS))
    }

    fn read_u8(&mut self, reg_addr: u8) -> Result<u8, i2c::Error> {
        self.bus.read_u8(self.device, reg_addr, FuriDuration::from_millis(Self::I2C_TIMEOUT_MS))
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
        self.bus.read_exact(self.device, reg_addr, buf, FuriDuration::from_millis(Self::I2C_TIMEOUT_MS))
    }

}
