use embassy_rp::{gpio, peripherals, spi};
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::RgbColor,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder},
    text::{Baseline, TextStyle},
};
use gpio::{Level, Output};
use {defmt_rtt as _, panic_probe as _};

use display_interface_spi::SPIInterface;
use ili9341::{Ili9341, Orientation};

pub type DisplayInterface = Ili9341<
    SPIInterface<
        embassy_rp::spi::Spi<'static, peripherals::SPI1, embassy_rp::spi::Blocking>,
        Output<'static, peripherals::PIN_15>,
        Output<'static, peripherals::PIN_13>,
    >,
    Output<'static, peripherals::PIN_14>,
>;

pub struct Display {
    pub interface: DisplayInterface,
    pub styles: Styles,
}

pub fn init(
    miso: peripherals::PIN_12,
    mosi: peripherals::PIN_11,
    clk: peripherals::PIN_10,
    cs: peripherals::PIN_13,
    reset: peripherals::PIN_14,
    dc: peripherals::PIN_15,
    spi: peripherals::SPI1,
) -> Display {
    let cs = Output::new(cs, Level::High);
    let reset = Output::new(reset, Level::Low);
    let dc = Output::new(dc, Level::Low);

    let display_spi = {
        let mut config = spi::Config::default();
        config.frequency = 16_000_000;
        config.polarity = spi::Polarity::IdleLow;
        config.phase = spi::Phase::CaptureOnFirstTransition;
        spi::Spi::new_blocking(spi, clk, mosi, miso, config)
    };

    let interface: DisplayInterface = {
        let mut delay = embassy_time::Delay {};
        Ili9341::new(
            SPIInterface::new(display_spi, dc, cs),
            reset,
            &mut delay,
            Orientation::LandscapeFlipped,
            ili9341::DisplaySize240x320,
        )
        .unwrap()
    };

    let styles = Styles::new();

    Display { interface, styles }
}

/// Some shared styles
pub struct Styles {
    pub char: MonoTextStyle<'static, Rgb565>,
    pub text: TextStyle,
    pub black_fill: PrimitiveStyle<Rgb565>,
    pub white_fill: PrimitiveStyle<Rgb565>,
}

impl Styles {
    fn new() -> Styles {
        let char = MonoTextStyle::new(&profont::PROFONT_24_POINT, Rgb565::WHITE);
        let text = TextStyle::with_baseline(Baseline::Top);
        let black_fill = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::BLACK)
            .build();
        let white_fill = PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::WHITE)
            .build();
        Styles {
            char,
            text,
            black_fill,
            white_fill,
        }
    }
}
