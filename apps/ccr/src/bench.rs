// CCR Performance Benchmark
//
// Measures SoC processing ceiling for MQTT message handling

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

/// Run all benchmarks and return results as a string
pub fn run_all_benchmarks() -> String {
    let tt = ticktimer_server::Ticktimer::new().unwrap();
    let iterations = 1000u32;  // Reduced for faster results

    log::info!("CCR: Starting benchmarks ({} iterations)", iterations);

    let mut results = String::new();

    // JSON parse benchmark
    let json_result = bench_json_parse(&tt, iterations);
    results.push_str(&format!("JSON Parse:\n  {} ops/sec\n\n", json_result));

    // MQTT parse benchmark
    let mqtt_result = bench_mqtt_parse(&tt, iterations);
    results.push_str(&format!("MQTT Parse:\n  {} ops/sec\n\n", mqtt_result));

    // Event queue benchmark
    let queue_result = bench_event_queue(&tt, iterations);
    results.push_str(&format!("Event Queue:\n  {} ops/sec\n\n", queue_result));

    // Full pipeline benchmark
    let pipeline_result = bench_full_pipeline(&tt, iterations);
    results.push_str(&format!("Full Pipeline:\n  {} ops/sec\n", pipeline_result));

    results
}

fn bench_json_parse(tt: &ticktimer_server::Ticktimer, iterations: u32) -> u32 {
    let sample = r#"{"type":"tool","id":"t1","tool":"Read","target":"main.rs"}"#;

    let start = tt.elapsed_ms();

    for _ in 0..iterations {
        let _ = parse_json_field(sample.as_bytes(), "type");
        let _ = parse_json_field(sample.as_bytes(), "id");
        let _ = parse_json_field(sample.as_bytes(), "tool");
    }

    let elapsed = tt.elapsed_ms() - start;
    if elapsed > 0 { ((iterations as u64) * 1000 / elapsed) as u32 } else { 0 }
}

fn bench_mqtt_parse(tt: &ticktimer_server::Ticktimer, iterations: u32) -> u32 {
    let packet = build_mqtt_packet("ccr/events", b"test payload");

    let start = tt.elapsed_ms();

    for _ in 0..iterations {
        let _ = parse_mqtt_packet(&packet);
    }

    let elapsed = tt.elapsed_ms() - start;
    if elapsed > 0 { ((iterations as u64) * 1000 / elapsed) as u32 } else { 0 }
}

fn bench_event_queue(tt: &ticktimer_server::Ticktimer, iterations: u32) -> u32 {
    let mut queue: Vec<u32> = Vec::with_capacity(64);

    let start = tt.elapsed_ms();

    for i in 0..iterations {
        queue.push(i);
        if queue.len() > 32 {
            queue.remove(0);
        }
    }

    let elapsed = tt.elapsed_ms() - start;
    if elapsed > 0 { ((iterations as u64) * 1000 / elapsed) as u32 } else { 0 }
}

fn bench_full_pipeline(tt: &ticktimer_server::Ticktimer, iterations: u32) -> u32 {
    let packet = build_mqtt_packet("ccr/events",
        r#"{"type":"tool","id":"t1"}"#.as_bytes());
    let mut queue: Vec<u32> = Vec::with_capacity(64);

    let start = tt.elapsed_ms();

    for i in 0..iterations {
        if let Some((_topic, payload)) = parse_mqtt_packet(&packet) {
            let _ = parse_json_field(payload, "type");
            queue.push(i);
            if queue.len() > 32 {
                queue.remove(0);
            }
        }
    }

    let elapsed = tt.elapsed_ms() - start;
    if elapsed > 0 { ((iterations as u64) * 1000 / elapsed) as u32 } else { 0 }
}

// Simple JSON field parser
fn parse_json_field<'a>(data: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let text = core::str::from_utf8(data).ok()?;
    let pattern = alloc::format!("\"{}\":\"", key);
    let start = text.find(&pattern)? + pattern.len();
    let end = text[start..].find('"')? + start;
    Some(&data[start..end])
}

// Simple MQTT PUBLISH packet builder
fn build_mqtt_packet(topic: &str, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::new();
    let remaining = 2 + topic.len() + payload.len();

    packet.push(0x30); // PUBLISH
    packet.push(remaining as u8);
    packet.push((topic.len() >> 8) as u8);
    packet.push((topic.len() & 0xFF) as u8);
    packet.extend_from_slice(topic.as_bytes());
    packet.extend_from_slice(payload);
    packet
}

// Simple MQTT PUBLISH packet parser
fn parse_mqtt_packet(packet: &[u8]) -> Option<(&str, &[u8])> {
    if packet.len() < 4 || (packet[0] >> 4) != 3 {
        return None;
    }

    let remaining_len = packet[1] as usize;
    if packet.len() < 2 + remaining_len {
        return None;
    }

    let topic_len = ((packet[2] as usize) << 8) | (packet[3] as usize);
    if packet.len() < 4 + topic_len {
        return None;
    }

    let topic = core::str::from_utf8(&packet[4..4 + topic_len]).ok()?;
    let payload = &packet[4 + topic_len..];
    Some((topic, payload))
}
