"""ram_guard.py — Auto-select Whisper model tier based on available system RAM.

Per agent.md:
  - Read total system RAM via psutil.
  - Select model: <4GB→tiny.en, 4-6GB→tiny, 6-10GB→base, >10GB→small.
  - If model_override arg or MODEL_OVERRIDE env is set, use that instead.
  - Expose: get_model(model_override=None) -> str
"""

import os
import logging
import psutil

logger = logging.getLogger(__name__)

# RAM thresholds in GB
THRESHOLDS = [
    (4,   "tiny.en"),
    (6,   "tiny"),
    (10,  "base"),
    (float("inf"), "small"),
]


def _ram_tier_model() -> str:
    """Determine model from total system RAM."""
    try:
        mem = psutil.virtual_memory()
        total_gb = mem.total / (1024 ** 3)
    except Exception as e:
        logger.warning("psutil.virtual_memory failed: %s. Defaulting to tiny.", e)
        return "tiny"

    for limit, model in THRESHOLDS:
        if total_gb < limit:
            return model

    return "small"  # fallback


def get_model(model_override: str | None = None) -> str:
    """Return the Whisper model name to use.

    Priority:
      1. model_override argument (explicit call-site override)
      2. MODEL_OVERRIDE environment variable
      3. RAM-based auto tier selection
    """
    if model_override:
        return model_override

    env_override = os.environ.get("MODEL_OVERRIDE", "").strip()
    if env_override:
        return env_override

    model = _ram_tier_model()
    logger.info("Auto-selected model: %s", model)
    return model
