#![allow(unsafe_code)]
#![no_main]
#![no_std]

mod scpi_framer;

use panic_halt as _;

use cortex_m::{asm, singleton};
use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use core::fmt::Write;

// HAL
use stm32f4xx_hal::stm32;
use stm32f4xx_hal::stm32::{
    interrupt,
};
use stm32f4xx_hal::{
    prelude::*,
    delay::Delay,
    serial
};

use heapless::spsc::Queue;
use heapless::consts::*;

use lazy_static::lazy_static;

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

// Semihosting
//use cortex_m_semihosting::hprintln;

// Git version
use git_version::git_version;
use stm32f4xx_hal::serial::config::Config;

const GIT_VERSION: &[u8] = git_version!().as_bytes();

mod jetson;
mod buffer;



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
fn USART2(){
}

#[entry]
fn main() -> ! {


    let dp = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    // Set up the system clock.
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze();
    let mut delay = Delay::new(cp.SYST, clocks);

    let gpioa = dp.GPIOA.split();
    let mut pa2 = gpioa.pa2;
    let mut pa3 = gpioa.pa3.into_push_pull_output();
    let mut pa9 = gpioa.pa9.into_push_pull_output();
    let gpiob = dp.GPIOB.split();
    let gpioc = dp.GPIOC.split();

    // Start jetson
    pa3.set_low().unwrap();
    delay.delay_ms(100u8);
    pa9.set_high();
    delay.delay_ms(100u8);
    pa9.set_low();
    delay.delay_ms(100u8);

    // SCPI serial
    dp.USART2.cr1.modify(|_,w| w.rxneie().set_bit());
    let (serial_tx, serial_rx) = serial::Serial::usart2(
        dp.USART2,
        (pa2.into_alternate_af7(), pa3.into_alternate_af7()),
        serial::config::Config::default()
            .baudrate(115200.bps()),
        clocks,
    ).unwrap().split();


    // SCPI
    let mut my_device = MyDevice { };

    let tree = Node {name: b"ROOT", optional: true, handler: None, sub: Some(&[
        // Create default IEEE488 mandated commands
        Node {name: b"*IDN", optional: false,
            handler: Some(&IdnCommand{
                manufacturer: b"GPA-Robotics",
                model: b"ash-carrier",
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

    let mut context = Context::new(&mut my_device, &mut errors, &tree);

    loop {

    }
}