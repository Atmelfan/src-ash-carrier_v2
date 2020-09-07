#![allow(unsafe_code)]
#![no_main]
#![no_std]

use panic_itm as _;

use cortex_m::interrupt::{self as int, Mutex};
use cortex_m::{asm, singleton};
use cortex_m_rt::{entry, exception, ExceptionFrame};
use cortex_m_semihosting::{heprintln, hprintln};

use core::fmt::Write;

use libm;

// HAL
use stm32f4xx_hal::stm32;
use stm32f4xx_hal::stm32::{
    interrupt, Interrupt, I2C2 as I2C2_PERIPH, NVIC, USART2 as USART2_PERIPH,
};
use stm32f4xx_hal::{delay::Delay, i2c, prelude::*, serial};

use lazy_static::lazy_static;

// I2C Stuff
use pwm_pca9685::{Pca9685, SlaveAddr};
use shared_bus::BusManager;

//Default commands
use scpi::ieee488::commands::*;
use scpi::info;
use scpi::prelude::*;
use scpi::response::{ArrayVecFormatter, Formatter};
use scpi::scpi::commands::*;
use scpi::{
    ieee488_cls,
    ieee488_ese,
    ieee488_esr,
    ieee488_idn,
    ieee488_opc,
    ieee488_rst,
    ieee488_sre,
    ieee488_stb,
    ieee488_tst,
    ieee488_wai,
    qonly,
    scpi_crate_version,

    scpi_status,
    scpi_system,
    //Helpers
    scpi_tree,
};

// Semihosting
//use cortex_m_semihosting::hprintln;

// Git version
use core::borrow::BorrowMut;
use core::cell::{Ref, RefCell};
use git_version::git_version;
use stm32f4xx_hal::serial::config::Config;
use stm32f4xx_hal::serial::{Event, Rx, Tx};

const GIT_VERSION: &[u8] = git_version!().as_bytes();

mod linereader;
mod jetson;
use linereader::LineReader;
mod servo_commands;
use servo_commands::*;
mod eyes_commands;
use eyes_commands::*;

use heapless::mpmc::Q16;

use core::convert::{TryFrom, TryInto};
use nalgebra as na;
use nalgebra::{Point3, Rotation3, Translation3};

use core::sync::atomic::{AtomicBool, Ordering};
use uom::si::angle::radian;
use uom::si::f32;
use uom::si::length::meter;
use arraydeque::ArrayDeque;

//***********************************************************************************
/// # SCPI code

struct MyDevice;
impl Device for MyDevice {
    fn cls(&mut self) -> Result<(), Error> {
        unimplemented!()
    }

