use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{
    gpio::{Input, Output},
    peripherals, spi,
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use ili9341::Ili9341;

pub type LedOutput = Output<'static, peripherals::PIN_25>;
pub type ButtonInput = Input<'static, peripherals::PIN_16>;

pub type MySpiBus = spi::Spi<'static, peripherals::SPI1, spi::Blocking>;

pub type MyDisplay = Ili9341<
    display_interface_spi::SPIInterface<
        SpiDeviceWithConfig<
            'static,
            NoopRawMutex,
            embassy_rp::spi::Spi<'static, peripherals::SPI1, embassy_rp::spi::Blocking>,
            Output<'static, peripherals::PIN_13>,
        >,
        Output<'static, peripherals::PIN_15>,
    >,
    Output<'static, peripherals::PIN_14>,
>;

pub fn init_my_spi_bus(
    miso: peripherals::PIN_12,
    mosi: peripherals::PIN_11,
    clk: peripherals::PIN_10,
    spi: peripherals::SPI1,
) -> MySpiBus {
    let mut config = spi::Config::default();
    config.frequency = 200_000;
    config.polarity = spi::Polarity::IdleLow;
    config.phase = spi::Phase::CaptureOnFirstTransition;
    spi::Spi::new_blocking(spi, clk, mosi, miso, config)
}

pub fn init_display_spi_config() -> spi::Config {
    let mut config = spi::Config::default();
    config.frequency = 16_000_000;
    config.polarity = spi::Polarity::IdleLow;
    config.phase = spi::Phase::CaptureOnFirstTransition;
    config
}
