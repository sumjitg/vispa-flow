"""recorder.py — Microphone capture via sounddevice.

Per agent.md:
  - Capture at 16kHz mono float32 (Whisper's required format).
  - On start_recording: begin buffering audio chunks.
  - On stop_recording: run VAD on buffer, return audio if speech, None if silence.
  - Max recording: 120 seconds, auto-stop if exceeded.
  - Expose: start(), stop() -> np.ndarray | None
"""

import logging
import threading
import time
import numpy as np
import sounddevice as sd

from vad import is_speech

logger = logging.getLogger(__name__)

MAX_RECORDING_SECONDS = 120
SAMPLE_RATE = 16000
CHANNELS = 1
DTYPE = np.float32

# Module-level recorder state
_buffer: list[np.ndarray] = []
_stream: sd.InputStream | None = None
_start_time: float = 0.0
_recording = False
_lock = threading.Lock()
_stop_timer: threading.Timer | None = None


def _audio_callback(indata: np.ndarray, _frames: int, _time_info, _status):
    """Called by sounddevice for each audio chunk. Append to buffer."""
    with _lock:
        if _recording:
            _buffer.append(indata.copy())


def _auto_stop():
    """Force-stop recording if max duration exceeded."""
    logger.warning("Max recording duration (%ds) reached — auto-stopping.", MAX_RECORDING_SECONDS)
    global _recording
    with _lock:
        _recording = False
    if _stream is not None:
        _stream.stop()


def start():
    """Begin capturing microphone audio at 16kHz mono float32."""
    global _stream, _recording, _buffer, _start_time, _stop_timer

    with _lock:
        if _recording:
            logger.warning("start() called while already recording — ignored.")
            return

        _buffer = []
        _recording = True
        _start_time = time.time()

        # Start auto-stop timer
        _stop_timer = threading.Timer(MAX_RECORDING_SECONDS, _auto_stop)
        _stop_timer.daemon = True
        _stop_timer.start()

        try:
            _stream = sd.InputStream(
                samplerate=SAMPLE_RATE,
                channels=CHANNELS,
                dtype=DTYPE,
                callback=_audio_callback,
            )
            _stream.start()
        except Exception as e:
            _recording = False
            logger.error("Failed to start audio stream: %s", e)
            raise


def stop() -> np.ndarray | None:
    """Stop recording, run VAD, return audio array or None if no speech.

    Returns:
        np.ndarray of float32 audio samples if speech detected, None if silence.
    """
    global _stream, _recording, _buffer, _stop_timer

    with _lock:
        _recording = False

        # Cancel auto-stop timer
        if _stop_timer is not None:
            _stop_timer.cancel()
            _stop_timer = None

        # Stop the audio stream
        if _stream is not None:
            try:
                _stream.stop()
                _stream.close()
            except Exception as e:
                logger.warning("Error stopping audio stream: %s", e)
            _stream = None

        # Concatenate all buffered chunks
        if not _buffer:
            logger.info("No audio captured.")
            return None

        audio = np.concatenate(_buffer, axis=0)

        if len(audio) == 0:
            logger.info("Empty audio buffer.")
            return None

    # Run VAD outside the lock (VAD can take time)
    duration = len(audio) / SAMPLE_RATE
    logger.info("Recorded %.1fs of audio (%d samples).", duration, len(audio))

    if is_speech(audio, SAMPLE_RATE):
        return audio

    logger.info("VAD detected no speech — skipping transcription.")
    return None
