//! Real OS-level (cross-application) drag-and-drop of files out of an
//! iced window — e.g. dragging a file from og-files onto Brave's upload
//! drop zone.
//!
//! iced/winit have no Wayland drag-and-drop support at all (checked: no
//! `wl_data_source`/`start_drag` anywhere in either crate). The trick,
//! copied from how `smithay-clipboard` implements copy/paste (which iced
//! already depends on transitively): open a *second*, independent
//! `wayland-client` connection wrapping the *same real* Wayland socket
//! winit is already using (`Backend::from_foreign_display`), bind our own
//! `wl_seat`/`wl_pointer`/`wl_data_device_manager` on it, and use
//! smithay-client-toolkit's `DragSource` to start a real drag. Our own
//! pointer gets real `enter`/`press` events (with real serials and a real
//! reference to winit's surface) because the compositor broadcasts input
//! events to every `wl_pointer` resource a client has bound for a seat —
//! not just the one the toolkit itself created.
//!
//! Only handles being the drag *source* here (files leaving the window).
//! Internal same-window drag-to-folder is each app's own business, via
//! plain iced mouse events — this crate doesn't need to know about it.

pub mod overlay;

use std::ffi::{c_void, CString};
use std::path::PathBuf;

use calloop::channel::{self, Sender};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::data_device_manager::data_device::DataDevice;
use smithay_client_toolkit::data_device_manager::data_source::DragSource;
use smithay_client_toolkit::data_device_manager::WritePipe;
use smithay_client_toolkit::data_device_manager::{data_device::DataDeviceHandler, DataDeviceManagerState};
use smithay_client_toolkit::data_device_manager::data_offer::{DataOfferHandler, DragOffer};
use smithay_client_toolkit::data_device_manager::data_source::DataSourceHandler;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::{
    delegate_data_device, delegate_pointer, delegate_registry, delegate_seat, registry_handlers,
};
use wayland_client::backend::Backend;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_data_device_manager::DndAction, wl_pointer::WlPointer, wl_seat::WlSeat, wl_surface::WlSurface};
use wayland_client::{Connection, Proxy, QueueHandle};

pub struct DragRequest {
    pub paths: Vec<PathBuf>,
}

/// Reads the real `wl_display*` behind a raw Wayland `wl_surface*` (as
/// handed to us by `iced::window::run_with_handle`) via libwayland-client's
/// own `wl_proxy_get_display`. `RTLD_NOLOAD` ensures we attach to the
/// *already-loaded* copy of the library (the one winit itself is using),
/// not load a second instance with separate internal state.
///
/// # Safety
/// `surface` must be a live `*mut wl_proxy` for as long as this call runs
/// (true here — it's read synchronously inside iced's own event loop via
/// `run_with_handle`, before the window can have closed).
unsafe fn display_from_surface(surface: *mut c_void) -> Option<*mut c_void> {
    let lib_name = CString::new("libwayland-client.so.0").ok()?;
    let handle = libc::dlopen(lib_name.as_ptr(), libc::RTLD_NOW | libc::RTLD_NOLOAD);
    if handle.is_null() {
        return None;
    }
    let sym_name = CString::new("wl_proxy_get_display").ok()?;
    let sym = libc::dlsym(handle, sym_name.as_ptr());
    if sym.is_null() {
        return None;
    }
    let func: extern "C" fn(*mut c_void) -> *mut c_void = std::mem::transmute(sym);
    let display = func(surface);
    if display.is_null() {
        None
    } else {
        Some(display)
    }
}

/// Extracts the raw Wayland surface pointer from an iced `WindowHandle`,
/// then resolves it to the real `wl_display*`. Returns `None` on any
/// non-Wayland backend (e.g. if this ever runs under X11) or FFI failure.
pub fn display_ptr_from_window_handle(handle: iced::window::raw_window_handle::WindowHandle<'_>) -> Option<usize> {
    use iced::window::raw_window_handle::RawWindowHandle;
    let RawWindowHandle::Wayland(wh) = handle.as_raw() else { return None };
    let surface_ptr = wh.surface.as_ptr();
    let display_ptr = unsafe { display_from_surface(surface_ptr) }?;
    Some(display_ptr as usize)
}

/// Spawns the worker thread and returns a channel to request drags on it.
/// `display_ptr` is the real `wl_display*` (as a plain integer so it can
/// cross the thread boundary — the pointer itself is reconstructed, and
/// wrapped in a `Connection`, inside the new thread).
pub fn spawn(display_ptr: usize) -> Sender<DragRequest> {
    let (tx, rx) = channel::channel();
    std::thread::Builder::new()
        .name("og-wayland-drag".into())
        .spawn(move || {
            // Safety: `display_ptr` came from `wl_proxy_get_display` on a
            // surface belonging to our own still-running window; the real
            // connection stays alive for the process's whole lifetime.
            let backend = unsafe { Backend::from_foreign_display(display_ptr as *mut _) };
            let connection = Connection::from_backend(backend);
            worker(connection, rx);
        })
        .expect("spawn og-wayland-drag thread");
    tx
}

