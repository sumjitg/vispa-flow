// ipc.rs — Python sidecar management + TCP socket communication
//
// Per agent.md:
//   - Spawn Python sidecar (python/main.py in dev mode).
//   - Communicate via local TCP socket on 127.0.0.1:51337.
//   - Message format: newline-delimited JSON.
//   - Rust → Python: {"cmd": "start_recording"} / {"cmd": "stop_recording"}
//   - Python → Rust: {"event": "transcribed", "text": "..."} / {"event": "state", "value": "..."}
//   - On transcribed: call injector.
//   - On state: update tray.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use log::{error, info, warn};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::{db, injector, tray};

const PYTHON_HOST: &str = "127.0.0.1";
const PYTHON_PORT: u16 = 51337;
const CONNECT_RETRIES: u32 = 20;
const RETRY_DELAY_MS: u64 = 500;

/// Shared write-half of the TCP connection to Python.
pub type PythonSender = Arc<Mutex<Option<TcpStream>>>;

/// Spawn Python sidecar and establish TCP connection.
/// Returns the PythonSender that can be used to send commands.
pub fn start_python_sidecar(app: AppHandle) -> PythonSender {
    let sender: PythonSender = Arc::new(Mutex::new(None));
    let sender_clone = sender.clone();

    thread::spawn(move || {
        // --- 1. Spawn Python process ---
        let (child, spawned) = spawn_sidecar_or_dev(&app);

        let mut child = match child {
            Some(c) => c,
            None => {
                error!("Failed to spawn Python sidecar.");
                notify_error(&app, "Could not start Python AI service.");
                return;
            }
        };

        // Wait for "READY:" line on stdout (skip if we used cmd.spawn directly)
        if spawned {
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    if line.starts_with("READY:") {
                        info!("Python sidecar ready: {}", line);
                        break;
                    }
                }
            }
        } else {
            // Sidecar mode — wait 1s for TCP to be up
            thread::sleep(Duration::from_millis(1000));
        }

        // --- 2. Connect to TCP socket with retries ---
        let mut stream: Option<TcpStream> = None;
        for attempt in 1..=CONNECT_RETRIES {
            match TcpStream::connect((PYTHON_HOST, PYTHON_PORT)) {
                Ok(s) => {
                    info!("Connected to Python sidecar on attempt {}", attempt);
                    stream = Some(s);
                    break;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }

        let stream = match stream {
            Some(s) => s,
            None => {
                error!("Could not connect to Python sidecar after {} retries.", CONNECT_RETRIES);
                notify_error(&app, "AI service not running - dictation unavailable");
                return;
            }
        };

        // Sidecar is connected — emit ready notification
        {
            let hotkey_str = crate::db::get("hotkey");
            let hotkey_str = if hotkey_str.is_empty() { "Ctrl+Alt+Space" } else { &hotkey_str };
            let displayed_hotkey = hotkey_str
                .split('+')
                .map(|k| {
                    let lower = k.trim();
                    let mut c = lower.chars();
                    match c.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" + ");
            let msg = format!("Ready. Hold {} to dictate.", displayed_hotkey);
            if let Some(win) = app.get_webview_window("main") {
                win.emit("vispa-notify", json!({ "body": msg })).ok();
            }
        }

        // Clone for reading
        let reader_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(e) => { error!("Failed to clone TCP stream: {}", e); return; }
        };

        // Store write half in shared sender
        {
            let mut guard = sender_clone.lock().unwrap();
            *guard = Some(stream);
        }

        // --- 3. Read events from Python ---
        let reader = BufReader::new(reader_stream);
        for line in reader.lines().map_while(Result::ok) {
            let line = line.trim().to_string();
            if line.is_empty() { continue; }

            match serde_json::from_str::<Value>(&line) {
                Ok(obj) => handle_python_event(&app, &obj),
                Err(e)  => warn!("Bad JSON from Python: {:?} — {}", line, e),
            }
        }

        info!("Python sidecar connection closed.");
        let mut guard = sender_clone.lock().unwrap();
        *guard = None;
    });

    sender
}

/// Send a JSON command to the Python sidecar.
pub fn send_cmd(sender: &PythonSender, cmd: &str) {
    let mut guard = sender.lock().unwrap();
    if let Some(ref mut stream) = *guard {
        let msg = format!("{}\n", json!({ "cmd": cmd }));
        if let Err(e) = stream.write_all(msg.as_bytes()) {
            error!("Failed to send '{}' to Python: {}", cmd, e);
        }
    } else {
        error!("send_cmd '{}': Python sidecar not connected.", cmd);
    }
}

