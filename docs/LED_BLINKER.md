```rust
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
    // CONFIGURE LEDS ///////////////////////////////////////////////////////////////////
    /////////////////////////////////////////////////////////////////////////////////////
    // set pins mode to output
    peripherals.GPIOD.moder.write(|w| w
        .moder12().output()
        .moder13().output()
        .moder14().output()
        .moder15().output()
    );

    // add push/pull resistor to pins
    peripherals.GPIOD.otyper.write(|w| w
        .ot12().push_pull()
        .ot13().push_pull()
        .ot14().push_pull()
        .ot15().push_pull()
    );

    loop {
        // set output voltage (flash LED)
        peripherals.GPIOD.odr.write(|w| w
            .odr12().high()
            .odr13().high()
        );

        delay.delay_ms(1000u32);

        peripherals.GPIOD.odr.write(|w| w
            .odr13().high()
            .odr14().high()
        );

        delay.delay_ms(1000u32);

        peripherals.GPIOD.odr.write(|w| w
            .odr14().high()
            .odr15().high()
        );

        delay.delay_ms(1000u32);

        peripherals.GPIOD.odr.write(|w| w
            .odr15().high()
            .odr12().high()
        );

        delay.delay_ms(1000u32);
    }
}
```
