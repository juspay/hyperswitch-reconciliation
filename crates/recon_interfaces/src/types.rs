/// Proxy configuration structure
#[derive(Debug, serde::Deserialize, Clone)]
#[serde(default)]
pub struct Proxy {
    /// The URL of the HTTP proxy server.
    pub http_url: Option<String>,

    /// The URL of the HTTPS proxy server.
    pub https_url: Option<String>,

    /// The timeout duration (in seconds) for idle connections in the proxy pool.
    pub idle_pool_connection_timeout: Option<u64>,

    /// A comma-separated list of hosts that should bypass the proxy.
    pub bypass_proxy_hosts: Option<String>,
}

impl Default for Proxy {
    fn default() -> Self {
        Self {
            http_url: Default::default(),
            https_url: Default::default(),
            idle_pool_connection_timeout: Some(90),
            bypass_proxy_hosts: Default::default(),
        }
    }
}
