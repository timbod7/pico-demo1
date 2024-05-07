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
type MySpiDevice<CS> = SpiDeviceWithConfig<'static, NoopRawMutex, MySpiBus, CS>;

pub type MyDisplay = Ili9341<
    display_interface_spi::SPIInterface<
        MySpiDevice<Output<'static, peripherals::PIN_13>>,
        Output<'static, peripherals::PIN_15>,
    >,
    Output<'static, peripherals::PIN_14>,
>;

pub type MyTouch = touch::Touch<MySpiDevice<Output<'static, peripherals::PIN_9>>>;

pub fn init_my_spi_bus(
    miso: peripherals::PIN_12,
    mosi: peripherals::PIN_11,
    clk: peripherals::PIN_10,
    spi: peripherals::SPI1,
) -> MySpiBus {
    let config = init_touch_spi_config();
    spi::Spi::new_blocking(spi, clk, mosi, miso, config)
}

pub fn init_display_spi_config() -> spi::Config {
    let mut config = spi::Config::default();
    config.frequency = 16_000_000;
    config.polarity = spi::Polarity::IdleLow;
    config.phase = spi::Phase::CaptureOnFirstTransition;
    config
}

pub fn init_touch_spi_config() -> spi::Config {
    let mut config = spi::Config::default();
    config.frequency = 200_000;
    config.polarity = spi::Polarity::IdleLow;
    config.phase = spi::Phase::CaptureOnFirstTransition;
    config
}

/// Driver for the XPT2046 resistive touchscreen sensor
pub mod touch {
    use embedded_hal_1::spi::{Operation, SpiDevice};

    struct Calibration {
        x1: i32,
        x2: i32,
        y1: i32,
        y2: i32,
        sx: i32,
        sy: i32,
    }

    const CALIBRATION: Calibration = Calibration {
        x1: 3880,
        x2: 340,
        y1: 262,
        y2: 3850,
        sx: 320,
        sy: 240,
    };

    pub struct Touch<SPI: SpiDevice> {
        spi: SPI,
    }

    impl<SPI> Touch<SPI>
    where
        SPI: SpiDevice,
    {
        pub fn new(spi: SPI) -> Self {
            Self { spi }
        }

        pub fn read(&mut self) -> Option<(i32, i32)> {
            let mut x = [0; 2];
            let mut y = [0; 2];
            self.spi
                .transaction(&mut [
                    Operation::Write(&[0x90]),
                    Operation::Read(&mut x),
                    Operation::Write(&[0xd0]),
                    Operation::Read(&mut y),
                ])
                .unwrap();

            let x = (u16::from_be_bytes(x) >> 3) as i32;
            let y = (u16::from_be_bytes(y) >> 3) as i32;

            let cal = &CALIBRATION;

            let x = ((x - cal.x1) * cal.sx / (cal.x2 - cal.x1)).clamp(0, cal.sx);
            let y = ((y - cal.y1) * cal.sy / (cal.y2 - cal.y1)).clamp(0, cal.sy);
            if x == 0 && y == 0 {
                None
            } else {
                Some((x, y))
            }
        }
    }
}