/// Send a set_config command to Python.
pub fn send_config(sender: &PythonSender, key: &str, value: &str) {
    let mut guard = sender.lock().unwrap();
    if let Some(ref mut stream) = *guard {
        let msg = format!("{}\n", json!({ "cmd": "set_config", "key": key, "value": value }));
        if let Err(e) = stream.write_all(msg.as_bytes()) {
            error!("Failed to send set_config to Python: {}", e);
        }
    }
}

// ---------------------------------------------------------------------------
// Timing instrumentation for end-to-end latency tracking
// ---------------------------------------------------------------------------

fn pending_timing() -> &'static Mutex<Option<(u64, Instant)>> {
    static PENDING: OnceLock<Mutex<Option<(u64, Instant)>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

/// Send a command with a request_id and start the latency timer.
/// Returns the generated request_id.
pub fn send_cmd_with_timing(sender: &PythonSender, cmd: &str) -> u64 {
    let request_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;

    let mut guard = sender.lock().unwrap();
    if let Some(ref mut stream) = *guard {
        let msg = format!("{}\n", json!({ "cmd": cmd, "request_id": request_id.to_string() }));
        if let Err(e) = stream.write_all(msg.as_bytes()) {
            error!("Failed to send '{}' to Python: {}", cmd, e);
        }
    } else {
        error!("send_cmd '{}': Python sidecar not connected.", cmd);
    }

    // Record the start time for this dictation
    if let Ok(mut p) = pending_timing().lock() {
        *p = Some((request_id, Instant::now()));
    }

    request_id
}

/// Log a complete per-dictation timing report.
/// Called when the `transcribed` event arrives from Python.
pub fn log_timing_report(request_id: u64, python_timing: &serde_json::Value) {
    let total_rust = pending_timing().lock().map(|mut p| {
        let elapsed = p.take().map(|(_, t0)| t0.elapsed());
        elapsed
    }).ok().flatten();

    let total_ms = total_rust.map(|d| d.as_secs_f64() * 1000.0);

    info!(
        "=== DICTATION TIMING REPORT tid={} ===",
        request_id
    );

    // Log Python-side stages
    if let Some(stages) = python_timing.as_object() {
        let mut sorted: Vec<_> = stages.iter().collect();
        sorted.sort_by_key(|(k, _)| *k);
        for (key, val) in &sorted {
            if !key.ends_with("_cumulative") {
                info!("  Python {}: {} ms", key, val);
            }
        }
    }

    // Log Rust-side total
    if let Some(ms) = total_ms {
        info!("  Rust total (hotkey → injection): {:.1} ms", ms);
        info!("=== END DICTATION TIMING tid={} total={:.1}ms ===", request_id, ms);
    } else {
        info!("=== END DICTATION TIMING tid={} ===", request_id);
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn handle_python_event(app: &AppHandle, obj: &Value) {
    let event = obj.get("event").and_then(Value::as_str).unwrap_or("");
    match event {
        "transcribed" => {
            let text = obj.get("text").and_then(Value::as_str).unwrap_or("");
            let timing = obj.get("timing");
            // Extract request_id from the timing dict sent back by Python
            let request_id = timing
                .and_then(|t| t.get("request_id"))
                .and_then(Value::as_str)
                .unwrap_or("0")
                .parse::<u64>()
                .unwrap_or(0);

            info!("Received transcribed: {:?} (tid={})", text, request_id);

            if !text.is_empty() {
                let t_inject = Instant::now();
                if let Err(e) = injector::inject_text(text) {
                    warn!("Injection failed: {}", e);
                }
                let inject_ms = t_inject.elapsed().as_secs_f64() * 1000.0;
                info!("TIMING: injection={:.1} ms", inject_ms);
            }

            // Log the complete per-dictation timing report
            if let Some(t) = timing {
                log_timing_report(request_id, t);
            }
            // Also emit to frontend for potential display
            if let Some(win) = app.get_webview_window("main") {
                win.emit("transcribed", text).ok();
            }
        }
        "state" => {
            let value = obj.get("value").and_then(Value::as_str).unwrap_or("idle");
            let new_state = match value {
                "recording"    => tray::TrayState::Recording,
                "transcribing" => tray::TrayState::Transcribing,
                _              => tray::TrayState::Idle,
            };
            tray::set_state(app, new_state);
        }
        "error" => {
            let msg = obj.get("message").and_then(Value::as_str).unwrap_or("Unknown error");
            warn!("Python error: {}", msg);
            notify_error(app, msg);
        }
        "pong" => {
            info!("Python pong received.");
        }
        _ => {
            warn!("Unknown event from Python: {:?}", obj);
        }
    }
}

/// Try spawning the bundled PyInstaller sidecar first, fall back to dev mode.
/// Returns (child, is_dev_spawn) where is_dev_spawn=true means we need to read stdout.
fn spawn_sidecar_or_dev(app: &AppHandle) -> (Option<Child>, bool) {
    // 1. Try bundled sidecar
    let sidecar_path = find_sidecar_exe();
    if std::path::Path::new(&sidecar_path).exists() {
        info!("Using bundled sidecar: {}", sidecar_path.display());
        let settings = db::load_all();
        let mut cmd = Command::new(&sidecar_path);
        cmd.stdout(Stdio::piped()).stderr(Stdio::inherit());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000);

        for (key, value) in &settings {
            let env_key = match key.as_str() {
                "model_override" => Some("MODEL_OVERRIDE"),
                "use_cloud"      => Some("USE_CLOUD"),
                "cloud_provider" => Some("CLOUD_PROVIDER"),
                "cloud_api_key"  => Some("CLOUD_API_KEY"),
                "language"       => Some("LANGUAGE"),
                "remove_fillers" => Some("REMOVE_FILLERS"),
                "warm_seconds"   => Some("WARM_SECONDS"),
                _                => None,
            };
            if let Some(ek) = env_key {
                cmd.env(ek, value);
            }
        }

        match cmd.spawn() {
            Ok(c) => return (Some(c), false),
            Err(e) => error!("Failed to spawn bundled sidecar: {}", e),
        }
    }

    // 2. Fall back to dev mode
    let python_exe = find_python_exe();
    let script = find_python_script();

    info!("Spawning Python sidecar (dev): {} {}", python_exe, script);

    let settings = db::load_all();
    let mut cmd = Command::new(&python_exe);
    cmd.arg(&script).stdout(Stdio::piped()).stderr(Stdio::inherit());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);

    for (key, value) in &settings {
        let env_key = match key.as_str() {
            "model_override" => Some("MODEL_OVERRIDE"),
            "use_cloud"      => Some("USE_CLOUD"),
            "cloud_provider" => Some("CLOUD_PROVIDER"),
            "cloud_api_key"  => Some("CLOUD_API_KEY"),
            "language"       => Some("LANGUAGE"),
            "remove_fillers" => Some("REMOVE_FILLERS"),
            "warm_seconds"   => Some("WARM_SECONDS"),
            _                => None,
        };
        if let Some(ek) = env_key {
            cmd.env(ek, value);
        }
    }

    match cmd.spawn() {
        Ok(c) => (Some(c), true),
        Err(e) => {
            error!("Failed to spawn Python sidecar: {}", e);
            notify_error(app, &format!("Could not start Python AI service: {}", e));
            (None, true)
        }
    }
}

