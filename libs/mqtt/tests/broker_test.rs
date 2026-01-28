//! Integration test for MQTT packet implementation against real broker
//!
//! Run with: cargo test -p xous-mqtt --test broker_test -- --nocapture

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

// Import from the library
use xous_mqtt::packet::{self, QoS, Packet};

fn connect_to_broker() -> std::io::Result<TcpStream> {
    let stream = TcpStream::connect("127.0.0.1:1883")?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    Ok(stream)
}

#[test]
fn test_connect_and_disconnect() {
    println!("\n=== Test: CONNECT and DISCONNECT ===");

    let mut stream = connect_to_broker().expect("Failed to connect to broker");
    println!("TCP connected to 127.0.0.1:1883");

    // Send CONNECT
    let connect_packet = packet::build_connect("xous-mqtt-test");
    println!("Sending CONNECT packet ({} bytes)", connect_packet.len());
    stream.write_all(&connect_packet).expect("Failed to send CONNECT");

    // Read CONNACK
    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("Failed to read CONNACK");
    println!("Received {} bytes", n);

    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse CONNACK");
    match parsed {
        Packet::Connack { session_present, code } => {
            println!("CONNACK: session_present={}, code={:?}", session_present, code);
            assert_eq!(code, packet::ConnackCode::Accepted);
        }
        _ => panic!("Expected CONNACK, got {:?}", parsed),
    }

    // Send DISCONNECT
    let disconnect_packet = packet::build_disconnect();
    stream.write_all(&disconnect_packet).expect("Failed to send DISCONNECT");
    println!("Sent DISCONNECT");

    println!("=== PASS ===\n");
}

#[test]
fn test_subscribe_and_publish() {
    println!("\n=== Test: SUBSCRIBE and PUBLISH ===");

    let mut stream = connect_to_broker().expect("Failed to connect to broker");

    // Connect
    let connect_packet = packet::build_connect("xous-mqtt-pubsub-test");
    stream.write_all(&connect_packet).expect("Failed to send CONNECT");

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("Failed to read CONNACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse CONNACK");
    assert!(matches!(parsed, Packet::Connack { code: packet::ConnackCode::Accepted, .. }));
    println!("Connected");

    // Subscribe
    let subscribe_packet = packet::build_subscribe(1, "test/xous-mqtt/#", QoS::AtMostOnce);
    println!("Sending SUBSCRIBE to test/xous-mqtt/# ({} bytes)", subscribe_packet.len());
    stream.write_all(&subscribe_packet).expect("Failed to send SUBSCRIBE");

    // Read SUBACK
    let n = stream.read(&mut buf).expect("Failed to read SUBACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse SUBACK");
    match parsed {
        Packet::Suback { packet_id, return_codes } => {
            println!("SUBACK: packet_id={}, return_codes={:?}", packet_id, return_codes);
            assert_eq!(packet_id, 1);
        }
        _ => panic!("Expected SUBACK, got {:?}", parsed),
    }

    // Publish (QoS 0 - no response expected)
    let payload = b"Hello from xous-mqtt test!";
    let publish_packet = packet::build_publish("test/xous-mqtt/greeting", payload, QoS::AtMostOnce);
    println!("Sending PUBLISH to test/xous-mqtt/greeting ({} bytes)", publish_packet.len());
    stream.write_all(&publish_packet).expect("Failed to send PUBLISH");

    // We should receive our own message back (since we're subscribed)
    let n = stream.read(&mut buf).expect("Failed to read PUBLISH");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse PUBLISH");
    match parsed {
        Packet::Publish { topic, payload: recv_payload, .. } => {
            println!("Received PUBLISH: topic={}, payload={:?}", topic, String::from_utf8_lossy(&recv_payload));
            assert_eq!(topic, "test/xous-mqtt/greeting");
            assert_eq!(recv_payload, payload);
        }
        _ => panic!("Expected PUBLISH, got {:?}", parsed),
    }

    // Disconnect
    stream.write_all(&packet::build_disconnect()).ok();
    println!("=== PASS ===\n");
}

