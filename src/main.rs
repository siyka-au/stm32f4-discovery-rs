#![deny(unsafe_code)]
#![no_main]
#![no_std]

mod image;

use cortex_m_rt::entry;
use panic_halt as _;

use stm32f4xx_hal::{
    self as hal,
    gpio::{Pull, Speed},
    spi::{NoMiso, Spi},
};

use crate::hal::{pac, prelude::*};

use embedded_hal::{
    digital::{ErrorType, OutputPin, PinState},
    spi::{Mode, Phase, Polarity},
};
use embedded_hal_bus::spi::ExclusiveDevice;

use core::convert::Infallible;

use display_interface_spi::SPIInterface;
use ili9341::{DisplaySize240x320, Ili9341, Orientation};

use embedded_graphics::{
    geometry::Point,
    pixelcolor::{Rgb565, RgbColor as _},
    prelude::*,
    text::{Alignment, Text},
};

use u8g2_fonts::{fonts, types::{FontColor, HorizontalAlignment, VerticalPosition}, FontRenderer};

#[allow(clippy::empty_loop)]
#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        pac::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = &mut rcc.cfgr.sysclk(160.MHz()).freeze();

        let systimer = cp.SYST;
        let mut delay = systimer.delay(&clocks);

        // let mut delay = dp.TIM1.delay_us(&clocks);

        // Set up I2C - SCL is PB8 and SDA is PB9; they are set to Alternate Function 4
        // as per the STM32F446xC/E datasheet page 60. Pin assignment as per the Nucleo-F446 board.
        let gpioa = dp.GPIOA.split();
        let gpiob = dp.GPIOB.split();
        let gpioc = dp.GPIOC.split();
        let gpiod = dp.GPIOD.split();
        let gpiof = dp.GPIOF.split();
        let gpiog = dp.GPIOG.split();

        let btn = gpioa.pa0.into_pull_down_input();

        let display_cs = gpioc.pc2.into_push_pull_output();

        let display_data_cmd = gpiod.pd13.into_push_pull_output();

        let mut green_led = gpiog.pg13.into_push_pull_output();

        let mut red_led = gpiog.pg14.into_push_pull_output();

        green_led.toggle();

        let sclk = gpiof
            .pf7
            .into_alternate()
            .internal_resistor(Pull::Up)
            .speed(Speed::Medium);

        let mosi = gpiof
            .pf9
            .into_alternate()
            .internal_resistor(Pull::Up)
            .speed(Speed::Medium);

        let miso = NoMiso::default();

        let mode = Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        };

        let spi = dp.SPI5.spi((sclk, miso, mosi), mode, 42.MHz(), &clocks);
        let display_bus_dev = ExclusiveDevice::new_no_delay(spi, display_cs).unwrap();

        let interface = SPIInterface::new(display_bus_dev, display_data_cmd);

        #[derive(Default)]
        pub struct DummyOutputPin;

        impl ErrorType for DummyOutputPin {
            type Error = Infallible;
        }

        impl OutputPin for DummyOutputPin {
            fn set_low(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
            fn set_high(&mut self) -> Result<(), Self::Error> {
                Ok(())
            }
            fn set_state(&mut self, _state: PinState) -> Result<(), Self::Error> {
                Ok(())
            }
        }

        let dummy_reset = DummyOutputPin::default();

        let mut display = Ili9341::new(
            interface,
            dummy_reset,
            &mut delay,
            Orientation::Portrait,
            DisplaySize240x320,
        )
        .unwrap();

        display.clear(Rgb565::BLACK).unwrap();

        let font = FontRenderer::new::<fonts::u8g2_font_inr21_mr>();

        // embedded_graphics::image::ImageRawLE::<Rgb565>::new(&image::IMAGE, 240)
        //     .draw(&mut display)
        //     .unwrap();

        // Set up state for the loop
        // let mut orientation = DisplayRotation::Rotate0;
        let mut was_pressed = btn.is_low();

        let mut i = 0u32;

        fn get_bit_at(input: u32, n: u8) -> bool {
            if n < 32 {
                input & (1 << n) != 0
            } else {
                false
            }
        }

        // This runs continuously, as fast as possible
        loop {
            // Check if the button has just been pressed.
            // Remember, active low.
            let is_pressed = btn.is_low();
            if !was_pressed && is_pressed {
                // Since the button was pressed, flip the screen upside down
                // orientation = get_next_rotation(orientation);
                // disp.set_rotation(orientation).unwrap();
                i += 1;
                // Now that we've flipped the screen, store the fact that the button is pressed.
                was_pressed = true;


                font.render_aligned(
                    format_args!("i={}", i),
                    display.bounding_box().center() + Point::new(10, 10),
                    VerticalPosition::Baseline,
                    HorizontalAlignment::Right,
                    FontColor::WithBackground { fg: Rgb565::GREEN, bg: Rgb565::BLACK },
                    &mut display,
                )
                .unwrap();

            } else if !is_pressed {
                // If the button is released, confirm this so that next time it's pressed we'll
                // know it's time to flip the screen.
                was_pressed = false;
            }

            green_led.set_state(stm32f4xx_hal::gpio::PinState::from(get_bit_at(i, 0)));
            red_led.set_state(stm32f4xx_hal::gpio::PinState::from(get_bit_at(i, 1)));
        }
    }

    loop {}
}
