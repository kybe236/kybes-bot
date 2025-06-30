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
const HANDSHAKE_ID: i32 = 0x0;
const STATUS_REQUEST_ID: i32 = 0x0;
const NEXT_STATE_STATUS: i32 = 1;

/// Represents an error during the ping process.
#[derive(Debug, Error)]
pub enum PingError {
    #[error("DNS SRV resolution failed: {0}")]
    SrvResolutionFailed(String),

    #[error("Host resolution failed: {0}")]
    HostResolutionFailed(#[from] std::io::Error),

    #[error("Connection timed out")]
    ConnectTimeout,

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Mineserver status information.
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

/// Pings a Minecraft server to retrieve its status.
pub async fn ping(
    hostname: &str,
    default_port: u16,
    protocol_version: i32,
) -> Result<ServerStatus, PingError> {
    let (host, port) = resolve_host(hostname, default_port).await?;
    let mut stream = connect(host.as_str(), port).await?;

    // Send handshake and status request
    stream
        .write_all(&handshake_packet(protocol_version, hostname, port))
        .await?;
    stream.write_all(&status_request_packet()).await?;

    // Read response
    let response = read_response(&mut stream).await?;
    validate_packet_id(response.packet_id)?;

    let mut status: ServerStatus = serde_json::from_str(&response.json)?;
    status.description = extract_text(&status.raw_description);
    Ok(status)
}

async fn resolve_host(hostname: &str, default_port: u16) -> Result<(String, u16), PingError> {
    // Try numeric IP first
    if hostname.parse::<IpAddr>().is_ok() {
        return Ok((hostname.to_string(), default_port));
    }

    // Fallback to SRV lookup
    let srv_name = format!("_minecraft._tcp.{}", hostname);
    if let Ok(lookup) = RESOLVER.srv_lookup(srv_name).await {
        if let Some(record) = lookup.iter().next() {
            return Ok((record.target().to_utf8(), record.port()));
        }
        return Err(PingError::SrvResolutionFailed(
            "no SRV records found".into(),
        ));
    }

    Ok((hostname.to_string(), default_port))
}

async fn connect(host: &str, port: u16) -> Result<TcpStream, PingError> {
    let mut lookup = tokio::net::lookup_host((host, port))
        .await
        .map_err(PingError::HostResolutionFailed)?;
    let addr = lookup
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "no addresses found"))?
        .ip();

    match timeout(CONNECT_TIMEOUT, TcpStream::connect((addr, port))).await {
        Ok(Ok(stream)) => Ok(stream),
        Ok(Err(e)) => Err(PingError::HostResolutionFailed(e)),
        Err(_) => Err(PingError::ConnectTimeout),
    }
}

struct Response {
    packet_id: i32,
    json: String,
}

async fn read_response(stream: &mut TcpStream) -> Result<Response, PingError> {
    let length = read_var_int_from_stream(stream).await?;
    let mut buf = vec![0; length as usize];
    stream.read_exact(&mut buf).await?;

    let mut idx = 0;
    let packet_id = read_var_int(&buf, Some(&mut idx));
    let json = read_string(&buf, &mut idx)
        .map_err(|e| PingError::Protocol(format!("read response: {}", e)))?;

    Ok(Response { packet_id, json })
}

fn validate_packet_id(id: i32) -> Result<(), PingError> {
    if id != STATUS_REQUEST_ID {
        return Err(PingError::Protocol(format!("unexpected packet id: {}", id)));
    }
    Ok(())
}

fn extract_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr.iter().map(extract_text).collect(),
        Value::Object(map) => {
            let mut text = String::new();
            if let Some(v) = map.get("text") {
                text.push_str(&extract_text(v));
            }
            if let Some(v) = map.get("extra") {
                text.push_str(&extract_text(v));
            }
            text
        }
        _ => String::new(),
    }
}

fn packet<F>(id: i32, write: F) -> Vec<u8>
where
    F: FnOnce(&mut Vec<u8>),
{
    let mut payload = Vec::new();
    write_var_int(&mut payload, &id);
    write(&mut payload);

    let mut pkt = Vec::new();
    write_var_int(&mut pkt, &(payload.len() as i32));
    pkt.extend(payload);
    pkt
}

fn handshake_packet(version: i32, address: &str, port: u16) -> Vec<u8> {
    packet(HANDSHAKE_ID, |buf| {
        write_var_int(buf, &version);
        write_string(buf, address);
        write_u16(buf, port);
        write_var_int(buf, &NEXT_STATE_STATUS);
    })
}

fn status_request_packet() -> Vec<u8> {
    packet(STATUS_REQUEST_ID, |_| {})
}
