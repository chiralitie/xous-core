//! MQTT Packet Encoding/Decoding
//!
//! Pure Rust implementation of MQTT 3.1.1 packet format.
//! No external dependencies, no_std compatible.

extern crate alloc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// MQTT packet types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    Connect = 1,
    Connack = 2,
    Publish = 3,
    Puback = 4,
    Pubrec = 5,
    Pubrel = 6,
    Pubcomp = 7,
    Subscribe = 8,
    Suback = 9,
    Unsubscribe = 10,
    Unsuback = 11,
    Pingreq = 12,
    Pingresp = 13,
    Disconnect = 14,
}

impl PacketType {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte >> 4 {
            1 => Some(Self::Connect),
            2 => Some(Self::Connack),
            3 => Some(Self::Publish),
            4 => Some(Self::Puback),
            5 => Some(Self::Pubrec),
            6 => Some(Self::Pubrel),
            7 => Some(Self::Pubcomp),
            8 => Some(Self::Subscribe),
            9 => Some(Self::Suback),
            10 => Some(Self::Unsubscribe),
            11 => Some(Self::Unsuback),
            12 => Some(Self::Pingreq),
            13 => Some(Self::Pingresp),
            14 => Some(Self::Disconnect),
            _ => None,
        }
    }
}

/// Quality of Service levels
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QoS {
    /// At most once (fire and forget)
    #[default]
    AtMostOnce = 0,
    /// At least once (acknowledged delivery)
    AtLeastOnce = 1,
    /// Exactly once (assured delivery)
    ExactlyOnce = 2,
}

impl QoS {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte & 0x03 {
            0 => Some(Self::AtMostOnce),
            1 => Some(Self::AtLeastOnce),
            2 => Some(Self::ExactlyOnce),
            _ => None,
        }
    }
}

/// CONNACK return codes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnackCode {
    Accepted = 0,
    UnacceptableProtocol = 1,
    IdentifierRejected = 2,
    ServerUnavailable = 3,
    BadCredentials = 4,
    NotAuthorized = 5,
}

impl ConnackCode {
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Accepted),
            1 => Some(Self::UnacceptableProtocol),
            2 => Some(Self::IdentifierRejected),
            3 => Some(Self::ServerUnavailable),
            4 => Some(Self::BadCredentials),
            5 => Some(Self::NotAuthorized),
            _ => None,
        }
    }
}

// ============================================================================
// Packet Builders
// ============================================================================

/// Build MQTT CONNECT packet
pub fn build_connect(client_id: &str) -> Vec<u8> {
    build_connect_with_options(client_id, None, None, true, 60)
}

/// Build MQTT CONNECT packet with full options
pub fn build_connect_with_options(
    client_id: &str,
    username: Option<&str>,
    password: Option<&[u8]>,
    clean_session: bool,
    keep_alive_secs: u16,
) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header
    let mut var_header = Vec::new();

    // Protocol name "MQTT"
    var_header.push(0x00);
    var_header.push(0x04);
    var_header.extend_from_slice(b"MQTT");

    // Protocol level (4 = MQTT 3.1.1)
    var_header.push(0x04);

    // Connect flags
    let mut flags: u8 = 0;
    if clean_session {
        flags |= 0x02;
    }
    if username.is_some() {
        flags |= 0x80;
    }
    if password.is_some() {
        flags |= 0x40;
    }
    var_header.push(flags);

    // Keep alive
    var_header.push((keep_alive_secs >> 8) as u8);
    var_header.push((keep_alive_secs & 0xFF) as u8);

    // Payload
    let mut payload = Vec::new();

    // Client ID (required)
    encode_string(&mut payload, client_id);

    // Username (optional)
    if let Some(user) = username {
        encode_string(&mut payload, user);
    }

    // Password (optional)
    if let Some(pass) = password {
        encode_bytes(&mut payload, pass);
    }

    // Fixed header
    let remaining_len = var_header.len() + payload.len();
    packet.push((PacketType::Connect as u8) << 4);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend(payload);
    packet
}

/// Build MQTT SUBSCRIBE packet
pub fn build_subscribe(packet_id: u16, topic: &str, qos: QoS) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header (packet ID)
    let mut var_header = Vec::new();
    var_header.push((packet_id >> 8) as u8);
    var_header.push((packet_id & 0xFF) as u8);

    // Payload (topic filter + QoS)
    let mut payload = Vec::new();
    encode_string(&mut payload, topic);
    payload.push(qos as u8);

    // Fixed header (SUBSCRIBE has reserved bits 0010)
    let remaining_len = var_header.len() + payload.len();
    packet.push(((PacketType::Subscribe as u8) << 4) | 0x02);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend(payload);
    packet
}

/// Build MQTT UNSUBSCRIBE packet
pub fn build_unsubscribe(packet_id: u16, topic: &str) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header (packet ID)
    let mut var_header = Vec::new();
    var_header.push((packet_id >> 8) as u8);
    var_header.push((packet_id & 0xFF) as u8);

    // Payload (topic filter)
    let mut payload = Vec::new();
    encode_string(&mut payload, topic);

    // Fixed header (UNSUBSCRIBE has reserved bits 0010)
    let remaining_len = var_header.len() + payload.len();
    packet.push(((PacketType::Unsubscribe as u8) << 4) | 0x02);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend(payload);
    packet
}

