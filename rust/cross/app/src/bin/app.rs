#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::dns::{self, DnsQueryType};
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, IpEndpoint, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0, PIO1};
use embassy_rp::pio::{Instance, InterruptHandler, Pio};
use embassy_rp::rtc::{DateTime, Rtc, RtcError};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::Vec;
use rand::RngCore;
use smart_leds::RGB8;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use cambindicator::{Date, Ws2812};

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

const WORLDTIME_HOST: &str = "worldtimeapi.org";
const SERVICELAYER3C_HOST: &str = "servicelayer3c.azure-api.net";
const NUM_LEDS: usize = 3;

const RED: RGB8 = RGB8::new(255, 0, 0);
const GREEN: RGB8 = RGB8::new(0, 255, 0);
const BLUE: RGB8 = RGB8::new(0, 0, 255);
const WHITE: RGB8 = RGB8::new(255, 255, 255);
const BLACK: RGB8 = RGB8::new(0, 0, 0);

const RED_DIM: RGB8 = RGB8::new(1, 0, 0);
const GREEN_DIM: RGB8 = RGB8::new(0, 1, 0);
const BLUE_DIM: RGB8 = RGB8::new(0, 0, 1);
const WHITE_DIM: RGB8 = RGB8::new(1, 1, 1);

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    PIO1_IRQ_0 => InterruptHandler<PIO1>;
});

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, PIN_23>,
        PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");

    let p = embassy_rp::init(Default::default());
    let mut rtc = Rtc::new(p.RTC);

    let mut pio1 = Pio::new(p.PIO1, Irqs);
    let mut ws2812 = Ws2812::new(&mut pio1.common, pio1.sm0, p.DMA_CH1, p.PIN_16);

    info!("Init LEDs");
    ws2812.write(&[GREEN_DIM, BLACK, BLACK]).await;

    info!("Init cyw43");
    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio0 = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio0.common,
        pio0.sm0,
        pio0.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;

    ws2812.write(&[BLACK, GREEN_DIM, BLACK]).await;

    info!("Init network");

    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Init network stack
    const SOCK_COUNT: usize = 8;
    static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
    static RESOURCES: StaticCell<StackResources<SOCK_COUNT>> = StaticCell::new();
    let stack = STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<SOCK_COUNT>::new()),
        RoscRng.next_u64(),
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    ws2812.write(&[BLACK, BLACK, GREEN_DIM]).await;

    info!("Join WIFI");

    loop {
        match control.join_wpa2(WIFI_SSID, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
                ws2812.write(&[BLACK, RED_DIM, GREEN_DIM]).await;
            }
        }
    }

    ws2812.write(&[GREEN_DIM, BLACK, GREEN_DIM]).await;

    info!("Waiting for DHCP...");
    let cfg = wait_for_config(stack).await;
    let local_addr = cfg.address.address();
    info!("IP address: {:?}", local_addr);

    ws2812.write(&[BLACK, BLACK, BLACK]).await;

    info!("Start main loop");

    /*
     * # Every hour
     *
     * - Resolve DNS to ip addresses
     * - Set RTC from API
     * - Fetch the next calendar events
     *
     * # Every min
     *
     * - work out if we should be showing a notification
     * - set the correct light show
     */

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(10)));

    loop {
        match client_tick(stack, &mut ws2812, &mut rtc, &mut socket, &mut buf)
            .await
            .inspect_err(|e| defmt::error!("Client tick: {}", e))
        {
            Ok(_) => {}
            Err(ClientError::Http(HttpError::Dns(_))) => {
                ws2812.write(&[RED_DIM, BLUE_DIM, WHITE_DIM]).await
            }
            Err(ClientError::Http(_)) => ws2812.write(&[RED_DIM, BLACK, WHITE_DIM]).await,
            Err(ClientError::Rtc(_)) => ws2812.write(&[RED_DIM, BLACK, BLUE_DIM]).await,
        };

        Timer::after(Duration::from_secs(60)).await;
    }
}

async fn client_tick<D, PIO: Instance, const SM: usize>(
    stack: &Stack<D>,
    ws2812: &mut Ws2812<'_, PIO, SM, NUM_LEDS>,
    rtc: &mut Rtc<'_, embassy_rp::peripherals::RTC>,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8; 4096],
) -> Result<(), ClientError>
where
    D: embassy_net::driver::Driver + 'static,
{
    let date = get_datetime(stack, socket, buf).await?;
    rtc.set_datetime(date)?;

    let calendar = get_calendar(stack, socket, buf).await.unwrap_or_default();
    for (a, b) in &calendar {
        println!("{} {}", a, b);
    }

    let now = rtc.now()?;
    let now_date: Date = (&now).into();

    let tomorrow = now_date.tomorrow();

    println!("\n\nCurrent time: {}\nTomorrow: {}", now_date, tomorrow);

    // list all of tomorrows calendar items
    let tomorrow_bins: Vec<BinColour, 16> = calendar
        .iter()
        .filter_map(|&(d, c)| if d == tomorrow { Some(c) } else { None })
        .collect();

    // if it is between 1800 and 2400
    let led_data = if now.hour > 18 {
        //do a light show
        match tomorrow_bins.len() {
            0 => [BLACK; 3],
            1 => [tomorrow_bins[0].into(); 3],
            2 => [tomorrow_bins[0].into(), BLACK, tomorrow_bins[1].into()],
            3 => [
                tomorrow_bins[0].into(),
                tomorrow_bins[1].into(),
                tomorrow_bins[2].into(),
            ],
            _ => [RED; 3],
        }
    } else {
        [BLACK; 3]
    };

    ws2812.write(&led_data).await;

    Ok(())
}

