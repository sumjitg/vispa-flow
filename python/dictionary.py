"""dictionary.py — custom user dictionary: hotwords + post-transcription replacements.

The CUSTOM_DICTIONARY env var holds JSON:
  {"words": ["VoiceType", "Tauri"], "replacements": [{"from": "voice type", "to": "VoiceType"}]}

  - words: passed to faster-whisper as hotwords to bias recognition.
  - replacements: applied to the transcribed text (case-insensitive, whole-word).

Expose: get_hotwords() -> str | None, apply_replacements(text: str) -> str
"""

import os
import json
import re
import logging

logger = logging.getLogger(__name__)

# Cache keyed on the raw env value so config updates take effect immediately
_cache_raw: str | None = None
_words: list[str] = []
_compiled: list[tuple[re.Pattern, str]] = []


def _load():
    """Re-parse CUSTOM_DICTIONARY if the env value changed since last call."""
    global _cache_raw, _words, _compiled

    raw = os.environ.get("CUSTOM_DICTIONARY", "")
    if raw == _cache_raw:
        return
    _cache_raw = raw

    words: list[str] = []
    repls: list[tuple[str, str]] = []
    if raw:
        try:
            data = json.loads(raw)
            words = [w.strip() for w in data.get("words", [])
                     if isinstance(w, str) and w.strip()]
            for r in data.get("replacements", []):
                if isinstance(r, dict):
                    f = str(r.get("from", "")).strip()
                    t = str(r.get("to", "")).strip()
                    if f and t:
                        repls.append((f, t))
        except (json.JSONDecodeError, AttributeError, TypeError) as e:
            logger.warning("Invalid CUSTOM_DICTIONARY JSON: %s", e)

    _words = words
    _compiled = [
        (re.compile(r"\b" + re.escape(f) + r"\b", re.IGNORECASE), t)
        for f, t in repls
    ]
    logger.info("Dictionary loaded: %d words, %d replacements", len(words), len(repls))


def get_hotwords() -> str | None:
    """Return a hotwords string for faster-whisper, or None if empty.

    Includes both the custom words and the replacement targets, so Whisper
    is biased toward producing the correct term directly.
    """
    _load()
    terms = list(_words) + [t for _, t in _compiled]
    # De-duplicate preserving order
    seen: set[str] = set()
    unique = [t for t in terms if not (t.lower() in seen or seen.add(t.lower()))]
    return " ".join(unique) if unique else None


def apply_replacements(text: str) -> str:
    """Apply user replacement rules (case-insensitive, whole-word)."""
    _load()
    for rx, to in _compiled:
        text = rx.sub(to, text)
    return text
