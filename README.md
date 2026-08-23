# blinky-plus

External LED + button on a Raspberry Pi Pico W, with structured logging and crash reporting over USB — no debug probe required.

<!-- TODO: hero photo of the actual breadboard, well lit, replace this line -->
`docs/images/breadboard.jpg`

<!-- TODO: 10-15s demo GIF — LED blinking, button press logged, then a deliberate panic and the recovered message on reboot -->
`docs/images/demo.gif`

---

## Features

- External LED blink at a rate set by a `const` (no onboard-LED trickery — see [Known limitations](#known-limitations)).
- Button press logs an event over USB-serial and toggles a mode.
- Structured logging via [`defmt`](https://defmt.ferrous-systems.com/), carried over USB-CDC serial instead of RTT — works with zero extra hardware.
- Crash reporting via [`panic-persist`](https://docs.rs/panic-persist): panic messages survive a reset and print on the next boot.
- Fully USB-only dev loop: flash, run, and read logs without ever touching a SWD probe.
- CI: format, lint, host build, firmware build, and a flash-size report on every push.

## To-Do list

- [x] blink external led.
- [x] button press logs an event over USB.
- [x] structured logging via defmt.
- [ ] implement debouncer.
- [ ] crash report using panic-persist.
- [ ] reboot and flash using button instead of BOOTSEL.

## Hardware

| Part | Qty | Notes |
| --- | --- | --- |
| Raspberry Pi Pico W (or WH) | 1 | RP2040 + CYW43439 |
| LED (any color) | 1 | with a 330 Ω series resistor |
| Momentary push button | 1 | uses the RP2040's internal pull-up |
| Breadboard + jumper wires | — | |
| USB micro-B cable | 1 | data-capable — some cables are charge-only |

No debug probe needed for this or any project in this series.

## Wiring

| Pico W pin | Signal | Notes |
| --- | --- | --- |
| GP13 (pin 17) | LED anode, via 330 Ω resistor | cathode to GND |
| GP12 (pin 16) | Button, other leg to GND | internal pull-up enabled in firmware |
| GND (pin 38) | Common ground | shared by LED and button |

<!-- TODO: docs/wiring.md with a labeled diagram -->
See [`docs/wiring.md`](docs/wiring.md) for the full diagram.

The button doubles as the "reboot to bootloader" trigger during development — hold it at boot to drop straight into flash mode without touching BOOTSEL. See [Architecture](#architecture).

## Quickstart

No debug probe required — this flashes entirely over USB.

```bash
# 1. one-time setup
rustup target add thumbv6m-none-eabi
cargo install elf2uf2-rs --locked
cargo install defmt-print --locked

# 2. put the board in bootloader mode: hold BOOTSEL, plug in USB, release
#    (after the first flash, hold the GP12 button at boot instead — no BOOTSEL needed)

# 3. build, flash, and reboot in one step
cargo run --release
```

Once it's running, watch the logs in a second terminal:

```bash
socat /dev/ttyACM1,rawer,b115200 STDOUT | defmt-print -e target/thumbv6m-none-eabi/release/blinky-plus
#it could be on ttyACM0
# (Windows/macOS: point defmt-print at the matching COM/tty port instead)
```

You should see the LED blinking, a log line on every button press, and a boot banner with the firmware version and git hash.

### Try the panic-persist demo

Press and hold the button for 3 seconds — this deliberately triggers a panic in `src/main.rs` (see the `debug_panic_after_hold` function) so you can see crash reporting work end-to-end:

1. Hold the button 3s → firmware panics and resets.
2. On the next boot, the recovered panic message prints first, before the normal boot banner.
3. Release the button; the board returns to normal blinking.

This is the closest thing to a live backtrace you get without a SWD probe attached — the message is stashed in a reserved RAM region across the reset and read back on boot.

## Architecture

```text
main.rs
├── init: USB device + CDC-ACM serial, defmt-serial transport
├── init: check for a persisted panic message, print it if present
├── idle loop:
│   ├── poll button (debounced) → log edge, toggle mode
│   ├── hold-check → reset_to_usb_boot() or debug_panic_after_hold()
│   └── blink LED at MODE-dependent rate
```

Everything lives in `main.rs` for this project — it's intentionally small. Later projects in this series split hardware-facing code from host-testable core logic; `blinky-plus` is the template that establishes the tooling (CI, logging, panic capture) those projects build on.

## Testing

```bash
cargo test --target x86_64-unknown-linux-gnu   # host tests: debounce logic
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

The debounce state machine (`src/debounce.rs`) is the only pure logic in this project and is unit-tested on the host — no hardware needed to run it. Everything else (LED timing, button wiring, panic-persist behavior) needs a real board; there's no hardware-in-the-loop test in this project yet

## Known limitations

- The onboard LED (tied to the CYW43439 Wi-Fi chip) is **not** used here — bringing it up requires the `cyw43` PIO/SPI stack, which is out of scope for a first project. An external LED is used instead. See the [roadmap](../embedded-rust-rp2040-roadmap-no-probe.md) §1.1.
- No SWD debug probe support in this build. Logging and panic capture are USB-serial based; there's no live breakpoint/step debugging. See the roadmap §2.9 for what changes if a probe is added later.
- Debounce timing (`Debouncer<N>`) is tuned for a generic tactile button; adjust `N` if you're using a noisier switch.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.

## Acknowledgements

Built against the [`rp2040-hal`](https://github.com/rp-rs/rp-hal), [`defmt`](https://github.com/knurling-rs/defmt), and [`panic-persist`](https://github.com/jamesmunns/panic-persist) crates. Datasheet references: *RP2040 Datasheet*, *Raspberry Pi Pico W Datasheet* (Raspberry Pi Ltd).
