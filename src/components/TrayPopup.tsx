import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

type TrayStateValue = "idle" | "recording" | "transcribing";

const STATE_CONFIG: Record<
  TrayStateValue,
  { label: string; sub: string; grad: string; ring: string; bars: boolean }
> = {
  idle:         { label: "Ready",         sub: "Standing by",     grad: "bg-idle-gradient",  ring: "shadow-glow-accent", bars: false },
  recording:    { label: "Recording",     sub: "Listening now…",  grad: "bg-rec-gradient",   ring: "shadow-glow-rec",    bars: true  },
  transcribing: { label: "Transcribing",  sub: "Cleaning text…",  grad: "bg-trans-gradient", ring: "shadow-glow-brand",  bars: true  },
};

// Per-bar heights + delays for the live waveform
const WAVE_BARS = [
  { h: 40, d: "0ms" },
  { h: 75, d: "120ms" },
  { h: 100, d: "60ms" },
  { h: 60, d: "200ms" },
  { h: 90, d: "90ms" },
  { h: 45, d: "160ms" },
  { h: 70, d: "30ms" },
];

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
  const keyCaps = hotkey
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1));

  return (
    <div className="relative min-h-screen overflow-hidden bg-surface-900 rounded-2xl border border-white/10 shadow-soft">
      {/* Ambient background */}
      <div className="aurora-bg" />

      <div className="relative z-10 flex flex-col gap-3.5 p-4">
        {/* Status row */}
        <div className="flex items-center gap-3">
          {/* Live orb — pulses & shows waveform when active */}
          <div className="relative flex h-11 w-11 items-center justify-center">
            {cfg.bars && (
              <span className={`absolute inset-0 rounded-2xl ${cfg.grad} animate-ring-pulse`} />
            )}
            <div className={`relative flex h-11 w-11 items-center justify-center rounded-2xl ${cfg.grad} ${cfg.ring}`}>
              {cfg.bars ? (
                <div className="flex h-4 items-center gap-[2px]">
                  {WAVE_BARS.slice(0, 5).map((b, i) => (
                    <span
                      key={i}
                      className="wave-bar bg-white animate-wave"
                      style={{ height: `${b.h}%`, animationDelay: b.d }}
                    />
                  ))}
                </div>
              ) : (
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 2a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                  <path d="M19 10v1a7 7 0 0 1-14 0v-1M12 18v4" />
                </svg>
              )}
            </div>
          </div>

          <div className="min-w-0">
            <p className="text-sm font-bold text-white leading-tight">{cfg.label}</p>
            <p className="text-xs text-gray-400">{cfg.sub}</p>
          </div>

          {/* Brand tag */}
          <span className="ml-auto text-[10px] font-bold uppercase tracking-[0.14em] text-transparent bg-clip-text bg-brand-gradient">
            VoiceType
          </span>
        </div>

        {/* Hotkey reminder */}
        <div className="rounded-xl bg-white/[0.04] border border-white/10 px-3 py-2.5">
          <p className="text-[10px] uppercase tracking-wider text-gray-500 mb-1.5">Hold to dictate</p>
          <div className="flex items-center gap-1.5">
            {keyCaps.map((cap, i) => (
              <span key={i} className="flex items-center gap-1.5">
                {i > 0 && <span className="text-gray-600 text-xs">+</span>}
                <kbd className="px-2 py-0.5 rounded-md bg-white/[0.06] border border-white/10 font-mono text-xs font-semibold text-brand-200">
                  {cap}
                </kbd>
              </span>
            ))}
          </div>
        </div>

        {/* Quick link */}
        <button
          id="popup-settings-link"
          onClick={openSettings}
          className="group flex items-center gap-2 rounded-xl px-3 py-2 text-xs font-medium text-gray-400 transition-all hover:bg-white/[0.05] hover:text-brand-300"
        >
          <svg className="transition-transform duration-500 group-hover:rotate-90" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3" />
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1Z" />
          </svg>
          Open Settings
          <svg className="ml-auto opacity-0 -translate-x-1 transition-all group-hover:opacity-100 group-hover:translate-x-0" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M5 12h14M13 6l6 6-6 6" />
          </svg>
        </button>
      </div>
    </div>
  );
}
