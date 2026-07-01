"""cleaner.py — Post-process transcribed text to remove filler words.

Per agent.md:
  - Only run if REMOVE_FILLERS env is "true".
  - Regex-based: remove um, uh, hmm, you know, basically, literally.
  - Strip leading/trailing so, right (case-insensitive).
  - Expose: clean(text: str) -> str
"""

import os
import re
import time
import logging

logger = logging.getLogger(__name__)

# Compile once — module-level cache
FILLER_WORDS = [
    r"\bum\b",
    r"\buh\b",
    r"\bhmm\b",
    r"\byou\s+know\b",
    r"\bbasically\b",
    r"\bliterally\b",
]

# Leading/trailing filler patterns
LEADING_SO = r"^\s*\bso\b[,.!?\s]*"
LEADING_RIGHT = r"^\s*\bright\b[,.!?\s]*"
TRAILING_SO = r"[,.!?\s]*\bso\b\s*$"
TRAILING_RIGHT = r"[,.!?\s]*\bright\b\s*$"

_FILLER_RE = re.compile("|".join(FILLER_WORDS), re.IGNORECASE)
_LEADING_SO_RE = re.compile(LEADING_SO, re.IGNORECASE)
_LEADING_RIGHT_RE = re.compile(LEADING_RIGHT, re.IGNORECASE)
_TRAILING_SO_RE = re.compile(TRAILING_SO, re.IGNORECASE)
_TRAILING_RIGHT_RE = re.compile(TRAILING_RIGHT, re.IGNORECASE)
_MULTISPACE_RE = re.compile(r"\s{2,}")


def clean(text: str) -> str:
    """Remove filler words from transcribed text.

    Returns text unchanged if REMOVE_FILLERS env is "false".
    """
    if os.environ.get("REMOVE_FILLERS", "true").lower() != "true":
        return text

    t0 = time.perf_counter()
    result = text

    # Remove filler words
    result = _FILLER_RE.sub("", result)

    # Strip leading "so" and "right"
    result = _LEADING_SO_RE.sub(" ", result)
    result = _LEADING_RIGHT_RE.sub(" ", result)

    # Strip trailing "so" and "right"
    result = _TRAILING_SO_RE.sub(" ", result)
    result = _TRAILING_RIGHT_RE.sub(" ", result)

    # Collapse multiple whitespace into single space
    result = _MULTISPACE_RE.sub(" ", result)

    # Trim leading/trailing whitespace
    result = result.strip()

    clean_ms = (time.perf_counter() - t0) * 1000
    if clean_ms > 0.5:
        logger.info("TIMING: clean=%s ms", round(clean_ms, 1))

    return result
