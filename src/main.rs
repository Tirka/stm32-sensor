#![no_std]
#![no_main]

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m::asm;
use cortex_m_rt::entry;
use stm32f4xx_hal as hal;
// use hal::prelude::*;

#[entry]
fn main() -> ! {
    let peripherals = hal::stm32::Peripherals::take().unwrap();
    
    peripherals.RCC.ahb1enr.write(|w| w.gpioden().set_bit());
    
    peripherals.GPIOD.moder.write(|w| w
        .moder12().bits(1u8)
        .moder13().bits(1u8)
        .moder14().bits(1u8)
        .moder15().bits(1u8)
    );
    peripherals.GPIOD.otyper.write(|w| w
        .ot12().clear_bit()
        .ot13().clear_bit()
        .ot14().clear_bit()
        .ot15().clear_bit()
    );
    peripherals.GPIOD.odr.write(|w| w
        .odr12().set_bit()
        .odr14().set_bit()
    );
    
    loop {
        // your code goes here
    }
}
