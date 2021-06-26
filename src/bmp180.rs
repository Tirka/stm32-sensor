use stm32f4xx_hal::prelude::*;
use stm32f4xx_hal::delay::Delay;
use stm32f4xx_hal::i2c::{Instance, I2c};

/// I2C bus address of BMP180 sensor.
/// Shifting left and W/R bit managed by HAL.
/// Ref. BST-BMP180-DS000-09.pdf, p. 20
static BMP_180_ADDRESS: u8 = 0x77;

/// Output register address: extra least significant byte.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_OUT_XLSB: u8 = 0xF8;

/// Output register address: least significant byte.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_OUT_LSB: u8 = 0xF7;

/// Output register address: most significant byte.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_OUT_MSB: u8 = 0xF6;

/// Measurement control register address.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_CTRL_MEAS: u8 = 0xF4;

/// If set to 0xB6, will perform the same sequence as power on reset.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_SOFT_RESET: u8 = 0xE0;

/// Register address for storing chip-id: (0x55)
/// Ref. (BST-BMP180-DS000-09.pdf, p. 18)
static ADDR_ID: u8 = 0xD0;

/// Calibration Data. Starting address of 11 words of 16 bit each.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 13)
static ADDR_CALIB0: u8 = 0xAA;

/// Control register value for temperature
/// Ref. (BST-BMP180-DS000-09.pdf, p. 21)
static MEASURE_TEMPERATURE: u8 = 0x2E;

/// Temperature, measured in tenth of Celcius (i.e 217 = 21.7 °C)
pub struct Temperature(pub i32);
impl Temperature {
    pub fn from_tenth_of_celcius(v: i32) -> Self { Temperature(v) }
}

/// Pressure, measured in Pascal
pub struct Pressure(pub i32);
impl Pressure {
    pub fn from_pascal(v: i32) -> Self { Pressure(v) }
}

/// "Oversampling Setting" controls the oversampling ratio of the pressure measurement.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 14)
#[derive(Clone, Copy)]
pub enum Oss {
    UltraLowPower = 0,
    Standard = 1,
    HighResolution = 2,
    UltraHighResolution = 3
}

/// Ref. (BST-BMP180-DS000-09.pdf, p. 21)
static TEMPERATURE_DELAY_US: u32 = 4500;

/// Calculating pressure and temperature.
/// Ref. (BST-BMP180-DS000-09.pdf, p. 15)
pub fn get_temperature_and_pressure<I: Instance, P>(
        i2c: &mut I2c<I, P>,
        oss: Oss,
        delay: &mut Delay,
    ) -> (Temperature, Pressure)
{
    // Ref. (BST-BMP180-DS000-09.pdf, p. 21)
    let pressure_delay_us: u32 = match oss {
        Oss::UltraLowPower => 4500,
        Oss::Standard => 7500,
        Oss::HighResolution => 13500,
        Oss::UltraHighResolution => 25500,
    };

    // global buffer
    let mut buffer: [u8; 22] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    // read calibration data
    i2c.write_read(BMP_180_ADDRESS, &[ADDR_CALIB0], &mut buffer).unwrap();
    let ac1: i16 = ((buffer[0] as u16) * 256 + buffer[1] as u16) as i16;
    let ac2: i16 = ((buffer[2] as u16) * 256 + buffer[3] as u16) as i16;
    let ac3: i16 = ((buffer[4] as u16) * 256 + buffer[5] as u16) as i16;
    let ac4: u16 = (buffer[6] as u16) * 256 + buffer[7] as u16;
    let ac5: u16 = (buffer[8] as u16) * 256 + buffer[9] as u16;
    let ac6: u16 = (buffer[10] as u16) * 256 + buffer[11] as u16;
    let b1: i16 = ((buffer[12] as u16) * 256 + buffer[13] as u16) as i16;
    let b2: i16 = ((buffer[14] as u16) * 256 + buffer[15] as u16) as i16;
    let _mb: i16 = ((buffer[16] as u16) * 256 + buffer[17] as u16) as i16;
    let mc: i16 = ((buffer[18] as u16) * 256 + buffer[19] as u16) as i16;
    let md: i16 = ((buffer[20] as u16) * 256 + buffer[21] as u16) as i16;

    // read uncompensated temperature value
    i2c.write(BMP_180_ADDRESS, &[ADDR_CTRL_MEAS, MEASURE_TEMPERATURE]).unwrap();
    delay.delay_us(TEMPERATURE_DELAY_US);
    i2c.write(BMP_180_ADDRESS, &[ADDR_OUT_MSB]).unwrap();
    i2c.read(BMP_180_ADDRESS, &mut buffer[0..2]).unwrap();
    let uncompensated_temperature = (buffer[0] as i32 * 256) + buffer[1] as i32;

    // read uncompensated pressure value
    let measure_pressure: u8 = 0x34 + ((oss as u8) << 6);
    i2c.write(BMP_180_ADDRESS, &[ADDR_CTRL_MEAS, measure_pressure]).unwrap();
    delay.delay_us(pressure_delay_us);
    i2c.write(BMP_180_ADDRESS, &[ADDR_OUT_MSB]).unwrap();
    i2c.read(BMP_180_ADDRESS, &mut buffer[0..3]).unwrap();
    let uncompensated_pressure: i32 = (buffer[0] as i32 * 65536 + buffer[1] as i32 * 256 + buffer[2] as i32) >> (8 - oss as u8);

    // calculate true temperature
    let x1 = (uncompensated_temperature - ac6 as i32) * ac5 as i32 / 32768;
    let x2 = mc as i32 * 2048 / (x1 + md as i32);
    let b5 = x1 + x2;
    let true_temperature = Temperature::from_tenth_of_celcius((b5 + 8) / 16);

    // calculate true pressure
    let b6 = b5 - 4000;
    let x1 = (b2 as i32 * (b6 * b6 / 4096)) / 2048;
    let x2 = ac2 as i32 * b6 / 2048;
    let x3 = x1 + x2;
    let b3 = (((ac1 as i32 * 4 + x3) << (oss as u8)) + 2) / 4;
    let x1 = ac3 as i32 * b6 / 8192;
    let x2 = (b1 as i32 * (b6 * b6 / 4096)) / 65536;
    let x3 = ((x1 + x2) + 2) / 4;
    let b4 = ac4 as u32 * ((x3 + 32768) as u32) / 32768;
    let b7 = (uncompensated_pressure - b3) as u32 * (50000 >> (oss as u8));
    let p = if b7 < 0x_8000_0000 { ((b7 * 2) / b4) as i32 } else { ((b7 / b4) * 2) as i32 };
    let x1 = (p / 256) * (p / 256);
    let x1 = (x1 * 3038) / 65536;
    let x2 = (-7357 * p) / 65536;
    let true_pressure = Pressure::from_pascal(p + (x1 + x2 + 3791) / 16);

    (true_temperature, true_pressure)
}
