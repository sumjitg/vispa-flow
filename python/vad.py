"""vad.py — Silero VAD wrapper for speech detection.

Per agent.md:
  - Load Silero VAD model once (lazy, ~1MB, always resident after first use).
  - Expose: is_speech(audio_chunk: np.ndarray, sample_rate: int = 16000) -> bool
  - Used by recorder.py to gate transcription — only send audio if speech detected.
  - Fail open: on VAD error, return True (don't block transcription).
"""

import logging
import numpy as np

logger = logging.getLogger(__name__)

# Module-level cache — loaded once, never unloaded (small model)
_model = None
_get_speech_timestamps = None


def _ensure_model():
    """Lazy-load Silero VAD model and get_speech_timestamps on first call."""
    global _model, _get_speech_timestamps

    if _model is not None:
        return

    try:
        # Delay imports to avoid PyTorch import at module load time
        import torch  # noqa: F401
        from silero_vad import load_silero_vad, get_speech_timestamps

        _model = load_silero_vad()
        _get_speech_timestamps = get_speech_timestamps
        logger.info("Silero VAD model loaded.")
    except Exception as e:
        logger.warning("Failed to load Silero VAD: %s. VAD will fail-open.", e)


def is_speech(audio_chunk: np.ndarray, sample_rate: int = 16000) -> bool:
    """Return True if speech is detected in the audio chunk.

    Args:
        audio_chunk: float32 numpy array of audio samples.
        sample_rate: Sample rate in Hz (default 16000).

    Returns:
        True if speech detected, False if silence.
        On VAD error: returns True (fail-open per agent.md).
    """
    _ensure_model()

    if _get_speech_timestamps is None or _model is None:
        logger.warning("VAD model unavailable — fail open.")
        return True

    try:
        # Ensure float32
        if audio_chunk.dtype != np.float32:
            audio_chunk = audio_chunk.astype(np.float32)

        # Silero VAD requires 1D mono audio — flatten 2D/stereo arrays
        if audio_chunk.ndim > 1:
            audio_chunk = audio_chunk.ravel()

        timestamps = _get_speech_timestamps(audio_chunk, _model, sampling_rate=sample_rate)
        return len(timestamps) > 0
    except Exception as e:
        logger.warning("VAD inference failed: %s. Failing open.", e)
        return True
