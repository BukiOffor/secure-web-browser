use serde::Serialize;

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