/// Build MQTT PUBLISH packet (QoS 0)
pub fn build_publish(topic: &str, payload: &[u8], qos: QoS) -> Vec<u8> {
    build_publish_with_id(topic, payload, qos, None, false)
}

/// Build MQTT PUBLISH packet with packet ID (for QoS 1/2)
pub fn build_publish_with_id(
    topic: &str,
    payload: &[u8],
    qos: QoS,
    packet_id: Option<u16>,
    retain: bool,
) -> Vec<u8> {
    let mut packet = Vec::new();

    // Variable header
    let mut var_header = Vec::new();
    encode_string(&mut var_header, topic);

    // Packet ID (required for QoS > 0)
    if let Some(id) = packet_id {
        var_header.push((id >> 8) as u8);
        var_header.push((id & 0xFF) as u8);
    }

    // Fixed header
    let remaining_len = var_header.len() + payload.len();
    let mut flags = (PacketType::Publish as u8) << 4;
    flags |= (qos as u8) << 1;
    if retain {
        flags |= 0x01;
    }
    packet.push(flags);
    encode_remaining_length(&mut packet, remaining_len);

    packet.extend(var_header);
    packet.extend_from_slice(payload);
    packet
}

/// Build MQTT PUBACK packet (QoS 1 acknowledgment)
pub fn build_puback(packet_id: u16) -> Vec<u8> {
    vec![
        (PacketType::Puback as u8) << 4,
        0x02,
        (packet_id >> 8) as u8,
        (packet_id & 0xFF) as u8,
    ]
}

/// Build MQTT PUBREC packet (QoS 2 step 1)
pub fn build_pubrec(packet_id: u16) -> Vec<u8> {
    vec![
        (PacketType::Pubrec as u8) << 4,
        0x02,
        (packet_id >> 8) as u8,
        (packet_id & 0xFF) as u8,
    ]
}

/// Build MQTT PUBREL packet (QoS 2 step 2)
pub fn build_pubrel(packet_id: u16) -> Vec<u8> {
    vec![
        ((PacketType::Pubrel as u8) << 4) | 0x02, // Reserved bits
        0x02,
        (packet_id >> 8) as u8,
        (packet_id & 0xFF) as u8,
    ]
}

/// Build MQTT PUBCOMP packet (QoS 2 step 3)
pub fn build_pubcomp(packet_id: u16) -> Vec<u8> {
    vec![
        (PacketType::Pubcomp as u8) << 4,
        0x02,
        (packet_id >> 8) as u8,
        (packet_id & 0xFF) as u8,
    ]
}

/// Build MQTT PINGREQ packet
pub fn build_pingreq() -> Vec<u8> {
    vec![(PacketType::Pingreq as u8) << 4, 0x00]
}

/// Build MQTT DISCONNECT packet
pub fn build_disconnect() -> Vec<u8> {
    vec![(PacketType::Disconnect as u8) << 4, 0x00]
}

// ============================================================================
// Packet Parsers
// ============================================================================

/// Parsed MQTT packet
#[derive(Debug, Clone)]
pub enum Packet {
    Connack {
        session_present: bool,
        code: ConnackCode,
    },
    Publish {
        topic: String,
        payload: Vec<u8>,
        qos: QoS,
        packet_id: Option<u16>,
        retain: bool,
        dup: bool,
    },
    Puback {
        packet_id: u16,
    },
    Pubrec {
        packet_id: u16,
    },
    Pubrel {
        packet_id: u16,
    },
    Pubcomp {
        packet_id: u16,
    },
    Suback {
        packet_id: u16,
        return_codes: Vec<u8>,
    },
    Unsuback {
        packet_id: u16,
    },
    Pingresp,
}

/// Parse error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// Not enough data
    Incomplete,
    /// Invalid packet format
    InvalidFormat,
    /// Unknown packet type
    UnknownType,
    /// Invalid UTF-8 string
    InvalidUtf8,
}

/// Parse a complete MQTT packet from buffer
/// Returns (packet, bytes_consumed) or error
pub fn parse_packet(data: &[u8]) -> Result<(Packet, usize), ParseError> {
    if data.is_empty() {
        return Err(ParseError::Incomplete);
    }

    let first_byte = data[0];
    let packet_type = PacketType::from_byte(first_byte).ok_or(ParseError::UnknownType)?;

    // Decode remaining length
    let (remaining_len, len_bytes) = decode_remaining_length(&data[1..])
        .ok_or(ParseError::Incomplete)?;

    let header_len = 1 + len_bytes;
    let total_len = header_len + remaining_len;

    if data.len() < total_len {
        return Err(ParseError::Incomplete);
    }

    let payload = &data[header_len..total_len];

    let packet = match packet_type {
        PacketType::Connack => parse_connack(payload)?,
        PacketType::Publish => parse_publish(first_byte, payload)?,
        PacketType::Puback => parse_puback(payload)?,
        PacketType::Pubrec => parse_pubrec(payload)?,
        PacketType::Pubrel => parse_pubrel(payload)?,
        PacketType::Pubcomp => parse_pubcomp(payload)?,
        PacketType::Suback => parse_suback(payload)?,
        PacketType::Unsuback => parse_unsuback(payload)?,
        PacketType::Pingresp => Packet::Pingresp,
        _ => return Err(ParseError::UnknownType),
    };

    Ok((packet, total_len))
}

