#![no_std]
#![no_main]

#![allow(unused)]

mod bmp180;

use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;

use stm32f4xx_hal as hal;

use hal::{prelude::*, stm32, delay::Delay};

#[entry]
fn main() -> ! {
    let cortex = cortex_m::Peripherals::take().unwrap();
    let stm32 = stm32::Peripherals::take().unwrap();

    // Configure clock speed?
    let rcc = stm32.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(24.mhz()).freeze();

    // Generate blocking delay
    let mut delay = Delay::new(cortex.SYST, clocks);

    // Configure PB6 and PB7 pins to I2C1.
    // Ref. stm32f407g.pdf, Table 9. Alternate function mapping, p.63
    let gpiob = stm32.GPIOB.split();
    let scl = gpiob.pb6.into_alternate_af4_open_drain();
    let sda = gpiob.pb7.into_alternate_af4_open_drain();

    let mut i2c = hal::i2c::I2c::new(
        stm32.I2C1,
        (scl, sda),
        100.khz(),
        clocks
    );

    loop {
        let (
            bmp180::Temperature(t),
            bmp180::Pressure(p),
        ) = bmp180::get_temperature_and_pressure(&mut i2c, bmp180::Oss::Standard, &mut delay);

        let true_temperature_cel = t as f64 / 10.;
        let true_pressure_mmhg = p as f64 * 0.0075;

        defmt::info!("T: {} [°C]. P: {} [mm Hg]", true_temperature_cel, true_pressure_mmhg);

        delay.delay_ms(5000_u32);
    }
}
