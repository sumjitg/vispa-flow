"""cloud.py — Optional Groq/OpenAI cloud STT fallback.

Per agent.md:
  - Only used when USE_CLOUD env is "true".
  - Uses openai SDK for both Groq and OpenAI (Groq is OpenAI-API-compatible).
    Groq base URL: https://api.groq.com/openai/v1
    Groq model: whisper-large-v3-turbo
    OpenAI model: whisper-1
  - Save audio to temp .wav, send to API, delete temp file.
  - On failure: log warning, raise — caller falls back to local transcription.
  - Expose: transcribe_cloud(audio: np.ndarray, provider: str, api_key: str) -> str
"""

import os
import io
import wave
import logging
import tempfile
import numpy as np

logger = logging.getLogger(__name__)

# Provider configs
PROVIDER_CONFIG = {
    "groq": {
        "base_url": "https://api.groq.com/openai/v1",
        "model": "whisper-large-v3-turbo",
    },
    "openai": {
        "base_url": "https://api.openai.com/v1",
        "model": "whisper-1",
    },
}


def _audio_to_wav_bytes(audio: np.ndarray, sample_rate: int = 16000) -> bytes:
    """Convert float32 numpy audio array ([-1.0, 1.0]) to WAV bytes."""
    # Clamp to [-1.0, 1.0] range
    audio = np.clip(audio, -1.0, 1.0)

    # Convert to 16-bit PCM
    pcm = (audio * 32767).astype(np.int16)

    buf = io.BytesIO()
    with wave.open(buf, "wb") as wf:
        wf.setnchannels(1)
        wf.setsampwidth(2)  # 16-bit
        wf.setframerate(sample_rate)
        wf.writeframes(pcm.tobytes())

    return buf.getvalue()


def transcribe_cloud(audio: np.ndarray, provider: str, api_key: str) -> str:
    """Transcribe audio using cloud STT API.

    Args:
        audio: float32 numpy array at 16kHz mono.
        provider: "groq" or "openai".
        api_key: API key for the selected provider.

    Returns:
        Transcribed text string.

    Raises:
        Exception: On API failure — caller should fall back to local transcription.
    """
    from openai import OpenAI

    config = PROVIDER_CONFIG.get(provider.lower())
    if config is None:
        raise ValueError(f"Unknown cloud provider: {provider}")

    logger.info("Cloud transcription: provider=%s, model=%s", provider, config["model"])

    wav_bytes = _audio_to_wav_bytes(audio, 16000)

    # Write to temp file
    fd, tmp_path = tempfile.mkstemp(suffix=".wav", prefix="vispa_cloud_")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(wav_bytes)

        client = OpenAI(base_url=config["base_url"], api_key=api_key)

        with open(tmp_path, "rb") as audio_file:
            transcript = client.audio.transcriptions.create(
                model=config["model"],
                file=audio_file,
                response_format="text",
            )

        logger.info("Cloud transcription succeeded: %d chars.", len(str(transcript)))
        return str(transcript).strip()

    except Exception as e:
        logger.warning("Cloud transcription failed: %s", e)
        raise

    finally:
        try:
            os.unlink(tmp_path)
        except OSError:
            pass
