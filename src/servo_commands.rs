use core::cell::RefCell;
use core::convert::{TryFrom, TryInto};
use scpi::error::Result;
use scpi::expression::numeric_list::{NumericList, Token as NumericItem};
use scpi::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct ServoControl {
    pub pulse_width: u16,
    pub enable: bool,
}

impl ServoControl {
    const PWIDTH_MIN: u16 = 0u16;
    const PWIDTH_MAX: u16 = 4095u16;

    pub fn new() -> Self {
        ServoControl {
            pulse_width: 1500,
            enable: false
        }
    }
}

macro_rules! servo_ctrl_new {
    () => {
    pub fn new(servos: &'a RefCell<[ServoControl]>) -> Self {
        Self {
            servos
        }
    }
    };
}

pub struct BodyServoPwidthAllCommand<'a> {
    servos: &'a RefCell<[ServoControl]>,
}

impl<'a> BodyServoPwidthAllCommand<'a> {
    servo_ctrl_new!();
}

impl<'a> Command for BodyServoPwidthAllCommand<'a> {
    fn event(&self, _context: &mut Context, args: &mut Tokenizer) -> Result<()> {
        let mut servo_pwidth = [0u16; 24];
        let mut num = 0u8;
        let mut pulses: NumericList = args.next_data(false)?.unwrap().try_into()?;
        for (i, puls) in pulses.enumerate() {
            if let NumericItem::Numeric(pwidth) = puls? {
                let pwidth: u16 = pwidth.numeric_range(
                    ServoControl::PWIDTH_MIN,
                    ServoControl::PWIDTH_MAX,
                    |_| Err(ErrorCode::IllegalParameterValue.into()),
                )?;
                servo_pwidth[i] = pwidth;
            } else {
                return Err(ErrorCode::IllegalParameterValue.into());
            }
            num += 1;
        }

        if num != 24 {
            Err(ErrorCode::IllegalParameterValue.into())
        } else {
            let mut servos = self.servos.borrow_mut();
            for (val, mut servo) in servo_pwidth.iter().zip(servos.iter_mut()) {
                servo.pulse_width = *val;
            }
            Ok(())
        }
    }

    fn query(
        &self,
        _context: &mut Context,
        _args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> Result<()> {
        let servos = self.servos.borrow();
        for servo in servos.iter() {
            response.data(servo.pulse_width);
        }
        response.finish()
    }
}

pub struct BodyServoPwidthSetCommand<'a> {
    servos: &'a RefCell<[ServoControl]>,
}

impl<'a> BodyServoPwidthSetCommand<'a> {
    servo_ctrl_new!();
}

impl<'a> Command for BodyServoPwidthSetCommand<'a> {
    fn event(&self, _context: &mut Context, args: &mut Tokenizer) -> Result<()> {
        let mut servos = self.servos.borrow_mut();
        let index: usize = args
            .next_data(false)?
            .unwrap()
            .numeric_range(1, 24, |_| Err(ErrorCode::IllegalParameterValue.into()))?
            - 1;
        servos[index].pulse_width = args.next_data(false)?.unwrap().numeric_range(
            ServoControl::PWIDTH_MIN,
            ServoControl::PWIDTH_MAX,
            |_| Err(ErrorCode::IllegalParameterValue.into()),
        )?;
        Ok(())
    }

    fn query(
        &self,
        _context: &mut Context,
        args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> Result<()> {
        let servos = self.servos.borrow();
        let index: usize = args
            .next_data(false)?
            .unwrap()
            .numeric_range(1, 24, |_| Err(ErrorCode::IllegalParameterValue.into()))?
            - 1;
        response.data(servos[index].pulse_width).finish()
    }
}

/// # `[:BODY]:SERVo:STATe:ALL <boolean>`
/// Enable/disable all servos
///
/// # `[:BODY]:SERVo:STATe:ALL?`
/// Query the state of all servos.
///
pub struct BodyServoStatAllCommand<'a> {
    servos: &'a RefCell<[ServoControl]>,
}

impl<'a> BodyServoStatAllCommand<'a> {
    servo_ctrl_new!();
}

impl<'a> Command for BodyServoStatAllCommand<'a> {
    fn event(&self, _context: &mut Context, args: &mut Tokenizer) -> Result<()> {
        let enable: bool = args.next_data(false)?.unwrap().try_into()?;
        let mut servos = self.servos.borrow_mut();
        for mut servo in servos.iter_mut() {
            servo.enable = enable;
        }
        Ok(())
    }

    fn query(
        &self,
        _context: &mut Context,
        _args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> Result<()> {
        let servos = self.servos.borrow();
        for servo in servos.iter() {
            response.data(servo.enable);
        }
        response.finish()
    }
}

/// # `[:BODY]:SERVo:STATe[:SET] <index>,<boolean>`
/// Enable/disable a servo
///
/// # `[:BODY]:SERVo:STATe[:SET]? <index>`
/// Query if a servo is enabled.
///
pub struct BodyServoStatSetCommand<'a> {
    servos: &'a RefCell<[ServoControl]>,
}

impl<'a> BodyServoStatSetCommand<'a> {
    servo_ctrl_new!();
}

impl<'a> Command for BodyServoStatSetCommand<'a> {
    fn event(&self, _context: &mut Context, args: &mut Tokenizer) -> Result<()> {
        let mut servos = self.servos.borrow_mut();
        let index: usize = args
            .next_data(false)?
            .unwrap()
            .numeric_range(1, 24, |_| Err(ErrorCode::IllegalParameterValue.into()))?
            - 1;
        servos[index].enable = args.next_data(false)?.unwrap().try_into()?;
        Ok(())
    }

    fn query(
        &self,
        _context: &mut Context,
        args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> Result<()> {
        let servos = self.servos.borrow();
        let index: usize = args
            .next_data(false)?
            .unwrap()
            .numeric_range(1, 24, |_| Err(ErrorCode::IllegalParameterValue.into()))?
            - 1;
        response.data(servos[index].enable).finish()
    }
}
