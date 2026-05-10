//! WebView-based editor for Hardwave PumpControl.
//!
//! Uses the same hwpacket bridge pattern as LoudLab/KickForge:
//! - Linux/macOS: Rust pushes state via `evaluate_script()`.
//! - Windows: Rust starts a local TCP server, JS polls via `fetch()`.

use crossbeam_channel::{Receiver, Sender, unbounded};
use nih_plug::editor::Editor;
use nih_plug::prelude::{GuiContext, ParentWindowHandle, Param};
use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Process-unique identifier for a plug-in instance — used as the
/// per-instance suffix for the WebView2 user-data folder.
fn unique_instance_id() -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    format!("{}-{}-{}", pid, nanos, n)
}

/// Cooperative shutdown signal — wakes the editor's worker threads
/// immediately when Drop fires so editor close takes <1ms.
struct ShutdownSignal {
    flag: Mutex<bool>,
    cv: Condvar,
}

impl ShutdownSignal {
    fn new() -> Self { Self { flag: Mutex::new(false), cv: Condvar::new() } }
    fn signal(&self) {
        let mut g = self.flag.lock();
        *g = true;
        self.cv.notify_all();
    }
    fn is_shutdown(&self) -> bool { *self.flag.lock() }
    fn wait(&self, timeout: Duration) -> bool {
        let mut g = self.flag.lock();
        if *g { return true; }
        let _ = self.cv.wait_for(&mut g, timeout);
        *g
    }
}

use crate::auth;
use crate::dsp::envelope::CurveData;
use crate::params::PumpControlParams;
use crate::presets;
use crate::protocol::PumpPacket;

const PUMPCONTROL_URL: &str = "https://pumpcontrol.hardwavestudios.com/vst/pumpcontrol";
const EDITOR_WIDTH: u32 = 900;
const EDITOR_HEIGHT: u32 = 560;
const MIN_WIDTH: u32 = 600;
const MIN_HEIGHT: u32 = 380;
const MAX_WIDTH: u32 = 2560;
const MAX_HEIGHT: u32 = 1600;

/// Wraps a raw window handle value (usize) so wry can use it via rwh 0.6.
struct RwhWrapper(usize);

unsafe impl Send for RwhWrapper {}
unsafe impl Sync for RwhWrapper {}

impl raw_window_handle::HasWindowHandle for RwhWrapper {
    fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::RawWindowHandle;

        #[cfg(target_os = "linux")]
        let raw = {
            let h = raw_window_handle::XlibWindowHandle::new(self.0 as _);
            RawWindowHandle::Xlib(h)
        };

        #[cfg(target_os = "macos")]
        let raw = {
            let ns_view = std::ptr::NonNull::new(self.0 as *mut _)
                .ok_or(raw_window_handle::HandleError::Unavailable)?;
            let h = raw_window_handle::AppKitWindowHandle::new(ns_view);
            RawWindowHandle::AppKit(h)
        };

        #[cfg(target_os = "windows")]
        let raw = {
            let hwnd = std::num::NonZeroIsize::new(self.0 as isize)
                .ok_or(raw_window_handle::HandleError::Unavailable)?;
            let h = raw_window_handle::Win32WindowHandle::new(hwnd);
            RawWindowHandle::Win32(h)
        };

        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(raw) })
    }
}

impl raw_window_handle::HasDisplayHandle for RwhWrapper {
    fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        use raw_window_handle::RawDisplayHandle;

        #[cfg(target_os = "linux")]
        let raw = RawDisplayHandle::Xlib(raw_window_handle::XlibDisplayHandle::new(None, 0));

        #[cfg(target_os = "macos")]
        let raw = RawDisplayHandle::AppKit(raw_window_handle::AppKitDisplayHandle::new());

        #[cfg(target_os = "windows")]
        let raw = RawDisplayHandle::Windows(raw_window_handle::WindowsDisplayHandle::new());

        Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(raw) })
    }
}

