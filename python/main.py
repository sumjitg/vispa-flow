"""main.py — Python AI sidecar entry point for Vispa Flow.

Per agent.md:
  - Start TCP socket server on 127.0.0.1:51337.
  - Listen for JSON commands from Rust via newline-delimited JSON.
  - On start_recording: call recorder.start().
  - On stop_recording: call recorder.stop() → VAD → transcribe → clean → send transcribed.
  - Send state events to Rust at each transition.
  - On connection: print READY:<port> to stdout (Rust sidecar waits for this).
"""

import os
import sys
import json
import socket
import logging
import threading
import time

import numpy as np

from recorder import start as recorder_start, stop as recorder_stop
from cleaner import clean
from ram_guard import get_model

# Configure logging — write to a file in %APPDATA%\VispaFlow.
# Stdout is reserved for the READY: protocol line and stderr for crash dumps.
_log_dir = os.path.join(os.environ.get("APPDATA", os.path.expanduser("~")), "VispaFlow")
os.makedirs(_log_dir, exist_ok=True)
_log_path = os.path.join(_log_dir, "vispa-python.log")

logging.basicConfig(
    level=logging.DEBUG if os.environ.get("VISPA_DEBUG") else logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    handlers=[
        logging.FileHandler(_log_path, encoding="utf-8"),
        logging.StreamHandler(sys.stderr),
    ],
)
logger = logging.getLogger("vispa")

# ---------------------------------------------------------------------------
# Timing instrumentation
# ---------------------------------------------------------------------------
_timings: dict | None = None  # active per-dictation timing dict


def _timing_start(request_id: str):
    """Initialise a fresh timing accumulator for this dictation."""
    global _timings
    _timings = {"request_id": request_id, "stages": {}, "t0": time.perf_counter()}


def _timing_stage(name: str):
    """Record the elapsed wall-clock time since the last stage (or t0).

    Call _timing_stage("stage_name") at the END of each pipeline step.
    """
    global _timings
    if _timings is None:
        return
    now = time.perf_counter()
    elapsed_ms = (now - _timings.get("_last", _timings["t0"])) * 1000
    cumulative_ms = (now - _timings["t0"]) * 1000
    _timings["stages"][name] = round(elapsed_ms, 1)
    _timings["stages"][f"{name}_cumulative"] = round(cumulative_ms, 1)
    _timings["_last"] = now


def _timing_report() -> str:
    """Format the timing dict as a human-readable log line."""
    global _timings
    if _timings is None:
        return ""
    parts = [f"tid={_timings.get('request_id','?')}"]
    stages = _timings.get("stages", {})
    for key, val in sorted(stages.items()):
        if not key.endswith("_cumulative"):
            cum = stages.get(f"{key}_cumulative", "?")
            parts.append(f"{key}={val}ms cum={cum}ms")
    total = round((time.perf_counter() - _timings["t0"]) * 1000, 1)
    parts.append(f"total_pipeline={total}ms")
    return " | ".join(parts)


def _timing_clear():
    global _timings
    _timings = None

HOST = "127.0.0.1"
PORT = 51337


def send_event(conn: socket.socket, event: str, **kwargs):
    """Send a JSON event to the Rust sidecar."""
    payload = {"event": event, **kwargs}
    try:
        msg = json.dumps(payload) + "\n"
        conn.sendall(msg.encode("utf-8"))
    except Exception as e:
        logger.error("Failed to send event %s: %s", event, e)


