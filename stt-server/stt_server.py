#!/usr/bin/env python3
"""
Minimal local speech-to-text server for og-voice.

POST /transcribe with a raw WAV file in the request body -> {"text": "..."}.
GET  /health                                              -> {"ok": true}

Uses faster-whisper (CTranslate2-based Whisper reimplementation, much
faster than openai-whisper on CPU) with a small/tiny model by default so
a 1.5s audio chunk transcribes well under the chunk interval on modest
hardware. Model size/device are overridable via env vars so this can be
pointed at a GPU or a bigger model without editing code.

Kept to stdlib http.server + faster-whisper (no FastAPI/uvicorn) — one
endpoint, one method, no routing/validation complexity that would
justify a whole ASGI framework. See scripts/install-stt.sh for the venv
this runs in.
"""
import io
import json
import os
import sys
import tempfile
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

try:
    from faster_whisper import WhisperModel
except ImportError:
    print(
        "faster-whisper not installed — run scripts/install-stt.sh first "
        "(this script must be run from the venv it creates).",
        file=sys.stderr,
    )
    sys.exit(1)

HOST = os.environ.get("OG_VOICE_STT_HOST", "127.0.0.1")
PORT = int(os.environ.get("OG_VOICE_STT_PORT", "8765"))
MODEL_SIZE = os.environ.get("OG_VOICE_STT_MODEL", "small.en")
DEVICE = os.environ.get("OG_VOICE_STT_DEVICE", "cpu")
COMPUTE_TYPE = os.environ.get("OG_VOICE_STT_COMPUTE", "int8")

print(f"og-voice STT server: loading model '{MODEL_SIZE}' ({DEVICE}/{COMPUTE_TYPE})…", file=sys.stderr)
_model = WhisperModel(MODEL_SIZE, device=DEVICE, compute_type=COMPUTE_TYPE)
_model_lock = threading.Lock()
print(f"og-voice STT server: ready on http://{HOST}:{PORT}", file=sys.stderr)


class Handler(BaseHTTPRequestHandler):
    # Quiet by default — one line per chunk at info level would spam
    # stderr every ~1.5s while recording; flip this to True while
    # debugging.
    def log_message(self, fmt, *args):
        if os.environ.get("OG_VOICE_STT_VERBOSE"):
            super().log_message(fmt, *args)

    def _send_json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/health":
            self._send_json(200, {"ok": True})
        else:
            self._send_json(404, {"error": "not found"})

    def do_POST(self):
        if self.path != "/transcribe":
            self._send_json(404, {"error": "not found"})
            return

        length = int(self.headers.get("Content-Length", "0"))
        if length <= 0:
            self._send_json(400, {"error": "empty body"})
            return
        wav_bytes = self.rfile.read(length)

        # faster-whisper wants a path or file-like object it can seek on;
        # a real temp file is the simplest thing that's definitely
        # seekable (an in-memory BytesIO works too, but a temp file also
        # makes a failed chunk trivially inspectable during debugging).
        try:
            with tempfile.NamedTemporaryFile(suffix=".wav") as tmp:
                tmp.write(wav_bytes)
                tmp.flush()
                # Model isn't documented thread-safe for concurrent
                # .transcribe() calls; og-voice only ever has one chunk
                # in flight at a time anyway (chunks are sent
                # sequentially, not pipelined), so this lock is just
                # cheap insurance against a future caller that isn't.
                with _model_lock:
                    segments, _info = _model.transcribe(tmp.name, beam_size=1)
                    text = "".join(seg.text for seg in segments).strip()
            self._send_json(200, {"text": text})
        except Exception as e:  # noqa: BLE001 - want any failure reported, not crash the server
            self._send_json(500, {"error": str(e)})


def main():
    server = ThreadingHTTPServer((HOST, PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