/// Build a map of param ID strings to ParamPtr for the IPC handler.
fn build_param_map(params: &PumpControlParams) -> HashMap<String, nih_plug::prelude::ParamPtr> {
    let mut map = HashMap::new();

    map.insert("enabled".into(), params.enabled.as_ptr());
    map.insert("input_gain".into(), params.input_gain.as_ptr());
    map.insert("output_gain".into(), params.output_gain.as_ptr());
    map.insert("mix".into(), params.mix.as_ptr());
    map.insert("depth".into(), params.depth.as_ptr());

    map.insert("trigger_mode".into(), params.trigger_mode.as_ptr());
    map.insert("sync_rate".into(), params.sync_rate.as_ptr());
    map.insert("rate_hz".into(), params.rate_hz.as_ptr());
    map.insert("phase_offset".into(), params.phase_offset.as_ptr());

    map.insert("sc_threshold".into(), params.sc_threshold.as_ptr());
    map.insert("sc_attack".into(), params.sc_attack.as_ptr());
    map.insert("sc_release".into(), params.sc_release.as_ptr());

    map.insert("multiband".into(), params.multiband.as_ptr());
    map.insert("xover_low".into(), params.xover_low.as_ptr());
    map.insert("xover_high".into(), params.xover_high.as_ptr());
    map.insert("depth_low".into(), params.depth_low.as_ptr());
    map.insert("depth_mid".into(), params.depth_mid.as_ptr());
    map.insert("depth_high".into(), params.depth_high.as_ptr());

    map
}

/// Create a snapshot of the current DAW params as a `PumpPacket`.
pub fn snapshot_params(params: &PumpControlParams) -> PumpPacket {
    let curve_points = params.curve_data.lock().points.clone();

    PumpPacket {
        enabled: params.enabled.value(),
        input_gain: params.input_gain.value(),
        output_gain: params.output_gain.value(),
        mix: params.mix.value(),
        depth: params.depth.value(),

        trigger_mode: params.trigger_mode.value() as i32,
        sync_rate: params.sync_rate.value(),
        rate_hz: params.rate_hz.value(),
        phase_offset: params.phase_offset.value(),

        sc_threshold: params.sc_threshold.value(),
        sc_attack: params.sc_attack.value(),
        sc_release: params.sc_release.value(),

        multiband: params.multiband.value(),
        xover_low: params.xover_low.value(),
        xover_high: params.xover_high.value(),
        depth_low: params.depth_low.value(),
        depth_mid: params.depth_mid.value(),
        depth_high: params.depth_high.value(),

        curve_points,

        input_peak_l: -120.0,
        input_peak_r: -120.0,
        output_peak_l: -120.0,
        output_peak_r: -120.0,
        current_phase: 0.0,
        current_gain: 1.0,
    }
}

/// Build the init JavaScript that gets injected into the webview on load.
fn ipc_init_script(params: &PumpControlParams) -> String {
    let snapshot = snapshot_params(params);
    let initial_json = serde_json::to_string(&snapshot).unwrap_or_else(|_| "null".into());
    let version = env!("CARGO_PKG_VERSION");

    format!(
        r#"
(function() {{
    var _focusTimer = null;
    window.addEventListener('mouseup', function(e) {{
        if (e.target.tagName !== 'INPUT') {{
            clearTimeout(_focusTimer);
            _focusTimer = setTimeout(function() {{
                try {{ window.ipc.postMessage(JSON.stringify({{ type: 'release_focus' }})); }} catch(_) {{}}
            }}, 500);
        }}
    }}, true);
    document.addEventListener('blur', function(e) {{
        if (e.target.tagName === 'INPUT') {{
            clearTimeout(_focusTimer);
            try {{ window.ipc.postMessage(JSON.stringify({{ type: 'release_focus' }})); }} catch(_) {{}}
        }}
    }}, true);
}})();

window.__HARDWAVE_VST = true;
window.__HARDWAVE_VST_VERSION = '{version}';
window.__hardwave = {{
    postMessage: function(msg) {{
        window.ipc.postMessage(JSON.stringify(msg));
    }}
}};

(function() {{
    var _init = {initial_json};
    function pushInit() {{
        if (window.__onPumpPacket) {{
            window.__onPumpPacket(_init);
        }} else {{
            setTimeout(pushInit, 50);
        }}
    }}
    if (document.readyState === 'complete') {{ pushInit(); }}
    else {{ window.addEventListener('load', pushInit); }}
}})();
"#,
    )
}

