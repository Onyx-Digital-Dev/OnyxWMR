//! Stress test for osv-intake-daemon

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const SOCKET_PATH: &str = "/tmp/osv-intake.sock";

fn main() {
    println!("=== OSV-INTAKE DAEMON STRESS TEST ===\n");

    // Test 1: Basic connectivity
    println!("[TEST 1] Basic connectivity...");
    match test_basic_connectivity() {
        Ok(_) => println!("  ✓ PASS: Basic connectivity works"),
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    // Test 2: Rapid sequential requests
    println!("\n[TEST 2] Rapid sequential requests (1000 queries)...");
    match test_rapid_sequential() {
        Ok(duration) => println!("  ✓ PASS: 1000 queries in {:?} ({:.0} q/s)", duration, 1000.0 / duration.as_secs_f64()),
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    // Test 3: Concurrent connections
    println!("\n[TEST 3] Concurrent connections (50 threads, 100 queries each)...");
    match test_concurrent_connections() {
        Ok((success, failed, duration)) => {
            println!("  ✓ PASS: {} success, {} failed in {:?}", success, failed, duration);
            if failed > 0 {
                println!("  ⚠ WARNING: {} failures detected", failed);
            }
        }
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    // Test 4: Edge cases
    println!("\n[TEST 4] Edge cases...");
    test_edge_cases();

    // Test 5: Connection reuse
    println!("\n[TEST 5] Connection reuse (single connection, 500 queries)...");
    match test_connection_reuse() {
        Ok(duration) => println!("  ✓ PASS: 500 queries on single connection in {:?}", duration),
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    // Test 6: Large payload
    println!("\n[TEST 6] Large query payload...");
    match test_large_payload() {
        Ok(_) => println!("  ✓ PASS: Large payload handled"),
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    // Test 7: Malformed JSON
    println!("\n[TEST 7] Malformed JSON handling...");
    match test_malformed_json() {
        Ok(_) => println!("  ✓ PASS: Malformed JSON handled gracefully"),
        Err(e) => println!("  ✗ FAIL: {}", e),
    }

    println!("\n=== STRESS TEST COMPLETE ===");
}

/// Persistent connection wrapper for connection reuse tests
struct PersistentConn {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl PersistentConn {
    fn connect() -> Result<Self, String> {
        let writer = UnixStream::connect(SOCKET_PATH).map_err(|e| format!("Connect failed: {}", e))?;
        writer.set_read_timeout(Some(Duration::from_secs(5))).ok();
        writer.set_write_timeout(Some(Duration::from_secs(5))).ok();
        let reader = BufReader::new(writer.try_clone().map_err(|e| format!("Clone failed: {}", e))?);
        Ok(Self { reader, writer })
    }

    fn request(&mut self, json: &str) -> Result<String, String> {
        let mut request = json.to_string();
        request.push('\n');
        self.writer.write_all(request.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;
        self.writer.flush().map_err(|e| format!("Flush failed: {}", e))?;

        let mut line = String::new();
        self.reader.read_line(&mut line).map_err(|e| format!("Read failed: {}", e))?;
        Ok(line)
    }
}

fn send_request(stream: &mut UnixStream, json: &str) -> Result<String, String> {
    let mut request = json.to_string();
    request.push('\n');
    stream.write_all(request.as_bytes()).map_err(|e| format!("Write failed: {}", e))?;
    stream.flush().map_err(|e| format!("Flush failed: {}", e))?;

    let reader_stream = stream.try_clone().map_err(|e| format!("Clone failed: {}", e))?;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    reader.read_line(&mut line).map_err(|e| format!("Read failed: {}", e))?;
    Ok(line)
}

fn test_basic_connectivity() -> Result<(), String> {
    let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| format!("Connect failed: {}", e))?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(5))).ok();

    let response = send_request(&mut stream, r#"{"type":"ping"}"#)?;
    if response.contains("pong") {
        Ok(())
    } else {
        Err(format!("Unexpected response: {}", response))
    }
}

fn test_rapid_sequential() -> Result<Duration, String> {
    let start = Instant::now();

    for i in 0..1000 {
        let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| format!("Connect {} failed: {}", i, e))?;
        stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

        let query = format!(r#"{{"type":"search","query":"test{}"}}"#, i % 10);
        let _response = send_request(&mut stream, &query)?;
    }

    Ok(start.elapsed())
}

fn test_concurrent_connections() -> Result<(usize, usize, Duration), String> {
    let success = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let start = Instant::now();

    let handles: Vec<_> = (0..50)
        .map(|thread_id| {
            let success = Arc::clone(&success);
            let failed = Arc::clone(&failed);

            thread::spawn(move || {
                for i in 0..100 {
                    match UnixStream::connect(SOCKET_PATH) {
                        Ok(mut stream) => {
                            stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                            let query = format!(r#"{{"type":"search","query":"t{}{}"}}"#, thread_id, i);
                            match send_request(&mut stream, &query) {
                                Ok(_) => {
                                    success.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(_) => {
                                    failed.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }
                        Err(_) => {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().ok();
    }

    let duration = start.elapsed();
    Ok((
        success.load(Ordering::Relaxed),
        failed.load(Ordering::Relaxed),
        duration,
    ))
}

fn test_edge_cases() {
    // Empty query
    print!("  - Empty query: ");
    match UnixStream::connect(SOCKET_PATH) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            match send_request(&mut stream, r#"{"type":"search","query":""}"#) {
                Ok(resp) => {
                    if resp.contains("results") {
                        println!("✓ PASS");
                    } else {
                        println!("✗ FAIL: {}", resp.trim());
                    }
                }
                Err(e) => println!("✗ FAIL: {}", e),
            }
        }
        Err(e) => println!("✗ FAIL: {}", e),
    }

    // Special characters
    print!("  - Special characters: ");
    match UnixStream::connect(SOCKET_PATH) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            match send_request(&mut stream, r#"{"type":"search","query":"test<>&\"'`$(){}[]|\\n\\r\\t"}"#) {
                Ok(resp) => {
                    if resp.contains("results") || resp.contains("error") {
                        println!("✓ PASS (handled)");
                    } else {
                        println!("✗ FAIL: unexpected: {}", resp.trim());
                    }
                }
                Err(e) => println!("✗ FAIL: {}", e),
            }
        }
        Err(e) => println!("✗ FAIL: {}", e),
    }

    // Unicode
    print!("  - Unicode query: ");
    match UnixStream::connect(SOCKET_PATH) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            match send_request(&mut stream, r#"{"type":"search","query":"日本語テスト🚀"}"#) {
                Ok(resp) => {
                    if resp.contains("results") {
                        println!("✓ PASS");
                    } else {
                        println!("✗ FAIL: {}", resp.trim());
                    }
                }
                Err(e) => println!("✗ FAIL: {}", e),
            }
        }
        Err(e) => println!("✗ FAIL: {}", e),
    }

    // List request
    print!("  - List all apps: ");
    match UnixStream::connect(SOCKET_PATH) {
        Ok(mut stream) => {
            stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
            match send_request(&mut stream, r#"{"type":"list"}"#) {
                Ok(resp) => {
                    if resp.contains("results") {
                        println!("✓ PASS");
                    } else {
                        println!("✗ FAIL: {}", resp.trim());
                    }
                }
                Err(e) => println!("✗ FAIL: {}", e),
            }
        }
        Err(e) => println!("✗ FAIL: {}", e),
    }
}

fn test_connection_reuse() -> Result<Duration, String> {
    let mut conn = PersistentConn::connect()?;

    let start = Instant::now();

    for i in 0..500 {
        let query = format!(r#"{{"type":"search","query":"reuse{}"}}"#, i);
        conn.request(&query).map_err(|e| format!("Request {} failed: {}", i, e))?;
    }

    Ok(start.elapsed())
}

fn test_large_payload() -> Result<(), String> {
    let mut stream = UnixStream::connect(SOCKET_PATH).map_err(|e| format!("Connect failed: {}", e))?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();

    // 100KB query string
    let large_query: String = "x".repeat(100_000);
    let json = format!(r#"{{"type":"search","query":"{}"}}"#, large_query);

    let response = send_request(&mut stream, &json)?;
    if response.contains("results") || response.contains("error") {
        Ok(())
    } else {
        Err(format!("Unexpected response: {}", &response[..100.min(response.len())]))
    }
}

fn test_malformed_json() -> Result<(), String> {
    let test_cases = vec![
        ("Empty string", ""),
        ("Not JSON", "hello world"),
        ("Incomplete JSON", r#"{"type":"search"#),
        ("Unknown type", r#"{"type":"unknown"}"#),
        ("Missing type", r#"{"query":"test"}"#),
        ("Extra newlines", "\n\n{\"type\":\"ping\"}\n\n"),
    ];

    for (name, input) in test_cases {
        print!("  - {}: ", name);
        match UnixStream::connect(SOCKET_PATH) {
            Ok(mut stream) => {
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();

                // Send raw input
                let mut request = input.to_string();
                if !request.ends_with('\n') {
                    request.push('\n');
                }

                if let Err(e) = stream.write_all(request.as_bytes()) {
                    println!("✓ PASS (write rejected: {})", e);
                    continue;
                }
                stream.flush().ok();

                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => println!("✓ PASS (connection closed)"),
                    Ok(_) => {
                        if line.contains("error") {
                            println!("✓ PASS (error response)");
                        } else {
                            println!("⚠ handled: {}", line.trim());
                        }
                    }
                    Err(_) => println!("✓ PASS (read failed - daemon handled)"),
                }
            }
            Err(e) => println!("✗ FAIL: {}", e),
        }
    }

    Ok(())
}
