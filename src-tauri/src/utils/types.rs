use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tauri_plugin_http::reqwest;
/// A status report for port numbers.
/// if this type exists, it means a udp port is open.
#[derive(Debug, Clone, Serialize)]
pub struct PortStatus {
    port: u32,
    running: bool,
}

impl PortStatus {
    pub fn new(port: u32, running: bool) -> Self {
        Self { port, running }
    }
}
/// A struct that identifies a running flagged process
#[derive(Debug, Clone, Serialize)]

pub struct ProcessIdentifier {
    pub process_id: i32,
    pub status: bool,
    pub parent: Option<i32>,
    pub start_time: u64,
    pub run_time: u64,
    pub cpu_usage: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebRtcReport {
    pub ports: Vec<PortStatus>,
    pub processes: Vec<ProcessIdentifier>,
}

impl WebRtcReport {
    pub fn is_running(&self) -> bool {
        self.ports.len() > 0 && self.processes.len() > 0
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum Triggers {
    DisAllowedInputDectected(Vec<USBDevice>),
    UDPDectected,
    RemoteApplicationDectected(WebRtcReport),
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct HostInfo {
    pub os: String,
    pub arch: String,
    pub mac_address: Option<String>,
    pub serial_number: Option<String>,
    pub processor_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct USBDevice {
    /// Platform specific unique ID
    pub id: String,
    /// Vendor ID
    pub vendor_id: u16,
    /// Product ID
    pub product_id: u16,
    /// Optional device description
    pub description: Option<String>,
    /// Optional serial number
    pub serial_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawUdpEndpoint {
    #[serde(rename = "LocalAddress")]
    pub local_address: String,
    #[serde(rename = "LocalPort")]
    pub local_port: u16,
    #[serde(rename = "ProcessName")]
    pub process_name: Option<String>,
    #[serde(rename = "CreationTime")]
    pub creation_time: String,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

#[derive(Debug)]
pub struct UdpEndpoint {
    pub local_address: String,
    pub local_port: u16,
    pub process_name: Option<String>,
    pub creation_time: String,
    pub status: Option<String>,
}

/// A valid response from the validator server
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerValidatorResponse {
    pub status: bool,
    pub message: String,
    pub ip_addr: String,
    pub port: u16,
}

///////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////        ERRORS         /////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Display, thiserror::Error)]
pub enum ModuleError {
    #[display("Internal server error: {}", _0)]
    Internal(String),

    RequsetError(#[from] reqwest::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl serde::Serialize for ModuleError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

impl From<String> for ModuleError {
    fn from(value: String) -> Self {
        Self::Internal(value)
    }
}