/// Handle IPC messages from the webview.
fn handle_ipc(
    context: &Arc<dyn GuiContext>,
    param_map: &HashMap<String, nih_plug::prelude::ParamPtr>,
    curve_data: &Arc<Mutex<CurveData>>,
    raw_body: &str,
    _parent_hwnd: usize,
    editor_size: &Arc<Mutex<(u32, u32)>>,
    resize_tx: &Arc<Mutex<Option<Sender<(u32, u32)>>>>,
) {
    let msg: serde_json::Value = match serde_json::from_str(raw_body) {
        Ok(v) => v,
        Err(_) => return,
    };

    let msg_type = msg.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match msg_type {
        "set_param" => {
            let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let value = msg.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if let Some(ptr) = param_map.get(id) {
                unsafe {
                    let normalized = ptr.preview_normalized(value as f32);
                    context.raw_begin_set_parameter(*ptr);
                    context.raw_set_parameter_normalized(*ptr, normalized);
                    context.raw_end_set_parameter(*ptr);
                }
            }
        }
        "set_curve" => {
            if let Some(points_val) = msg.get("points") {
                if let Ok(points) = serde_json::from_value::<Vec<crate::dsp::envelope::CurvePoint>>(points_val.clone()) {
                    *curve_data.lock() = CurveData { points };
                }
            }
        }
        "load_preset" => {
            if let Some(name) = msg.get("name").and_then(|v| v.as_str()) {
                if let Some(curve) = presets::load_preset(name) {
                    *curve_data.lock() = curve;
                }
            }
        }
        "release_focus" => {
            #[cfg(target_os = "windows")]
            unsafe {
                use windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus;
                SetFocus(_parent_hwnd as windows_sys::Win32::Foundation::HWND);
            }
        }
        "resize" => {
            let w = msg.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let h = msg.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            if w >= MIN_WIDTH && w <= MAX_WIDTH && h >= MIN_HEIGHT && h <= MAX_HEIGHT {
                *editor_size.lock() = (w, h);
                if context.request_resize() {
                    if let Some(tx) = resize_tx.lock().as_ref() {
                        let _ = tx.send((w, h));
                    }
                }
            }
        }
        "save_token" => {
            if let Some(token) = msg.get("token").and_then(|v| v.as_str()) {
                let _ = auth::save_token(token);
            }
        }
        "clear_token" => {
            let _ = auth::clear_token();
        }
        _ => {}
    }
}

pub struct PumpEditor {
    params: Arc<PumpControlParams>,
    packet_rx: Arc<Mutex<Receiver<PumpPacket>>>,
    auth_token: Option<String>,
    scale_factor: Mutex<f32>,
    editor_size: Arc<Mutex<(u32, u32)>>,
    resize_tx: Arc<Mutex<Option<Sender<(u32, u32)>>>>,
    /// Process-unique instance ID for the per-instance WebView2 dir.
    instance_id: String,
}

impl PumpEditor {
    pub fn new(
        params: Arc<PumpControlParams>,
        packet_rx: Arc<Mutex<Receiver<PumpPacket>>>,
        auth_token: Option<String>,
    ) -> Self {
        Self {
            params,
            packet_rx,
            auth_token,
            scale_factor: Mutex::new(1.0),
            editor_size: Arc::new(Mutex::new((EDITOR_WIDTH, EDITOR_HEIGHT))),
            resize_tx: Arc::new(Mutex::new(None)),
            instance_id: unique_instance_id(),
        }
    }