    fn rst(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

struct BodyAttRotCommand {
    rotation: RefCell<na::Rotation3<f32>>,
}

impl Command for BodyAttRotCommand {
    fn event(&self, context: &mut Context, args: &mut Tokenizer) -> scpi::error::Result<()> {
        let rotation = Rotation3::<f32>::from_euler_angles(
            f32::Angle::try_from(args.next_data(false)?.unwrap())?.get::<radian>(),
            f32::Angle::try_from(args.next_data(false)?.unwrap())?.get::<radian>(),
            f32::Angle::try_from(args.next_data(false)?.unwrap())?.get::<radian>(),
        );
        self.rotation.replace(rotation);
        Ok(())
    }

    fn query(
        &self,
        context: &mut Context,
        args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> scpi::error::Result<()> {
        let (roll, pitch, yaw) = self.rotation.borrow().euler_angles();
        response.data(roll).data(pitch).data(yaw).finish()
    }
}

struct BodyAttTranCommand<'a> {
    translation: &'a RefCell<na::Translation3<f32>>,
}

impl<'a> Command for BodyAttTranCommand<'a> {
    fn event(&self, context: &mut Context, args: &mut Tokenizer) -> scpi::error::Result<()> {
        let mut translation = Translation3::<f32>::new(
            f32::Length::try_from(args.next_data(false)?.unwrap())?.get::<meter>(),
            f32::Length::try_from(args.next_data(false)?.unwrap())?.get::<meter>(),
            f32::Length::try_from(args.next_data(false)?.unwrap())?.get::<meter>(),
        );
        self.translation.replace(translation);
        Ok(())
    }

    fn query(
        &self,
        context: &mut Context,
        args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> scpi::error::Result<()> {
        let vec = self.translation.borrow();
        response.data(vec.x).data(vec.y).data(vec.z).finish()
    }
}

struct DiagServoAngle<'a> {
    servos: &'a RefCell<[f32]>,
}

impl<'a> Command for DiagServoAngle<'a> {
    fn event(&self, context: &mut Context, args: &mut Tokenizer) -> scpi::error::Result<()> {
        let servo: usize = args.next_data(false)?.unwrap().try_into()?;
        let angle: f32::Angle = args.next_data(false)?.unwrap().try_into()?;
        let mut servos = self.servos.borrow_mut();
        if let Some(mut s) = servos.get_mut(servo) {
            *s = angle.get::<radian>();
            Ok(())
        } else {
            Err(ErrorCode::DataOutOfRange.into())
        }
    }

    fn query(
        &self,
        context: &mut Context,
        args: &mut Tokenizer,
        response: &mut ResponseUnit,
    ) -> scpi::error::Result<()> {
        unimplemented!()
    }
}

struct BodyServoCommand<'a> {
    servos: &'a RefCell<[f32]>,
}

//***********************************************************************************
/// # Uart

lazy_static!{
    static ref RXBUFFER: Mutex<RefCell<ArrayDeque<[u8; 64]>>> = Mutex::new(RefCell::new(ArrayDeque::new()));
}

static RX: Mutex<RefCell<Option<Rx<USART2_PERIPH>>>> = Mutex::new(RefCell::new(None));
static BUFFER_OVERFLOW: AtomicBool = AtomicBool::new(false);

#[interrupt]
fn USART2() {
    //cortex_m::asm::bkpt();
    int::free(|cs| {
        let mut rx = RX.borrow(cs).borrow_mut();
        let mut rx = rx.as_mut().unwrap();
        match rx.read() {
            Ok(c) => {

                if RXBUFFER.borrow(cs).borrow_mut().push_back(c).is_err() {
                    BUFFER_OVERFLOW.store(true, Ordering::Relaxed);
                    heprintln!("Overflow").unwrap();
                }
            },
            Err(err) => {
                heprintln!("{:?}", err).unwrap();
            },
        }
    });
}

/// # Main code

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();
    //let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    // Set up the system clock.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();
    let gpioa = dp.GPIOA.split();
    let gpiob = dp.GPIOB.split();

    /**************************************** USART2 ****************************************/
    let mut pa2 = gpioa.pa2.into_floating_input();
    let mut pa3 = gpioa.pa3.into_push_pull_output();
    dp.USART2.cr1.modify(|_, w| w.rxneie().set_bit());
    let mut usart2 = serial::Serial::usart2(
        dp.USART2,
        (pa2.into_alternate_af7(), pa3.into_alternate_af7()),
        serial::config::Config::default().baudrate(9600.bps()),
        clocks,
    )
    .unwrap();
    usart2.listen(Event::Rxne);
    let (mut serial_tx, mut serial_rx) = usart2.split();
    int::free(|cs| {
        RX.borrow(cs).replace(Some(serial_rx));
    });

    /**************************************** I2C2 ****************************************/
    let servos = RefCell::new([ServoControl::new(); 24]);

