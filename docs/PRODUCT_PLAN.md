# VoiceType — Product Plan & Roadmap

> **One-liner:** Wispr Flow quality, pay once (~$25), your audio never leaves your PC. The Windows-first, open-source, local-first dictation app.
>
> Product name: **VoiceType** (renamed from working title "Vispa Flow", 2026-07-26 — see D6). Domain: voicetypeapp.com · GitHub: voicetypeapp
>
> Status: DRAFT v1 — 2026-07-26. Decisions marked ⚠️ need your call. Everything else is a recommendation you can override.

---

## 1. Why this product / why now

- **Wispr Flow** dominates mindshare but costs **$12–15/mo ($144+/yr, forever)** and is **cloud-only** — audio leaves the machine.
- **VoiceInk** proved the counter-model: open-source core + **$25–49 lifetime license**, 100% local, 200k+ downloads, 4.9 rating. **But it is Mac-only.**
- **There is no strong VoiceInk-equivalent on Windows.** superwhisper's Windows port is young; Wispr's Windows app is cloud-only. That's the gap.
- Subscription fatigue is real: VoiceInk's own testimonials are Wispr Flow subscribers who switched to escape the monthly bill.

**Positioning (important):** We are NOT "cheap Wispr clone." We are a *different category*: **no subscription + private/local + open source**. Cheap-clone framing = gimmick perception. Category framing = people already pay for this on Mac.

### Competitive snapshot (verified July 2026)

| Product | Platform | Price | Engine |
|---|---|---|---|
| Wispr Flow | Mac/Win/iOS/Android | Free 2k words/wk; Pro $12/mo annual ($15 monthly) | Cloud only |
| superwhisper | Mac/Win/iOS | Free (small local models); Pro $8/mo; lifetime option | Local + cloud, BYO keys |
| Aqua Voice | Mac (+Win) | Free 1k words; Pro $8/mo annual | Own cloud engine |
| VoiceInk | **Mac only** | **$25/$39/$49 lifetime** (1/2/3 devices) | Local, BYO cloud keys, open source |
| MacWhisper | Mac | ~€39–59 one-time | Local, file transcription focus |
| Windows 11 built-in (Win+H) | Windows | Free | Cloud/local, mediocre quality, no polish |

**Honest risk:** Whisper dictation is commoditizing. Our moat is *polish + trust + price + Windows focus*, not technology. Shipping speed and reliability ARE the product.

---

## 2. Current state of the codebase (what exists today)

| Area | Status |
|---|---|
| Tauri shell: tray, global hotkey (hold-to-talk), settings window | ✅ Working |
| Python sidecar: mic capture, Silero VAD, faster-whisper (int8, lazy-load, idle-unload), RAM guard | ✅ Working |
| Filler-word cleaning (`cleaner.py`) | ✅ Basic |
| Cloud fallback (Groq/OpenAI, `cloud.py`) | ✅ Basic |
| Text injection (clipboard + Ctrl-V, SendInput, modifier release, clipboard restore) | ✅ Working, needs hardening |
| SQLite settings, autostart (registry), crash log | ✅ Working |
| Auto-updater (`update.rs`, tauri-plugin-updater) | 🔨 In progress |
| PyInstaller sidecar bundling (`vispa-python.spec`, `src-tauri/binaries/`) | 🔨 In progress |
| CI (`.github/`) | 🔨 In progress |
| Custom dictionary / vocabulary | ❌ Missing |
| History / stats UI | ❌ Missing |
| AI polish layer (tone, formatting) | ❌ Missing |
| Licensing / payments | ❌ Missing |
| Code signing | ❌ Missing |
| Website / landing page | ❌ Missing |

---

## 3. Quality bar — what users don't forgive

These 5 things decide whether it feels like a product or a toy. Every phase-1 task maps to one of these.

