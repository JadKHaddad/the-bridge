#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use bincode_bridge::{
    decode::framed_read::FramedRead, demo::DemoMessage, embedded_io::Compat,
    encode::framed_write::FramedWrite,
};
use embassy_executor::Spawner;
use embassy_futures::select::Either;
use embassy_net::{tcp::TcpSocket, Config, Ipv4Address, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, systimer::SystemTimer,
    timer::TimerGroup, Rng,
};
use esp_wifi::wifi::{ClientConfiguration, Configuration};
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use esp_wifi::{initialize, EspWifiInitFor};
use static_cell::make_static;

const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");

// const SSID: &str = "Pixel4a";
// const PASSWORD: &str = "1234567890";

#[main]
async fn main(spawner: Spawner) -> ! {
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    let timer = SystemTimer::new(peripherals.SYSTIMER).alarm0;

    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();

    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timer_group0);

    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed
    ));

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(&stack)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    log::info!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            log::info!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    loop {
        Timer::after(Duration::from_millis(1_000)).await;

        let mut socket = TcpSocket::new(&stack, &mut rx_buffer, &mut tx_buffer);
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

        let read_buf: &mut [u8] = &mut [0; 100];
        let write_buf: &mut [u8] = &mut [0; 100];

        let mut reader: FramedRead<'_, _, DemoMessage> =
            FramedRead::new(Compat::new(tcp_reader), read_buf);

        let mut writer: FramedWrite<'_, _, DemoMessage> =
            FramedWrite::new(Compat::new(tcp_writer), write_buf);

        loop {
            let time_fut = Timer::after(Duration::from_millis(3_000));
            let read_fut = reader.read_frame();

            match embassy_futures::select::select(time_fut, read_fut).await {
                Either::First(_) => {
                    let message = DemoMessage::Measurement(1024);

                    match writer.write_frame(&message).await {
                        Ok(_) => {
                            log::info!("Sent message: {:?}", message);
                        }
                        Err(e) => {
                            log::error!("Error: {:?}", e);
                            break;
                        }
                    }
                }

                Either::Second(read_result) => match read_result {
                    Ok(message) => {
                        log::info!("Received message: {:?}", message);

                        match message {
                            DemoMessage::Ping(u) => {
                                let message = DemoMessage::Pong(u);

                                match writer.write_frame(&message).await {
                                    Ok(_) => {
                                        log::info!("Sent message: {:?}", message);
                                    }
                                    Err(e) => {
                                        log::error!("Error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        log::error!("Error: {:?}", e);
                        break;
                    }
                },
            }
        }
    }
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    log::info!("start connection task");
    log::info!("Device capabilities: {:?}", controller.get_capabilities());
    loop {
        match esp_wifi::wifi::get_wifi_state() {
            WifiState::StaConnected => {
                // wait until we're no longer connected
                controller.wait_for_event(WifiEvent::StaDisconnected).await;
                Timer::after(Duration::from_millis(5000)).await
            }
            _ => {}
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: SSID.try_into().unwrap(),
                password: PASSWORD.try_into().unwrap(),
                auth_method: esp_wifi::wifi::AuthMethod::WPA2Personal,
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            log::info!("Starting wifi");
            controller.start().await.unwrap();
            log::info!("Wifi started!");
        }
        log::info!("About to connect...");

        match controller.connect().await {
            Ok(_) => log::info!("Wifi connected!"),
            Err(e) => {
                log::info!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}