fn worker(connection: Connection, rx: channel::Channel<DragRequest>) {
    let (globals, event_queue) = match registry_queue_init::<State>(&connection) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("og-wayland: registry_queue_init failed: {e:?}");
            return;
        }
    };
    let qh = event_queue.handle();

    let data_device_manager_state = match DataDeviceManagerState::bind(&globals, &qh) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("og-wayland: DataDeviceManagerState::bind failed (compositor has no wl_data_device_manager?): {e:?}");
            return;
        }
    };

    let mut event_loop: EventLoop<State> = EventLoop::try_new().expect("calloop init");
    let mut state = State {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        data_device_manager_state,
        seat: None,
        pointer: None,
        data_device: None,
        latest_serial: 0,
        latest_surface: None,
        active_drag: None,
    };

    WaylandSource::new(connection, event_queue).insert(event_loop.handle()).expect("insert wayland source");

    event_loop
        .handle()
        .insert_source(rx, move |event, _, state: &mut State| {
            if let channel::Event::Msg(DragRequest { paths }) = event {
                state.start_drag(&qh, paths);
            }
        })
        .expect("insert drag-request channel");

    loop {
        if event_loop.dispatch(None, &mut state).is_err() {
            break;
        }
    }
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    data_device_manager_state: DataDeviceManagerState,
    seat: Option<WlSeat>,
    pointer: Option<WlPointer>,
    data_device: Option<DataDevice>,
    latest_serial: u32,
    latest_surface: Option<WlSurface>,
    /// Kept alive for the duration of the drag — dropping a `DragSource`
    /// cancels it. Also holds the URI-list bytes to hand back on `send`.
    active_drag: Option<(DragSource, Vec<u8>)>,
}

impl State {
    fn start_drag(&mut self, qh: &QueueHandle<Self>, paths: Vec<PathBuf>) {
        let Some(surface) = self.latest_surface.clone() else { return };
        let Some(seat) = self.seat.clone() else { return };

        if self.data_device.is_none() {
            self.data_device = Some(self.data_device_manager_state.get_data_device(qh, &seat));
        }
        let Some(device) = &self.data_device else { return };

        let uri_list = paths
            .iter()
            .filter_map(|p| p.canonicalize().ok())
            .map(|p| format!("file://{}", p.display()))
            .collect::<Vec<_>>()
            .join("\r\n");

        let source = self.data_device_manager_state.create_drag_and_drop_source(
            qh,
            ["text/uri-list"],
            DndAction::Copy,
        );
        source.start_drag(device, &surface, None, self.latest_serial);
        self.active_drag = Some((source, uri_list.into_bytes()));
    }
}

impl PointerHandler for State {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &WlPointer, events: &[PointerEvent]) {
        for event in events {
            // Any event's surface is a live reference to whatever real
            // surface currently has pointer focus — that's winit's window.
            self.latest_surface = Some(event.surface.clone());
            if let PointerEventKind::Press { serial, .. } | PointerEventKind::Release { serial, .. } = event.kind {
                self.latest_serial = serial;
            }
        }
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: WlSeat) {
        if self.seat.is_none() {
            self.seat = Some(seat);
        }
    }

    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: WlSeat, capability: Capability) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seat_state.get_pointer(qh, &seat).ok();
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                if pointer.version() >= 3 {
                    pointer.release();
                }
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

impl DataSourceHandler for State {
    fn send_request(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource, mime: String, write_pipe: WritePipe) {
        if mime != "text/uri-list" {
            return;
        }
        let Some((_, content)) = &self.active_drag else { return };
        use std::io::Write;
        let mut pipe = write_pipe;
        let _ = pipe.write_all(content);
    }

    fn cancelled(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource) {
        self.active_drag = None;
    }

    fn accept_mime(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource, _mime: Option<String>) {}

    fn dnd_dropped(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource) {}

    fn dnd_finished(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource) {
        self.active_drag = None;
    }

    fn action(&mut self, _: &Connection, _: &QueueHandle<Self>, _source: &wayland_client::protocol::wl_data_source::WlDataSource, _action: DndAction) {}
}

impl DataDeviceHandler for State {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _device: &wayland_client::protocol::wl_data_device::WlDataDevice, _x: f64, _y: f64, _surface: &WlSurface) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _device: &wayland_client::protocol::wl_data_device::WlDataDevice) {}
    fn motion(&mut self, _: &Connection, _: &QueueHandle<Self>, _device: &wayland_client::protocol::wl_data_device::WlDataDevice, _x: f64, _y: f64) {}
    fn drop_performed(&mut self, _: &Connection, _: &QueueHandle<Self>, _device: &wayland_client::protocol::wl_data_device::WlDataDevice) {}
    fn selection(&mut self, _: &Connection, _: &QueueHandle<Self>, _device: &wayland_client::protocol::wl_data_device::WlDataDevice) {}
}

impl DataOfferHandler for State {
    fn source_actions(&mut self, _: &Connection, _: &QueueHandle<Self>, _offer: &mut DragOffer, _actions: DndAction) {}
    fn selected_action(&mut self, _: &Connection, _: &QueueHandle<Self>, _offer: &mut DragOffer, _actions: DndAction) {}
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![SeatState];
}

delegate_seat!(State);
delegate_pointer!(State);
delegate_data_device!(State);
delegate_registry!(State);
