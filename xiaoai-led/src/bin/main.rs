#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

extern crate alloc;

use alloc::string::String;
use core::net::Ipv4Addr;
use core::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use embassy_executor::Spawner;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config as NetConfig, DhcpConfig, Runner, StackResources};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Duration, Timer};

use esp_hal::{
    clock::CpuClock,
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_println::println;

use esp_radio::wifi::{
    ClientConfig, Config as WifiConfig, ModeConfig, WifiController, WifiDevice,
    WifiEvent, WifiStaState,
};
use mqttrust::{
    Config as MqttConfig, IpBroker, MqttClient, MqttStack, QoS, State, Subscribe, SubscribeTopic,
    transport::embedded_nal::NalTransport,
};

use esp_alloc as _;

use blinksy::{
    ControlBuilder,
    driver::ClocklessDriver,
    layout::Layout1d,
    layout1d,
    leds::Ws2812,
    patterns::rainbow::{Rainbow, RainbowParams},
};
use blinksy_esp::{rmt::ClocklessRmtBuilder, time::elapsed};

use led::config;

pub static CURRENT_RSSI: AtomicI32 = AtomicI32::new(0);

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

layout1d!(Layout, 1);

pub async fn sleep(millis: u64) {
    Timer::after(Duration::from_millis(millis)).await;
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    println!("start connection task");

    loop {
        if let WifiStaState::Connected = esp_radio::wifi::sta_state() {
            if let Ok(rssi) = controller.rssi() {
                CURRENT_RSSI.store(rssi, Ordering::Relaxed);
            }

            embassy_futures::select::select(
                controller.wait_for_event(WifiEvent::StaDisconnected),
                sleep(4_000),
            )
            .await;
        }
        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = ModeConfig::Client(
                ClientConfig::default()
                    .with_ssid(String::from(config::WIFI_SSID))
                    .with_password(String::from(config::WIFI_PASSWORD)),
            );
            controller.set_config(&client_config).unwrap();
            println!("Starting wifi");
            controller
                .start_async()
                .await
                .expect("couldn't start controller");
            println!("Wifi started!");
        }
        println!("About to connect...");

        match controller.connect_async().await {
            Ok(()) => {
                println!("Wifi connected!");
            }
            Err(e) => {
                println!("Failed to connect to wifi: {:?}", e);
                sleep(5_000).await;
            }
        }
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn mqtt_task(
    mut mqtt_stack: MqttStack<'static, NoopRawMutex>,
    broker: IpBroker,
    stack: embassy_net::Stack<'static>,
) {
    println!("MQTT task started");
    
    // Create TCP client state
    static TCP_CLIENT_STATE: static_cell::StaticCell<TcpClientState<1, 1024, 1024>> = static_cell::StaticCell::new();
    let tcp_client_state = TCP_CLIENT_STATE.init(TcpClientState::new());
    
    // Create TCP client
    let tcp_client = TcpClient::new(stack, tcp_client_state);
    
    // Create NAL transport
    let mut transport = NalTransport::new(&tcp_client, broker);
    
    println!("Running MQTT stack...");
    
    // Run the MQTT stack - this handles connection, reconnection, keepalive, etc.
    mqtt_stack.run(&mut transport).await;
}

#[embassy_executor::task(pool_size = 1)]
async fn mqtt_subscription(
    client: &'static MqttClient<'static, NoopRawMutex>,
    topic: &'static str,
    led_on: &'static AtomicBool,
) {
    println!("Subscription task started for: {}", topic);
    
    loop {
        // Wait for MQTT connection
        client.wait_connected().await;
        println!("MQTT connected, subscribing to: {}", topic);
        
        // Create subscription topic with QoS 0 (AtMostOnce)
        let sub_topic = SubscribeTopic::builder()
            .topic_path(topic)
            .maximum_qos(QoS::AtMostOnce)
            .build();
        
        // Subscribe to topic
        let mut subscription = match client
            .subscribe::<1>(Subscribe::builder().topics(&[sub_topic]).build())
            .await
        {
            Ok(sub) => {
                println!("Subscribed to: {}", topic);
                sub
            }
            Err(e) => {
                println!("Subscribe error: {:?}, retrying...", e);
                sleep(1_000).await;
                continue;
            }
        };
        
        // Process incoming messages
        println!("Waiting for messages on: {}", topic);
        while let Some(message) = subscription.next_message().await {
            println!("Received on {}: {:?}", message.topic_name(), message.payload());
            
            // Check for on/off commands
            let payload = message.payload();
            if config::is_on_command(payload) {
                led_on.store(true, Ordering::Relaxed);
                println!("-> LED ON");
            } else if config::is_off_command(payload) {
                led_on.store(false, Ordering::Relaxed);
                println!("-> LED OFF");
            }
        }
        
        println!("Subscription ended, waiting for reconnection...");
    }
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // Initialize logger
    esp_println::logger::init_logger_from_env();
    
    // Initialize hardware
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    
    // Initialize heap
    esp_alloc::heap_allocator!(size: 72 * 1024);
    
    // Initialize esp-rtos scheduler
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);
    
    println!("========================================");
    println!("  ESP32-S3 LED MQTT Controller");
    println!("========================================");
    
    // Initialize LED driver
    let ws2812_driver = {
        let data_pin = peripherals.GPIO48;
        let rmt_clk_freq = esp_hal::time::Rate::from_mhz(80);

        let rmt = esp_hal::rmt::Rmt::new(peripherals.RMT, rmt_clk_freq).unwrap();
        let rmt_channel = rmt.channel0;

        ClocklessDriver::default()
            .with_led::<Ws2812>()
            .with_writer(
                ClocklessRmtBuilder::default()
                    .with_rmt_buffer_size::<{ Layout::PIXEL_COUNT * 3 * 8 + 1 }>()
                    .with_led::<Ws2812>()
                    .with_channel(rmt_channel)
                    .with_pin(data_pin)
                    .build(),
            )
    };

    let mut control = ControlBuilder::new_1d()
        .with_layout::<Layout, { Layout::PIXEL_COUNT }>()
        .with_pattern::<Rainbow>(RainbowParams {
            ..Default::default()
        })
        .with_driver(ws2812_driver)
        .with_frame_buffer_size::<{ Ws2812::frame_buffer_size(Layout::PIXEL_COUNT) }>()
        .build();
    
    println!("LED driver initialized");
    
    // Initialize esp-radio
    println!("Initializing WiFi radio...");
    let esp_radio_ctrl = &*mk_static!(esp_radio::Controller<'static>, esp_radio::init().unwrap());
    
    let (controller, interfaces) =
        esp_radio::wifi::new(esp_radio_ctrl, peripherals.WIFI, WifiConfig::default()).unwrap();
    println!("WiFi controller created");
    
    let wifi_interface = interfaces.sta;

    // Setup embassy-net stack
    let net_config = NetConfig::dhcpv4(DhcpConfig::default());
    let rng = Rng::new();
    let seed = (u64::from(rng.random())) << 32 | u64::from(rng.random());

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        net_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    // Wait for network
    println!("Waiting for network...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        println!("Got IP: {}", config.address);
    }

    // Resolve MQTT broker address via DNS
    println!("Resolving {} via DNS...", config::MQTT_SERVER);
    let broker_ip = match stack.dns_query(config::MQTT_SERVER, embassy_net::dns::DnsQueryType::A).await {
        Ok(addresses) if !addresses.is_empty() => {
            println!("DNS resolved to: {:?}", addresses[0]);
            // DNS query for A record returns IPv4 only
            match addresses[0] {
                embassy_net::IpAddress::Ipv4(ip) => ip,
            }
        }
        _ => {
            println!("DNS failed, using fallback IP");
            Ipv4Addr::new(119, 91, 109, 180)
        }
    };

    // Setup MQTT client using mqttrust
    println!("Setting up MQTT client...");
    
    // Create MQTT state with TX/RX buffers
    static MQTT_STATE: static_cell::StaticCell<State<NoopRawMutex, 1024, 1024>> = static_cell::StaticCell::new();
    let mqtt_state = MQTT_STATE.init(State::new());
    
    // Configure MQTT client
    // Use 30s keepalive to maintain connection stability
    let mqtt_config = MqttConfig::builder()
        .client_id(config::MQTT_CLIENT_ID.try_into().unwrap())
        .keepalive_interval(Duration::from_secs(30))
        .build();
    
    let (mqtt_stack, mqtt_client) = mqttrust::new(mqtt_state, mqtt_config);
    
    println!("MQTT client created with ID: {}", config::MQTT_CLIENT_ID);

    // Store client and LED state statically
    static MQTT_CLIENT: static_cell::StaticCell<MqttClient<'static, NoopRawMutex>> = static_cell::StaticCell::new();
    let client = MQTT_CLIENT.init(mqtt_client);
    
    static LED_ON: AtomicBool = AtomicBool::new(false);
    
    // Create broker
    let broker = IpBroker::new(broker_ip, config::MQTT_PORT);
    
    // Spawn MQTT task
    spawner.spawn(mqtt_task(mqtt_stack, broker, stack)).ok();
    
    // Spawn subscription task
    spawner.spawn(mqtt_subscription(client, config::MQTT_SUBSCRIBE_TOPIC, &LED_ON)).ok();

    println!("=== Ready ===");
    println!("Waiting for commands on: {}", config::MQTT_SUBSCRIBE_TOPIC);

    // Main loop - update LED based on state
    loop {
        let led_on = LED_ON.load(Ordering::Relaxed);
        
        if led_on {
            control.set_color_correction(blinksy::color::ColorCorrection {
                red: 0.0,
                green: 1.0,
                blue: 0.0,
            });
        } else {
            control.set_color_correction(blinksy::color::ColorCorrection {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
            });
        }
        
        let elapsed_us = elapsed().as_micros();
        let _ = control.tick(elapsed_us);
        
        sleep(100).await;
    }
}
