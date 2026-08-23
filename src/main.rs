#![no_std]
#![no_main]
mod debouncer;
use debouncer::{ButtonEvent, ButtonMonitor};

use core::cell::RefCell;
use cortex_m_rt::entry;
use critical_section::Mutex;
use embedded_hal::digital::OutputPin;
use static_cell::StaticCell;

use panic_halt as _;

use hal::{
    clocks::init_clocks_and_plls, gpio::Pins, pac, sio::Sio, timer::Timer, usb::UsbBus,
    watchdog::Watchdog,
};
use rp2040_hal::{self as hal};

use usb_device::{class_prelude::*, prelude::*};
use usbd_serial::SerialPort;

type UsbState = (UsbDevice<'static, UsbBus>, SerialPort<'static, UsbBus>);
static USB_STATE: Mutex<RefCell<Option<UsbState>>> = Mutex::new(RefCell::new(None));

const BLINK_TICKS: u64 = 500_000;
// --- Button-triggered behavior tuning (timer ticks = microseconds) ---
const DEBOUNCE_TICKS: u64 = 20_000;
const MULTI_CLICKS_WINDOW_TICKS: u64 = 400_000;
const HOLD_TICKS: u64 = 1_500_000;

struct DefmtUsbWriter;

impl embedded_io::ErrorType for DefmtUsbWriter {
    type Error = core::convert::Infallible;
}

impl embedded_io::Write for DefmtUsbWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        critical_section::with(|cs| {
            if let Some((usb_dev, serial)) = USB_STATE.borrow_ref_mut(cs).as_mut() {
                usb_dev.poll(&mut [serial]); // service the endpoint so buffered bytes actually flush
                if !serial.dtr() {
                    // No host terminal attached — drop the log instead of risking a stall.
                    return Ok(buf.len());
                }
                match serial.write(buf) {
                    Ok(n) => Ok(n),
                    Err(_) => Ok(buf.len()), // still never claim 0 progress — avoid retry loops upstream
                }
            } else {
                Ok(buf.len())
            }
        })
    }
    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

static WRITER: StaticCell<DefmtUsbWriter> = StaticCell::new();
static USB_BUS: StaticCell<UsbBusAllocator<UsbBus>> = StaticCell::new();

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
    //GPIO12 as button (pin 16)
    let button_pin = pins.gpio12.into_pull_up_input();

    //Timer
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    //USB comms
    let usb_bus = USB_BUS.init(UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    )));
    let serial = SerialPort::new(usb_bus);
    let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .strings(&[StringDescriptors::new(LangID::EN).product("blinky-plus")])
        .unwrap()
        .device_class(usbd_serial::USB_CLASS_CDC)
        .build();

    critical_section::with(|cs| {
        *USB_STATE.borrow_ref_mut(cs) = Some((usb_dev, serial));
    });

    defmt_serial::defmt_serial(WRITER.init(DefmtUsbWriter));
    defmt::info!("blinky-plus up!");

    let mut led_on = false;
    let mut last_toggle = 0u64;
    let now0 = timer.get_counter().ticks();
    let mut button = ButtonMonitor::new(
        button_pin,
        true,
        DEBOUNCE_TICKS,
        MULTI_CLICKS_WINDOW_TICKS,
        HOLD_TICKS,
        now0,
    )
    .unwrap();
    loop {
        critical_section::with(|cs| {
            if let Some((usb_dev, serial)) = USB_STATE.borrow_ref_mut(cs).as_mut() {
                usb_dev.poll(&mut [serial]);
            }
        });
        let now = timer.get_counter().ticks();
        match button.update(now).unwrap() {
            ButtonEvent::HoldTriggered => {
                defmt::info!("button held should reboot on this");
            }
            ButtonEvent::Clicks(n) if n >= 3 => {
                defmt::info!("should panic on this");
            }
            ButtonEvent::Clicks(n) => {
                defmt::info!("clicked {} time(s)", n);
            }
            ButtonEvent::None => {}
        }

        if now - last_toggle >= BLINK_TICKS {
            last_toggle = now;
            led_on = !led_on;
            if led_on {
                defmt::info!("on !");
                led.set_high().unwrap();
            } else {
                defmt::info!("off !");
                led.set_low().unwrap();
            }
        }
    }
}
