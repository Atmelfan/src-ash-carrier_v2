use core::cell::RefCell;
use core::convert::{TryFrom, TryInto};
use scpi::error::Result;
use scpi::prelude::*;

struct EyeControl {
    pitch_pulse_width: u16,
    yaw_pulse_width: u16,
    enable: bool,
}

impl EyeControl {

}

struct BodyEyeLookCommand<'a> {
    ec: &'a RefCell<EyeControl>
}

impl<'a> Command for BodyEyeLookCommand<'a> {
    fn event(&self, context: &mut Context, args: &mut Tokenizer) -> Result<()> {
        unimplemented!()
    }

    fn query(&self, context: &mut Context, args: &mut Tokenizer, response: &mut ResponseUnit) -> Result<()> {
        unimplemented!()
    }
}

