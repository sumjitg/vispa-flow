import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------
interface Settings {
  hotkey: string;
  model_override: string;
  language: string;
  remove_fillers: string;
  use_cloud: string;
  cloud_provider: string;
  cloud_api_key: string;
}

const DEFAULT_SETTINGS: Settings = {
  hotkey:          "ctrl+alt+space",
  model_override:  "",
  language:        "auto",
  remove_fillers:  "true",
  use_cloud:       "false",
  cloud_provider:  "groq",
  cloud_api_key:   "",
};

const SETTING_KEYS = Object.keys(DEFAULT_SETTINGS) as (keyof Settings)[];

const MODEL_OPTIONS = [
  { value: "",          label: "Auto (RAM-based)" },
  { value: "tiny.en",  label: "tiny.en  (~75MB)" },
  { value: "tiny",     label: "tiny     (~75MB)" },
  { value: "base",     label: "base     (~145MB)" },
  { value: "small",    label: "small    (~465MB)" },
];

const LANGUAGE_OPTIONS = [
  { value: "auto", label: "Auto-detect" },
  { value: "en",   label: "English" },
  { value: "hi",   label: "Hindi" },
  { value: "es",   label: "Spanish" },
  { value: "fr",   label: "French" },
  { value: "de",   label: "German" },
  { value: "zh",   label: "Chinese" },
  { value: "ja",   label: "Japanese" },
  { value: "ko",   label: "Korean" },
  { value: "pt",   label: "Portuguese" },
  { value: "ru",   label: "Russian" },
  { value: "ar",   label: "Arabic" },
];

// ---------------------------------------------------------------------------
// Toggle component
// ---------------------------------------------------------------------------
function Toggle({
  value,
  onChange,
  id,
}: {
  value: boolean;
  onChange: (v: boolean) => void;
  id: string;
}) {
  return (
    <button
      id={id}
      role="switch"
      aria-checked={value}
      onClick={() => onChange(!value)}
      className={`toggle ${value ? "on" : "off"}`}
    >
      <span className="toggle-thumb" />
    </button>
  );
}