    let scl = gpiob.pb10.into_alternate_af4().set_open_drain();
    let sda = gpiob.pb11.into_alternate_af4().set_open_drain();
    let i2c = i2c::I2c::i2c2(dp.I2C2, (scl, sda), 100.khz(), clocks);
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);
    let mut servos_1 = Pca9685::new(
        i2c_bus.acquire_i2c(),
        SlaveAddr::Alternative(false, false, false, true, true, false),
    );
    servos_1.enable().unwrap();
    servos_1.set_prescale(49).unwrap();
    servos_1.set_all_on_off(&[0u16; 16], &[750u16; 16]).unwrap();
    let mut servos_2 = Pca9685::new(
        i2c_bus.acquire_i2c(),
        SlaveAddr::Alternative(false, false, false, true, true, true),
    );
    servos_2.enable().unwrap();
    servos_2.set_prescale(49).unwrap();
    servos_2.set_all_on_off(&[0u16; 16], &[750u16; 16]).unwrap();
    // let mut servos_eye = Pca9685::new(
    //     i2c_bus.acquire(),
    //     SlaveAddr::Alternative(false, false, true, false, false, false),
    // );
    // servos_eye.enable().unwrap();
    // servos_eye.set_prescale(49).unwrap();
    // servos_eye
    //     .set_all_on_off(&[0u16; 16], &[750u16; 16])
    //     .unwrap();

    /**************************************** SCPI ****************************************/

    let translation = RefCell::new(Translation3::<f32>::new(0.0, 0.0, 0.0));
    let points = RefCell::new([Point3::<f32>::new(0.0, 0.0, 0.0); 8]);

    let mut my_device = MyDevice {};

    let tra = &BodyAttTranCommand {
        translation: &translation,
    };

    let servo_pwidth_all = &BodyServoPwidthAllCommand::new(&servos);
    let servo_pwidth_set = &BodyServoPwidthSetCommand::new(&servos);
    let servo_stat_all = &BodyServoStatAllCommand::new(&servos);
    let servo_stat_set = &BodyServoStatSetCommand::new(&servos);


    let tree = scpi_tree![
        // Create default IEEE488 mandated commands
        ieee488_idn!(b"GPA-Robotics", b"ash-carrier", b"0", GIT_VERSION),
        ieee488_cls!(),
        ieee488_ese!(),
        ieee488_esr!(),
        ieee488_opc!(),
        ieee488_rst!(),
        ieee488_sre!(),
        ieee488_stb!(),
        ieee488_tst!(),
        ieee488_wai!(),
        // Create default SCPI mandated STATus subsystem
        scpi_status!(),
        // Create default SCPI mandated SYSTem subsystem
        scpi_system!(),
        //
        scpi_crate_version!(),
        Node {
            name: b"BODY",
            optional: true,
            handler: None,
            sub: &[
                Node {
                    name: b"SERVos",
                    optional: false,
                    handler: None,
                    sub: &[
                        Node {
                            name: b"PWIDth",
                            optional: false,
                            handler: None,
                            sub: &[
                                Node {
                                    name: b"ALL",
                                    optional: true,
                                    handler: Some(servo_pwidth_all),
                                    sub: &[]
                                },
                                Node {
                                    name: b"SET",
                                    optional: false,
                                    handler: Some(servo_pwidth_set),
                                    sub: &[]
                                },
                            ]
                        },
                        Node {
                            name: b"STATe",
                            optional: false,
                            handler: None,
                            sub: &[
                                Node {
                                    name: b"ALL",
                                    optional: true,
                                    handler: Some(servo_stat_all),
                                    sub: &[]
                                },
                                Node {
                                    name: b"SET",
                                    optional: false,
                                    handler: Some(servo_stat_set),
                                    sub: &[]
                                },
                            ]
                        },
                    ]
                },
                Node {
                    name: b"EYE",
                    optional: false,
                    handler: Some(tra),
                    sub: &[]
                },
            ]
        }
    ];
    let mut errors = ArrayErrorQueue::<[Error; 10]>::new();
    let mut context = Context::new(&mut my_device, &mut errors, &tree);
    let mut formatter = ArrayVecFormatter::<[u8; 256]>::new();
    let mut reader = LineReader::new();

    // Enable interrupts
    NVIC::unpend(Interrupt::USART2);
    unsafe {
        NVIC::unmask(Interrupt::USART2);
    };

    cortex_m::asm::bkpt();
    loop {
        // SCPI communication
        while let Some(c) = int::free(|cs| {
            RXBUFFER.borrow(cs).borrow_mut().pop_front()
        }) {
            // Move read bytes into line buffer and execute any lines
            if let Some(line) = reader.push(c).ok() {
                //hprintln!("{:?}", line).unwrap();
                if context.run(line, &mut formatter).is_ok() {
                    let response = formatter.as_slice();
                    if !response.is_empty() {
                        //cortex_m::asm::bkpt();
                        for c in response {
                            serial_tx.write_char(*c as char).unwrap();
                        }
                    }
                }
                // Clear line buffer
                reader.clear();
            }
        }
        // Update servos
        let servos = servos.borrow();
        let mut on_time = [1500u16; 16];
        for (index, s) in servos.iter().step_by(2).enumerate() {
            on_time[index] = s.pulse_width;
        }
        servos_1.set_all_on_off(&[0u16; 16], &on_time).unwrap();
        for (index, s) in servos.iter().skip(1).step_by(2).enumerate() {
            on_time[index] = s.pulse_width;
        }
        servos_2.set_all_on_off(&[0u16; 16], &on_time).unwrap();


    }
}

#[exception]
fn HardFault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}
