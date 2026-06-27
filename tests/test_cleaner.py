"""Tests for cleaner.py — no heavy deps required."""
import os
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

import pytest
from cleaner import clean


@pytest.fixture(autouse=True)
def enable_fillers():
    """Ensure filler removal is enabled for all tests."""
    os.environ["REMOVE_FILLERS"] = "true"
    yield
    os.environ.pop("REMOVE_FILLERS", None)


def test_removes_um():
    assert clean("Um this is a test") == "this is a test"

def test_removes_uh():
    assert clean("uh I think uh so") == "I think"

def test_removes_hmm():
    assert clean("Hmm okay") == "okay"

def test_removes_you_know():
    assert clean("you know what I mean") == "what I mean"

def test_removes_basically():
    assert clean("basically it works") == "it works"

def test_removes_literally():
    assert clean("literally amazing") == "amazing"

def test_strips_leading_so():
    assert clean("So this is the result") == "this is the result"

def test_strips_trailing_right():
    assert clean("Makes sense right") == "Makes sense"

def test_combined():
    result = clean("Um so basically you know I think uh right")
    assert "um" not in result.lower()
    assert "basically" not in result.lower()
    assert "you know" not in result.lower()
    assert "uh" not in result.lower()

def test_passthrough_when_disabled():
    os.environ["REMOVE_FILLERS"] = "false"
    text = "Um this should not be cleaned"
    assert clean(text) == text

def test_no_change_clean_text():
    text = "Hello world, this is clean."
    assert clean(text) == text

def test_collapses_whitespace():
    result = clean("um uh basically yes")
    assert "  " not in result  # no double spaces
