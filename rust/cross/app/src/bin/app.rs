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
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::rtc::{DateTime, Rtc};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use heapless::Vec;
use rand::RngCore;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

const WORLDTIME_HOST: &str = "worldtimeapi.org";
const SERVICELAYER3C_HOST: &str = "servicelayer3c.azure-api.net";

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
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

    let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
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
    let stack = &*STACK.init(Stack::new(
        net_device,
        config,
        RESOURCES.init(StackResources::<SOCK_COUNT>::new()),
        RoscRng.next_u64(),
    ));

    unwrap!(spawner.spawn(net_task(stack)));

    loop {
        match control.join_wpa2(WIFI_SSID, WIFI_PASSWORD).await {
            Ok(_) => break,
            Err(err) => {
                info!("join failed with status={}", err.status);
            }
        }
    }

    info!("Waiting for DHCP...");
    let cfg = wait_for_config(stack).await;
    let local_addr = cfg.address.address();
    info!("IP address: {:?}", local_addr);

    client(stack, &mut rtc, &mut control).await
}

async fn client<D>(
    stack: &Stack<D>,
    rtc: &mut Rtc<'_, embassy_rp::peripherals::RTC>,
    control: &mut cyw43::Control<'_>,
) -> !
where
    D: embassy_net::driver::Driver + 'static,
{
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

    control.gpio_set(0, false).await;

    loop {
        set_rtc(stack, &mut socket, &mut buf, rtc).await;
        let calendar = get_calendar(stack, &mut socket, &mut buf).await;

        Timer::after(Duration::from_secs(60)).await;
    }
}

async fn get_calendar<D>(
    stack: &Stack<D>,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8; 4096],
) -> Result<Vec<(u32, BinColour), 16>, HttpClientError>
where
    D: embassy_net::driver::Driver,
{
    // Update dns resolution
    let servicelayer3c_address = resolve_dns(stack, SERVICELAYER3C_HOST).await.unwrap();

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
        let values: Vec<_, 16> =
            nom::combinator::iterator(&buf[..n], ical_parser::parse_event::<()>)
                .map(|(time, bin)| {
                    let bin = if bin.starts_with(b"Black") {
                        BinColour::Black
                    } else if bin.starts_with(b"Blue") {
                        BinColour::Blue
                    } else {
                        BinColour::Green
                    };

                    (time, bin)
                })
                .collect();

        for (a, b) in &values {
            println!("{} {}", a, b);
        }
        values
    })
}

async fn set_rtc<D>(
    stack: &Stack<D>,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8; 4096],
    rtc: &mut Rtc<'_, embassy_rp::peripherals::RTC>,
) where
    D: embassy_net::driver::Driver,
{
    // Update dns resolution
    let worldtime_address = resolve_dns(stack, WORLDTIME_HOST).await.unwrap();

    // http://worldtimeapi.org/api/ip.txt
    match http_get(
        (worldtime_address, 80),
        WORLDTIME_HOST,
        "/api/ip.txt",
        socket,
        buf,
    )
    .await
    {
        Ok(n) => {
            let (year, month, day, hour, minute, second) =
                worldtime_parser::parse::<()>(&buf[..n]).unwrap().1;

            let date = DateTime {
                year,
                month,
                day,
                day_of_week: embassy_rp::rtc::DayOfWeek::Monday,
                hour,
                minute,
                second,
            };

            info!("worldtime: {:#?}", Debug2Format(&date));
            rtc.set_datetime(date).unwrap()
        }
        Err(e) => error!("failed to get worldtime: {}", e),
    };
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
enum HttpClientError {
    Connect(embassy_net::tcp::ConnectError),
    Error(embassy_net::tcp::Error),
}

impl From<embassy_net::tcp::ConnectError> for HttpClientError {
    fn from(value: embassy_net::tcp::ConnectError) -> Self {
        Self::Connect(value)
    }
}

impl From<embassy_net::tcp::Error> for HttpClientError {
    fn from(value: embassy_net::tcp::Error) -> Self {
        Self::Error(value)
    }
}

async fn http_get<T: Into<IpEndpoint>>(
    remote_endpoint: T,
    host: &str,
    path: &str,
    socket: &mut TcpSocket<'_>,
    buf: &mut [u8],
) -> Result<usize, HttpClientError> {
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

#[derive(Clone, Copy, PartialEq, Eq, Debug, Format)]
enum BinColour {
    Blue,
    Green,
    Black,
}