#[test]
fn test_qos1_publish() {
    println!("\n=== Test: QoS 1 PUBLISH ===");

    let mut stream = connect_to_broker().expect("Failed to connect to broker");

    // Connect
    let connect_packet = packet::build_connect("xous-mqtt-qos1-test");
    stream.write_all(&connect_packet).expect("Failed to send CONNECT");

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("Failed to read CONNACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse CONNACK");
    assert!(matches!(parsed, Packet::Connack { code: packet::ConnackCode::Accepted, .. }));
    println!("Connected");

    // Publish with QoS 1
    let payload = b"QoS 1 message";
    let packet_id = 42u16;
    let publish_packet = packet::build_publish_with_id(
        "test/xous-mqtt/qos1",
        payload,
        QoS::AtLeastOnce,
        Some(packet_id),
        false,
    );
    println!("Sending QoS 1 PUBLISH (packet_id={}, {} bytes)", packet_id, publish_packet.len());
    stream.write_all(&publish_packet).expect("Failed to send PUBLISH");

    // Should receive PUBACK
    let n = stream.read(&mut buf).expect("Failed to read PUBACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse PUBACK");
    match parsed {
        Packet::Puback { packet_id: recv_id } => {
            println!("Received PUBACK: packet_id={}", recv_id);
            assert_eq!(recv_id, packet_id);
        }
        _ => panic!("Expected PUBACK, got {:?}", parsed),
    }

    // Disconnect
    stream.write_all(&packet::build_disconnect()).ok();
    println!("=== PASS ===\n");
}

#[test]
fn test_qos2_publish() {
    println!("\n=== Test: QoS 2 PUBLISH (Exactly Once) ===");

    let mut stream = connect_to_broker().expect("Failed to connect to broker");

    // Connect
    let connect_packet = packet::build_connect("xous-mqtt-qos2-test");
    stream.write_all(&connect_packet).expect("Failed to send CONNECT");

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("Failed to read CONNACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse CONNACK");
    assert!(matches!(parsed, Packet::Connack { code: packet::ConnackCode::Accepted, .. }));
    println!("Connected");

    // Publish with QoS 2
    let payload = b"QoS 2 exactly-once message";
    let packet_id = 100u16;
    let publish_packet = packet::build_publish_with_id(
        "test/xous-mqtt/qos2",
        payload,
        QoS::ExactlyOnce,
        Some(packet_id),
        false,
    );
    println!("Sending QoS 2 PUBLISH (packet_id={}, {} bytes)", packet_id, publish_packet.len());
    stream.write_all(&publish_packet).expect("Failed to send PUBLISH");

    // Step 1: Should receive PUBREC
    let n = stream.read(&mut buf).expect("Failed to read PUBREC");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse PUBREC");
    match parsed {
        Packet::Pubrec { packet_id: recv_id } => {
            println!("Received PUBREC: packet_id={}", recv_id);
            assert_eq!(recv_id, packet_id);
        }
        _ => panic!("Expected PUBREC, got {:?}", parsed),
    }

    // Step 2: Send PUBREL
    let pubrel_packet = packet::build_pubrel(packet_id);
    println!("Sending PUBREL (packet_id={})", packet_id);
    stream.write_all(&pubrel_packet).expect("Failed to send PUBREL");

    // Step 3: Should receive PUBCOMP
    let n = stream.read(&mut buf).expect("Failed to read PUBCOMP");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse PUBCOMP");
    match parsed {
        Packet::Pubcomp { packet_id: recv_id } => {
            println!("Received PUBCOMP: packet_id={}", recv_id);
            assert_eq!(recv_id, packet_id);
        }
        _ => panic!("Expected PUBCOMP, got {:?}", parsed),
    }

    println!("QoS 2 handshake complete!");

    // Disconnect
    stream.write_all(&packet::build_disconnect()).ok();
    println!("=== PASS ===\n");
}

#[test]
fn test_ping() {
    println!("\n=== Test: PINGREQ/PINGRESP ===");

    let mut stream = connect_to_broker().expect("Failed to connect to broker");

    // Connect
    let connect_packet = packet::build_connect("xous-mqtt-ping-test");
    stream.write_all(&connect_packet).expect("Failed to send CONNECT");

    let mut buf = [0u8; 256];
    let n = stream.read(&mut buf).expect("Failed to read CONNACK");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse CONNACK");
    assert!(matches!(parsed, Packet::Connack { code: packet::ConnackCode::Accepted, .. }));
    println!("Connected");

    // Send PINGREQ
    let ping_packet = packet::build_pingreq();
    println!("Sending PINGREQ ({} bytes)", ping_packet.len());
    stream.write_all(&ping_packet).expect("Failed to send PINGREQ");

    // Should receive PINGRESP
    let n = stream.read(&mut buf).expect("Failed to read PINGRESP");
    let (parsed, _) = packet::parse_packet(&buf[..n]).expect("Failed to parse PINGRESP");
    match parsed {
        Packet::Pingresp => {
            println!("Received PINGRESP");
        }
        _ => panic!("Expected PINGRESP, got {:?}", parsed),
    }

    // Disconnect
    stream.write_all(&packet::build_disconnect()).ok();
    println!("=== PASS ===\n");
}
