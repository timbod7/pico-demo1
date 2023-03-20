use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals;
use embassy_rp::spi;
use embassy_rp::spi::{Blocking, Spi};
use ili9341::{Ili9341, Orientation};

use core::cell::RefCell;

use display_interface::{DataFormat, DisplayError, WriteOnlyDataCommand};
use embedded_hal_1::digital::OutputPin;
use embedded_hal_1::spi::{SpiBusWrite, SpiDevice};

use self::shared_spi::SpiDeviceWithCs;

const DISPLAY_FREQ: u32 = 16_000_000;
const TOUCH_FREQ: u32 = 200_000;

pub struct TouchDisplay<'a> {
    pub touch: MyTouch<'a>,
    pub display: Display<'a>,
}

type SharedSpi<'a> = Spi<'a, peripherals::SPI1, Blocking>;
type MyTouch<'a> = touch::Touch<SpiDeviceWithCs<'a, SharedSpi<'a>, Output<'a, peripherals::PIN_9>>>;
type Display<'a> = Ili9341<
    SPIDisplayInterface<
        SpiDeviceWithCs<'a, SharedSpi<'a>, Output<'a, peripherals::PIN_13>>,
        Output<'a, peripherals::PIN_15>,
    >,
    Output<'a, peripherals::PIN_14>,
>;

pub fn new_display_spi<'a>(
    spi1: peripherals::SPI1,
    miso: peripherals::PIN_12,
    mosi: peripherals::PIN_11,
    clk: peripherals::PIN_10,
) -> RefCell<SharedSpi<'a>> {
    // create SPI
    let mut config = spi::Config::default();
    config.phase = spi::Phase::CaptureOnSecondTransition;
    config.polarity = spi::Polarity::IdleHigh;
    let spi = Spi::new_blocking(spi1, clk, mosi, miso, config);
    RefCell::new(spi)
}

pub fn new_touch_display<'a>(
    spi_bus: &'a RefCell<SharedSpi<'a>>,
    display_cs: peripherals::PIN_13,
    touch_cs: peripherals::PIN_9,
    reset: peripherals::PIN_14,
    dc: peripherals::PIN_15,
) -> TouchDisplay<'a> {
    let display_spi =
        SpiDeviceWithCs::new(&spi_bus, Output::new(display_cs, Level::High), DISPLAY_FREQ);
    let touch_spi = SpiDeviceWithCs::new(&spi_bus, Output::new(touch_cs, Level::High), TOUCH_FREQ);

    let touch: MyTouch = touch::Touch::new(touch_spi);

    let di = SPIDisplayInterface::new(display_spi, Output::new(dc, Level::Low));
    let mut delay = embassy_time::Delay {};

    let display = Ili9341::new(
        di,
        Output::new(reset, Level::Low),
        &mut delay,
        Orientation::LandscapeFlipped,
        ili9341::DisplaySize240x320,
    )
    .unwrap();

    TouchDisplay { touch, display }
}

/// SPI display interface.
///
/// This combines the SPI peripheral and a data/command pin
pub struct SPIDisplayInterface<SPI, DC> {
    spi: SPI,
    dc: DC,
}

impl<SPI, DC> SPIDisplayInterface<SPI, DC>
where
    SPI: SpiDevice,
    SPI::Bus: SpiBusWrite,
    DC: OutputPin,
{
    /// Create new SPI interface for communciation with a display driver
    pub fn new(spi: SPI, dc: DC) -> Self {
        Self { spi, dc }
    }
}

impl<SPI, DC> WriteOnlyDataCommand for SPIDisplayInterface<SPI, DC>
where
    SPI: SpiDevice,
    SPI::Bus: SpiBusWrite,
    DC: OutputPin,
{
    fn send_commands(&mut self, cmds: DataFormat<'_>) -> Result<(), DisplayError> {
        let r = self.spi.transaction(|bus| {
            // 1 = data, 0 = command
            if let Err(_) = self.dc.set_low() {
                return Ok(Err(DisplayError::DCError));
            }

            // Send words over SPI
            send_u8(bus, cmds)?;

            Ok(Ok(()))
        });
        r.map_err(|_| DisplayError::BusWriteError)?
    }

    fn send_data(&mut self, buf: DataFormat<'_>) -> Result<(), DisplayError> {
        let r = self.spi.transaction(|bus| {
            // 1 = data, 0 = command
            if let Err(_) = self.dc.set_high() {
                return Ok(Err(DisplayError::DCError));
            }

            // Send words over SPI
            send_u8(bus, buf)?;

            Ok(Ok(()))
        });
        r.map_err(|_| DisplayError::BusWriteError)?
    }
}