fn parse_connack(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let session_present = (data[0] & 0x01) != 0;
    let code = ConnackCode::from_byte(data[1]).ok_or(ParseError::InvalidFormat)?;
    Ok(Packet::Connack { session_present, code })
}

fn parse_publish(first_byte: u8, data: &[u8]) -> Result<Packet, ParseError> {
    let dup = (first_byte & 0x08) != 0;
    let qos = QoS::from_byte((first_byte >> 1) & 0x03).ok_or(ParseError::InvalidFormat)?;
    let retain = (first_byte & 0x01) != 0;

    let mut offset = 0;

    // Topic
    let (topic, topic_len) = decode_string(&data[offset..])?;
    offset += topic_len;

    // Packet ID (only for QoS > 0)
    let packet_id = if qos != QoS::AtMostOnce {
        if data.len() < offset + 2 {
            return Err(ParseError::Incomplete);
        }
        let id = ((data[offset] as u16) << 8) | (data[offset + 1] as u16);
        offset += 2;
        Some(id)
    } else {
        None
    };

    // Payload
    let payload = data[offset..].to_vec();

    Ok(Packet::Publish {
        topic,
        payload,
        qos,
        packet_id,
        retain,
        dup,
    })
}

fn parse_puback(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    Ok(Packet::Puback { packet_id })
}

fn parse_pubrec(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    Ok(Packet::Pubrec { packet_id })
}

fn parse_pubrel(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    Ok(Packet::Pubrel { packet_id })
}

fn parse_pubcomp(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    Ok(Packet::Pubcomp { packet_id })
}

fn parse_suback(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 3 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    let return_codes = data[2..].to_vec();
    Ok(Packet::Suback { packet_id, return_codes })
}

fn parse_unsuback(data: &[u8]) -> Result<Packet, ParseError> {
    if data.len() < 2 {
        return Err(ParseError::InvalidFormat);
    }
    let packet_id = ((data[0] as u16) << 8) | (data[1] as u16);
    Ok(Packet::Unsuback { packet_id })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Encode MQTT remaining length (variable length encoding)
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

/// Decode MQTT remaining length, returns (length, bytes_consumed)
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

/// Encode a UTF-8 string with length prefix
fn encode_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.push((bytes.len() >> 8) as u8);
    buf.push((bytes.len() & 0xFF) as u8);
    buf.extend_from_slice(bytes);
}

/// Encode binary data with length prefix
fn encode_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    buf.push((data.len() >> 8) as u8);
    buf.push((data.len() & 0xFF) as u8);
    buf.extend_from_slice(data);
}

/// Decode a UTF-8 string with length prefix, returns (string, bytes_consumed)
fn decode_string(data: &[u8]) -> Result<(String, usize), ParseError> {
    if data.len() < 2 {
        return Err(ParseError::Incomplete);
    }
    let len = ((data[0] as usize) << 8) | (data[1] as usize);
    if data.len() < 2 + len {
        return Err(ParseError::Incomplete);
    }
    let s = core::str::from_utf8(&data[2..2 + len])
        .map_err(|_| ParseError::InvalidUtf8)?;
    Ok((String::from(s), 2 + len))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connect_packet() {
        let packet = build_connect("test-client");
        assert_eq!(packet[0] >> 4, PacketType::Connect as u8);
    }

    #[test]
    fn test_publish_roundtrip() {
        let original = build_publish("test/topic", b"hello world", QoS::AtMostOnce);
        let (parsed, len) = parse_packet(&original).unwrap();
        assert_eq!(len, original.len());

        if let Packet::Publish { topic, payload, qos, .. } = parsed {
            assert_eq!(topic, "test/topic");
            assert_eq!(payload, b"hello world");
            assert_eq!(qos, QoS::AtMostOnce);
        } else {
            panic!("Expected Publish packet");
        }
    }

    #[test]
    fn test_subscribe_packet() {
        let packet = build_subscribe(1, "events/#", QoS::AtLeastOnce);
        assert_eq!(packet[0] >> 4, PacketType::Subscribe as u8);
    }

    #[test]
    fn test_pingreq() {
        let packet = build_pingreq();
        assert_eq!(packet, vec![0xC0, 0x00]);
    }

    #[test]
    fn test_remaining_length_encoding() {
        let mut buf = Vec::new();
        encode_remaining_length(&mut buf, 127);
        assert_eq!(buf, vec![127]);

        buf.clear();
        encode_remaining_length(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);

        buf.clear();
        encode_remaining_length(&mut buf, 16383);
        assert_eq!(buf, vec![0xFF, 0x7F]);
    }
}
