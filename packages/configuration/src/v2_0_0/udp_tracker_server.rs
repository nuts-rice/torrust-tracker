use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct UdpTrackerServer {
    #[serde(default = "UdpTrackerServer::default_ip_bans_reset_interval_in_seconds")] 
    pub ip_bans_reset_interval_in_seconds: Duration,                          
    
}

impl Default for UdpTrackerServer {
    fn default() -> Self {
        Self {
            ip_bans_reset_interval_in_seconds: Self::default_ip_bans_reset_interval_in_seconds(),
        }
    }

}

impl UdpTrackerServer {
    fn default_ip_bans_reset_interval_in_seconds() -> Duration {
        Duration::from_secs( 3600 * 24)
    }

}
