//! CCR MQTT Client
//!
//! Minimal MQTT 3.1.1 client for event streaming.
//! Based on phase3 implementation.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

/// MQTT packet types
const CONNECT: u8 = 1;
const CONNACK: u8 = 2;
const PUBLISH: u8 = 3;
const SUBSCRIBE: u8 = 8;
const SUBACK: u8 = 9;
const PINGREQ: u8 = 12;
const PINGRESP: u8 = 13;

/// Build MQTT CONNECT packet
pub fn build_connect_packet(client_id: &str) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header
    let mut var_header = Vec::new();
    // Protocol name "MQTT"
    var_header.push(0x00);
    var_header.push(0x04);
    var_header.extend_from_slice(b"MQTT");
    // Protocol level (4 = 3.1.1)
    var_header.push(0x04);
    // Connect flags (clean session)
    var_header.push(0x02);
    // Keep alive (60 seconds)
    var_header.push(0x00);
    var_header.push(0x3C);

    // Payload (client ID)
    let mut payload = Vec::new();
    payload.push((client_id.len() >> 8) as u8);
    payload.push((client_id.len() & 0xFF) as u8);
    payload.extend_from_slice(client_id.as_bytes());

    // Fixed header
    let remaining_len = var_header.len() + payload.len();
    packet.push(CONNECT << 4);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend(payload);
    packet
}

/// Build MQTT SUBSCRIBE packet
pub fn build_subscribe_packet(packet_id: u16, topic: &str) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header (packet ID)
    let mut var_header = Vec::new();
    var_header.push((packet_id >> 8) as u8);
    var_header.push((packet_id & 0xFF) as u8);

    // Payload (topic filter + QoS)
    let mut payload = Vec::new();
    payload.push((topic.len() >> 8) as u8);
    payload.push((topic.len() & 0xFF) as u8);
    payload.extend_from_slice(topic.as_bytes());
    payload.push(0x00); // QoS 0

    // Fixed header
    let remaining_len = var_header.len() + payload.len();
    packet.push((SUBSCRIBE << 4) | 0x02);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend(payload);
    packet
}

/// Build MQTT PUBLISH packet with QoS 1
pub fn build_publish_packet(topic: &str, payload: &[u8], packet_id: u16) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header (topic + packet ID for QoS 1)
    let mut var_header = Vec::new();
    var_header.push((topic.len() >> 8) as u8);
    var_header.push((topic.len() & 0xFF) as u8);
    var_header.extend_from_slice(topic.as_bytes());

    // Packet ID (for QoS 1)
    var_header.push((packet_id >> 8) as u8);
    var_header.push((packet_id & 0xFF) as u8);

    // Fixed header (PUBLISH with QoS 1)
    let remaining_len = var_header.len() + payload.len();
    packet.push((PUBLISH << 4) | 0x02);  // QoS 1
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend_from_slice(payload);
    packet
}

/// Build MQTT PINGREQ packet
pub fn build_pingreq_packet() -> Vec<u8> {
    vec![PINGREQ << 4, 0x00]
}

/// Parse MQTT packet type from first byte
pub fn packet_type(byte: u8) -> u8 {
    byte >> 4
}

/// Parse MQTT PUBLISH packet, returns (topic, payload)
pub fn parse_publish_packet(packet: &[u8]) -> Option<(String, Vec<u8>)> {
    if packet.is_empty() || packet_type(packet[0]) != PUBLISH {
        return None;
    }

    let mut offset = 1;

    // Decode remaining length
    let (_remaining_len, len_bytes) = decode_remaining_length(&packet[offset..])?;
    offset += len_bytes;

    // Topic length
    if offset + 2 > packet.len() {
        return None;
    }
    let topic_len = ((packet[offset] as usize) << 8) | (packet[offset + 1] as usize);
    offset += 2;

    // Topic
    if offset + topic_len > packet.len() {
        return None;
    }
    let topic = core::str::from_utf8(&packet[offset..offset + topic_len]).ok()?;
    offset += topic_len;

    // Payload
    let payload = packet[offset..].to_vec();

    Some((String::from(topic), payload))
}

/// Check if packet is CONNACK with success
pub fn is_connack_success(packet: &[u8]) -> bool {
    packet.len() >= 4
        && packet_type(packet[0]) == CONNACK
        && packet[3] == 0x00 // Return code 0 = accepted
}

/// Check if packet is SUBACK
pub fn is_suback(packet: &[u8]) -> bool {
    packet.len() >= 2 && packet_type(packet[0]) == SUBACK
}

/// Check if packet is PINGRESP
pub fn is_pingresp(packet: &[u8]) -> bool {
    packet.len() >= 2 && packet_type(packet[0]) == PINGRESP
}

/// Encode remaining length (MQTT variable length encoding)
fn encode_remaining_length(packet: &mut Vec<u8>, mut len: usize) {
    loop {
        let mut byte = (len & 0x7F) as u8;
        len >>= 7;
        if len > 0 {
            byte |= 0x80;
        }
        packet.push(byte);
        if len == 0 {
            break;
        }
    }
}

/// Decode remaining length, returns (length, bytes_consumed)
fn decode_remaining_length(data: &[u8]) -> Option<(usize, usize)> {
    let mut len = 0usize;
    let mut multiplier = 1usize;
    let mut bytes_consumed = 0;

    for &byte in data.iter().take(4) {
        bytes_consumed += 1;
        len += ((byte & 0x7F) as usize) * multiplier;
        multiplier *= 128;
        if (byte & 0x80) == 0 {
            return Some((len, bytes_consumed));
        }
    }
    None
}

/// MQTT connection state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MqttState {
    Disconnected,
    Connecting,
    Connected,
}

/// MQTT client configuration
pub struct MqttConfig {
    pub broker_addr: String,
    pub client_id: String,
    pub subscribe_topic: String,
    pub publish_topic: String,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker_addr: String::from("127.0.0.1:1883"),
            client_id: String::from("ccr-precursor"),
            subscribe_topic: String::from("ccr/events"),
            publish_topic: String::from("ccr/responses"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connect_packet() {
        let packet = build_connect_packet("test-client");
        assert_eq!(packet[0] >> 4, CONNECT);
    }

    #[test]
    fn test_build_publish_packet() {
        let packet = build_publish_packet("test/topic", b"hello");
        assert_eq!(packet[0] >> 4, PUBLISH);
    }

    #[test]
    fn test_parse_publish_packet() {
        let packet = build_publish_packet("test/topic", b"hello");
        let (topic, payload) = parse_publish_packet(&packet).unwrap();
        assert_eq!(topic, "test/topic");
        assert_eq!(payload, b"hello");
    }
}
