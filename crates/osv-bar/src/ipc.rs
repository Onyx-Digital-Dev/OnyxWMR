//! IPC client for communication with osvwm compositor.
//!
//! Connects to osvwm's Unix socket to send commands.

use anyhow::{Context, Result};
use osv_ipc::socket::Socket;
use osv_ipc::{Action, Request, Response, WorkspaceReferenceArg};
use tracing::{info, warn};

/// Number of workspaces in OSV
pub const WORKSPACE_COUNT: usize = 8;

/// IPC client handle
pub struct IpcClient {
    socket: Socket,
}

impl IpcClient {
    /// Connect to osvwm
    pub fn connect() -> Result<Self> {
        info!("Connecting to osvwm socket...");
        let socket = Socket::connect().context("Failed to connect to osvwm socket")?;
        Ok(Self { socket })
    }

    /// Switch to a specific workspace
    pub fn switch_workspace(&mut self, idx: usize) -> Result<()> {
        if idx >= WORKSPACE_COUNT {
            return Err(anyhow::anyhow!("Invalid workspace index: {}", idx));
        }

        let action = Action::FocusWorkspace {
            reference: WorkspaceReferenceArg::Index(idx as u8 + 1),
        };

        let reply = self
            .socket
            .send(Request::Action(action))
            .context("Failed to send FocusWorkspace action")?;

        match reply {
            Ok(Response::Handled) => {
                info!("Switched to workspace {}", idx + 1);
                Ok(())
            }
            Ok(other) => {
                warn!("Unexpected response: {:?}", other);
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!("FocusWorkspace failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_count() {
        assert_eq!(WORKSPACE_COUNT, 8);
    }
}
