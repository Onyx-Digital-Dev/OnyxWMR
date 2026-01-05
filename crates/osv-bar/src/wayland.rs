//! Wayland client implementation using layer-shell protocol.

use anyhow::{Context, Result};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use std::cell::RefCell;
use std::rc::Rc;
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_output, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::bar::{render_bar, BarState, BAR_HEIGHT, BAR_MARGIN_TOP};
use crate::ipc::IpcClient;
use crate::total_bar_height;

/// Wayland client state
pub struct WaylandState {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub shm_state: Shm,
    pub layer_shell: LayerShell,

    pub layer_surface: Option<LayerSurface>,
    pub pool: Option<SlotPool>,
    pub buffer: Option<Buffer>,
    pub width: u32,
    pub height: u32,
    pub configured: bool,
    pub running: bool,

    pub bar_state: Rc<RefCell<BarState>>,
    pub ipc_client: Rc<RefCell<Option<IpcClient>>>,
}

impl WaylandState {
    pub fn new(
        registry_state: RegistryState,
        output_state: OutputState,
        compositor_state: CompositorState,
        shm_state: Shm,
        layer_shell: LayerShell,
        bar_state: Rc<RefCell<BarState>>,
        ipc_client: Rc<RefCell<Option<IpcClient>>>,
    ) -> Self {
        Self {
            registry_state,
            output_state,
            compositor_state,
            shm_state,
            layer_shell,
            layer_surface: None,
            pool: None,
            buffer: None,
            width: 0,
            height: total_bar_height(),
            configured: false,
            running: true,
            bar_state,
            ipc_client,
        }
    }

    /// Create the layer surface for the bar
    pub fn create_layer_surface(&mut self, qh: &QueueHandle<Self>) {
        let surface = self.compositor_state.create_surface(qh);
        let layer_surface = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Top,
            Some("osv-bar"),
            None, // Use first output
        );

        // Configure the layer surface
        // Floating bar: anchor top, use full height for surface (includes margins/shadow)
        let surface_height = total_bar_height();
        // Exclusive zone: just the bar + top margin (not including shadow that goes down)
        let exclusive = (BAR_HEIGHT + BAR_MARGIN_TOP) as i32;

        layer_surface.set_anchor(Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        layer_surface.set_size(0, surface_height); // Width auto-fills
        layer_surface.set_exclusive_zone(exclusive);
        layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);

        layer_surface.commit();

        self.layer_surface = Some(layer_surface);
    }

    /// Poll IPC for updates and apply to bar state
    fn poll_ipc(&mut self) -> bool {
        let ipc_borrow = self.ipc_client.borrow();
        if let Some(ref client) = *ipc_borrow {
            let mut state = self.bar_state.borrow_mut();
            client.update_bar_state(&mut state)
        } else {
            false
        }
    }

    /// Render and submit a new frame
    pub fn render(&mut self) {
        if !self.configured || self.width == 0 {
            return;
        }

        // Poll IPC for any updates
        self.poll_ipc();

        let pool = match &mut self.pool {
            Some(pool) => pool,
            None => return,
        };

        let layer_surface = match &self.layer_surface {
            Some(ls) => ls,
            None => return,
        };

        let stride = self.width as i32 * 4;
        let _size = stride * self.height as i32;

        // Get or create buffer
        let (buffer, canvas) = pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create buffer");

        // Render bar to pixmap
        let bar_state = self.bar_state.borrow();
        let pixmap = render_bar(self.width, self.height, &bar_state);

        // Copy pixmap data to canvas (convert RGBA to ARGB)
        let pixmap_data = pixmap.data();
        for (i, chunk) in canvas.chunks_exact_mut(4).enumerate() {
            if i * 4 + 3 < pixmap_data.len() {
                // tiny-skia uses RGBA, Wayland expects ARGB (actually BGRA in memory)
                let r = pixmap_data[i * 4];
                let g = pixmap_data[i * 4 + 1];
                let b = pixmap_data[i * 4 + 2];
                let a = pixmap_data[i * 4 + 3];
                chunk[0] = b;
                chunk[1] = g;
                chunk[2] = r;
                chunk[3] = a;
            }
        }

        // Attach and commit
        buffer.attach_to(layer_surface.wl_surface()).unwrap();
        layer_surface
            .wl_surface()
            .damage_buffer(0, 0, self.width as i32, self.height as i32);
        layer_surface.wl_surface().commit();

        self.buffer = Some(buffer);
    }
}

// Implement required traits

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Re-render at new scale
        self.render();
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.render();
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for WaylandState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.running = false;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.width = configure.new_size.0;
        self.height = configure.new_size.1.max(total_bar_height());

        // Create pool if needed
        if self.pool.is_none() {
            let pool = SlotPool::new(
                self.width as usize * self.height as usize * 4,
                &self.shm_state,
            )
            .expect("Failed to create pool");
            self.pool = Some(pool);
        }

        self.configured = true;
        self.render();

        // Request frame callback for continuous updates (clock + IPC)
        layer.wl_surface().frame(qh, layer.wl_surface().clone());
        layer.wl_surface().commit();
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }

    registry_handlers!(OutputState);
}

// Delegate implementations
delegate_compositor!(WaylandState);
delegate_output!(WaylandState);
delegate_shm!(WaylandState);
delegate_layer!(WaylandState);
delegate_registry!(WaylandState);

/// Connect to Wayland and run the bar
pub fn run_bar(
    bar_state: Rc<RefCell<BarState>>,
    ipc_client: Rc<RefCell<Option<IpcClient>>>,
) -> Result<()> {
    let conn = Connection::connect_to_env().context("Failed to connect to Wayland")?;

    let (globals, mut event_queue) =
        registry_queue_init(&conn).context("Failed to initialize registry")?;

    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state =
        CompositorState::bind(&globals, &qh).context("Compositor not available")?;
    let shm_state = Shm::bind(&globals, &qh).context("SHM not available")?;
    let layer_shell = LayerShell::bind(&globals, &qh).context("Layer shell not available")?;

    let mut state = WaylandState::new(
        registry_state,
        output_state,
        compositor_state,
        shm_state,
        layer_shell,
        bar_state,
        ipc_client,
    );

    // Create the layer surface
    state.create_layer_surface(&qh);

    // Event loop
    while state.running {
        event_queue
            .blocking_dispatch(&mut state)
            .context("Event dispatch failed")?;
    }

    Ok(())
}
