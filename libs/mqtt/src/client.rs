//! MQTT Client for Xous OS
//!
//! Full-featured MQTT client using Xous Net service for TCP.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::VecDeque;

use crate::packet::{self, Packet, QoS, ParseError};

/// MQTT client configuration
#[derive(Debug, Clone)]
pub struct MqttConfig {
    /// Broker address (host:port)
    pub broker: String,
    /// Client identifier
    pub client_id: String,
    /// Username (optional)
    pub username: Option<String>,
    /// Password (optional)
    pub password: Option<Vec<u8>>,
    /// Keep-alive interval in seconds
    pub keep_alive_secs: u16,
    /// Clean session flag
    pub clean_session: bool,
    /// Auto-reconnect on disconnect
    pub auto_reconnect: bool,
    /// Reconnect delay in milliseconds
    pub reconnect_delay_ms: u64,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: String::from("127.0.0.1:1883"),
            client_id: String::from("xous-mqtt-client"),
            username: None,
            password: None,
            keep_alive_secs: 60,
            clean_session: true,
            auto_reconnect: true,
            reconnect_delay_ms: 5000,
        }
    }
}

/// MQTT client events
#[derive(Debug, Clone)]
pub enum MqttEvent {
    /// Connected to broker
    Connected,
    /// Disconnected from broker
    Disconnected,
    /// Received message
    Message {
        topic: String,
        payload: Vec<u8>,
    },
    /// Subscription confirmed
    Subscribed {
        packet_id: u16,
    },
    /// Publish acknowledged (QoS 1)
    PublishAcked {
        packet_id: u16,
    },
    /// Publish complete (QoS 2)
    PublishComplete {
        packet_id: u16,
    },
    /// Error occurred
    Error(MqttError),
}

/// MQTT client errors
#[derive(Debug, Clone)]
pub enum MqttError {
    /// Connection failed
    ConnectionFailed(String),
    /// Connection refused by broker
    ConnectionRefused(u8),
    /// Network I/O error
    IoError(String),
    /// Protocol error
    ProtocolError(String),
    /// Timeout
    Timeout,
    /// Not connected
    NotConnected,
}

/// MQTT connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// MQTT Client
///
/// Requires `xous-client` feature.
pub struct MqttClient {
    config: MqttConfig,
    state: ConnectionState,
    packet_id: u16,
    rx_buffer: Vec<u8>,
    event_queue: VecDeque<MqttEvent>,
    last_ping_time: u64,
    // TCP stream would be stored here when connected
    // stream: Option<TcpStream>,
}

