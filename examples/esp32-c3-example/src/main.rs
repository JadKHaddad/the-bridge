#![no_std]
#![no_main]

use cody_c::embedded_io_async::Compat;
use cody_c::{FramedRead, FramedWrite};
use embassy_executor::Spawner;
use embassy_futures::select::Either;
use embassy_net::{tcp::TcpSocket, Config, Ipv4Address, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::timer::systimer::{SystemTimer, Target};
use esp_hal::{
    clock::ClockControl, peripherals::Peripherals, rng::Rng, system::SystemControl,
    timer::timg::TimerGroup,
};
use esp_println::println;
use esp_wifi::{
    initialize,
    wifi::{
        ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiStaDevice,
        WifiState,
    },
    EspWifiInitFor,
};
use futures::{pin_mut, SinkExt, StreamExt};
use the_bridge::demo::DemoMessage;
use the_bridge::Codec;

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    let peripherals = Peripherals::take();

    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);

    let init = initialize(
        EspWifiInitFor::Wifi,
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let systimer = SystemTimer::new(peripherals.SYSTIMER).split::<Target>();
    esp_hal_embassy::init(&clocks, systimer.alarm0);

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack = &*mk_static!(
        Stack<WifiDevice<'_, WifiStaDevice>>,
        Stack::new(
            wifi_interface,
            config,
            mk_static!(StackResources<3>, StackResources::<3>::new()),
            seed
        )
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(stack)).ok();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));
        let remote_endpoint = (Ipv4Address::new(192, 168, 178, 97), 5000);

        log::info!("connecting...");
        let r = socket.connect(remote_endpoint).await;
        if let Err(e) = r {
            log::info!("connect error: {:?}", e);
            continue;
        }

        log::info!("connected!");

        let (tcp_reader, tcp_writer) = socket.split();

        let read_buf: &mut [u8] = &mut [0; 128];
        let write_buf: &mut [u8] = &mut [0; 128];

        let framed_read = FramedRead::new(
            Compat::new(tcp_reader),
            Codec::<DemoMessage>::new(),
            read_buf,
        )
        .into_stream();

        let framed_write = FramedWrite::new(
            Compat::new(tcp_writer),
            Codec::<DemoMessage>::new(),
            write_buf,
        )
        .into_sink();

        pin_mut!(framed_read, framed_write);
        loop {
            let time_fut = Timer::after(Duration::from_millis(3_000));
            let read_fut = framed_read.next();

            match embassy_futures::select::select(time_fut, read_fut).await {
                Either::First(_) => {
                    let message = DemoMessage::Measurement(1024);
                    log::info!("Sending message: {:?}", message);

                    match framed_write.send(message).await {
                        Ok(_) => {
                            log::info!("Message sent");
                        }
                        Err(e) => {
                            log::error!("Error: {:?}", e);
                            break;
                        }
                    }
                }

                Either::Second(read_result) => match read_result {
                    Some(Ok(message)) => {
                        log::info!("Received message: {:?}", message);

                        if let DemoMessage::Ping(u) = message {
                            let message = DemoMessage::Pong(u);
                            log::info!("Sending message: {:?}", message);

                            match framed_write.send(message).await {
                                Ok(_) => {
                                    log::info!("Message sent");
                                }
                                Err(e) => {
                                    log::error!("Error: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        log::error!("Error: {:?}", e);
                        break;
                    }
                    None => {
                        log::info!("Connection closed");
                        break;
                    }
                },
            }
        }
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");
    println!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        if esp_wifi::wifi::get_wifi_state() == WifiState::StaConnected {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after(Duration::from_millis(5000)).await
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            println!("Starting wifi");
            controller.start().await.unwrap();
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect().await {
            Ok(_) => println!("Wifi connected!"),
            Err(e) => {
                println!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
