//! IPC client for communication with osvwm compositor.
//!
//! Connects to osvwm's Unix socket and subscribes to the event stream
//! to receive workspace and window updates.

use anyhow::{Context, Result};
use osv_ipc::socket::Socket;
use osv_ipc::{Event, Request, Response, Window, Workspace};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tracing::{debug, error, info, warn};

use crate::bar::{BarState, WORKSPACE_COUNT};

/// Messages sent from IPC thread to main thread
#[derive(Debug, Clone)]
pub enum IpcMessage {
    /// Full workspace state update
    WorkspacesChanged(Vec<WorkspaceInfo>),
    /// Single workspace activated/focused
    WorkspaceActivated { idx: usize, focused: bool },
    /// Workspace urgency changed
    WorkspaceUrgencyChanged { idx: usize, urgent: bool },
    /// Window count changed on a workspace
    WindowCountChanged { idx: usize, count: u32 },
    /// Connection lost
    Disconnected,
}

/// Simplified workspace info for the bar
#[derive(Debug, Clone)]
pub struct WorkspaceInfo {
    pub idx: usize,
    pub is_active: bool,
    pub is_focused: bool,
    pub is_urgent: bool,
    pub window_count: u32,
}

/// IPC client handle
pub struct IpcClient {
    receiver: Receiver<IpcMessage>,
}

impl IpcClient {
    /// Connect to osvwm and start receiving events
    pub fn connect() -> Result<Self> {
        let (sender, receiver) = mpsc::channel();

        // Spawn IPC thread
        thread::spawn(move || {
            if let Err(e) = run_ipc_loop(sender.clone()) {
                error!("IPC loop error: {}", e);
                let _ = sender.send(IpcMessage::Disconnected);
            }
        });

        Ok(Self { receiver })
    }

    /// Try to receive a message (non-blocking)
    pub fn try_recv(&self) -> Option<IpcMessage> {
        self.receiver.try_recv().ok()
    }

    /// Receive a message (blocking)
    pub fn recv(&self) -> Option<IpcMessage> {
        self.receiver.recv().ok()
    }

    /// Apply received messages to bar state
    pub fn update_bar_state(&self, state: &mut BarState) -> bool {
        let mut updated = false;

        while let Some(msg) = self.try_recv() {
            match msg {
                IpcMessage::WorkspacesChanged(workspaces) => {
                    for ws in workspaces {
                        if ws.idx < WORKSPACE_COUNT {
                            state.workspaces[ws.idx].active = ws.is_focused;
                            state.workspaces[ws.idx].window_count = ws.window_count;
                            state.workspaces[ws.idx].urgent = ws.is_urgent;
                            if ws.is_focused {
                                state.active_workspace = ws.idx;
                            }
                        }
                    }
                    updated = true;
                }
                IpcMessage::WorkspaceActivated { idx, focused } => {
                    if idx < WORKSPACE_COUNT {
                        if focused {
                            // Clear old active
                            for ws in &mut state.workspaces {
                                ws.active = false;
                            }
                            state.workspaces[idx].active = true;
                            state.active_workspace = idx;
                        }
                        updated = true;
                    }
                }
                IpcMessage::WorkspaceUrgencyChanged { idx, urgent } => {
                    if idx < WORKSPACE_COUNT {
                        state.workspaces[idx].urgent = urgent;
                        updated = true;
                    }
                }
                IpcMessage::WindowCountChanged { idx, count } => {
                    if idx < WORKSPACE_COUNT {
                        state.workspaces[idx].window_count = count;
                        updated = true;
                    }
                }
                IpcMessage::Disconnected => {
                    warn!("IPC disconnected");
                }
            }
        }

        updated
    }
}

/// Main IPC event loop (runs in background thread)
fn run_ipc_loop(sender: Sender<IpcMessage>) -> Result<()> {
    info!("Connecting to osvwm socket...");

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

    // Track windows per workspace
    let mut window_counts: HashMap<u64, u32> = HashMap::new();
    let mut workspace_id_to_idx: HashMap<u64, usize> = HashMap::new();

    loop {
        let event = read_event().context("Failed to read event")?;
        debug!("Received event: {:?}", event);

        match event {
            Event::WorkspacesChanged { workspaces } => {
                let infos = process_workspaces(&workspaces, &window_counts);

                // Update workspace ID to index mapping
                workspace_id_to_idx.clear();
                for ws in &workspaces {
                    // OSV uses fixed workspaces 0-7, map by idx
                    if (ws.idx as usize) < WORKSPACE_COUNT {
                        workspace_id_to_idx.insert(ws.id, ws.idx as usize);
                    }
                }

                sender.send(IpcMessage::WorkspacesChanged(infos))?;
            }

            Event::WorkspaceActivated { id, focused } => {
                if let Some(&idx) = workspace_id_to_idx.get(&id) {
                    sender.send(IpcMessage::WorkspaceActivated { idx, focused })?;
                }
            }

            Event::WorkspaceUrgencyChanged { id, urgent } => {
                if let Some(&idx) = workspace_id_to_idx.get(&id) {
                    sender.send(IpcMessage::WorkspaceUrgencyChanged { idx, urgent })?;
                }
            }

            Event::WindowsChanged { windows } => {
                // Rebuild window counts
                window_counts.clear();
                for window in &windows {
                    if let Some(ws_id) = window.workspace_id {
                        *window_counts.entry(ws_id).or_insert(0) += 1;
                    }
                }

                // Send updated counts
                for (&ws_id, &count) in &window_counts {
                    if let Some(&idx) = workspace_id_to_idx.get(&ws_id) {
                        sender.send(IpcMessage::WindowCountChanged { idx, count })?;
                    }
                }

                // Also send zero counts for workspaces with no windows
                for (&ws_id, &idx) in &workspace_id_to_idx {
                    if !window_counts.contains_key(&ws_id) {
                        sender.send(IpcMessage::WindowCountChanged { idx, count: 0 })?;
                    }
                }
            }

            Event::WindowOpenedOrChanged { window } => {
                // Update window count for the workspace
                if let Some(ws_id) = window.workspace_id {
                    // This is a simplified approach - ideally we'd track window IDs
                    // For now, just increment (will be corrected by next WindowsChanged)
                    if let Some(&idx) = workspace_id_to_idx.get(&ws_id) {
                        let count = window_counts.entry(ws_id).or_insert(0);
                        *count += 1;
                        sender.send(IpcMessage::WindowCountChanged { idx, count: *count })?;
                    }
                }
            }

            Event::WindowClosed { id: _ } => {
                // Window closed - we'll get accurate counts from next WindowsChanged
                // For immediate feedback, we could track window IDs but it's complex
            }

            // Ignore other events
            _ => {}
        }
    }
}

/// Process workspace list into simplified bar info
fn process_workspaces(
    workspaces: &[Workspace],
    window_counts: &HashMap<u64, u32>,
) -> Vec<WorkspaceInfo> {
    workspaces
        .iter()
        .filter_map(|ws| {
            let idx = ws.idx as usize;
            if idx < WORKSPACE_COUNT {
                Some(WorkspaceInfo {
                    idx,
                    is_active: ws.is_active,
                    is_focused: ws.is_focused,
                    is_urgent: ws.is_urgent,
                    window_count: window_counts.get(&ws.id).copied().unwrap_or(0),
                })
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_info() {
        let info = WorkspaceInfo {
            idx: 0,
            is_active: true,
            is_focused: true,
            is_urgent: false,
            window_count: 2,
        };
        assert_eq!(info.idx, 0);
        assert!(info.is_focused);
        assert_eq!(info.window_count, 2);
    }
}
