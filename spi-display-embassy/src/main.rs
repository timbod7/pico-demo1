#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::cell::RefCell;

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::spi;
use embassy_rp::spi::Spi;

use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::text::Text;

use crate::display::{new_display_spi, new_touch_display, TouchDisplay};

use {defmt_rtt as _, panic_probe as _};

mod display;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Hello World!");

    let rst = p.PIN_14;
    let display_cs = p.PIN_13;
    let dc = p.PIN_15;
    let miso = p.PIN_12;
    let mosi = p.PIN_11;
    let clk = p.PIN_10;
    let touch_cs = p.PIN_9;

    let spi_bus = new_display_spi(p.SPI1, miso, mosi, clk);

    let TouchDisplay {
        mut touch,
        mut display,
    } = new_touch_display(&spi_bus, display_cs, touch_cs, rst, dc);

    display.clear(Rgb565::BLACK).unwrap();

    let raw_image_data = ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(34, 68));

    // Display the image
    ferris.draw(&mut display).unwrap();

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    Text::new(
        "Hello embedded_graphics \n + embassy + RP2040!",
        Point::new(20, 200),
        style,
    )
    .draw(&mut display)
    .unwrap();

    loop {
        if let Some((x, y)) = touch.read() {
            let style = PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLUE)
                .build();

            Rectangle::new(Point::new(x - 1, y - 1), Size::new(3, 3))
                .into_styled(style)
                .draw(&mut display)
                .unwrap();
        }
    }
}
