#![no_std]
#![no_main]

use crate::hardware::{init_touch_spi_config, touch::Touch, MyTouch};
use core::cell::RefCell;
use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio;
use embassy_sync::blocking_mutex::NoopMutex;
use embassy_time::{Duration, Timer};
use gpio::{Input, Level, Output, Pull};
use static_cell::StaticCell;
use utils::StateAndSignal;

use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};

use ili9341::Ili9341;
use ili9341::Orientation;

use hardware::{init_display_spi_config, init_my_spi_bus, MyDisplay, MySpiBus};

mod display;
mod hardware;
mod utils;

static SPI_BUS: StaticCell<NoopMutex<RefCell<MySpiBus>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start");

    let p = embassy_rp::init(Default::default());

    let spi_bus = SPI_BUS.init(NoopMutex::new(RefCell::new(init_my_spi_bus(
        p.PIN_12, p.PIN_11, p.PIN_10, p.SPI1,
    ))));

    let display: MyDisplay = {
        let spi_device = SpiDeviceWithConfig::new(
            spi_bus,
            Output::new(p.PIN_13, Level::High),
            init_display_spi_config(),
        );
        let di =
            display_interface_spi::SPIInterface::new(spi_device, Output::new(p.PIN_15, Level::Low));
        let mut delay = embassy_time::Delay {};
        Ili9341::new(
            di,
            Output::new(p.PIN_14, Level::Low),
            &mut delay,
            Orientation::LandscapeFlipped,
            ili9341::DisplaySize240x320,
        )
        .unwrap()
    };

    let touch = {
        let spi_device = SpiDeviceWithConfig::new(
            spi_bus,
            Output::new(p.PIN_9, Level::High),
            init_touch_spi_config(),
        );
        Touch::new(spi_device)
    };

    let led = Output::new(p.PIN_25, Level::Low);
    let button = Input::new(p.PIN_16, Pull::Up);

    unwrap!(spawner.spawn(blinker(led, Duration::from_millis(200))));
    unwrap!(spawner.spawn(button_monitor(button)));
    unwrap!(spawner.spawn(touch_monitor(touch)));
    unwrap!(spawner.spawn(display_refresh(display)));
}

/// Blink the physical LED, and a matching indicator on the LCD display
#[embassy_executor::task]
async fn blinker(mut led: Output<'static>, interval: Duration) {
    let mut blink = false;
    loop {
        led.set_level(if blink { Level::Low } else { Level::High });
        let istate = IndicatorState::from_bool(blink);
        DISPLAY_STATE.update(|s| s.indicator1 = istate);
        blink = !blink;
        Timer::after(interval).await;
    }
}

/// Monitor the button, and show an indicator on the LCD display
#[embassy_executor::task]
async fn button_monitor(mut button: Input<'static>) {
    loop {
        button.wait_for_any_edge().await;
        let level = button.get_level();
        let istate = IndicatorState::from_bool(level == Level::High);
        DISPLAY_STATE.update(|s| s.indicator2 = istate);
    }
}

/// Monitor the touch screen, and show an indicator on the LCD display
#[embassy_executor::task]
async fn touch_monitor(mut touch: MyTouch) {
    loop {
        Timer::after_millis(100).await;
        let _ = touch.read();
    }
}

#[derive(Clone)]
struct DisplayState {
    indicator1: IndicatorState,
    indicator2: IndicatorState,
}

#[derive(Clone, Copy)]
enum IndicatorState {
    GRAY,
    RED,
    GREEN,
    BLUE,
}

impl IndicatorState {
    fn from_bool(v: bool) -> Self {
        if v {
            IndicatorState::GREEN
        } else {
            IndicatorState::GRAY
        }
    }
}

static DISPLAY_STATE: StateAndSignal<DisplayState, ()> = StateAndSignal::new(DisplayState {
    indicator1: IndicatorState::GRAY,
    indicator2: IndicatorState::GRAY,
});

// Keep the display up to date
#[embassy_executor::task]
async fn display_refresh(mut display: MyDisplay) {
    let styles = display::Styles::new();
    render_background(&mut display, &styles);
    loop {
        let state = DISPLAY_STATE.wait(|_, s| s.clone()).await;
        render_indicator(&mut display, Point::new(120, 120), state.indicator1);
        render_indicator(&mut display, Point::new(180, 120), state.indicator2);
    }
}

fn render_background(display: &mut MyDisplay, styles: &display::Styles) {
    let test_text = "Pixel Blinky";
    Rectangle::new(Point::new(0, 0), Size::new(320, 240))
        .into_styled(styles.black_fill)
        .draw(display)
        .unwrap();
    Text::with_text_style(test_text, Point::new(60, 0), styles.char, styles.text)
        .draw(display)
        .unwrap();
}

/// Draw an "LED" on the LCD display
fn render_indicator(display: &mut MyDisplay, centre: Point, state: IndicatorState) -> () {
    let led_size: u32 = 30;
    let led_at = Point::new(
        centre.x - (led_size as i32) / 2,
        centre.y - (led_size as i32) / 2,
    );

    let color = match state {
        IndicatorState::GRAY => Rgb565::CSS_DARK_GRAY,
        IndicatorState::RED => Rgb565::RED,
        IndicatorState::GREEN => Rgb565::GREEN,
        IndicatorState::BLUE => Rgb565::BLUE,
    };

    Circle::new(led_at, led_size)
        .into_styled(PrimitiveStyleBuilder::new().fill_color(color).build())
        .draw(display)
        .unwrap();
}