/// Find the bundled sidecar executable.
fn find_sidecar_exe() -> std::path::PathBuf {
    let triple = "x86_64-pc-windows-msvc";
    let exe_name = format!("vispa-python-{}.exe", triple);

    // Tauri bundles externalBin next to the main exe in the same directory
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let candidate = dir.join(&exe_name);
            if candidate.exists() {
                return candidate;
            }
        }
    }

    // Dev fallback: check src-tauri/binaries/
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = vec![
        cwd.join(format!("binaries/{}", exe_name)),
        cwd.join(format!("../binaries/{}", exe_name)),
    ];
    for p in candidates {
        if p.exists() {
            return p;
        }
    }

    exe_name.into()
}

fn find_python_exe() -> String {
    // In dev: prefer py launcher, then specific versions, then python
    for candidate in &["py", "python3.11", "python3.14", "python3", "python"] {
        if which_exists(candidate) {
            return candidate.to_string();
        }
    }
    // Try absolute paths for common Windows install locations
    let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let candidates = vec![
        format!(r"{}\Programs\Python\Python311\python.exe", localappdata),
        format!(r"{}\Programs\Python\Python314\python.exe", localappdata),
        format!(r"{}\Programs\Python\Python313\python.exe", localappdata),
        format!(r"{}\Programs\Python\Python312\python.exe", localappdata),
        r"C:\Python311\python.exe".to_string(),
        r"C:\Python314\python.exe".to_string(),
    ];
    for path in candidates {
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }
    "python".to_string()
}

fn find_python_script() -> String {
    // Resolve relative to the Tauri project root
    let cwd = std::env::current_dir().unwrap_or_default();
    // During tauri dev, cwd is src-tauri/; go up one level
    let candidates = vec![
        cwd.join("../python/main.py"),
        cwd.join("python/main.py"),
        std::path::PathBuf::from("python/main.py"),
    ];
    for p in candidates {
        if let Ok(canon) = p.canonicalize() {
            return canon.to_string_lossy().to_string();
        }
    }
    "python/main.py".to_string()
}

fn which_exists(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

fn notify_error(app: &AppHandle, msg: &str) {
    if let Some(win) = app.get_webview_window("main") {
        win.emit("vispa-notify", json!({ "body": msg })).ok();
    }
}