// ---------------------------------------------------------------------------
// Main Settings page
// ---------------------------------------------------------------------------
export default function Settings() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [dirty, setDirty]       = useState<Partial<Settings>>({});
  const [saving, setSaving]     = useState(false);
  const [saved, setSaved]       = useState(false);
  const [error, setError]       = useState<string | null>(null);
  const [captureMode, setCaptureMode] = useState(false);

  // Load all settings on mount
  useEffect(() => {
    const load = async () => {
      const loaded: Partial<Settings> = {};
      await Promise.all(
        SETTING_KEYS.map(async (key) => {
          try {
            const val = await invoke<string>("get_setting", { key });
            (loaded as any)[key] = val;
          } catch {
            // Keep default
          }
        })
      );
      setSettings((prev) => ({ ...prev, ...loaded }));
    };
    load();
  }, []);

  const update = useCallback((key: keyof Settings, value: string) => {
    setSettings((prev) => ({ ...prev, [key]: value }));
    setDirty((prev) => ({ ...prev, [key]: value }));
    setSaved(false);
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      await Promise.all(
        (Object.entries(dirty) as [keyof Settings, string][]).map(([key, value]) =>
          invoke("cmd_apply_setting", { key, value })
        )
      );
      setDirty({});
      setSaved(true);
      setTimeout(() => setSaved(false), 2500);
    } catch (e: any) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  // Hotkey capture
  const handleHotkeyCapture = () => {
    setCaptureMode(true);
  };

  useEffect(() => {
    if (!captureMode) return;

    const listener = (e: KeyboardEvent) => {
      // Allow Escape to cancel
      if (e.code === "Escape") {
        e.preventDefault();
        setCaptureMode(false);
        return;
      }
      e.preventDefault();
      const parts: string[] = [];
      if (e.ctrlKey)  parts.push("ctrl");
      if (e.altKey)   parts.push("alt");
      if (e.shiftKey) parts.push("shift");
      if (e.metaKey)  parts.push("meta");
      const key = e.code.replace("Key", "").replace("Digit", "").toLowerCase();
      if (!["controlleft","controlright","altleft","altright","shiftleft","shiftright","metaleft","metaright"].includes(e.code.toLowerCase())) {
        parts.push(key);
        update("hotkey", parts.join("+"));
        setCaptureMode(false);
      }
    };

    // 10-second timeout for capture mode
    const timeout = setTimeout(() => setCaptureMode(false), 10_000);

    window.addEventListener("keydown", listener);
    return () => {
      clearTimeout(timeout);
      window.removeEventListener("keydown", listener);
    };
  }, [captureMode, update]);

  const cloudEnabled = settings.use_cloud === "true";

  return (
    <div className="min-h-screen bg-surface-900 flex flex-col">
      {/* Header */}
      <header className="px-6 pt-6 pb-4 border-b border-white/5">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-brand-600 flex items-center justify-center text-white text-sm font-bold">V</div>
          <div>
            <h1 className="text-base font-semibold text-white">Vispa Flow Settings</h1>
            <p className="text-xs text-gray-500">Changes apply immediately on save</p>
          </div>
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-5 space-y-5">

        {/* Hotkey */}
        <div>
          <label className="field-label" htmlFor="hotkey-display">Hotkey</label>
          <div className="flex gap-2 items-center">
            <div
              id="hotkey-display"
              className={`flex-1 px-3 py-2.5 rounded-lg bg-surface-700 border text-sm font-mono
                ${captureMode
                  ? "border-brand-500/70 text-brand-300 animate-pulse"
                  : "border-white/10 text-gray-200"}`}
            >
              {captureMode ? "Press your hotkey combination…" : settings.hotkey}
            </div>
            <button
              id="hotkey-change-btn"
              onClick={handleHotkeyCapture}
              className="btn-secondary"
            >
              {captureMode ? "Cancel" : "Change"}
            </button>
          </div>
        </div>

        {/* Model */}
        <div>
          <label className="field-label" htmlFor="model-select">Whisper Model</label>
          <select
            id="model-select"
            className="field-select"
            value={settings.model_override}
            onChange={(e) => update("model_override", e.target.value)}
          >
            {MODEL_OPTIONS.map((o) => (
              <option key={o.value} value={o.value} className="bg-surface-800">
                {o.label}
              </option>
            ))}
          </select>
        </div>

        {/* Language */}
        <div>
          <label className="field-label" htmlFor="language-select">Language</label>
          <select
            id="language-select"
            className="field-select"
            value={settings.language}
            onChange={(e) => update("language", e.target.value)}
          >
            {LANGUAGE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value} className="bg-surface-800">
                {o.label}
              </option>
            ))}
          </select>
        </div>

        {/* Remove fillers */}
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-200">Remove filler words</p>
            <p className="text-xs text-gray-500">Strips um, uh, you know, basically…</p>
          </div>
          <Toggle
            id="remove-fillers-toggle"
            value={settings.remove_fillers === "true"}
            onChange={(v) => update("remove_fillers", v ? "true" : "false")}
          />
        </div>

        <div className="section-divider" />

        {/* Cloud STT section */}
        <p className="text-xs font-semibold text-gray-500 uppercase tracking-widest">Cloud API</p>

        {/* Enable cloud */}
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium text-gray-200">Enable cloud STT</p>
            <p className="text-xs text-gray-500">Use Groq or OpenAI instead of local model</p>
          </div>
          <Toggle
            id="cloud-toggle"
            value={cloudEnabled}
            onChange={(v) => update("use_cloud", v ? "true" : "false")}
          />
        </div>

        {/* Provider — shown only when cloud is enabled */}
        {cloudEnabled && (
          <>
            <div>
              <label className="field-label" htmlFor="provider-select">Provider</label>
              <select
                id="provider-select"
                className="field-select"
                value={settings.cloud_provider}
                onChange={(e) => update("cloud_provider", e.target.value)}
              >
                <option value="groq"   className="bg-surface-800">Groq (recommended)</option>
                <option value="openai" className="bg-surface-800">OpenAI</option>
              </select>
            </div>

            <div>
              <label className="field-label" htmlFor="api-key-input">API Key</label>
              <input
                id="api-key-input"
                type="password"
                className="field-input"
                placeholder="sk-…"
                value={settings.cloud_api_key}
                onChange={(e) => update("cloud_api_key", e.target.value)}
                autoComplete="off"
                spellCheck={false}
              />
              <p className="text-xs text-gray-600 mt-1">
                {settings.cloud_provider === "groq"
                  ? "Get a free key at console.groq.com"
                  : "Get a key at platform.openai.com"}
              </p>
            </div>
          </>
        )}
      </div>

      {/* Footer */}
      <footer className="px-6 pb-6 pt-4 border-t border-white/5">
        {error && (
          <p className="text-xs text-red-400 mb-3">{error}</p>
        )}
        <div className="flex items-center gap-3">
          <button
            id="save-btn"
            className="btn-primary flex-1"
            onClick={handleSave}
            disabled={saving || Object.keys(dirty).length === 0}
          >
            {saving ? "Saving…" : saved ? "✓ Saved" : "Save"}
          </button>
          {Object.keys(dirty).length > 0 && (
            <p className="text-xs text-amber-400">{Object.keys(dirty).length} unsaved change{Object.keys(dirty).length > 1 ? "s" : ""}</p>
          )}
        </div>
      </footer>
    </div>
  );
}
