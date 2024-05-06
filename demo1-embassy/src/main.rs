#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::*;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::{gpio, peripherals};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::{Mutex, NoopMutex};
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use gpio::{Input, Level, Output, Pull};
use static_cell::StaticCell;
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

type LedOutput = Output<'static, peripherals::PIN_25>;
type ButtonInput = Input<'static, peripherals::PIN_16>;

static SPI_BUS: StaticCell<NoopMutex<RefCell<MySpiBus>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Program start");

    let p = embassy_rp::init(Default::default());

    let spi_bus = SPI_BUS.init(NoopMutex::new(RefCell::new(init_my_spi_bus(
        p.PIN_12, p.PIN_11, p.PIN_10, p.SPI1,
    ))));
    let display_spi = SpiDeviceWithConfig::new(
        spi_bus,
        Output::new(p.PIN_13, Level::High),
        init_display_spi_config(),
    );

    let di = hardware::display_interface::SPIDeviceInterface::new(
        display_spi,
        Output::new(p.PIN_15, Level::Low),
    );
    let display: MyDisplay = {
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

    let led: LedOutput = Output::new(p.PIN_25, Level::Low);
    let button: ButtonInput = Input::new(p.PIN_16, Pull::Up);

    unwrap!(spawner.spawn(blinker(led, Duration::from_millis(500))));
    unwrap!(spawner.spawn(button_monitor(button)));
    unwrap!(spawner.spawn(display_refresh(display)));
}

/// Blink the physical LED, and a matching indicator on the LCD display
#[embassy_executor::task]
async fn blinker(mut led: LedOutput, interval: Duration) {
    let mut blink = false;
    loop {
        led.set_level(if blink { Level::Low } else { Level::High });
        display_state_update(|s| s.indicator1 = blink);
        blink = !blink;
        Timer::after(interval).await;
    }
}

/// Monitor the button, and show an indicator on the LCD display
#[embassy_executor::task]
async fn button_monitor(mut button: ButtonInput) {
    loop {
        button.wait_for_any_edge().await;
        let level = button.get_level();
        display_state_update(|s| s.indicator2 = level == Level::High);
    }
}

#[derive(Clone)]
struct DisplayState {
    indicator1: bool,
    indicator2: bool,
}

static DISPLAY_STATE: Mutex<ThreadModeRawMutex, RefCell<DisplayState>> =
    Mutex::new(RefCell::new(DisplayState {
        indicator1: false,
        indicator2: false,
    }));
static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Keep the display up to date
#[embassy_executor::task]
async fn display_refresh(mut display: MyDisplay) {
    let styles = display::Styles::new();
    render_background(&mut display, &styles);
    loop {
        DISPLAY_SIGNAL.wait().await;
        let state = DISPLAY_STATE.lock(|s| s.borrow().clone());
        render_indicator(&mut display, Point::new(120, 120), state.indicator1);
        render_indicator(&mut display, Point::new(180, 120), state.indicator2);
    }
}

fn display_state_update<F>(mut sfn: F)
where
    F: FnMut(&mut DisplayState) -> (),
{
    DISPLAY_STATE.lock(|s| sfn(&mut s.borrow_mut()));
    DISPLAY_SIGNAL.signal(());
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
fn render_indicator(display: &mut MyDisplay, centre: Point, state: bool) -> () {
    let led_size: u32 = 30;
    let led_at = Point::new(
        centre.x - (led_size as i32) / 2,
        centre.y - (led_size as i32) / 2,
    );

    if state {
        Circle::new(led_at, led_size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::GREEN)
                    .build(),
            )
            .draw(display)
            .unwrap();
    } else {
        Circle::new(led_at, led_size)
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::CSS_DARK_GRAY)
                    .build(),
            )
            .draw(display)
            .unwrap();
    }
}