    fn scaled_size(&self) -> (u32, u32) {
        let (w, h) = *self.editor_size.lock();
        let f = *self.scale_factor.lock();
        ((w as f32 * f) as u32, (h as f32 * f) as u32)
    }
}

impl Editor for PumpEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let packet_rx = Arc::clone(&self.packet_rx);
        let (width, height) = self.scaled_size();

        let version = env!("CARGO_PKG_VERSION");
        let url = match &self.auth_token {
            Some(t) => format!("{}?token={}&v={}", PUMPCONTROL_URL, t, version),
            None => format!("{}?v={}", PUMPCONTROL_URL, version),
        };

        let param_map = Arc::new(build_param_map(&self.params));
        let curve_data = Arc::clone(&self.params.curve_data);
        let init_js = ipc_init_script(&self.params);
        let raw_handle = extract_raw_handle(&parent);

        let (resize_tx_val, resize_rx) = unbounded::<(u32, u32)>();
        *self.resize_tx.lock() = Some(resize_tx_val);

        let editor_size = Arc::clone(&self.editor_size);
        let resize_tx = Arc::clone(&self.resize_tx);

        #[cfg(target_os = "windows")]
        {
            spawn_windows(raw_handle, url, width, height, packet_rx, context, param_map, curve_data, init_js, resize_rx, editor_size, resize_tx, self.instance_id.clone())
        }

        #[cfg(not(target_os = "windows"))]
        {
            spawn_unix(raw_handle, url, width, height, packet_rx, context, param_map, curve_data, init_js, resize_rx, editor_size, resize_tx, self.instance_id.clone())
        }
    }

    fn size(&self) -> (u32, u32) {
        self.scaled_size()
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        // Clamp the host-supplied DPI scale to a sane range so a misbehaving
        // host can't shrink the editor to zero pixels (which then makes the
        // webview layout glitch on resize).
        let clamped = factor.clamp(0.5, 4.0);
        *self.scale_factor.lock() = clamped;
        true
    }

    fn set_size(&self, width: u32, height: u32) {
        let w = width.clamp(MIN_WIDTH, MAX_WIDTH);
        let h = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
        *self.editor_size.lock() = (w, h);
        if let Some(tx) = self.resize_tx.lock().as_ref() {
            let _ = tx.send((w, h));
        }
    }

    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}
    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}
    fn param_values_changed(&self) {}
}

fn extract_raw_handle(parent: &ParentWindowHandle) -> usize {
    match *parent {
        #[cfg(target_os = "linux")]
        ParentWindowHandle::X11Window(id) => id as usize,
        #[cfg(target_os = "macos")]
        ParentWindowHandle::AppKitNsView(ptr) => ptr as usize,
        #[cfg(target_os = "windows")]
        ParentWindowHandle::Win32Hwnd(h) => h as usize,
        _ => 0,
    }
}

// ─── Shared: persistent WebView data directory ─────────────────────────────

fn webview_data_dir(instance_id: &str) -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("hardwave")
        .join("pumpcontrol-webview")
        .join(instance_id)
}

// ─── Windows: TCP polling approach ─────────────────────────────────────────

/// Per-instance WebView2 user-data folder. Two PumpControls on different
/// tracks no longer collide on the same UserDataFolder lock.
#[cfg(target_os = "windows")]
fn webview2_data_dir(instance_id: &str) -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("hardwave")
        .join("pumpcontrol-webview2")
        .join(instance_id)
}

