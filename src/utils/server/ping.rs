use std::{net::IpAddr, time::Duration};

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use trust_dns_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};

use crate::utils::server::{
    string::{read_string, write_string},
    u16::write_u16,
    varint::{read_var_int, read_var_int_from_stream, write_var_int},
};

static RESOLVER: Lazy<TokioAsyncResolver> =
    Lazy::new(|| TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()));

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

const HANDSHAKE_PACKET_ID: i32 = 0x0;
const STATUS_REQUEST_PACKET_ID: i32 = 0x0;
const NEXT_STATE_STATUS: i32 = 1;

#[derive(Debug, Error)]
pub enum PingError {
    #[error("DNS SRV resolution failed: {0}")]
    SrvResolutionFailed(String),

    #[error("Target host could not be resolved: {0}")]
    HostResolutionFailed(std::io::Error),

    #[error("TCP connection failed: {0}")]
    TcpConnectFailed(#[from] std::io::Error),

    #[error("TCP connection timed out")]
    TcpConnectTimeout,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerStatus {
    pub version: Version,
    pub players: Players,
    #[serde(rename = "description")]
    pub raw_description: Value,
    #[serde(skip)]
    pub description: String,
    pub favicon: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub name: String,
    pub protocol: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Players {
    pub max: u32,
    pub online: u32,
    pub sample: Option<Vec<PlayerSample>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerSample {
    pub name: String,
    pub id: String,
}

pub async fn ping(
    hostname: &str,
    default_port: u16,
    protocol_version: i32,
) -> Result<ServerStatus, PingError> {
    let (target_host, port) = match hostname.parse::<IpAddr>() {
        Ok(ip) => (ip.to_string(), default_port),
        Err(_) => {
            let srv_name = format!("_minecraft._tcp.{}", hostname);
            match RESOLVER.srv_lookup(srv_name).await {
                Ok(lookup) => {
                    let srv = lookup.iter().next().ok_or_else(|| {
                        PingError::SrvResolutionFailed("No SRV records found".into())
                    })?;
                    (srv.target().to_utf8(), srv.port())
                }
                Err(_) => (hostname.to_string(), default_port),
            }
        }
    };

    let mut addrs = tokio::net::lookup_host((target_host.as_str(), port))
        .await
        .map_err(PingError::HostResolutionFailed)?;

    let real_ip = addrs
        .next()
        .ok_or_else(|| {
            PingError::HostResolutionFailed(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No addresses found",
            ))
        })?
        .ip();

    let stream = timeout(CONNECT_TIMEOUT, TcpStream::connect((real_ip, port))).await;
    let mut stream = match stream {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(PingError::TcpConnectFailed(e)),
        Err(_) => return Err(PingError::TcpConnectTimeout),
    };

    let handshake_packet =
        create_handshake_packet(protocol_version, hostname, port, NEXT_STATE_STATUS);
    stream.write_all(&handshake_packet).await?;

    let status_request = create_status_request();
    stream.write_all(&status_request).await?;

    let len = read_var_int_from_stream(&mut stream).await?;
    let mut response_bytes = vec![0; len as usize];
    stream.read_exact(&mut response_bytes).await?;

    let mut index = 0;
    let packet_id = read_var_int(&response_bytes, Some(&mut index));
    let response = read_string(&response_bytes, &mut index)
        .map_err(|e| PingError::Protocol(format!("Failed to read response: {}", e)))?;

    if packet_id != 0 {
        return Err(PingError::Protocol(format!(
            "Unexpected response code: {}",
            packet_id
        )));
    }

    let mut status: ServerStatus = serde_json::from_str(&response)?;
    status.description = extract_text(&status.raw_description);
    Ok(status)
}

fn extract_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr.iter().map(extract_text).collect(),
        Value::Object(map) => {
            let mut result = String::new();

            if let Some(text_val) = map.get("text") {
                result.push_str(&extract_text(text_val));
            }

            if let Some(extra_val) = map.get("extra") {
                result.push_str(&extract_text(extra_val));
            }

            result
        }
        _ => String::new(),
    }
}

fn create_packet(packet_id: i32, payload_writer: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut payload = Vec::new();
    write_var_int(&mut payload, &packet_id);
    payload_writer(&mut payload);

    let mut packet = Vec::new();
    write_var_int(&mut packet, &(payload.len() as i32));
    packet.extend_from_slice(&payload);
    packet
}

fn create_handshake_packet(
    protocol_version: i32,
    server_address: &str,
    server_port: u16,
    next_state: i32,
) -> Vec<u8> {
    create_packet(HANDSHAKE_PACKET_ID, |buf| {
        write_var_int(buf, &protocol_version);
        write_string(buf, server_address);
        write_u16(buf, server_port);
        write_var_int(buf, &next_state);
    })
}

fn create_status_request() -> Vec<u8> {
    create_packet(STATUS_REQUEST_PACKET_ID, |_| {})
}
