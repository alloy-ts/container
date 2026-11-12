//! Connection tuning for [`super::GuestAgent`].

use std::time::Duration;

/// Control-plane configuration.
///
/// Defaults match the host runtime: the guest init listens on the agent port
/// and streams kernel/agent logs on the log port, both reachable over whatever
/// transport [`super::StreamConnector`] opens for that port.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Port the guest RPC service listens on.
    pub server_port: u32,
    /// Port the guest streams kernel/agent logs on.
    pub log_port: u32,
    /// How long [`super::GuestAgent::connect`] keeps retrying the dial.
    /// The guest init is not up the instant the VM boots, so this is a
    /// retry budget, not a single-attempt timeout.
    pub dial_timeout: Duration,
    /// Per-request budget enforced by the `tower` timeout layer. Long-lived
    /// blocking RPCs (e.g. `wait_process`) bypass it.
    pub request_timeout: Duration,
    /// Upper bound on concurrent in-flight RPCs.
    pub max_in_flight: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_port: 1024,
            log_port: 1025,
            dial_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_in_flight: 64,
        }
    }
}

impl AgentConfig {
    /// Config for a guest whose RPC service listens on `server_port`.
    pub fn new(server_port: u32) -> Self {
        Self {
            server_port,
            ..Self::default()
        }
    }

    pub fn with_log_port(mut self, log_port: u32) -> Self {
        self.log_port = log_port;
        self
    }

    pub fn with_dial_timeout(mut self, dial_timeout: Duration) -> Self {
        self.dial_timeout = dial_timeout;
        self
    }

    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }

    pub fn with_max_in_flight(mut self, max_in_flight: usize) -> Self {
        self.max_in_flight = max_in_flight.max(1);
        self
    }
}
