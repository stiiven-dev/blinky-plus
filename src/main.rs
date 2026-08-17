#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::{delay::DelayNs, digital::OutputPin};

use panic_halt as _;

use hal::{
    clocks::init_clocks_and_plls, gpio::Pins, pac, sio::Sio, timer::Timer, watchdog::Watchdog,
};
use rp2040_hal::{self as hal};

/// Second-stage bootloader. Required: the RP2040 boot ROM reads this first
/// to know how to talk to QSPI flash. Without it, the chip can't validate
/// the flashed image and falls back to bootloader/mass-storage mode on
/// every reset instead of running the program.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

#[entry]
fn main() -> ! {
    //take ownership of rp2040 peripherals (singleton)
    let mut pac = pac::Peripherals::take().unwrap();

    //Watchdog
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    //Initialize clocks
    let clocks = init_clocks_and_plls(
        12_000_000,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    //SIO gives access to GPIO
    let sio = Sio::new(pac.SIO);

    //Initialize GPIO Pins
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    //GPIO13 as led (pin 17)
    let mut led = pins.gpio13.into_push_pull_output();

    //Timer
    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    loop {
        led.set_high().unwrap();
        timer.delay_ms(500);

        led.set_low().unwrap();
        timer.delay_ms(500);
    }
}