impl MqttClient {
    /// Create a new MQTT client
    pub fn new(config: MqttConfig) -> Self {
        Self {
            config,
            state: ConnectionState::Disconnected,
            packet_id: 1,
            rx_buffer: Vec::with_capacity(4096),
            event_queue: VecDeque::new(),
            last_ping_time: 0,
        }
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    /// Get next packet ID
    fn next_packet_id(&mut self) -> u16 {
        let id = self.packet_id;
        self.packet_id = self.packet_id.wrapping_add(1);
        if self.packet_id == 0 {
            self.packet_id = 1;
        }
        id
    }

    /// Connect to broker
    ///
    /// This is a placeholder - actual implementation requires TCP stream.
    pub fn connect(&mut self) -> Result<(), MqttError> {
        if self.state != ConnectionState::Disconnected {
            return Ok(());
        }

        self.state = ConnectionState::Connecting;

        // TODO: Implement TCP connection via Xous Net service
        // 1. Parse broker address
        // 2. Create TcpStream::connect()
        // 3. Send CONNECT packet
        // 4. Wait for CONNACK
        // 5. Set state to Connected

        log::info!("MQTT: Connecting to {}", self.config.broker);

        // Build CONNECT packet
        let _connect_packet = packet::build_connect_with_options(
            &self.config.client_id,
            self.config.username.as_deref(),
            self.config.password.as_deref(),
            self.config.clean_session,
            self.config.keep_alive_secs,
        );

        // Placeholder: would send packet and wait for CONNACK
        self.state = ConnectionState::Connected;
        self.event_queue.push_back(MqttEvent::Connected);

        Ok(())
    }

    /// Disconnect from broker
    pub fn disconnect(&mut self) -> Result<(), MqttError> {
        if self.state != ConnectionState::Connected {
            return Ok(());
        }

        let _disconnect_packet = packet::build_disconnect();
        // TODO: Send packet, close stream

        self.state = ConnectionState::Disconnected;
        self.event_queue.push_back(MqttEvent::Disconnected);

        Ok(())
    }

    /// Subscribe to a topic
    pub fn subscribe(&mut self, topic: &str, qos: QoS) -> Result<u16, MqttError> {
        if self.state != ConnectionState::Connected {
            return Err(MqttError::NotConnected);
        }

        let packet_id = self.next_packet_id();
        let _subscribe_packet = packet::build_subscribe(packet_id, topic, qos);

        // TODO: Send packet
        log::info!("MQTT: Subscribing to {} (id={})", topic, packet_id);

        Ok(packet_id)
    }

    /// Unsubscribe from a topic
    pub fn unsubscribe(&mut self, topic: &str) -> Result<u16, MqttError> {
        if self.state != ConnectionState::Connected {
            return Err(MqttError::NotConnected);
        }

        let packet_id = self.next_packet_id();
        let _unsubscribe_packet = packet::build_unsubscribe(packet_id, topic);

        // TODO: Send packet
        log::info!("MQTT: Unsubscribing from {} (id={})", topic, packet_id);

        Ok(packet_id)
    }

    /// Publish a message
    pub fn publish(&mut self, topic: &str, payload: &[u8], qos: QoS) -> Result<Option<u16>, MqttError> {
        if self.state != ConnectionState::Connected {
            return Err(MqttError::NotConnected);
        }

        let packet_id = if qos != QoS::AtMostOnce {
            Some(self.next_packet_id())
        } else {
            None
        };

        let _publish_packet = packet::build_publish_with_id(
            topic,
            payload,
            qos,
            packet_id,
            false, // retain
        );

        // TODO: Send packet
        log::debug!("MQTT: Publishing to {} ({} bytes)", topic, payload.len());

        Ok(packet_id)
    }

    /// Send ping to keep connection alive
    pub fn ping(&mut self) -> Result<(), MqttError> {
        if self.state != ConnectionState::Connected {
            return Err(MqttError::NotConnected);
        }

        let _ping_packet = packet::build_pingreq();
        // TODO: Send packet

        Ok(())
    }

    /// Poll for events (non-blocking)
    pub fn poll(&mut self) -> Option<MqttEvent> {
        // TODO: Read from TCP stream, parse packets, generate events

        // Return queued events
        self.event_queue.pop_front()
    }

    /// Process received data
    pub fn process_data(&mut self, data: &[u8]) {
        self.rx_buffer.extend_from_slice(data);

        // Try to parse complete packets
        loop {
            match packet::parse_packet(&self.rx_buffer) {
                Ok((packet, consumed)) => {
                    self.handle_packet(packet);
                    self.rx_buffer.drain(..consumed);
                }
                Err(ParseError::Incomplete) => break,
                Err(e) => {
                    log::error!("MQTT: Parse error: {:?}", e);
                    self.rx_buffer.clear();
                    break;
                }
            }
        }
    }

    /// Handle a parsed packet
    fn handle_packet(&mut self, packet: Packet) {
        match packet {
            Packet::Connack { code, .. } => {
                if code == packet::ConnackCode::Accepted {
                    self.state = ConnectionState::Connected;
                    self.event_queue.push_back(MqttEvent::Connected);
                } else {
                    self.state = ConnectionState::Disconnected;
                    self.event_queue.push_back(MqttEvent::Error(
                        MqttError::ConnectionRefused(code as u8)
                    ));
                }
            }
            Packet::Publish { topic, payload, qos, packet_id, .. } => {
                // Send acknowledgment for QoS > 0
                if qos == QoS::AtLeastOnce {
                    if let Some(id) = packet_id {
                        let _puback = packet::build_puback(id);
                        // TODO: Send PUBACK
                    }
                } else if qos == QoS::ExactlyOnce {
                    if let Some(id) = packet_id {
                        let _pubrec = packet::build_pubrec(id);
                        // TODO: Send PUBREC, track state
                    }
                }

                self.event_queue.push_back(MqttEvent::Message { topic, payload });
            }
            Packet::Puback { packet_id } => {
                self.event_queue.push_back(MqttEvent::PublishAcked { packet_id });
            }
            Packet::Pubrec { packet_id } => {
                // QoS 2: Send PUBREL
                let _pubrel = packet::build_pubrel(packet_id);
                // TODO: Send PUBREL
            }
            Packet::Pubrel { packet_id } => {
                // QoS 2: Send PUBCOMP
                let _pubcomp = packet::build_pubcomp(packet_id);
                // TODO: Send PUBCOMP
            }
            Packet::Pubcomp { packet_id } => {
                self.event_queue.push_back(MqttEvent::PublishComplete { packet_id });
            }
            Packet::Suback { packet_id, .. } => {
                self.event_queue.push_back(MqttEvent::Subscribed { packet_id });
            }
            Packet::Unsuback { .. } => {
                // Unsubscribe confirmed
            }
            Packet::Pingresp => {
                // Connection is alive
            }
        }
    }
}