1. **Latency**: key-release → text in field, target **p50 < 800ms, p95 < 1.5s** (local, `small`/distil model, int8). You already built timing instrumentation — use it.
2. **Accuracy on real speech**: punctuation, filler removal, and **custom dictionary** (names, jargon). Table stakes; every competitor has it.
3. **Injection reliability**: text must appear in Slack, Chrome, terminals, Electron apps, RDP, admin windows. One failure = uninstall.
4. **"It just works"**: autostart, tray, auto-update, graceful mic/model errors, fully offline capable.
5. **Trust**: signed binary (no SmartScreen scare), open-source repo, clear "audio never leaves your PC" claim.

---

## 4. Decisions needed ⚠️

Answer these before/during Phase 1. My recommendation is bolded.

| # | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Business model | (a) **Open-source core + paid lifetime license ($19–29)**, (b) freemium subscription, (c) fully free + donations | **(a)** — VoiceInk model. Subscriptions put you against Wispr's war chest and add server costs. |
| D2 | What's free vs paid | (a) **Everything free while building audience; add license later**, (b) paywall from day 1 | **(a)** — launch free & open source, gather users/stars/feedback, add licensing in Phase 3. Early users grandfathered = evangelists. |
| D3 | Price point | $19 / $25 / $29 lifetime; multi-device tiers like VoiceInk ($25/$39/$49) | **$25 single, $39 for 3 devices** at launch; can raise later (raising is easy, lowering is bad optics). |
| D4 | Merchant | Gumroad / **Lemon Squeezy** / Polar / Paddle | **Lemon Squeezy or Polar** — merchant of record (they handle global VAT), built-in license key API. |
| D5 | Open-source license | MIT (current) vs GPL vs source-available (FSL/BUSL) | **RESOLVED 2026-07-26: GPLv3** — deters closed-source forks selling our work (VoiceInk precedent). Switch LICENSE file during the rename, before any publicity. |
| D6 | Product name | "Vispa Flow" is close to "Wispr Flow" | **RESOLVED 2026-07-26: rename to "VoiceType"** — domain `voicetypeapp.com` + GitHub `voicetypeapp` verified available. Contains the real search phrase "voice typing app", zero Wispr similarity, safe for paid phase (no "free" in name). SEO strategy: brand ranks via long-tail content pages ("wispr flow alternative windows", "free dictation software windows"), not the domain alone. Rename is a Phase-1 task; register domain (~$12/yr) only when executing the rename. |
| D7 | Code signing | (a) **$0 path: unsigned at first → SignPath Foundation (free for OSS) at launch**, (b) Azure Trusted Signing ~$10/mo, (c) OV cert $100–300/yr | **(a)** — RESOLVED 2026-07-26: ship unsigned during beta (early adopters click through SmartScreen; winget distribution avoids most of it), apply to SignPath Foundation (free OSS signing, verified) at open-source launch. Revisit paid signing only in Phase 4, funded by revenue. |
| D8 | Cloud STT strategy | (a) **BYO API key only (Groq/OpenAI)**, (b) run our own proxy/cloud tier | **(a)** for v1 — zero server cost, zero liability. A hosted "no-setup cloud pass" can be a later upsell. |
| D9 | AI polish layer (tone rewrite, formatting) | BYO key (Groq llama / Gemini free tier) vs skip for v1 | **Include basic version in Phase 2** via BYO key — it's Wispr's stickiest feature and costs us nothing to proxy-free. |
| D10 | Distribution channels | Direct download only vs + winget vs + Microsoft Store | Direct + **winget** (free, devs love it). MS Store later (reach + no SmartScreen issues, but 12–15% cut if paid there). |

---

## 5. Roadmap

### Phase 1 — "Feels like a product" (target: ~4–6 weeks)
Goal: a stranger installs the MSI and it works flawlessly for a week. Nothing else matters until this is true.

