//! Configuration module for WiFi and MQTT settings.
//!
//! These values are loaded from environment variables at compile time.
//! Copy `.env.example` to `.env` and fill in your values.

/// WiFi SSID
pub const WIFI_SSID: &str = env!("WIFI_SSID");

/// WiFi password
pub const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

/// MQTT client ID (user private key for bemfa.com)
pub const MQTT_CLIENT_ID: &str = env!("MQTT_CLIENT_ID");

/// MQTT topic (e.g., "light002")
pub const MQTT_TOPIC: &str = env!("MQTT_TOPIC");

/// MQTT server address (bemfa.com)
pub const MQTT_SERVER: &str = "bemfa.com";

/// MQTT server IP address (resolved from bemfa.com)
pub const MQTT_SERVER_IP: &str = "47.93.234.178";

/// MQTT server port (9501 for non-TLS on bemfa.com)
pub const MQTT_PORT: u16 = 9501;

/// MQTT topic for receiving commands (the topic itself, not topic/set)
/// Note: /set is used by the sender when publishing, not for subscription
pub const MQTT_SUBSCRIBE_TOPIC: &str = env!("MQTT_TOPIC");

/// MQTT topic for publishing status (topic/up)
pub const MQTT_UP_TOPIC: &str = concat!(env!("MQTT_TOPIC"), "/up");

/// Check if a message is an "on" command
pub fn is_on_command(msg: &[u8]) -> bool {
    msg == b"on" || msg == b"1" || msg == b"true"
}

/// Check if a message is an "off" command
pub fn is_off_command(msg: &[u8]) -> bool {
    msg == b"off" || msg == b"0" || msg == b"false"
}