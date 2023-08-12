#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use core::str::FromStr;

use cyw43_pio::PioSpi;
use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::tcp::TcpSocket;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_25, PIO0};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use static_cell::make_static;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

const WIFI_SSID: &str = env!("WIFI_SSID");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

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

    let state = make_static!(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(wifi_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guaranteed to be random.

    // Init network stack
    let stack = &*make_static!(Stack::new(
        net_device,
        config,
        make_static!(StackResources::<2>::new()),
        seed
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

    client(stack, &mut control).await
    // server(stack, &mut control).await
}

async fn client<'a, D: embassy_net::driver::Driver>(
    stack: &Stack<D>,
    control: &mut cyw43::Control<'a>,
) -> ! {
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        control.gpio_set(0, false).await;

        loop {
            match http_get(&mut socket, &mut buf).await {
                Ok(n) => info!(
                    "worldtime: {:?}",
                    worldtime_parser::parse::<()>(&buf[..n]).unwrap().1
                ),
                Err(e) => error!("failed to get worldtime: {}", e),
            };

            Timer::after(Duration::from_secs(10)).await;
        }
    }
}

#[derive(defmt::Format)]
enum HttpClientError {
    Connect(embassy_net::tcp::ConnectError),
    Write(embedded_io_async::WriteAllError<embassy_net::tcp::Error>),
    Error(embassy_net::tcp::Error),
}

impl From<embassy_net::tcp::ConnectError> for HttpClientError {
    fn from(value: embassy_net::tcp::ConnectError) -> Self {
        Self::Connect(value)
    }
}

impl From<embedded_io_async::WriteAllError<embassy_net::tcp::Error>> for HttpClientError {
    fn from(value: embedded_io_async::WriteAllError<embassy_net::tcp::Error>) -> Self {
        Self::Write(value)
    }
}

impl From<embassy_net::tcp::Error> for HttpClientError {
    fn from(value: embassy_net::tcp::Error) -> Self {
        Self::Error(value)
    }
}

async fn http_get(socket: &mut TcpSocket<'_>, buf: &mut [u8]) -> Result<usize, HttpClientError> {
    let msg = b"GET /api/ip.txt HTTP/1.1\r\nAccept: */*\r\nConnection: close\r\n\r\n";
    let host_addr = embassy_net::Ipv4Address::from_str("213.188.196.246").unwrap();

    socket.abort();
    socket.flush().await?;
    socket.connect((host_addr, 80)).await?;
    socket.write_all(msg).await?;
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
