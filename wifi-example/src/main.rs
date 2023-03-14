#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::cell::RefCell;

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources, StaticConfig, Ipv4Cidr, IpAddress};
use embassy_rp::gpio::{Flex, Level, Output};
use embassy_rp::peripherals::{PIN_23, PIN_25};
use embassy_sync::blocking_mutex::raw::{CriticalSectionRawMutex, ThreadModeRawMutex};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Timer};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::Text;
use embedded_hal_async::spi::{ExclusiveDevice};
use embedded_io::asynch::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use heapless::String;
use ufmt::uwrite;

use wifi_spi::WifiSpi;

mod wifi_spi;
mod display;

macro_rules! singleton {
    ($val:expr) => {{
        type T = impl Sized;
        static STATIC_CELL: StaticCell<T> = StaticCell::new();
        STATIC_CELL.init_with(move || $val)
    }};
}

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static, PIN_23>, ExclusiveDevice<WifiSpi, Output<'static, PIN_25>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    let p = embassy_rp::init(Default::default());

    // Include the WiFi firmware and Country Locale Matrix (CLM) blobs.
    let fw = include_bytes!("../firmware/43439A0.bin");
    let clm = include_bytes!("../firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs-cli download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs-cli download 43439A0.clm_blob --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 224190) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let clk = Output::new(p.PIN_29, Level::Low);
    let mut dio = Flex::new(p.PIN_24);
    dio.set_low();
    dio.set_as_output();

    let bus = WifiSpi { clk, dio };
    let spi = ExclusiveDevice::new(bus, cs);

    let state = singleton!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    spawner.spawn(wifi_task(runner)).unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    //control.join_open(env!("WIFI_NETWORK")).await;
    let ssid = env!("WIFI_NETWORK");
    let password =  env!("WIFI_PASSWORD");

    control.join_wpa2(ssid, password).await;

    let config = Config::Dhcp(Default::default());
    //let config = embassy_net::Config::Static(embassy_net::Config {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

    // Init network stack
    let stack = &*singleton!(Stack::new(
        net_device,
        config,
        singleton!(StackResources::<2>::new()),
        seed
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    // And now we can use it!

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    // Keep the display up to date
    let display = display::init(p.PIN_12, p.PIN_11, p.PIN_10, p.PIN_13, p.PIN_14, p.PIN_15, p.SPI1);
    unwrap!(spawner.spawn(display_refresh(display)));
    display_state_update(|ds| {
        ds.ssid = ssid;
        ds.connected = None;
    });

    let config = wait_for_config(stack).await;
    display_state_update(|ds| {
        ds.address = Some(config.address);
    });

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(embassy_net::SmolDuration::from_secs(10)));

        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        info!("Received connection from {:?}", socket.remote_endpoint());

        display_state_update(|ds| ds.connected = socket.remote_endpoint().map(|ep| ep.addr));

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {:02x}", &buf[..n]);

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
        }

        display_state_update(|ds| ds.connected = None);
    }
}

async fn wait_for_config(stack: &'static Stack<cyw43::NetDriver<'static>>) -> StaticConfig {
    loop {
        if let Some(config) = stack.config() {
            return config.clone();
        }
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[derive(Clone)]
struct DisplayState {
    address: Option<Ipv4Cidr>,
    ssid: &'static str,
    connected: Option<IpAddress>,
}

static DISPLAY_STATE: Mutex<ThreadModeRawMutex, RefCell<DisplayState>> =
    Mutex::new(RefCell::new(DisplayState {
        address: Option::None,
        ssid: "???",
        connected: None,
    }));

static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

fn display_state_update<F>(mut sfn: F)
where
    F: FnMut(&mut DisplayState) -> (),
{
    DISPLAY_STATE.lock(|s| sfn(&mut s.borrow_mut()));
    DISPLAY_SIGNAL.signal(());
}


// Keep the display up to date
#[embassy_executor::task]
async fn display_refresh(mut display: display::Display) {
    loop {
        DISPLAY_SIGNAL.wait().await;

        Rectangle::new(Point::zero(), display.interface.size())
        .into_styled(display.styles.black_fill)
        .draw(&mut display.interface)
        .unwrap();

        Text::with_text_style(
            "Wifi demo",
            Point::new(14, 0),
            display.styles.char,
            display.styles.text,
        )
        .draw(&mut display.interface)
        .unwrap();

        let state = DISPLAY_STATE.lock(|s| s.borrow().clone());
        Text::with_text_style(
            state.ssid,
            Point::new(14, 14),
            display.styles.char,
            display.styles.text,
        )
        .draw(&mut display.interface)
        .unwrap();

        {
            let mut dhcp = String::<32>::new();
            match state.address {
                Some(addr) => uwrite!(dhcp, "{}.{}.{}.{}", addr.address().0[0], addr.address().0[1], addr.address().0[2], addr.address().0[3]),
                None => uwrite!(dhcp, "awaiting DHCP..."),
            }.unwrap();

            Text::with_text_style(
                &dhcp, 
                Point::new(14, 28),
                display.styles.char,
                display.styles.text,
            )
            .draw(&mut display.interface)
            .unwrap();
        }

        {
            let mut client = String::<32>::new();
            match state.connected {
                Some(IpAddress::Ipv4(addr)) => {
                    uwrite!(client, "client: {}.{}.{}.{}", addr.0[0], addr.0[1], addr.0[2], addr.0[3])
                }
                None => uwrite!(client, "accepting..."),
            }.unwrap();
            Text::with_text_style(
                &client,
                Point::new(14, 56),
                display.styles.char,
                display.styles.text,
            )
            .draw(&mut display.interface)
            .unwrap();
        }
    }
}

