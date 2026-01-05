//! osv-intake - Application launcher for osvwm

mod protocol;

use anyhow::Result;
use protocol::{AppInfo, Request, Response, SOCKET_PATH};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::rc::Rc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

slint::include_modules!();

const MAX_RESULTS: usize = 8;

struct DaemonClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
}

impl DaemonClient {
    fn connect() -> Result<Self> {
        let writer = UnixStream::connect(SOCKET_PATH)?;
        writer.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        writer.set_write_timeout(Some(std::time::Duration::from_secs(2)))?;
        let reader = BufReader::new(writer.try_clone()?);
        Ok(Self { reader, writer })
    }

    fn search(&mut self, query: &str) -> Result<Vec<AppInfo>> {
        let req = if query.is_empty() {
            Request::List
        } else {
            Request::Search { query: query.to_string() }
        };

        let mut json = serde_json::to_string(&req)?;
        json.push('\n');
        self.writer.write_all(json.as_bytes())?;
        self.writer.flush()?;

        let mut line = String::new();
        self.reader.read_line(&mut line)?;

        match serde_json::from_str(&line)? {
            Response::Results { apps } => Ok(apps),
            Response::Error { message } => anyhow::bail!("Daemon error: {}", message),
            _ => anyhow::bail!("Unexpected response"),
        }
    }
}

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    info!("osv-intake starting...");

    let daemon = match DaemonClient::connect() {
        Ok(client) => {
            info!("Connected to osv-intake-daemon");
            Some(Rc::new(RefCell::new(client)))
        }
        Err(e) => {
            warn!("Failed to connect to daemon: {}", e);
            warn!("Falling back to direct loading");
            None
        }
    };

    let ui = IntakeLauncher::new()?;

    let results_model: Rc<VecModel<AppResult>> = Rc::new(VecModel::default());
    ui.set_results(ModelRc::from(results_model.clone()));

    let current_results: Rc<RefCell<Vec<AppInfo>>> = Rc::new(RefCell::new(Vec::new()));

    let daemon_for_search = daemon.clone();
    let results_for_search = results_model.clone();
    let current_for_search = current_results.clone();
    let ui_weak = ui.as_weak();

    ui.on_search_changed(move |query| {
        let query_str = query.to_string();
        results_for_search.set_vec(Vec::new());

        if query_str.is_empty() {
            current_for_search.borrow_mut().clear();
            return;
        }

        let apps = if let Some(ref daemon) = daemon_for_search {
            match daemon.borrow_mut().search(&query_str) {
                Ok(apps) => apps,
                Err(e) => {
                    warn!("Daemon query failed: {}", e);
                    Vec::new()
                }
            }
        } else {
            Vec::new()
        };

        let mut slint_results = Vec::new();
        let mut matched_apps = Vec::new();

        for app in apps.into_iter().take(MAX_RESULTS) {
            slint_results.push(AppResult {
                name: SharedString::from(&app.name),
                description: SharedString::from(app.generic_name.as_deref().unwrap_or("")),
                exec: SharedString::from(&app.exec),
            });
            matched_apps.push(app);
        }

        results_for_search.set_vec(slint_results);
        *current_for_search.borrow_mut() = matched_apps;

        if let Some(ui) = ui_weak.upgrade() {
            ui.set_selected_index(0);
        }
    });

    let current_for_launch = current_results.clone();
    let ui_weak_launch = ui.as_weak();

    ui.on_launch_selected(move || {
        let selected = if let Some(ui) = ui_weak_launch.upgrade() {
            ui.get_selected_index() as usize
        } else {
            return;
        };

        let results = current_for_launch.borrow();
        if let Some(app) = results.get(selected) {
            launch_app(&app.exec, app.terminal);
        } else if let Some(ui) = ui_weak_launch.upgrade() {
            let query = ui.get_search_query().to_string();
            if !query.is_empty() {
                launch_command(&query);
            }
        }
        slint::quit_event_loop().ok();
    });

    let current_for_click = current_results.clone();
    ui.on_launch_at_index(move |idx| {
        let results = current_for_click.borrow();
        if let Some(app) = results.get(idx as usize) {
            launch_app(&app.exec, app.terminal);
        }
        slint::quit_event_loop().ok();
    });

    let ui_weak_nav = ui.as_weak();
    let results_for_nav = results_model.clone();

    ui.on_navigate_up(move || {
        if let Some(ui) = ui_weak_nav.upgrade() {
            let current = ui.get_selected_index();
            if current > 0 {
                ui.set_selected_index(current - 1);
            }
        }
    });

    let ui_weak_nav2 = ui.as_weak();
    ui.on_navigate_down(move || {
        if let Some(ui) = ui_weak_nav2.upgrade() {
            let current = ui.get_selected_index();
            let count = results_for_nav.row_count() as i32;
            if current < count - 1 {
                ui.set_selected_index(current + 1);
            }
        }
    });

    ui.on_close_launcher(|| {
        info!("Launcher closed");
        slint::quit_event_loop().ok();
    });

    ui.run()?;
    info!("osv-intake exiting");
    Ok(())
}

fn launch_app(exec: &str, terminal: bool) {
    info!("Launching app: {}", exec);
    let result = if terminal {
        Command::new("sh")
            .arg("-c")
            .arg(format!("x-terminal-emulator -e {}", exec))
            .spawn()
    } else {
        Command::new("sh").arg("-c").arg(exec).spawn()
    };

    if let Err(e) = result {
        warn!("Failed to launch '{}': {}", exec, e);
    }
}

fn launch_command(cmd: &str) {
    info!("Launching command: {}", cmd);
    if let Err(e) = Command::new("sh").arg("-c").arg(cmd).spawn() {
        warn!("Failed to launch '{}': {}", cmd, e);
    }
}
