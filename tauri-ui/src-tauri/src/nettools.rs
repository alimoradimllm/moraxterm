//! Ping/traceroute-style network tools and the port scanner.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use crate::system::executable_available;

pub(crate) fn format_socket_address(target: &str, port: u16) -> String {
    let target = target.trim().trim_matches(['[', ']']);
    if target.contains(':') {
        format!("[{}]:{}", target, port)
    } else {
        format!("{}:{}", target, port)
    }
}

#[tauri::command]
pub(crate) fn run_network_tool(tool: String, target: String, port: Option<u16>) -> Result<String, String> {
    if target.trim().is_empty() {
        return Err("network target cannot be empty".into());
    }
    if target.len() > 255 || target.chars().any(|character| character.is_control()) {
        return Err("network target is invalid or too long".into());
    }
    let tool = tool.to_ascii_lowercase();
    if tool == "port" {
        let port = port.ok_or("port is required for a TCP check")?;
        if port == 0 {
            return Err("port must be non-zero".into());
        }
        let address = format_socket_address(&target, port);
        let resolved = address.to_socket_addrs().map_err(|e| e.to_string())?;
        for socket in resolved {
            if TcpStream::connect_timeout(&socket, Duration::from_secs(3)).is_ok() {
                return Ok(format!("TCP {} is open", address));
            }
        }
        return Ok(format!("TCP {} is closed or unreachable", address));
    }
    if tool == "dns" {
        let lookup = if target.contains(':') {
            format!("[{}]:0", target.trim_matches(['[', ']']))
        } else {
            format!("{}:0", target)
        };
        let mut addresses = Vec::new();
        for address in lookup
            .to_socket_addrs()
            .map_err(|error| error.to_string())?
        {
            let value = address.ip().to_string();
            if !addresses.contains(&value) {
                addresses.push(value);
            }
        }
        if addresses.is_empty() {
            return Err("DNS lookup returned no addresses".into());
        }
        return Ok(addresses.join("\n"));
    }
    let (program, args): (&str, Vec<String>) = match tool.as_str() {
        "ping" if cfg!(windows) => (
            "ping",
            vec!["-n".into(), "4".into(), "-w".into(), "1000".into(), target],
        ),
        "ping" => (
            "ping",
            vec!["-c".into(), "4".into(), "-W".into(), "1".into(), target],
        ),
        "traceroute" if cfg!(windows) => {
            ("tracert", vec!["/w:1000".into(), "/h:12".into(), target])
        }
        "traceroute" => (
            "traceroute",
            vec!["-m".into(), "12".into(), "-w".into(), "1".into(), target],
        ),
        _ => return Err("tool must be ping, traceroute, dns, or port".into()),
    };
    if !executable_available(program) {
        return Err(format!("network tool is unavailable: {program}"));
    }
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run network tool: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Ok(text)
}

#[tauri::command]
pub(crate) fn scan_ports(target: String, start_port: u16, end_port: u16) -> Result<String, String> {
    if target.trim().is_empty()
        || target.len() > 255
        || target.chars().any(|character| character.is_control())
    {
        return Err("network target is invalid or too long".into());
    }
    if start_port == 0 || end_port == 0 || end_port < start_port {
        return Err("port range is invalid".into());
    }
    if u32::from(end_port) - u32::from(start_port) + 1 > 32 {
        return Err("port range cannot exceed 32 ports".into());
    }
    let mut lines = Vec::new();
    for port in start_port..=end_port {
        let address = format_socket_address(&target, port);
        let started = Instant::now();
        let open = address
            .to_socket_addrs()
            .map_err(|error| error.to_string())?
            .any(|socket| TcpStream::connect_timeout(&socket, Duration::from_millis(250)).is_ok());
        let elapsed_ms = started.elapsed().as_millis();
        lines.push(format!(
            "{} {} {}ms",
            port,
            if open { "open" } else { "closed" },
            elapsed_ms
        ));
        std::thread::sleep(Duration::from_millis(25));
    }
    Ok(lines.join("\n"))
}
