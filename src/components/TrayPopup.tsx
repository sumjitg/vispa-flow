import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type TrayStateValue = "idle" | "recording" | "transcribing";

const STATE_CONFIG: Record<TrayStateValue, { label: string; color: string }> = {
  idle:         { label: "Ready",         color: "gray" },
  recording:    { label: "Recording…",    color: "red" },
  transcribing: { label: "Transcribing…", color: "amber" },
};

export default function TrayPopup() {
  const [state, setState]    = useState<TrayStateValue>("idle");
  const [hotkey, setHotkey]  = useState("ctrl+alt+space");

  useEffect(() => {
    // Load hotkey from settings
    invoke<string>("get_setting", { key: "hotkey" })
      .then((v) => setHotkey(v || "ctrl+alt+space"))
      .catch(() => {});

    // Listen for tray-state events from Rust
    const unlisten = listen<string>("tray-state", (e) => {
      setState((e.payload as TrayStateValue) || "idle");
    });

    // Close on Escape
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);

    // Close on outside click (blur)
    const onBlur = () => {
      setTimeout(() => {
        if (!document.hasFocus()) getCurrentWindow().hide();
      }, 200);
    };
    window.addEventListener("blur", onBlur);

    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  const openSettings = useCallback(() => {
    invoke("cmd_apply_setting", { key: "__open_settings__", value: "" }).catch(() => {});
    // Tauri will handle this via the tray menu — just route to settings window
    getCurrentWindow().hide();
  }, []);

  const cfg = STATE_CONFIG[state];

  return (
    <div
      className="min-h-screen bg-surface-800 rounded-xl border border-white/10 shadow-2xl p-4 flex flex-col gap-4"
      style={{ cursor: "default" }}
    >
      {/* Status row */}
      <div className="flex items-center gap-3">
        <span className={`status-dot ${state}`} />
        <div>
          <p className="text-sm font-semibold text-white">{cfg.label}</p>
          <p className="text-xs text-gray-500">Vispa Flow</p>
        </div>
      </div>

      {/* Hotkey reminder */}
      <div className="rounded-lg bg-surface-700 border border-white/5 px-3 py-2">
        <p className="text-xs text-gray-500 mb-1">Hold to dictate</p>
        <p className="text-xs font-mono text-brand-400 font-semibold tracking-wide">
          {hotkey.split("+").map((k) => k.charAt(0).toUpperCase() + k.slice(1)).join(" + ")}
        </p>
      </div>

      {/* Quick link */}
      <button
        id="popup-settings-link"
        onClick={openSettings}
        className="text-xs text-gray-500 hover:text-brand-400 transition-colors text-left"
      >
        ⚙ Open Settings
      </button>
    </div>
  );
}
