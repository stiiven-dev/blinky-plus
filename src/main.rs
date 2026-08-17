#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::OutputPin;

use panic_halt as _;

use hal::{
    clocks::init_clocks_and_plls, gpio::Pins, pac, sio::Sio, timer::Timer, usb::UsbBus,
    watchdog::Watchdog,
};
use rp2040_hal::{self as hal};

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

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
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    //USB comms
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::new(LangID::EN).product("blinky-plus")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    let mut led_on = false;
    let mut last_toggle = 0u64;
    loop {
        usb_dev.poll(&mut [&mut serial]);

        let now = timer.get_counter().ticks();
        if now - last_toggle >= 500_000 {
            last_toggle = now;
            led_on = !led_on;
            if led_on {
                let _ = serial.write(b"on\n");
                led.set_high().unwrap();
            } else {
                let _ = serial.write(b"off\n");
                led.set_low().unwrap();
            }
        }
    }
}
