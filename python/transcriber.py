"""transcriber.py — faster-whisper wrapper with lazy load and warm window.

Per agent.md:
  - Lazy load: do not load Whisper at startup. Load on first transcription call.
  - Warm window: after transcription, keep model in RAM for warm_seconds (default 60).
    Unload if idle longer than that.
  - Unload: set model to None, call gc.collect().
  - Expose: transcribe(audio: np.ndarray, model_name: str, language: str | None) -> str
  - All exceptions caught, logged, re-raised as TranscriptionError.
  - Log load/unload at INFO level.
"""

import os
import gc
import time
import logging
import threading
import numpy as np

logger = logging.getLogger(__name__)

# Module-level model cache + idle timer
_model = None
_model_name: str | None = None
_last_transcription_time: float = 0.0
_unload_timer: threading.Timer | None = None
_load_lock = threading.Lock()


class TranscriptionError(Exception):
    """Raised when transcription fails."""
    pass


def _get_warm_seconds() -> int:
    """Read warm_seconds from env, default to 60."""
    try:
        return int(os.environ.get("WARM_SECONDS", "60"))
    except (ValueError, TypeError):
        return 60


def _schedule_unload():
    """Schedule model unload after warm window expires."""
    global _unload_timer

    if _unload_timer is not None:
        _unload_timer.cancel()

    delay = _get_warm_seconds()
    if delay <= 0:
        _unload_hot()
        return

    _unload_timer = threading.Timer(delay, _unload_hot)
    _unload_timer.daemon = True
    _unload_timer.start()


def _unload_hot():
    """Unload the Whisper model and free GPU/RAM."""
    global _model, _model_name, _unload_timer

    with _load_lock:
        if _model is not None:
            model_label = _model_name or "unknown"
            logger.info("Unloading Whisper model (%s) — idle window expired.", model_label)
            _model = None
            _model_name = None
            gc.collect()

    _unload_timer = None


def _load_model(model_name: str):
    """Load faster-whisper model with the given name."""
    global _model, _model_name

    from faster_whisper import WhisperModel

    # Determine compute type
    # float16 on CUDA/Apple Silicon, int8 on CPU
    compute_type = "int8"

    logger.info("Loading Whisper model: %s (compute=%s)", model_name, compute_type)
    _model = WhisperModel(model_name, device="auto", compute_type=compute_type)
    _model_name = model_name
    logger.info("Whisper model loaded: %s", model_name)


def transcribe(audio: np.ndarray, model_name: str, language: str | None = None) -> str:
    """Transcribe audio using faster-whisper.

    Args:
        audio: float32 numpy array at 16kHz mono.
        model_name: Whisper model name (tiny.en, tiny, base, small, etc.).
        language: Language code (en, hi, es, etc.) or None for auto-detect.

    Returns:
        Transcribed text string.

    Raises:
        TranscriptionError: If transcription fails.
    """
    global _last_transcription_time

    # Cancel any pending unload
    if _unload_timer is not None:
        _unload_timer.cancel()

    try:
        with _load_lock:
            # Load model if not loaded or if model name changed
            if _model is None or _model_name != model_name:
                if _model is not None:
                    _unload_hot()
                _load_model(model_name)

        # Run transcription
        lang_kwargs = {}
        if language and language.lower() != "auto":
            lang_kwargs["language"] = language.lower()

        segments, _info = _model.transcribe(audio, beam_size=5, **lang_kwargs)
        text = " ".join(segment.text.strip() for segment in segments)

        _last_transcription_time = time.time()

        # Schedule unload after warm window
        _schedule_unload()

        return text.strip()

    except Exception as e:
        logger.error("Transcription failed: %s", e, exc_info=True)
        raise TranscriptionError(str(e)) from e
