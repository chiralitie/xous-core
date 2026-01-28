//! Xous MQTT Client Library
//!
//! Minimal MQTT 3.1.1 client for Xous OS, designed for Precursor hardware.
//!
//! # Features
//!
//! - `default` - Packet encoding/decoding only (no networking)
//! - `xous-client` - Full client with TCP networking via Xous Net service
//! - `tls-support` - MQTT over TLS (port 8883)
//! - `qos1` - At-least-once delivery
//! - `qos2` - Exactly-once delivery
//!
//! # Example (packet-only mode)
//!
//! ```rust
//! use xous_mqtt::packet;
//!
//! let connect = packet::build_connect("my-client-id");
//! let subscribe = packet::build_subscribe(1, "events/#", packet::QoS::AtMostOnce);
//! let publish = packet::build_publish("status", b"online", packet::QoS::AtMostOnce);
//! ```
//!
//! # Example (full client, requires `xous-client` feature)
//!
//! ```rust,ignore
//! use xous_mqtt::{MqttClient, MqttConfig, MqttEvent};
//!
//! let config = MqttConfig {
//!     broker: "192.168.1.100:1883".into(),
//!     client_id: "precursor-001".into(),
//!     ..Default::default()
//! };
//!
//! let mut client = MqttClient::new(config)?;
//! client.connect()?;
//! client.subscribe("events/#", QoS::AtLeastOnce)?;
//!
//! loop {
//!     match client.poll()? {
//!         MqttEvent::Message { topic, payload } => {
//!             // Handle message
//!         }
//!         MqttEvent::Disconnected => {
//!             client.reconnect()?;
//!         }
//!         _ => {}
//!     }
//! }
//! ```

#![no_std]

extern crate alloc;

pub mod packet;

#[cfg(feature = "qos1")]
pub mod qos1;

#[cfg(feature = "qos2")]
pub mod qos2;

#[cfg(feature = "xous-client")]
pub mod client;

#[cfg(feature = "xous-client")]
pub use client::{MqttClient, MqttConfig, MqttEvent, MqttError};

pub use packet::QoS;

/// MQTT protocol version
pub const MQTT_VERSION: u8 = 4; // MQTT 3.1.1

/// Default keep-alive interval in seconds
pub const DEFAULT_KEEP_ALIVE: u16 = 60;

/// Default MQTT port (unencrypted)
pub const MQTT_PORT: u16 = 1883;

/// Default MQTT over TLS port
pub const MQTTS_PORT: u16 = 8883;
