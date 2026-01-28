// CCR Performance Benchmark
//
// Measures SoC processing ceiling for MQTT message handling

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::fmt::Write;

/// PDDB dictionary name for CCR
const CCR_DICT: &str = "ccr.bench";

/// Run all benchmarks and return results as a string
pub fn run_all_benchmarks(with_pddb: bool) -> String {
    let tt = ticktimer_server::Ticktimer::new().unwrap();
    let iterations = 1000u32;

    log::info!("CCR: Starting benchmarks ({} iterations)", iterations);

    let mut results = String::new();

    // RAM-only benchmarks
    let json_result = bench_json_parse(&tt, iterations);
    writeln!(results, "JSON Parse:\n  {} ops/sec", json_result).ok();

    let mqtt_result = bench_mqtt_parse(&tt, iterations);
    writeln!(results, "MQTT Parse:\n  {} ops/sec", mqtt_result).ok();

    let queue_result = bench_event_queue(&tt, iterations);
    writeln!(results, "Event Queue:\n  {} ops/sec", queue_result).ok();

    let pipeline_result = bench_full_pipeline(&tt, iterations);
    writeln!(results, "Full Pipeline:\n  {} ops/sec", pipeline_result).ok();

    // PDDB benchmarks (if requested and available)
    if with_pddb {
        writeln!(results, "\n--- PDDB Tests ---").ok();

        let poller = pddb::PddbMountPoller::new();
        if poller.is_mounted_nonblocking() {
            let pddb = pddb::Pddb::new();
            let pddb_iterations = 100u32;  // Fewer iterations for slow PDDB

            let write_result = bench_pddb_write(&tt, &pddb, pddb_iterations);
            writeln!(results, "PDDB Write:\n  {} ops/sec", write_result).ok();

            let read_result = bench_pddb_read(&tt, &pddb, pddb_iterations);
            writeln!(results, "PDDB Read:\n  {} ops/sec", read_result).ok();

            // Cleanup
            pddb.delete_key(CCR_DICT, "bench_key", None).ok();
            pddb.delete_dict(CCR_DICT, None).ok();
        } else {
            writeln!(results, "PDDB not mounted\n(init root keys first)").ok();
        }
    }

    results
}

/// Run benchmarks without PDDB (default)
pub fn run_ram_benchmarks() -> String {
    run_all_benchmarks(false)
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

fn bench_pddb_write(tt: &ticktimer_server::Ticktimer, pddb: &pddb::Pddb, iterations: u32) -> u32 {
    use std::io::Write;

    let test_data = b"test_value_for_benchmark_1234567890";

    let start = tt.elapsed_ms();

    for i in 0..iterations {
        let key = format!("bench_{}", i);
        if let Ok(mut pddb_key) = pddb.get(CCR_DICT, &key, None, true, true, None, None::<fn()>) {
            pddb_key.write(test_data).ok();
        }
    }
    pddb.sync().ok();

    let elapsed = tt.elapsed_ms() - start;

    // Cleanup
    for i in 0..iterations {
        let key = format!("bench_{}", i);
        pddb.delete_key(CCR_DICT, &key, None).ok();
    }

    if elapsed > 0 { ((iterations as u64) * 1000 / elapsed) as u32 } else { 0 }
}

fn bench_pddb_read(tt: &ticktimer_server::Ticktimer, pddb: &pddb::Pddb, iterations: u32) -> u32 {
    use std::io::{Read, Write};

    // Setup: write a key to read
    let test_data = b"test_value_for_benchmark_1234567890";
    if let Ok(mut pddb_key) = pddb.get(CCR_DICT, "bench_key", None, true, true, None, None::<fn()>) {
        pddb_key.write(test_data).ok();
    }
    pddb.sync().ok();

    let start = tt.elapsed_ms();

    for _ in 0..iterations {
        if let Ok(mut pddb_key) = pddb.get(CCR_DICT, "bench_key", None, false, false, None, None::<fn()>) {
            let mut buf = [0u8; 64];
            pddb_key.read(&mut buf).ok();
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
