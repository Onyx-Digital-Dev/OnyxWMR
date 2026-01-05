//! IPC client for communication with osvwm compositor.
//!
//! Connects to osvwm's Unix socket and subscribes to the event stream
//! to receive real-time workspace and window updates.

use anyhow::{Context, Result};
use osv_ipc::socket::Socket;
use osv_ipc::{Action, Event, Request, Response, WorkspaceReferenceArg};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tracing::{debug, error, info, warn};

/// Number of workspaces in OSV
pub const WORKSPACE_COUNT: usize = 8;

/// Messages sent from IPC thread to main thread
#[derive(Debug, Clone)]
pub enum IpcMessage {
    /// Full workspace state update
    WorkspacesChanged(Vec<WorkspaceInfo>),
    /// Windows changed - update window counts
    WindowsChanged(Vec<WindowInfo>),
    /// Single workspace activated
    WorkspaceActivated { idx: usize },
    /// Connection lost
    Disconnected,
}

/// Simplified workspace info for the bar
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub idx: usize,
    pub id: u64,
    pub is_focused: bool,
    pub is_urgent: bool,
}

/// Simplified window info for the bar
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub workspace_id: Option<u64>,
}

/// IPC client handle for sending commands
pub struct IpcCommandClient {
    socket: Socket,
}

impl IpcCommandClient {
    /// Connect to osvwm for sending commands
    pub fn connect() -> Result<Self> {
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

/// IPC event stream client (daemon mode)
pub struct IpcEventClient {
    receiver: Receiver<IpcMessage>,
}

impl IpcEventClient {
    /// Connect to osvwm and start receiving events
    pub fn connect() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        // Spawn IPC thread
        thread::spawn(move || {
            if let Err(e) = run_event_loop(sender.clone()) {
                error!("IPC event loop error: {}", e);
                let _ = sender.send(IpcMessage::Disconnected);
            }
        });

        Ok(Self { receiver })
    }

    /// Try to receive a message (non-blocking)
    pub fn try_recv(&self) -> Option<IpcMessage> {
        self.receiver.try_recv().ok()
    }
}

/// Main IPC event loop (runs in background thread)
fn run_event_loop(sender: Sender<IpcMessage>) -> Result<()> {
    info!("Connecting to osvwm event stream...");

    let mut socket = Socket::connect().context("Failed to connect to osvwm socket")?;

    // Request event stream
    let reply = socket
        .send(Request::EventStream)
        .context("Failed to send EventStream request")?;

    match reply {
        Ok(Response::Handled) => {
            info!("Subscribed to osvwm event stream");
        }
        Ok(other) => {
            warn!("Unexpected response to EventStream: {:?}", other);
        }
        Err(e) => {
            return Err(anyhow::anyhow!("EventStream request failed: {}", e));
        }
    }

    // Read events
    let mut read_event = socket.read_events();

    // Track workspace ID to index mapping
    let mut workspace_id_to_idx: HashMap<u64, usize> = HashMap::new();

    loop {
        let event = read_event().context("Failed to read event")?;
        debug!("Received event: {:?}", event);

        match event {
            Event::WorkspacesChanged { workspaces } => {
                // Update workspace ID to index mapping
                workspace_id_to_idx.clear();
                let mut infos = Vec::new();

                for ws in &workspaces {
                    let idx = ws.idx as usize;
                    if idx < WORKSPACE_COUNT {
                        workspace_id_to_idx.insert(ws.id, idx);
                        infos.push(WorkspaceInfo {
                            idx,
                            id: ws.id,
                            is_focused: ws.is_focused,
                            is_urgent: ws.is_urgent,
                        });
                    }
                }

                sender.send(IpcMessage::WorkspacesChanged(infos))?;
            }

            Event::WorkspaceActivated { id, focused: _ } => {
                if let Some(&idx) = workspace_id_to_idx.get(&id) {
                    sender.send(IpcMessage::WorkspaceActivated { idx })?;
                }
            }

            Event::WindowsChanged { windows } => {
                let infos: Vec<WindowInfo> = windows
                    .iter()
                    .map(|w| WindowInfo {
                        workspace_id: w.workspace_id,
                    })
                    .collect();
                sender.send(IpcMessage::WindowsChanged(infos))?;
            }

            Event::WindowOpenedOrChanged { .. }
            | Event::WindowClosed { .. } => {
                // Handled via WindowsChanged events
            }

            _ => {}
        }
    }
}

