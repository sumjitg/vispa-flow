# Taste (Continuously Learned by [CommandCode][cmd])

[cmd]: https://commandcode.ai/


# python
- Use `compute_type = "int8"` for faster-whisper instead of `"auto"`, which is unsupported and crashes on CPU-only machines. Confidence: 0.70
- Keep stdout completely clean before the READY handshake line: defer all logging setup until after socket bind, print READY only on success, and redirect all subsequent logging to file only. Guard against None stdout/stderr at the top of the file for Tauri-spawned processes. Confidence: 0.85

# tauri
- For Python sidecar processes on Windows, use Rust `std::process::Command` with `Stdio::piped()` instead of `tauri-plugin-shell`. The Tauri shell plugin wraps processes in a way that gives Python an invalid stdout file descriptor, causing `[Errno 22] Invalid argument` on any stdout write. Confidence: 0.70
