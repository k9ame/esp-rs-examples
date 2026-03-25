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
    gpio::{Level, Output, OutputConfig},
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

use xiaoai_led_c3::config;

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
    relay_on: &'static AtomicBool,
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
                relay_on.store(true, Ordering::Relaxed);
                println!("-> Relay ON");
            } else if config::is_off_command(payload) {
                relay_on.store(false, Ordering::Relaxed);
                println!("-> Relay OFF");
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
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    
    println!("========================================");
    println!("  ESP32-C3 Relay MQTT Controller");
    println!("========================================");
    
    // Initialize relay GPIO (GPIO8, active high)
    let mut relay_pin = Output::new(
        peripherals.GPIO8,
        Level::Low,
        OutputConfig::default(),
    );
    println!("Relay GPIO initialized on GPIO8");
    
    // Initialize esp-radio
    println!("Initializing WiFi radio...");
    let esp_radio_ctrl = &*mk_static!(esp_radio::Controller<'static>, esp_radio::init().unwrap());
    
    let (mut controller, interfaces) =
        esp_radio::wifi::new(esp_radio_ctrl, peripherals.WIFI, WifiConfig::default()).unwrap();
    
    // Disable WiFi power save mode for stable connection
    controller.set_power_saving(esp_radio::wifi::PowerSaveMode::None).unwrap();
    println!("WiFi controller created (power save disabled)");
    
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
    // Use 10s keepalive to maintain connection stability
    let mqtt_config = MqttConfig::builder()
        .client_id(config::MQTT_CLIENT_ID.try_into().unwrap())
        .keepalive_interval(Duration::from_secs(10))
        .build();
    
    let (mqtt_stack, mqtt_client) = mqttrust::new(mqtt_state, mqtt_config);
    
    println!("MQTT client created with ID: {}", config::MQTT_CLIENT_ID);

    // Store client and relay state statically
    static MQTT_CLIENT: static_cell::StaticCell<MqttClient<'static, NoopRawMutex>> = static_cell::StaticCell::new();
    let client = MQTT_CLIENT.init(mqtt_client);
    
    static RELAY_ON: AtomicBool = AtomicBool::new(false);
    
    // Create broker
    let broker = IpBroker::new(broker_ip, config::MQTT_PORT);
    
    // Spawn MQTT task
    spawner.spawn(mqtt_task(mqtt_stack, broker, stack)).ok();
    
    // Spawn subscription task
    spawner.spawn(mqtt_subscription(client, config::MQTT_SUBSCRIBE_TOPIC, &RELAY_ON)).ok();

    println!("=== Ready ===");
    println!("Waiting for commands on: {}", config::MQTT_SUBSCRIBE_TOPIC);

    // Main loop - control relay based on state
    loop {
        let relay_on = RELAY_ON.load(Ordering::Relaxed);
        
        if relay_on {
            relay_pin.set_high();
        } else {
            relay_pin.set_low();
        }
        
        sleep(100).await;
    }
}
