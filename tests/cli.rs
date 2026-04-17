use assert_cmd::Command;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn start_mock_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf); // drain the request
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            }
        }
    });
    port
}

#[test]
fn exits_nonzero_with_no_mirrors() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .assert()
        .failure();
}

#[test]
fn exits_nonzero_when_all_mirrors_fail() {
    Command::cargo_bin("select-mirror")
        .unwrap()
        .args(["http://127.0.0.1:1", "http://127.0.0.1:2"])
        .args(["--timeout", "1"])
        .assert()
        .failure();
}

#[test]
fn outputs_fastest_mirror_url() {
    let port = start_mock_server();
    let mirror = format!("http://127.0.0.1:{}", port);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror])
        .args(["--probe-path", "/"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        mirror.as_str()
    );
}

#[test]
fn fastest_of_two_mirrors_wins() {
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        winner == mirror_a || winner == mirror_b,
        "unexpected winner: {winner}"
    );
}

#[test]
fn early_exit_when_fast_count_met() {
    let ports: Vec<u16> = (0..5).map(|_| start_mock_server()).collect();
    let mirrors: Vec<String> = ports
        .iter()
        .map(|p| format!("http://127.0.0.1:{}", p))
        .collect();

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args(&mirrors)
        .args(["--probe-path", "/"])
        .args(["--fast-count", "2"])
        .args(["--fast-threshold", "500"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        mirrors.iter().any(|m| m == winner),
        "winner {winner} not in mirror list"
    );
}

#[test]
fn fast_count_exceeds_mirror_count_still_succeeds() {
    let port_a = start_mock_server();
    let port_b = start_mock_server();
    let mirror_a = format!("http://127.0.0.1:{}", port_a);
    let mirror_b = format!("http://127.0.0.1:{}", port_b);

    let output = Command::cargo_bin("select-mirror")
        .unwrap()
        .args([&mirror_a, &mirror_b])
        .args(["--probe-path", "/"])
        .args(["--fast-count", "10"])
        .args(["--fast-threshold", "500"])
        .output()
        .unwrap();

    assert!(output.status.success(), "expected exit 0");
    let winner = String::from_utf8(output.stdout).unwrap();
    let winner = winner.trim();
    assert!(
        winner == mirror_a || winner == mirror_b,
        "unexpected winner: {winner}"
    );
}