#[cfg(target_os = "windows")]
fn spawn_windows(
    raw_handle: usize,
    url: String,
    width: u32,
    height: u32,
    packet_rx: Arc<Mutex<Receiver<PumpPacket>>>,
    context: Arc<dyn GuiContext>,
    param_map: Arc<HashMap<String, nih_plug::prelude::ParamPtr>>,
    curve_data: Arc<Mutex<CurveData>>,
    base_init_js: String,
    resize_rx: Receiver<(u32, u32)>,
    editor_size: Arc<Mutex<(u32, u32)>>,
    resize_tx: Arc<Mutex<Option<Sender<(u32, u32)>>>>,
    instance_id: String,
) -> Box<dyn std::any::Any + Send> {
    use std::io::{Read as IoRead, Write as IoWrite};
    use std::net::TcpListener;

    let shutdown = Arc::new(ShutdownSignal::new());
    let shutdown_for_handle = Arc::clone(&shutdown);

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[HardwavePumpControl] failed to bind TCP: {}", e);
            return Box::new(EditorHandle {
                shutdown: shutdown_for_handle,
                _webview: None,
                _web_context: None,
                _server_thread: None,
                _editor_thread: None,
            });
        }
    };
    let port = match listener.local_addr() {
        Ok(a) => a.port(),
        Err(e) => {
            eprintln!("[HardwavePumpControl] failed to read local_addr: {}", e);
            return Box::new(EditorHandle {
                shutdown: shutdown_for_handle,
                _webview: None,
                _web_context: None,
                _server_thread: None,
                _editor_thread: None,
            });
        }
    };
    let latest_json = Arc::new(Mutex::new(String::from("{}")));
    let latest_json_server = Arc::clone(&latest_json);
    let shutdown_server = Arc::clone(&shutdown);

    let server_thread = std::thread::spawn(move || {
        listener.set_nonblocking(true).ok();
        while !shutdown_server.is_shutdown() {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = latest_json_server.lock().clone();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
            if let Some(rx) = packet_rx.try_lock() {
                while let Ok(pkt) = rx.try_recv() {
                    if let Ok(json) = serde_json::to_string(&pkt) {
                        *latest_json.lock() = json;
                    }
                }
            }
            while resize_rx.try_recv().is_ok() {}
            if shutdown_server.wait(Duration::from_millis(8)) {
                break;
            }
        }
    });

    let poll_script = format!(
        r#"
(function() {{
    var _port = {port};
    function poll() {{
        fetch('http://127.0.0.1:' + _port)
            .then(function(r) {{ return r.json(); }})
            .then(function(data) {{
                if (window.__onPumpPacket) window.__onPumpPacket(data);
            }})
            .catch(function() {{}});
        setTimeout(poll, 16);
    }}
    poll();
}})();
"#,
    );

    let init_js = format!("{}\n{}", base_init_js, poll_script);
    let ctx = Arc::clone(&context);
    let pmap = Arc::clone(&param_map);
    let cdata = Arc::clone(&curve_data);
    let esize = Arc::clone(&editor_size);
    let rtx = Arc::clone(&resize_tx);

    let data_dir = webview2_data_dir(&instance_id);
    let _ = std::fs::create_dir_all(&data_dir);
    let mut web_context = wry::WebContext::new(Some(data_dir));

    let wrapper = RwhWrapper(raw_handle);

    use wry::WebViewBuilderExtWindows;
    let webview = wry::WebViewBuilder::with_web_context(&mut web_context)
        .with_url(&url)
        .with_initialization_script(&init_js)
        .with_ipc_handler(move |msg| {
            handle_ipc(&ctx, &pmap, &cdata, &msg.body(), raw_handle, &esize, &rtx);
        })
        .with_bounds(wry::Rect {
            position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
            size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(width as f64, height as f64)),
        })
        .with_transparent(false)
        .with_devtools(false)
        // Disable WebView2 browser accelerator keys (Ctrl+P / Ctrl+S /
        // Ctrl+R / F5 / F12 / Ctrl+Shift+I) at the OS level so the print
        // / save / devtools shortcuts are dead even before the JS
        // keydown blocker attaches.
        .with_browser_accelerator_keys(false)
        .with_background_color((10, 10, 11, 255))
        .build(&wrapper)
        .ok();

    Box::new(EditorHandle {
        shutdown: shutdown_for_handle,
        _webview: webview,
        _web_context: Some(web_context),
        _server_thread: Some(server_thread),
        _editor_thread: None,
    })
}

// ─── Linux / macOS: evaluate_script approach ───────────────────────────────