fn send_u8<T: SpiBusWrite>(spi: &mut T, words: DataFormat<'_>) -> Result<(), T::Error> {
    match words {
        DataFormat::U8(slice) => spi.write(slice),
        DataFormat::U16(slice) => {
            use byte_slice_cast::*;
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16LE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_le();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U16BE(slice) => {
            use byte_slice_cast::*;
            for v in slice.as_mut() {
                *v = v.to_be();
            }
            spi.write(slice.as_byte_slice())
        }
        DataFormat::U8Iter(iter) => {
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.into_iter() {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf)?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i])?;
            }

            Ok(())
        }
        DataFormat::U16LEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 32];
            let mut i = 0;

            for v in iter.map(u16::to_le) {
                buf[i] = v;
                i += 1;

                if i == buf.len() {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        DataFormat::U16BEIter(iter) => {
            use byte_slice_cast::*;
            let mut buf = [0; 64];
            let mut i = 0;
            let len = buf.len();

            for v in iter.map(u16::to_be) {
                buf[i] = v;
                i += 1;

                if i == len {
                    spi.write(&buf.as_byte_slice())?;
                    i = 0;
                }
            }

            if i > 0 {
                spi.write(&buf[..i].as_byte_slice())?;
            }

            Ok(())
        }
        _ => unimplemented!(),
    }
}

pub mod shared_spi {
    use core::cell::RefCell;
    use core::fmt::Debug;

    use embedded_hal_1::digital::OutputPin;
    use embedded_hal_1::spi;
    use embedded_hal_1::spi::SpiDevice;

    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    pub enum SpiDeviceWithCsError<BUS, CS> {
        #[allow(unused)] // will probably use in the future when adding a flush() to SpiBus
        Spi(BUS),
        Cs(CS),
    }

    impl<BUS, CS> spi::Error for SpiDeviceWithCsError<BUS, CS>
    where
        BUS: spi::Error + Debug,
        CS: Debug,
    {
        fn kind(&self) -> spi::ErrorKind {
            match self {
                Self::Spi(e) => e.kind(),
                Self::Cs(_) => spi::ErrorKind::Other,
            }
        }
    }

    pub struct SpiDeviceWithCs<'a, BUS, CS> {
        bus: &'a RefCell<BUS>,
        cs: CS,
        freq: u32,
    }

    impl<'a, BUS, CS> SpiDeviceWithCs<'a, BUS, CS> {
        pub fn new(bus: &'a RefCell<BUS>, cs: CS, freq: u32) -> Self {
            Self { bus, cs, freq }
        }
    }

    impl<'a, BUS, CS> spi::ErrorType for SpiDeviceWithCs<'a, BUS, CS>
    where
        BUS: spi::ErrorType,
        CS: OutputPin,
    {
        type Error = SpiDeviceWithCsError<BUS::Error, CS::Error>;
    }

    impl<'a, BUS, CS> SpiDevice for SpiDeviceWithCs<'a, BUS, CS>
    where
        BUS: spi::SpiBusFlush + crate::display::SpiSetFrequency,
        CS: OutputPin,
    {
        type Bus = BUS;

        fn transaction<R>(
            &mut self,
            f: impl FnOnce(&mut Self::Bus) -> Result<R, BUS::Error>,
        ) -> Result<R, Self::Error> {
            let mut bus = self.bus.borrow_mut();
            bus.set_frequency(self.freq);
            self.cs.set_low().map_err(SpiDeviceWithCsError::Cs)?;

            let f_res = f(&mut bus);

            // On failure, it's important to still flush and deassert CS.
            let flush_res = bus.flush();
            let cs_res = self.cs.set_high();

            let f_res = f_res.map_err(SpiDeviceWithCsError::Spi)?;
            flush_res.map_err(SpiDeviceWithCsError::Spi)?;
            cs_res.map_err(SpiDeviceWithCsError::Cs)?;

            Ok(f_res)
        }
    }
}

pub trait SpiSetFrequency {
    fn set_frequency(&mut self, freq: u32);
}

impl<'d, T: embassy_rp::spi::Instance, M: embassy_rp::spi::Mode> SpiSetFrequency for Spi<'d, T, M> {
    fn set_frequency(&mut self, freq: u32) {
        self.set_frequency(freq);
    }
}

/// Driver for the XPT2046 resistive touchscreen sensor
pub mod touch {
    use embedded_hal_1::spi::{SpiBus, SpiBusRead, SpiBusWrite, SpiDevice};

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
        SPI::Bus: SpiBus,
    {
        pub fn new(spi: SPI) -> Self {
            Self { spi }
        }

        pub fn read(&mut self) -> Option<(i32, i32)> {
            let mut x = [0; 2];
            let mut y = [0; 2];
            self.spi
                .transaction(|bus| {
                    bus.write(&[0x90])?;
                    bus.read(&mut x)?;
                    bus.write(&[0xd0])?;
                    bus.read(&mut y)?;
                    Ok(())
                })
                .unwrap();

            let x = (u16::from_be_bytes(x) >> 3) as i32;
            let y = (u16::from_be_bytes(y) >> 3) as i32;

            let cal = &CALIBRATION;

            let x = cal.sx - ((x - cal.x1) * cal.sx / (cal.x2 - cal.x1)).clamp(0, cal.sx);
            let y = ((y - cal.y1) * cal.sy / (cal.y2 - cal.y1)).clamp(0, cal.sy);
            if x == 0 && y == 0 {
                None
            } else {
                Some((x, y))
            }
        }
    }
}
