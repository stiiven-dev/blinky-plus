use embedded_hal::digital::InputPin;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    None,
    Pressed,
    Released,
}

pub struct Debouncer<P> {
    pin: P,
    //physically pressed or not
    stable_pressed: bool,
    //raw reading currently being checked
    candidate_pressed: bool,
    //timer tick at which the candidate reading last changed
    candidate_since: u64,
    //threshold
    debounce_ticks: u64,

    active_low: bool,
}

impl<P, E> Debouncer<P>
where
    P: InputPin<Error = E>,
{
    // Create a debouncer, reading the pin once to seed the initial stable
    // state (avoids reporting a spurious edge on the very first `update`).
    //
    // `now` should be the current timer tick count at construction time.
    pub fn new(pin: P, active_low: bool, debounce_ticks: u64, now: u64) -> Result<Self, E> {
        let mut this = Self {
            pin,
            stable_pressed: false,
            candidate_pressed: false,
            candidate_since: now,
            debounce_ticks,
            active_low,
        };
        let pressed = this.read_pressed()?;
        this.stable_pressed = pressed;
        this.candidate_pressed = pressed;
        Ok(this)
    }

    fn read_pressed(&mut self) -> Result<bool, E> {
        let level_high = self.pin.is_high()?;
        Ok(if self.active_low {
            !level_high
        } else {
            level_high
        })
    }

    pub fn update(&mut self, now: u64) -> Result<Edge, E> {
        let raw_pressed = self.read_pressed()?;

        if raw_pressed != self.candidate_pressed {
            //raw reading changed
            self.candidate_pressed = raw_pressed;
            self.candidate_since = now;
            return Ok(Edge::None);
        }

        if self.candidate_pressed != self.stable_pressed
            && now.wrapping_sub(self.candidate_since) >= self.debounce_ticks
        {
            //candidate for enough time to be counted as valid
            self.stable_pressed = self.candidate_pressed;
            return Ok(if self.stable_pressed {
                Edge::Pressed
            } else {
                Edge::Released
            });
        }
        Ok(Edge::None)
    }

    #[allow(dead_code)]
    pub fn is_pressed(&self) -> bool {
        self.stable_pressed
    }
}
