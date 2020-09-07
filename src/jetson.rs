use embedded_hal::{blocking::delay::DelayMs, digital::v2::OutputPin};

struct Jetson<PWR, BATOC> {
    pwr: PWR,
    batoc: BATOC,
}

impl<PWR, BATOC, E> Jetson<PWR, BATOC>
where
    PWR: OutputPin<Error = E>,
    BATOC: OutputPin<Error = E>,
{
    pub fn new(pwr: PWR, batoc: BATOC) -> Self {
        Jetson { pwr, batoc }
    }

    pub fn bat_oc(&mut self, ok: bool) -> Result<(), E> {
        if ok {
            self.batoc.set_high()
        } else {
            self.batoc.set_low()
        }
    }

    pub fn turn_on(&mut self) -> Result<(), E> {
        Ok(())
    }

    pub fn force_off(&mut self) -> Result<(), E> {
        Ok(())
    }
}
