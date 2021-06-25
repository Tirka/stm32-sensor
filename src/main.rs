#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;

use stm32f4xx_hal as hal;

use hal::{prelude::*, stm32, delay::Delay};

#[entry]
fn main() -> ! {
    let peripherals = stm32::Peripherals::take().unwrap();
    let cortex = cortex_m::Peripherals::take().unwrap();

    // AHB1 clocks GPIOD, GPOIB and other
    peripherals.RCC.ahb1enr.write(|w| w.gpioden().enabled());

    // configure clock speed?
    let rcc = peripherals.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

    // generate blocking delay
    let mut delay = Delay::new(cortex.SYST, clocks);

    /////////////////////////////////////////////////////////////////////////////////////
    // CONFIGURE I2C ////////////////////////////////////////////////////////////////////
    /////////////////////////////////////////////////////////////////////////////////////
    let gpiob = peripherals.GPIOB.split();
    let scl = gpiob.pb6.into_alternate_af4_open_drain();
    let sda = gpiob.pb7.into_alternate_af4_open_drain();

    let mut i2c = hal::i2c::I2c::new(
        peripherals.I2C1,
        (scl, sda),
        100.khz(),
        clocks
    );

    delay.delay_ms(100_u32);

    /////////////////////////////////////////////////////////////////////////////////////
    // MAIN LOGIC ///////////////////////////////////////////////////////////////////////
    /////////////////////////////////////////////////////////////////////////////////////
    #[derive(defmt::Format)]
    struct CalibrationData {
        ac1: i16,
        ac2: i16,
        ac3: i16,
        ac4: u16,
        ac5: u16,
        ac6: u16,
        b1: i16,
        b2: i16,
        mb: i16,
        mc: i16,
        md: i16
    }

    // I2C address of BMP180 sensor
    static BMP_180_ADDRESS: u8 = 0x77;

    // oversampling setting (OSS)
    static OSS_ULTRA_LOW_POWER: u8 = 0;
    static OSS_STANDARD: u8 = 1;
    static OSS_HIGH_RESOLUTION: u8 = 2;
    static OSS_ULTRA_HIGH_RESOLUTION: u8 = 3;

    // read calibration data
    let mut calibration_data: [u8; 22] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    i2c.write_read(BMP_180_ADDRESS, &[0xAA], &mut calibration_data).unwrap();
    let cd = CalibrationData {
        ac1: ((calibration_data[0] as u16) * 256 + calibration_data[1] as u16) as i16,
        ac2: ((calibration_data[2] as u16) * 256 + calibration_data[3] as u16) as i16,
        ac3: ((calibration_data[4] as u16) * 256 + calibration_data[5] as u16) as i16,
        ac4: (calibration_data[6] as u16) * 256 + calibration_data[7] as u16,
        ac5: (calibration_data[8] as u16) * 256 + calibration_data[9] as u16,
        ac6: (calibration_data[10] as u16) * 256 + calibration_data[11] as u16,
        b1: ((calibration_data[12] as u16) * 256 + calibration_data[13] as u16) as i16,
        b2: ((calibration_data[14] as u16) * 256 + calibration_data[15] as u16) as i16,
        mb: ((calibration_data[16] as u16) * 256 + calibration_data[17] as u16) as i16,
        mc: ((calibration_data[18] as u16) * 256 + calibration_data[19] as u16) as i16,
        md: ((calibration_data[20] as u16) * 256 + calibration_data[21] as u16) as i16
    };

    loop {
        // read uncompensated temperature value
        let mut ut: [u8; 2] = [0, 0];
        i2c.write(BMP_180_ADDRESS, &[0xF4, 0x2E]).unwrap();
        delay.delay_us(4500_u32);
        i2c.write(BMP_180_ADDRESS, &[0xF6]).unwrap();
        i2c.read(BMP_180_ADDRESS, &mut ut).unwrap();
        let ut = (ut[0] as i32 * 256) + ut[1] as i32;

        // read uncompensated pressure value
        let mut up: [u8; 3] = [0, 0, 0];
        i2c.write(BMP_180_ADDRESS, &[0xF4, 0x34 + OSS_ULTRA_HIGH_RESOLUTION * 64]).unwrap();
        delay.delay_us(23500_u32); // 4500, 7500, 13500, 23500 us depending on OSS
        i2c.write(BMP_180_ADDRESS, &[0xF6]).unwrap();
        i2c.read(BMP_180_ADDRESS, &mut up).unwrap();
        let up: i32 = (up[0] as i32 * 65536 + up[1] as i32 * 256 + up[2] as i32) / 32; // TODO: [/ 32] = [>> (8 - oss)]

        // calculate true temperature
        let x1 = (ut - cd.ac6 as i32) * cd.ac5 as i32 / 32768;
        let x2 = cd.mc as i32 * 2048 / (x1 + cd.md as i32);
        let b5 = x1 + x2;
        let true_temperature = (b5 + 8) / 16; // in 0.1 deg. Celcius

        // calculate true pressure
        let b6 = b5 - 4000;
        let x1 = (cd.b2 as i32 * (b6 * b6 / 4096)) / 2048;
        let x2 = cd.ac2 as i32 * b6 / 2048;
        let x3 = x1 + x2;
        let b3 = (((cd.ac1 as i32 * 4 + x3) * 8) + 2) / 4; // TODO [* 8] = [<< oss]
        let x1 = cd.ac3 as i32 * b6 / 8192;
        let x2 = (cd.b1 as i32 * (b6 * b6 / 4096)) / 65536;
        let x3 = ((x1 + x2) + 2) / 4;
        let b4 = cd.ac4 as u32 * ((x3 + 32768) as u32) / 32768;
        let b7 = (up - b3) as u32 * (50000 / 8); // TODO [/ 8] = [>> oss]
        let p = if b7 < 0x_8000_0000 { ((b7 * 2) / b4) as i32 } else { ((b7 / b4) * 2) as i32 };
        let x1 = (p / 256) * (p / 256);
        let x1 = (x1 * 3038) / 65536;
        let x2 = (-7357 * p) / 65536;
        let true_pressure = p + (x1 + x2 + 3791) / 16; // Pa

        let true_temperature_cel = true_temperature as f64 / 10.;
        let true_pressure_mmhg = true_pressure as f64 * 0.0075;
        defmt::info!("T: {} [deg. C]. P: {} [mm. Hg]", true_temperature_cel, true_pressure_mmhg);

        delay.delay_ms(5000_u32);
    }
}