def handle_command(cmd: str, params: dict, conn: socket.socket):
    """Dispatch a command received from the Rust sidecar."""
    if cmd == "start_recording":
        logger.info("Starting recording…")
        send_event(conn, "state", value="recording")
        try:
            recorder_start()
        except Exception as e:
            logger.error("recorder.start failed: %s", e)
            send_event(conn, "error", message=f"Microphone error: {e}")
            send_event(conn, "state", value="idle")

    elif cmd == "stop_recording":
        request_id = params.get("request_id", "")
        _timing_start(request_id)
        logger.info("Stopping recording… (tid=%s)", request_id)

        try:
            audio = recorder_stop()
            _timing_stage("recorder_stop")

            if audio is None or (isinstance(audio, np.ndarray) and len(audio) == 0):
                logger.info("No speech detected — skipping transcription.")
                send_event(conn, "state", value="idle")
                _timing_clear()
                return

            send_event(conn, "state", value="transcribing")

            # Determine whether to use cloud or local
            use_cloud = os.environ.get("USE_CLOUD", "false").lower() == "true"
            text = None

            if use_cloud:
                provider = os.environ.get("CLOUD_PROVIDER", "groq")
                api_key = os.environ.get("CLOUD_API_KEY", "")
                if api_key:
                    try:
                        from cloud import transcribe_cloud
                        text = transcribe_cloud(audio, provider, api_key)
                    except Exception as e:
                        logger.warning("Cloud transcription failed — falling back to local. Error: %s", e)

            if text is None:
                # Local transcription
                model_name = get_model()
                language = os.environ.get("LANGUAGE", "auto")
                if language.lower() == "auto":
                    language = None

                from transcriber import transcribe, TranscriptionError
                try:
                    text = transcribe(audio, model_name, language)
                except TranscriptionError as e:
                    logger.error("Local transcription failed: %s", e)
                    send_event(conn, "error", message=f"Transcription failed: {e}")
                    send_event(conn, "state", value="idle")
                    _timing_clear()
                    return

            _timing_stage("whisper")

            # Clean filler words
            if text:
                text = clean(text)
            _timing_stage("clean")

            # Build timing payload and log report
            stage_timings = {k: v for k, v in _timings.get("stages", {}).items()
                            if not k.startswith("_")}
            logger.info("TIMING REPORT: %s", _timing_report())

            logger.info("Transcribed: %r", text)
            send_event(conn, "transcribed", text=text, timing=stage_timings)
            send_event(conn, "state", value="idle")
            _timing_clear()

        except Exception as e:
            logger.error("Unexpected error in stop_recording: %s", e, exc_info=True)
            send_event(conn, "error", message=str(e))
            send_event(conn, "state", value="idle")

    elif cmd == "set_config":
        key = params.get("key", "")
        value = params.get("value", "")
        logger.info("Config update: %s = %s", key, value)

        env_map = {
            "model_override": "MODEL_OVERRIDE",
            "use_cloud":      "USE_CLOUD",
            "cloud_provider": "CLOUD_PROVIDER",
            "cloud_api_key":  "CLOUD_API_KEY",
            "language":       "LANGUAGE",
            "remove_fillers": "REMOVE_FILLERS",
            "warm_seconds":   "WARM_SECONDS",
        }

        env_key = env_map.get(key)
        if env_key:
            os.environ[env_key] = value

    elif cmd == "ping":
        send_event(conn, "pong")

    else:
        logger.warning("Unknown command: %s", cmd)


def handle_client(conn: socket.socket):
    """Read newline-delimited JSON commands from a connected client."""
    try:
        peer = conn.getpeername()
        logger.info("Client connected: %s:%d", *peer)
    except OSError:
        logger.info("Client connected (peer unknown)")

    # Read initial one-time settings from env and send as config
    buffer = ""
    try:
        while True:
            data = conn.recv(4096)
            if not data:
                break

            buffer += data.decode("utf-8")
            while "\n" in buffer:
                line, buffer = buffer.split("\n", 1)
                line = line.strip()
                if not line:
                    continue

                try:
                    msg = json.loads(line)
                except json.JSONDecodeError as e:
                    logger.warning("Invalid JSON from client: %s", e)
                    continue

                cmd = msg.get("cmd", "")
                handle_command(cmd, msg, conn)

    except ConnectionResetError:
        logger.info("Client disconnected.")
    except Exception as e:
        logger.error("Error in client handler: %s", e, exc_info=True)
    finally:
        try:
            conn.close()
        except OSError:
            pass


def main():
    """Start the TCP socket server and accept connections."""
    # Print READY line to stdout — Rust sidecar waits for this
    print(f"READY:listening on {HOST}:{PORT}", flush=True)

    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)

    try:
        server.bind((HOST, PORT))
        server.listen(1)
        logger.info("Vispa Flow Python sidecar ready on %s:%d", HOST, PORT)
    except OSError as e:
        logger.critical("Failed to bind to %s:%d: %s", HOST, PORT, e)
        # Print READY anyway so Rust doesn't hang forever
        print(f"READY:failed to bind {e}", flush=True)
        sys.exit(1)

    try:
        while True:
            conn, addr = server.accept()
            # Handle each client in its own thread
            client_thread = threading.Thread(target=handle_client, args=(conn,), daemon=True)
            client_thread.start()
    except KeyboardInterrupt:
        logger.info("Shutting down.")
    finally:
        server.close()


if __name__ == "__main__":
    main()