#[cfg(not(target_os = "windows"))]
fn spawn_unix(
    raw_handle: usize,
    url: String,
    width: u32,
    height: u32,
    packet_rx: Arc<Mutex<Receiver<PumpPacket>>>,
    context: Arc<dyn GuiContext>,
    param_map: Arc<HashMap<String, nih_plug::prelude::ParamPtr>>,
    curve_data: Arc<Mutex<CurveData>>,
    init_js: String,
    resize_rx: Receiver<(u32, u32)>,
    editor_size: Arc<Mutex<(u32, u32)>>,
    resize_tx: Arc<Mutex<Option<Sender<(u32, u32)>>>>,
    instance_id: String,
) -> Box<dyn std::any::Any + Send> {
    let shutdown = Arc::new(ShutdownSignal::new());
    let shutdown_for_handle = Arc::clone(&shutdown);
    let shutdown_thread = Arc::clone(&shutdown);

    let editor_thread = std::thread::spawn(move || {
        #[cfg(target_os = "linux")]
        {
            let _ = gtk::init();
        }

        let wrapper = RwhWrapper(raw_handle);
        let ctx = Arc::clone(&context);
        let pmap = Arc::clone(&param_map);
        let cdata = Arc::clone(&curve_data);
        let esize = Arc::clone(&editor_size);
        let rtx = Arc::clone(&resize_tx);

        let data_dir = webview_data_dir(&instance_id);
        let _ = std::fs::create_dir_all(&data_dir);
        let mut web_context = wry::WebContext::new(Some(data_dir));

        let webview = match wry::WebViewBuilder::with_web_context(&mut web_context)
            .with_url(&url)
            .with_initialization_script(&init_js)
            .with_ipc_handler(move |msg| {
                handle_ipc(&ctx, &pmap, &cdata, &msg.body(), raw_handle, &esize, &rtx);
            })
            .with_bounds(wry::Rect {
                position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(width as f64, height as f64)),
            })
            .with_devtools(false)
            .build_as_child(&wrapper)
        {
            Ok(wv) => wv,
            Err(e) => {
                eprintln!("[PumpControl] failed to create WebView: {}", e);
                return;
            }
        };

        while !shutdown_thread.is_shutdown() {
            while let Ok((w, h)) = resize_rx.try_recv() {
                let _ = webview.set_bounds(wry::Rect {
                    position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(0.0, 0.0)),
                    size: wry::dpi::Size::Logical(wry::dpi::LogicalSize::new(w as f64, h as f64)),
                });
            }

            if let Some(rx) = packet_rx.try_lock() {
                while let Ok(pkt) = rx.try_recv() {
                    if let Ok(json) = serde_json::to_string(&pkt) {
                        let js = format!(
                            "window.__onPumpPacket && window.__onPumpPacket({})",
                            json
                        );
                        let _ = webview.evaluate_script(&js);
                    }
                }
            }

            #[cfg(target_os = "linux")]
            {
                while gtk::events_pending() {
                    gtk::main_iteration_do(false);
                }
            }

            if shutdown_thread.wait(Duration::from_millis(16)) {
                break;
            }
        }
    });

    Box::new(EditorHandle {
        shutdown: shutdown_for_handle,
        _webview: None,
        _web_context: None,
        _server_thread: None,
        _editor_thread: Some(editor_thread),
    })
}

// ─── Editor handle (dropped when DAW closes editor) ───────────────────────

struct EditorHandle {
    shutdown: Arc<ShutdownSignal>,
    _webview: Option<wry::WebView>,
    _web_context: Option<wry::WebContext>,
    _server_thread: Option<std::thread::JoinHandle<()>>,
    _editor_thread: Option<std::thread::JoinHandle<()>>,
}

unsafe impl Send for EditorHandle {}

impl Drop for EditorHandle {
    fn drop(&mut self) {
        self.shutdown.signal();
        if let Some(h) = self._server_thread.take() {
            let _ = h.join();
        }
        if let Some(h) = self._editor_thread.take() {
            let _ = h.join();
        }
    }
}