- [ ] **Finish sidecar bundling**: PyInstaller build reproducible, CI builds MSI end-to-end (`.github/` workflow: build sidecar → build Tauri → attach updater manifest; signing step added free via SignPath later)
- [ ] **Finish auto-updater**: `update.rs` + signed update manifest hosted (GitHub Releases is fine)
- [x] **Custom dictionary** ✅ 2026-07-26 (biggest missing table-stakes feature):
  - Settings UI: add words/phrases + "sounds like" replacement pairs (Dictionary section)
  - Implementation: `python/dictionary.py` — hotwords passed to faster-whisper + case-insensitive whole-word replacements after cleaning; stored as JSON in the `custom_dictionary` setting, live-updates via set_config
- [ ] **Injection hardening**: test matrix — Notepad, Chrome/Edge (incl. Google Docs), VS Code, Slack/Discord (Electron), Windows Terminal, cmd, RDP client, elevated apps (document limitation if uninjectable). Fallback chain: paste → SendInput unicode → beep + keep in clipboard with toast.
- [ ] **Latency pass**: use existing timing instrumentation; measure p50/p95 on 10-word and 50-word utterances; keep model warm during active hours (revisit idle-unload thresholds); pre-roll mic capture on hotkey-down.
- [ ] **Error UX**: mic missing/busy, model download progress on first run, sidecar crash auto-restart with tray notification
- [ ] **First-run experience**: model download with progress bar, mic test, hotkey tutorial, "try it here" textbox
- [ ] **History panel**: last N transcriptions in tray popup (copy button) — cheap feature, users love it, safety net for injection failures
- [ ] **Stats**: words dictated today/total, estimated time saved (Wispr markets "4x faster than typing" — copy this framing)
- [ ] **Code signing — $0 path** (D7): ship beta unsigned; apply to **SignPath Foundation** (free code signing for open-source projects — they sign builds from your public repo) when the repo goes public. Add a "Windows protected your PC → More info → Run anyway" note on the download page for beta users. Paid cert only later, from revenue.
- [ ] **Model picker in settings**: tiny/base/small/distil + language selection (auto-detect default)

**Exit criteria:** you personally dictate every message/email for 1 week with zero manual fixes of app behavior (transcription errors OK, app misbehavior not OK). p95 latency < 1.5s local.

### Phase 2 — "Differentiation" (~3–4 weeks, overlaps Phase 1 tail)

- [ ] **AI polish modes (BYO key)**: per-app or per-hotkey modes — e.g. "Email (formal)", "Chat (casual)", "Code comment", "Prompt for AI" — one LLM call to Groq/Gemini after transcription. This is the Wispr-parity feature.
- [ ] **Snippets/commands**: "new line", "period", spoken punctuation toggle; text expansion ("my email" → address)
- [ ] **Per-app awareness** (detect foreground app → auto-select mode)
- [ ] **Cloud STT UX**: clean BYO-key setup for Groq (whisper-large-v3-turbo — near-free, ~10x realtime) as "max accuracy" option
- [ ] Polish pass: settings redesign if needed, dark mode, sounds/haptics feel
- [ ] **Website**: single landing page — demo video (15s screen recording of dictating into Slack), comparison table vs Wispr (price over 3 years: $432+ vs $25), download button, GitHub link. Host on Cloudflare Pages/Vercel, free.

### Phase 3 — "Launch & audience" (~2–3 weeks)

- [ ] Publish repo properly: README with GIF demo, screenshots, roadmap, CONTRIBUTING
- [ ] **Launch sequence** (order matters):
  1. Soft launch: X/Twitter build-in-public thread, r/windows, r/software, r/productivity
  2. **Show HN** ("Show HN: Open-source Wispr Flow alternative for Windows — local, no subscription")
  3. Product Hunt
  4. Submit to winget; reach out to alternativeto.net, "best dictation app" listicle authors, YouTube reviewers of Wispr/superwhisper
