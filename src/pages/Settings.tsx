import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  custom_dictionary: string;
}

const DEFAULT_SETTINGS: Settings = {
  hotkey:          "ctrl+alt+space",
  model_override:  "",
  language:        "auto",
  remove_fillers:  "true",
  use_cloud:       "false",
  cloud_provider:  "groq",
  cloud_api_key:   "",
  custom_dictionary: "",
};

// Parsed shape of the custom_dictionary JSON setting
interface DictReplacement { from: string; to: string; }
interface Dict { words: string[]; replacements: DictReplacement[]; }

const EMPTY_DICT: Dict = { words: [], replacements: [] };

function parseDict(raw: string): Dict {
  if (!raw) return EMPTY_DICT;
  try {
    const d = JSON.parse(raw);
    return {
      words: Array.isArray(d.words) ? d.words.filter((w: unknown) => typeof w === "string") : [],
      replacements: Array.isArray(d.replacements)
        ? d.replacements.filter((r: any) => r && typeof r.from === "string" && typeof r.to === "string")
        : [],
    };
  } catch {
    return EMPTY_DICT;
  }
}

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

type TrayStateValue = "idle" | "recording" | "transcribing";

const LIVE_STATE: Record<TrayStateValue, { label: string; grad: string; ring: string }> = {
  idle:         { label: "Ready to dictate", grad: "bg-idle-gradient",  ring: "shadow-glow-accent" },
  recording:    { label: "Recording…",       grad: "bg-rec-gradient",   ring: "shadow-glow-rec" },
  transcribing: { label: "Transcribing…",    grad: "bg-trans-gradient", ring: "shadow-glow-brand" },
};

// ---------------------------------------------------------------------------
// Small presentational helpers
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

function IconChip({ gradient, children }: { gradient: string; children: React.ReactNode }) {
  return <div className={`icon-chip ${gradient}`}>{children}</div>;
}