async fn get_calendar<D>(
    stack: &Stack<D>,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8; 4096],
) -> Result<Vec<(Date, BinColour), 16>, HttpError>
where
    D: embassy_net::driver::Driver,
{
    // Update dns resolution
    let servicelayer3c_address = resolve_dns(stack, SERVICELAYER3C_HOST).await?;

    // http://servicelayer3c.azure-api.net/wastecalendar/calendar/ical/200004185983
    http_get(
        (servicelayer3c_address, 80),
        SERVICELAYER3C_HOST,
        "/wastecalendar/calendar/ical/200004185983",
        socket,
        buf,
    )
    .await
    .map(|n| {
        nom::combinator::iterator(&buf[..n], ical_parser::parse_event::<()>)
            .map(|((year, month, day), bin)| {
                let bin = if bin.starts_with(b"Black") {
                    BinColour::Black
                } else if bin.starts_with(b"Blue") {
                    BinColour::Blue
                } else {
                    BinColour::Green
                };

                (Date { year, month, day }, bin)
            })
            .take(16)
            .collect()
    })
}

async fn get_datetime<D>(
    stack: &Stack<D>,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8; 4096],
) -> Result<DateTime, HttpError>
where
    D: embassy_net::driver::Driver,
{
    // Update dns resolution
    let worldtime_address = resolve_dns(stack, WORLDTIME_HOST).await?;

    // http://worldtimeapi.org/api/ip.txt
    http_get(
        (worldtime_address, 80),
        WORLDTIME_HOST,
        "/api/ip.txt",
        socket,
        buf,
    )
    .await
    .map(|n| {
        let (year, month, day, hour, minute, second) =
            worldtime_parser::parse::<()>(&buf[..n]).unwrap().1;

        DateTime {
            year,
            month,
            day,
            day_of_week: embassy_rp::rtc::DayOfWeek::Monday,
            hour,
            minute,
            second,
        }
    })
}

async fn resolve_dns<D>(stack: &Stack<D>, host: &str) -> Result<embassy_net::IpAddress, dns::Error>
where
    D: embassy_net::driver::Driver,
{
    let query = stack.dns_query(host, DnsQueryType::A);
    let address = query.await?.first().cloned().ok_or(dns::Error::Failed)?;
    info!("DNS resolution: {} - {}", host, address);
    Ok(address)
}

#[derive(defmt::Format)]
enum HttpError {
    TcpConnect(embassy_net::tcp::ConnectError),
    Tcp(embassy_net::tcp::Error),
    Dns(embassy_net::dns::Error),
}

impl From<embassy_net::tcp::ConnectError> for HttpError {
    fn from(value: embassy_net::tcp::ConnectError) -> Self {
        Self::TcpConnect(value)
    }
}

impl From<embassy_net::tcp::Error> for HttpError {
    fn from(value: embassy_net::tcp::Error) -> Self {
        Self::Tcp(value)
    }
}

impl From<embassy_net::dns::Error> for HttpError {
    fn from(value: embassy_net::dns::Error) -> Self {
        Self::Dns(value)
    }
}

enum ClientError {
    Http(HttpError),
    Rtc(RtcError),
}

impl Format for ClientError {
    fn format(&self, fmt: Formatter) {
        match self {
            ClientError::Http(e) => defmt::write!(fmt, "Http({:x})", e),
            ClientError::Rtc(RtcError::InvalidDateTime(_)) => {
                defmt::write!(fmt, "Rtc(InvalidDateTime)")
            }
            ClientError::Rtc(RtcError::NotRunning) => {
                defmt::write!(fmt, "NotRunning")
            }
        }
    }
}

impl From<HttpError> for ClientError {
    fn from(value: HttpError) -> Self {
        Self::Http(value)
    }
}

impl From<RtcError> for ClientError {
    fn from(value: RtcError) -> Self {
        Self::Rtc(value)
    }
}

async fn http_get<T: Into<IpEndpoint>>(
    remote_endpoint: T,
    host: &str,
    path: &str,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8],
) -> Result<usize, HttpError> {
    socket.abort();
    socket.flush().await?;
    socket.connect(remote_endpoint).await?;
    socket.write_all(b"GET ").await?;
    socket.write_all(path.as_bytes()).await?;
    socket.write_all(b" HTTP/1.1\nHost: ").await?;
    socket.write_all(host.as_bytes()).await?;
    socket
        .write_all(b"\nAccept: */*\nConnection: close\n\n")
        .await?;

    let result = socket.read(buf).await;
    socket.close();
    Ok(result?)
}

async fn wait_for_config<D: embassy_net::driver::Driver + 'static>(
    stack: &Stack<D>,
) -> embassy_net::StaticConfigV4 {
    loop {
        if let Some(config) = stack.config_v4() {
            return config;
        }
        yield_now().await;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Format)]
enum BinColour {
    Blue,
    Green,
    Black,
}

impl From<BinColour> for RGB8 {
    fn from(value: BinColour) -> Self {
        match value {
            BinColour::Blue => BLUE,
            BinColour::Green => GREEN,
            BinColour::Black => WHITE,
        }
    }
}