- [ ] Set up feedback channels: GitHub issues + Discord (or GitHub Discussions)
- [ ] Instrument (privacy-respecting, opt-in) anonymous telemetry OR just in-app stats + a "share feedback" button. ⚠️ Decide: telemetry helps but conflicts with privacy positioning — recommend **opt-in only, aggregate counts only**.

**Exit criteria:** 1,000+ downloads, 200+ GitHub stars, active feedback loop, injection-failure reports triaged.

### Phase 4 — "Monetize" (when there's pull, not before)

- [ ] Lemon Squeezy/Polar store + license key validation in app (offline-tolerant: validate once, cache, re-check lazily)
- [ ] Decide free/paid split. Recommended: **free = fully functional local dictation (tiny/base models)**; **paid $25 = small/distil models UI, AI polish modes, per-app modes, dictionary sync, priority support**. Keep free genuinely useful — it's the marketing.
- [ ] Grandfather all pre-launch users with free license keys (announce this — goodwill engine)
- [ ] 14-day refund policy, no questions (VoiceInk does this; kills purchase anxiety)
- [ ] Later upsells: hosted cloud pass for no-API-key users; team/enterprise (shared dictionary, volume licenses)

**Revenue sanity check:** at $25, ~40 sales = $1k. VoiceInk-level traction (200k downloads, even 1–2% conversion) = $50–100k lifetime-to-date. This is an indie business, not a VC business — that's the point.

### Phase 5 — "Expand" (later, optional)

- Meeting/file transcription (MacWhisper territory) as a second paid tier
- macOS port (Tauri makes this plausible; enters crowded market though)
- Streaming transcription (text appears while speaking) — big wow factor, significant engineering
- Parakeet/Moonshine models (faster than Whisper family) if quality holds

---

## 6. Metrics that matter

Track from Phase 1 (locally, in the stats DB):

| Metric | Target |
|---|---|
| Key-release → injected text, p50 / p95 (local small model) | < 800ms / < 1.5s |
| First-run → first successful dictation | < 3 min incl. model download |
| Injection success rate across test matrix | 100% on matrix apps |
| Sidecar crash rate | ~0; auto-recovery always |
| Post-launch: D7 retention, words/user/day, downloads→weekly-active | learn, then set targets |
| Post-monetization: free→paid conversion | 1–3% is healthy for this model |

---

## 7. Costs (annual, lean — $0 until revenue)

| Item | Cost |
|---|---|
| Code signing — SignPath Foundation (free for OSS) / unsigned beta | **$0** |
| Domain (optional at first — GitHub Pages + github.io subdomain works) | $0 → ~$12/yr when ready |
| Website/updates hosting (Cloudflare Pages or GitHub Pages + GitHub Releases) | $0 |
| Merchant of record (Phase 4 only) | % of sales only (~5% + fees) — no fixed cost |
| STT/LLM inference | $0 (local + BYO keys) |
| **Total fixed until monetization** | **$0** (≈$12/yr if you buy a domain early) |

> Principle (standard indie-hacker practice): **spend $0 until users prove demand; every later cost is paid out of revenue, not savings.** The only thing money can't substitute for here is shipping speed and polish.

---

## 8. Immediate next actions (this week)

1. ~~Decide D5 (license) and D6 (name)~~ ✅ DONE: **GPLv3** + rename to **VoiceType** (voicetypeapp.com / github.com/voicetypeapp).
2. ~~Execute the rename~~ ✅ DONE 2026-07-26: all code/config/UI renamed to VoiceType; LICENSE switched to GPLv3. **Deferred by user:** domain purchase (buy later when 100% sure of the name) and GitHub repo rename (later; name may still change — in-code renames are cheap to redo).
3. Build **custom dictionary** (first build task — biggest missing table-stakes feature).
4. Finish **CI → MSI → auto-update** pipeline (unsigned for now — $0; SignPath later).
5. Start the **injection test matrix** doc — test 10 apps, record results.
6. Begin **dogfooding daily** and logging every annoyance as a GitHub issue.