// Inline SVG icons (no external deps)
const Icons = {
  keyboard: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" />
    </svg>
  ),
  cpu: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="5" y="5" width="14" height="14" rx="2" /><rect x="9" y="9" width="6" height="6" /><path d="M9 2v2M15 2v2M9 20v2M15 20v2M2 9h2M2 15h2M20 9h2M20 15h2" />
    </svg>
  ),
  globe: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" /><path d="M2 12h20M12 2a15 15 0 0 1 0 20 15 15 0 0 1 0-20Z" />
    </svg>
  ),
  sparkle: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 3v4M12 17v4M3 12h4M17 12h4M6 6l2 2M16 16l2 2M18 6l-2 2M8 16l-2 2" />
    </svg>
  ),
  power: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2v10" /><path d="M18.4 6.6a9 9 0 1 1-12.8 0" />
    </svg>
  ),
  cloud: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M17.5 19a4.5 4.5 0 0 0 .5-9 6 6 0 0 0-11.5-1.5A4 4 0 0 0 6.5 19h11Z" />
    </svg>
  ),
  key: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="7.5" cy="15.5" r="4.5" /><path d="M10.7 12.3 21 2M17 6l3 3M15 8l2 2" />
    </svg>
  ),
  book: (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" /><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2Z" />
    </svg>
  ),
};

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
  const [autostart, setAutostart]     = useState(true);
  const [liveState, setLiveState]     = useState<TrayStateValue>("idle");
  const [newWord, setNewWord]         = useState("");
  const [newFrom, setNewFrom]         = useState("");
  const [newTo, setNewTo]             = useState("");

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

      // Load autostart state from DB
      try {
        const val = await invoke<string>("get_setting", { key: "autostart" });
        setAutostart(val === "true");
      } catch {
        // Keep default (true)
      }
    };
    load();
  }, []);

  // Live status indicator — reuses the existing tray-state event
  useEffect(() => {
    const unlisten = listen<string>("tray-state", (e) => {
      setLiveState((e.payload as TrayStateValue) || "idle");
    });
    return () => {
      unlisten.then((fn) => fn());
    };
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
  const dirtyCount = Object.keys(dirty).length;
  const live = LIVE_STATE[liveState];

  // --- Custom dictionary helpers ---
  const dict = parseDict(settings.custom_dictionary);

  const saveDict = (d: Dict) => {
    const raw = d.words.length === 0 && d.replacements.length === 0 ? "" : JSON.stringify(d);
    update("custom_dictionary", raw);
  };

  const addWord = () => {
    const w = newWord.trim();
    if (!w || dict.words.some((x) => x.toLowerCase() === w.toLowerCase())) return;
    saveDict({ ...dict, words: [...dict.words, w] });
    setNewWord("");
  };

  const removeWord = (w: string) => {
    saveDict({ ...dict, words: dict.words.filter((x) => x !== w) });
  };

  const addReplacement = () => {
    const f = newFrom.trim();
    const t = newTo.trim();
    if (!f || !t || dict.replacements.some((r) => r.from.toLowerCase() === f.toLowerCase())) return;
    saveDict({ ...dict, replacements: [...dict.replacements, { from: f, to: t }] });
    setNewFrom("");
    setNewTo("");
  };

  const removeReplacement = (from: string) => {
    saveDict({ ...dict, replacements: dict.replacements.filter((r) => r.from !== from) });
  };

  // Render the hotkey as pretty key caps
  const hotkeyCaps = settings.hotkey
    .split("+")
    .map((k) => k.charAt(0).toUpperCase() + k.slice(1));

  return (
    <div className="relative min-h-screen bg-surface-950 flex flex-col overflow-hidden">
      {/* Ambient background */}
      <div className="aurora-bg" />
      <div className="grain" />

      {/* Header */}
      <header className="relative z-10 px-6 pt-6 pb-5">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            {/* Animated logo mark (letterform, brand gradient) */}
            <div className="relative">
              <div className="w-11 h-11 rounded-2xl bg-brand-gradient bg-[length:200%_200%] animate-gradient-pan flex items-center justify-center shadow-glow-brand">
                <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 2a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                  <path d="M19 10v1a7 7 0 0 1-14 0v-1M12 18v4" />
                </svg>
              </div>
            </div>
            <div>
              <h1 className="text-lg font-extrabold tracking-tight text-gradient bg-brand-gradient bg-[length:200%_200%] animate-gradient-pan">
                VoiceType
              </h1>
              <p className="text-xs text-gray-500">Voice dictation · settings</p>
            </div>
          </div>

          {/* Live status pill */}
          <div className="flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.03] px-3 py-1.5">
            <span className="relative flex h-2.5 w-2.5">
              {liveState !== "idle" && (
                <span className={`absolute inline-flex h-full w-full rounded-full ${live.grad} animate-ring-pulse`} />
              )}
              <span className={`relative inline-flex h-2.5 w-2.5 rounded-full ${live.grad}`} />
            </span>
            <span className="text-xs font-medium text-gray-300">{live.label}</span>
          </div>
        </div>
      </header>

      {/* Content */}
      <div className="relative z-10 flex-1 overflow-y-auto px-6 pb-4 space-y-3">

        {/* Hotkey — hero card */}
        <div className="setting-card animate-float-up">
          <label className="field-label" htmlFor="hotkey-display">Dictation Hotkey</label>
          <div className="flex gap-2 items-center">
            <div
              id="hotkey-display"
              className={`flex-1 flex items-center gap-1.5 px-3 py-2.5 rounded-xl bg-surface-900/80 border text-sm transition-all
                ${captureMode
                  ? "border-brand-500/70 ring-4 ring-brand-500/15"
                  : "border-white/10"}`}
            >
              {captureMode ? (
                <span className="text-brand-300 font-medium animate-pulse">Press your hotkey combination…</span>
              ) : (
                hotkeyCaps.map((cap, i) => (
                  <span key={i} className="flex items-center gap-1.5">
                    {i > 0 && <span className="text-gray-600 text-xs">+</span>}
                    <kbd className="px-2 py-1 rounded-md bg-white/[0.06] border border-white/10 font-mono text-xs font-semibold text-brand-200 shadow-sm">
                      {cap}
                    </kbd>
                  </span>
                ))
              )}
            </div>
            <button
              id="hotkey-change-btn"
              onClick={handleHotkeyCapture}
              className="btn-secondary"
            >
              {captureMode ? "Cancel" : "Change"}
            </button>
          </div>
          <p className="text-[11px] text-gray-600 mt-2">Hold this combo anywhere, speak, release — text appears where you type.</p>
        </div>

        {/* Model */}
        <div className="setting-card animate-float-up animation-delay-100">
          <div className="flex items-center gap-3 mb-2.5">
            <IconChip gradient="bg-brand-sheen">{Icons.cpu}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Whisper Model</p>
              <p className="text-xs text-gray-500">Bigger models are more accurate but slower</p>
            </div>
          </div>
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
        <div className="setting-card animate-float-up animation-delay-100">
          <div className="flex items-center gap-3 mb-2.5">
            <IconChip gradient="bg-[linear-gradient(135deg,#06c9ae,#22e6c8)]">{Icons.globe}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Language</p>
              <p className="text-xs text-gray-500">Set a language or let VoiceType detect it</p>
            </div>
          </div>
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
        <div className="setting-card flex items-center justify-between animate-float-up animation-delay-200">
          <div className="flex items-center gap-3">
            <IconChip gradient="bg-[linear-gradient(135deg,#f43fb0,#ff6ac1)]">{Icons.sparkle}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Remove filler words</p>
              <p className="text-xs text-gray-500">Strips um, uh, you know, basically…</p>
            </div>
          </div>
          <Toggle
            id="remove-fillers-toggle"
            value={settings.remove_fillers === "true"}
            onChange={(v) => update("remove_fillers", v ? "true" : "false")}
          />
        </div>

        {/* Launch on startup */}
        <div className="setting-card flex items-center justify-between animate-float-up animation-delay-200">
          <div className="flex items-center gap-3">
            <IconChip gradient="bg-[linear-gradient(135deg,#5b3df5,#8482ff)]">{Icons.power}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Launch on startup</p>
              <p className="text-xs text-gray-500">Start VoiceType automatically when you log in</p>
            </div>
          </div>
          <Toggle
            id="autostart-toggle"
            value={autostart}
            onChange={async (v) => {
              setAutostart(v);
              try {
                await invoke("cmd_toggle_autostart", { enabled: v });
              } catch (e) {
                console.error("Autostart toggle failed:", e);
                // Revert on failure
                setAutostart(!v);
              }
            }}
          />
        </div>

        {/* Custom dictionary section */}
        <div className="flex items-center gap-3 pt-3 pb-1">
          <span className="section-label">Dictionary</span>
          <span className="flex-1 section-divider" />
        </div>

        <div className="setting-card space-y-4 animate-float-up animation-delay-300">
          <div className="flex items-center gap-3">
            <IconChip gradient="bg-[linear-gradient(135deg,#f59e0b,#f97316)]">{Icons.book}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Custom words</p>
              <p className="text-xs text-gray-500">Names, jargon, brands — helps recognition get them right</p>
            </div>
          </div>

          {/* Add word */}
          <div className="flex gap-2">
            <input
              id="dict-word-input"
              type="text"
              className="field-input flex-1"
              placeholder="e.g. VoiceType, Tauri, Soumyajit"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && addWord()}
              spellCheck={false}
            />
            <button className="btn-secondary" onClick={addWord} disabled={!newWord.trim()}>
              Add
            </button>
          </div>

          {/* Word chips */}
          {dict.words.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {dict.words.map((w) => (
                <span
                  key={w}
                  className="inline-flex items-center gap-1 rounded-full bg-white/[0.06] border border-white/10 pl-2.5 pr-1 py-1 text-xs text-gray-200"
                >
                  {w}
                  <button
                    onClick={() => removeWord(w)}
                    className="rounded-full p-0.5 text-gray-500 hover:text-red-300 hover:bg-white/10 transition-colors"
                    aria-label={`Remove ${w}`}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                      <path d="M18 6 6 18M6 6l12 12" />
                    </svg>
                  </button>
                </span>
              ))}
            </div>
          )}

          {/* Replacements */}
          <div className="pt-1 border-t border-white/[0.06]">
            <p className="text-sm font-semibold text-gray-100 mt-2">Replacements</p>
            <p className="text-xs text-gray-500 mb-2.5">Always rewrite a misheard phrase — e.g. “voice type” → “VoiceType”</p>
            <div className="flex gap-2 items-center">
              <input
                id="dict-from-input"
                type="text"
                className="field-input flex-1 min-w-0"
                placeholder="When I say…"
                value={newFrom}
                onChange={(e) => setNewFrom(e.target.value)}
                spellCheck={false}
              />
              <span className="text-gray-600 text-xs shrink-0">→</span>
              <input
                id="dict-to-input"
                type="text"
                className="field-input flex-1 min-w-0"
                placeholder="Write…"
                value={newTo}
                onChange={(e) => setNewTo(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && addReplacement()}
                spellCheck={false}
              />
              <button className="btn-secondary shrink-0" onClick={addReplacement} disabled={!newFrom.trim() || !newTo.trim()}>
                Add
              </button>
            </div>

            {dict.replacements.length > 0 && (
              <div className="mt-2.5 space-y-1.5">
                {dict.replacements.map((r) => (
                  <div
                    key={r.from}
                    className="flex items-center gap-2 rounded-lg bg-white/[0.04] border border-white/[0.08] px-3 py-1.5 text-xs"
                  >
                    <span className="text-gray-300 truncate">{r.from}</span>
                    <span className="text-gray-600 shrink-0">→</span>
                    <span className="text-brand-200 font-medium truncate">{r.to}</span>
                    <button
                      onClick={() => removeReplacement(r.from)}
                      className="ml-auto rounded-full p-0.5 text-gray-500 hover:text-red-300 hover:bg-white/10 transition-colors shrink-0"
                      aria-label={`Remove replacement ${r.from}`}
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                        <path d="M18 6 6 18M6 6l12 12" />
                      </svg>
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Cloud STT section */}
        <div className="flex items-center gap-3 pt-3 pb-1">
          <span className="section-label">Cloud API</span>
          <span className="flex-1 section-divider" />
        </div>

        {/* Enable cloud */}
        <div className="setting-card flex items-center justify-between animate-float-up animation-delay-300">
          <div className="flex items-center gap-3">
            <IconChip gradient={cloudEnabled ? "bg-[linear-gradient(135deg,#06c9ae,#5b3df5)]" : "bg-surface-600"}>{Icons.cloud}</IconChip>
            <div>
              <p className="text-sm font-semibold text-gray-100">Enable cloud STT</p>
              <p className="text-xs text-gray-500">Use Groq or OpenAI instead of local model</p>
            </div>
          </div>
          <Toggle
            id="cloud-toggle"
            value={cloudEnabled}
            onChange={(v) => update("use_cloud", v ? "true" : "false")}
          />
        </div>

        {/* Provider + key — shown only when cloud is enabled */}
        {cloudEnabled && (
          <div className="setting-card space-y-4 animate-pop-in">
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
              <div className="relative">
                <span className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-gray-500">
                  {Icons.key}
                </span>
                <input
                  id="api-key-input"
                  type="password"
                  className="field-input pl-10"
                  placeholder="sk-…"
                  value={settings.cloud_api_key}
                  onChange={(e) => update("cloud_api_key", e.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                />
              </div>
              <p className="text-[11px] text-gray-600 mt-1.5">
                {settings.cloud_provider === "groq"
                  ? "Get a free key at console.groq.com"
                  : "Get a key at platform.openai.com"}
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <footer className="relative z-10 px-6 pb-6 pt-4">
        {error && (
          <div className="mb-3 rounded-xl border border-red-500/30 bg-red-500/10 px-3 py-2">
            <p className="text-xs text-red-300">{error}</p>
          </div>
        )}
        <div className="flex items-center gap-3">
          <button
            id="save-btn"
            className="btn-primary flex-1"
            onClick={handleSave}
            disabled={saving || dirtyCount === 0}
          >
            <span className="relative z-10 inline-flex items-center justify-center gap-2">
              {saving ? (
                <>
                  <svg className="animate-spin" width="15" height="15" viewBox="0 0 24 24" fill="none">
                    <circle cx="12" cy="12" r="9" stroke="white" strokeOpacity="0.25" strokeWidth="3" />
                    <path d="M21 12a9 9 0 0 0-9-9" stroke="white" strokeWidth="3" strokeLinecap="round" />
                  </svg>
                  Saving…
                </>
              ) : saved ? (
                <>
                  <svg className="animate-checkmark" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M20 6 9 17l-5-5" />
                  </svg>
                  Saved
                </>
              ) : (
                <>Save changes</>
              )}
            </span>
          </button>
          {dirtyCount > 0 && (
            <div className="flex items-center gap-1.5 rounded-full bg-signal-trans/10 border border-signal-trans/20 px-3 py-1.5 animate-pop-in">
              <span className="h-1.5 w-1.5 rounded-full bg-signal-trans animate-pulse" />
              <p className="text-xs font-medium text-signal-trans">
                {dirtyCount} unsaved{dirtyCount > 1 ? "" : ""}
              </p>
            </div>
          )}
        </div>
      </footer>
    </div>
  );
}
