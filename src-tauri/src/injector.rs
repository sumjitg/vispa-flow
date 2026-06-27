// injector.rs — Text injection into the active window via clipboard + Ctrl+V
//
// Per agent.md:
//   - Write text to clipboard, then simulate Ctrl+V into the focused window.
//   - Windows API: SetClipboardData / GetClipboardData + SendInput.
//   - Wait 50ms before simulating paste to allow focus to settle.
//   - Save clipboard content before injection, restore it after.

use std::time::Duration;
use log::{info, warn};

/// Inject `text` into the currently focused window using clipboard + Ctrl+V.
pub fn inject_text(text: &str) -> Result<(), String> {
    info!("Injecting {} chars into active window.", text.len());

    #[cfg(target_os = "windows")]
    {
        let saved = clipboard_get_text();
        clipboard_set_text(text)?;
        std::thread::sleep(Duration::from_millis(50));
        send_ctrl_v()?;
        std::thread::sleep(Duration::from_millis(100));
        if let Some(prev) = saved {
            if let Err(e) = clipboard_set_text(&prev) {
                warn!("Could not restore clipboard: {}", e);
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        warn!("inject_text: non-Windows platform — no-op.");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Windows clipboard helpers — using winapi-style raw calls to avoid
// version conflicts with the `windows` crate that Tauri already pulls in.
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn clipboard_get_text() -> Option<String> {
    use std::ptr;
    unsafe {
        // OpenClipboard(NULL)
        if open_clipboard_raw() == 0 {
            return None;
        }

        // CF_UNICODETEXT = 13
        let handle = get_clipboard_data_raw(13);
        let result = if !handle.is_null() {
            let ptr = global_lock_raw(handle) as *const u16;
            if !ptr.is_null() {
                // Find null terminator
                let mut len = 0usize;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr, len);
                let text = String::from_utf16_lossy(slice);
                global_unlock_raw(handle);
                Some(text)
            } else {
                None
            }
        } else {
            None
        };

        close_clipboard_raw();
        result
    }
}

#[cfg(target_os = "windows")]
fn clipboard_set_text(text: &str) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ffi::OsStr;

    unsafe {
        let wide: Vec<u16> = OsStr::new(text)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect();
        let byte_len = wide.len() * 2;

        // GMEM_MOVEABLE = 0x0002
        let hmem = global_alloc_raw(0x0002, byte_len);
        if hmem.is_null() {
            return Err("GlobalAlloc failed".into());
        }

        let dst = global_lock_raw(hmem) as *mut u16;
        if dst.is_null() {
            global_free_raw(hmem);
            return Err("GlobalLock failed".into());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), dst, wide.len());
        global_unlock_raw(hmem);

        if open_clipboard_raw() == 0 {
            global_free_raw(hmem);
            return Err("OpenClipboard failed".into());
        }

        empty_clipboard_raw();
        // CF_UNICODETEXT = 13
        let result = set_clipboard_data_raw(13, hmem);
        close_clipboard_raw();

        if result.is_null() {
            return Err("SetClipboardData failed".into());
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Simulate Ctrl+V via SendInput (pure Windows API, no crate dependency)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn send_ctrl_v() -> Result<(), String> {
    unsafe {
        // INPUT structure layout for keyboard events
        // We use raw Windows API via extern "system" to avoid crate version conflicts
        #[repr(C)]
        struct KeybdInput {
            r#type: u32,    // INPUT_KEYBOARD = 1
            wVk: u16,
            wScan: u16,
            dwFlags: u32,
            time: u32,
            dwExtraInfo: usize,
            _padding: [u8; 8], // Alignment padding (INPUT union is 40 bytes on x64)
        }

        // VK_CONTROL = 0x11, VK_V = 0x56
        // KEYEVENTF_KEYUP = 0x0002
        let inputs: [KeybdInput; 4] = [
            KeybdInput { r#type: 1, wVk: 0x11, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0, _padding: [0; 8] },
            KeybdInput { r#type: 1, wVk: 0x56, wScan: 0, dwFlags: 0, time: 0, dwExtraInfo: 0, _padding: [0; 8] },
            KeybdInput { r#type: 1, wVk: 0x56, wScan: 0, dwFlags: 0x0002, time: 0, dwExtraInfo: 0, _padding: [0; 8] },
            KeybdInput { r#type: 1, wVk: 0x11, wScan: 0, dwFlags: 0x0002, time: 0, dwExtraInfo: 0, _padding: [0; 8] },
        ];

        let sent = send_input_raw(4, inputs.as_ptr() as *const _, std::mem::size_of::<KeybdInput>() as i32);
        if sent != 4 {
            return Err(format!("SendInput sent {}/4 events", sent));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Raw Windows API bindings (avoids version conflicts with the `windows` crate)
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
    fn SendInput(cInputs: u32, pInputs: *const std::ffi::c_void, cbSize: i32) -> u32;
}

#[cfg(target_os = "windows")]
unsafe fn open_clipboard_raw() -> i32 { OpenClipboard(std::ptr::null_mut()) }
#[cfg(target_os = "windows")]
unsafe fn close_clipboard_raw() { CloseClipboard(); }
#[cfg(target_os = "windows")]
unsafe fn empty_clipboard_raw() { EmptyClipboard(); }
#[cfg(target_os = "windows")]
unsafe fn get_clipboard_data_raw(fmt: u32) -> *mut std::ffi::c_void { GetClipboardData(fmt) }
#[cfg(target_os = "windows")]
unsafe fn set_clipboard_data_raw(fmt: u32, h: *mut std::ffi::c_void) -> *mut std::ffi::c_void { SetClipboardData(fmt, h) }
#[cfg(target_os = "windows")]
unsafe fn global_alloc_raw(flags: u32, size: usize) -> *mut std::ffi::c_void { GlobalAlloc(flags, size) }
#[cfg(target_os = "windows")]
unsafe fn global_lock_raw(h: *mut std::ffi::c_void) -> *mut std::ffi::c_void { GlobalLock(h) }
#[cfg(target_os = "windows")]
unsafe fn global_unlock_raw(h: *mut std::ffi::c_void) { GlobalUnlock(h); }
#[cfg(target_os = "windows")]
unsafe fn global_free_raw(h: *mut std::ffi::c_void) { GlobalFree(h); }
#[cfg(target_os = "windows")]
unsafe fn send_input_raw(n: u32, p: *const std::ffi::c_void, sz: i32) -> u32 { SendInput(n, p, sz) }
