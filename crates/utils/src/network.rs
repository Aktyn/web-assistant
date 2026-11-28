use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

pub async fn check_network_connection(timeout_sec: u16) -> anyhow::Result<bool> {
    let duration = Duration::from_secs(u64::from(timeout_sec));

    // Try to connect to Google's DNS (8.8.8.8) on port 53
    let addr: SocketAddr = "8.8.8.8:53"
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse hardcoded DNS address: {e}"))?;

    match timeout(duration, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => Ok(true),
        Ok(Err(e)) => {
            // Connection failed (e.g., connection refused, network unreachable)
            Err(anyhow::anyhow!("Network connection check failed: {e}"))
        }
        Err(_) => {
            // Timeout elapsed
            Err(anyhow::anyhow!(
                "Network connection check timed out after {timeout_sec} seconds"
            ))
        }
    }
}
