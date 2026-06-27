"""Tests for ram_guard.py — mocks psutil to avoid hardware dependency."""
import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

from unittest.mock import patch, MagicMock
import pytest
import ram_guard


def make_mem(total_gb: float):
    mem = MagicMock()
    mem.total = int(total_gb * 1024 ** 3)
    return mem


@patch("psutil.virtual_memory")
def test_less_than_4gb(mock_mem):
    mock_mem.return_value = make_mem(3.0)
    assert ram_guard.get_model() == "tiny.en"


@patch("psutil.virtual_memory")
def test_4_to_6gb(mock_mem):
    mock_mem.return_value = make_mem(5.0)
    assert ram_guard.get_model() == "tiny"


@patch("psutil.virtual_memory")
def test_6_to_10gb(mock_mem):
    mock_mem.return_value = make_mem(8.0)
    assert ram_guard.get_model() == "base"


@patch("psutil.virtual_memory")
def test_above_10gb(mock_mem):
    mock_mem.return_value = make_mem(16.0)
    assert ram_guard.get_model() == "small"


@patch("psutil.virtual_memory")
def test_override_argument(mock_mem):
    mock_mem.return_value = make_mem(16.0)
    # Even with lots of RAM, override wins
    assert ram_guard.get_model(model_override="tiny.en") == "tiny.en"


@patch("psutil.virtual_memory")
def test_override_env(mock_mem, monkeypatch):
    mock_mem.return_value = make_mem(16.0)
    monkeypatch.setenv("MODEL_OVERRIDE", "base")
    assert ram_guard.get_model() == "base"
    monkeypatch.delenv("MODEL_OVERRIDE")
