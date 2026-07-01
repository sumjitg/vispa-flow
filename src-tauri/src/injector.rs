// injector.rs — Text injection into the active window via clipboard + Ctrl+V
//
// Per agent.md:
//   - Write text to clipboard, then simulate Ctrl+V into the focused window.
//   - Windows API: SetClipboardData / GetClipboardData + SendInput.
//   - Wait 50ms before simulating paste to allow focus to settle.
//   - Save clipboard content before injection, restore it after.

use std::time::{Duration, Instant};
use log::{info, warn};

/// Inject `text` into the currently focused window using clipboard + Ctrl+V.
pub fn inject_text(text: &str) -> Result<(), String> {
    info!("Injecting {} chars into active window.", text.len());

    #[cfg(target_os = "windows")]
    {
        let t0 = Instant::now();

        let saved = clipboard_get_text();
        let t_clip_save = t0.elapsed();

        clipboard_set_text(text)?;
        let t_clip_set = t0.elapsed();

        // Release all modifier keys first, then allow focus to settle
        release_modifiers();
        std::thread::sleep(Duration::from_millis(100));
        send_ctrl_v()?;
        let t_sendinput = t0.elapsed();

        std::thread::sleep(Duration::from_millis(150));
        if let Some(prev) = saved {
            if let Err(e) = clipboard_set_text(&prev) {
                warn!("Could not restore clipboard: {}", e);
            }
        }
        let t_total = t0.elapsed();

        info!(
            "TIMING: injection clip_save={:.1}ms clip_set={:.1}ms sendinput={:.1}ms total={:.1}ms",
            t_clip_save.as_secs_f64() * 1000.0,
            t_clip_set.as_secs_f64() * 1000.0,
            t_sendinput.as_secs_f64() * 1000.0,
            t_total.as_secs_f64() * 1000.0,
        );

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("inject_text: non-Windows platform — no-op.");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Windows clipboard helpers (raw Win32 FFI — safe, minimal, no crate conflicts)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn clipboard_get_text() -> Option<String> {
    unsafe {
        if open_clipboard_raw() == 0 {
            return None;
        }
        let handle = get_clipboard_data_raw(13); // CF_UNICODETEXT
        let result = if !handle.is_null() {
            let ptr = global_lock_raw(handle) as *const u16;
            if !ptr.is_null() {
                let mut len = 0usize;
                while *ptr.add(len) != 0 { len += 1; }
                let slice = std::slice::from_raw_parts(ptr, len);
                let text = String::from_utf16_lossy(slice);
                global_unlock_raw(handle);
                Some(text)
            } else { None }
        } else { None };
        close_clipboard_raw();
        result
    }
}

#[cfg(target_os = "windows")]
fn clipboard_set_text(text: &str) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    unsafe {
        let wide: Vec<u16> = OsStr::new(text).encode_wide().chain(std::iter::once(0u16)).collect();
        let byte_len = wide.len() * 2;
        let hmem = global_alloc_raw(0x0002, byte_len); // GMEM_MOVEABLE
        if hmem.is_null() { return Err("GlobalAlloc failed".into()); }
        let dst = global_lock_raw(hmem) as *mut u16;
        if dst.is_null() { global_free_raw(hmem); return Err("GlobalLock failed".into()); }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        global_unlock_raw(hmem);
        if open_clipboard_raw() == 0 { global_free_raw(hmem); return Err("OpenClipboard failed".into()); }
        empty_clipboard_raw();
        let result = set_clipboard_data_raw(13, hmem);
        close_clipboard_raw();
        if result.is_null() { return Err("SetClipboardData failed".into()); }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Ctrl+V via SendInput using the windows crate (already a dependency)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn release_modifiers() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN, VK_SHIFT,
    };

    let mut inputs = Vec::new();
    for &vk in &[VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN] {
        let mut ki = KEYBDINPUT::default();
        ki.wVk = vk;
        ki.dwFlags = KEYEVENTF_KEYUP;
        inputs.push(INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki },
        });
    }
    unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32); }
}

#[cfg(target_os = "windows")]
fn send_ctrl_v() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };

    // Key down: Ctrl
    let mut ctrl_down = KEYBDINPUT::default();
    ctrl_down.wVk = VK_CONTROL;
    // Key down: V
    let mut v_down = KEYBDINPUT::default();
    v_down.wVk = VK_V;
    // Key up: V
    let mut v_up = KEYBDINPUT::default();
    v_up.wVk = VK_V;
    v_up.dwFlags = KEYEVENTF_KEYUP;
    // Key up: Ctrl
    let mut ctrl_up = KEYBDINPUT::default();
    ctrl_up.wVk = VK_CONTROL;
    ctrl_up.dwFlags = KEYEVENTF_KEYUP;

    let inputs = [
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: ctrl_down } },
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: v_down } },
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: v_up } },
        INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: ctrl_up } },
    ];

    let size = std::mem::size_of::<INPUT>() as i32;
    unsafe {
        let sent = SendInput(&inputs, size);
        if sent != 4 {
            return Err(format!("SendInput sent {}/4 events", sent));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Raw Win32 clipboard FFI
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
extern "system" {
    fn OpenClipboard(hWndNewOwner: *mut std::ffi::c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(uFormat: u32) -> *mut std::ffi::c_void;
    fn SetClipboardData(uFormat: u32, hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut std::ffi::c_void;
    fn GlobalLock(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(hMem: *mut std::ffi::c_void) -> i32;
    fn GlobalFree(hMem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
unsafe fn open_clipboard_raw() -> i32 { OpenClipboard(std::ptr::null_mut()) }
#[cfg(target_os = "windows")]
unsafe fn close_clipboard_raw() { CloseClipboard(); }
#[cfg(target_os = "windows")]
unsafe fn empty_clipboard_raw() { EmptyClipboard(); }
#[cfg(target_os = "windows")]
unsafe fn get_clipboard_data_raw(f: u32) -> *mut std::ffi::c_void { GetClipboardData(f) }
#[cfg(target_os = "windows")]
unsafe fn set_clipboard_data_raw(f: u32, h: *mut std::ffi::c_void) -> *mut std::ffi::c_void { SetClipboardData(f, h) }
#[cfg(target_os = "windows")]
unsafe fn global_alloc_raw(f: u32, s: usize) -> *mut std::ffi::c_void { GlobalAlloc(f, s) }
#[cfg(target_os = "windows")]
unsafe fn global_lock_raw(h: *mut std::ffi::c_void) -> *mut std::ffi::c_void { GlobalLock(h) }
#[cfg(target_os = "windows")]
unsafe fn global_unlock_raw(h: *mut std::ffi::c_void) { GlobalUnlock(h); }
#[cfg(target_os = "windows")]
unsafe fn global_free_raw(h: *mut std::ffi::c_void) { GlobalFree(h); }
