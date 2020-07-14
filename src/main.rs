#![allow(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

use cortex_m::{asm, singleton};
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use core::fmt::Write;

// HAL
use stm32f4xx_hal::stm32::{
    interrupt,
    I2C1
};
use stm32f4xx_hal::{
    prelude::*,
};

// I2C Stuff
use shared_bus::BusManager;

//Default commands
use scpi::prelude::*;
use scpi::ieee488::commands::*;
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
    scpi_status,
    scpi_system,

    //Helpers
    qonly
};
use scpi::response::{ArrayVecFormatter, Formatter};

use core::str;
use git_version::git_version;

use cortex_m_semihosting::hprintln;

const GIT_VERSION: &[u8] = git_version!().as_bytes();

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

//***********************************************************************************
/// # Main code

#[interrupt]
fn USART1(){

}

#[entry]
fn main() -> ! {

    // SCPI
    let mut my_device = MyDevice { };

    let mut tree = Node {name: b"ROOT", optional: true, handler: None, sub: Some(&[
        // Create default IEEE488 mandated commands
        Node {name: b"*IDN", optional: false,
            handler: Some(&IdnCommand{
                manufacturer: b"GPA-Robotics",
                model: b"ash-power",
                serial: b"0",
                firmware: GIT_VERSION
            }),
            sub: None
        },
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
    ])};

    let mut errors = ArrayErrorQueue::<[Error; 10]>::new();

    let mut context = Context::new(&mut my_device, &mut errors, &mut tree);


    loop {

    }
}