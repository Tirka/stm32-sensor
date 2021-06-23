#![no_std]
#![no_main]

use defmt_rtt as _;
use panic_probe as _;

use cortex_m_rt::entry;
use stm32f4xx_hal as hal;

#[entry]
fn main() -> ! {
    let peripherals = hal::stm32::Peripherals::take().unwrap();

    peripherals.RCC.ahb1enr.write(|w| w.gpioden().enabled());

    peripherals.GPIOD.moder.write(|w| w
        .moder12().output()
        .moder13().output()
        .moder14().output()
        .moder15().output()
    );

    peripherals.GPIOD.otyper.write(|w| w
        .ot12().push_pull()
        .ot13().push_pull()
        .ot14().push_pull()
        .ot15().push_pull()
    );

    peripherals.GPIOD.odr.write(|w| w
        .odr12().high()
        .odr13().high()
        .odr14().high()
        .odr15().high()
    );

    loop {
        // your code goes here
    }
}
