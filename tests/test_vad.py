"""Tests for vad.py — mocks the silero_vad module to avoid PyTorch download."""
import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

from unittest.mock import patch, MagicMock
import numpy as np
import pytest


def test_vad_returns_bool():
    """VAD result must always be a bool."""
    # Mock silero_vad at the import level
    mock_model = MagicMock()
    mock_timestamps = [{"start": 0, "end": 100}]

    with patch.dict("sys.modules", {
        "silero_vad": MagicMock(
            load_silero_vad=MagicMock(return_value=mock_model),
            get_speech_timestamps=MagicMock(return_value=mock_timestamps),
        ),
        "torch": MagicMock(),
    }):
        # Reset module-level model cache
        import vad
        vad._model = None
        vad._utils = None

        audio = np.random.randn(16000).astype(np.float32) * 0.5
        result = vad.is_speech(audio)
        assert isinstance(result, bool)


def test_vad_silence_no_speech():
    """Silence should return no timestamps → False."""
    mock_model = MagicMock()

    with patch.dict("sys.modules", {
        "silero_vad": MagicMock(
            load_silero_vad=MagicMock(return_value=mock_model),
            get_speech_timestamps=MagicMock(return_value=[]),
        ),
        "torch": MagicMock(),
    }):
        import vad
        vad._model = None
        vad._utils = None

        silence = np.zeros(16000, dtype=np.float32)
        assert vad.is_speech(silence) is False


def test_vad_fails_open():
    """If VAD raises, is_speech returns True (fail open)."""
    mock_model = MagicMock()

    with patch.dict("sys.modules", {
        "silero_vad": MagicMock(
            load_silero_vad=MagicMock(return_value=mock_model),
            get_speech_timestamps=MagicMock(side_effect=RuntimeError("GPU OOM")),
        ),
        "torch": MagicMock(),
    }):
        import vad
        vad._model = None
        vad._utils = None

        audio = np.zeros(16000, dtype=np.float32)
        # Should not raise, should return True (fail open)
        assert vad.is_speech(audio) is True
