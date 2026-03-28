"""
offline_intelligence - Python bindings for Offline Intelligence Library
Version: 0.1.4

Pure-Python HTTP client that communicates with the Offline Intelligence
server (default port 9999). Supports streaming via Server-Sent Events.
"""

from __future__ import annotations

import os
import json
from typing import Any, Dict, Generator, Iterator, Optional

try:
    import requests
    from requests import Response
except ImportError as e:
    raise ImportError(
        "The 'requests' package is required. Install it with: pip install requests"
    ) from e

__version__ = "0.1.4"
__all__ = ["Config", "OfflineIntelligence", "OfflineIntelligenceException"]


# ---------------------------------------------------------------------------
# Exception
# ---------------------------------------------------------------------------

class OfflineIntelligenceException(Exception):
    """Raised when the Offline Intelligence server returns an error."""
    def __init__(self, message: str, status_code: Optional[int] = None):
        super().__init__(message)
        self.status_code = status_code


# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

class Config:
    """Configuration for the Offline Intelligence HTTP client."""

    def __init__(self):
        self.model_path: str = "default.gguf"
        self.llama_bin: str = "llama-server"
        self.llama_host: str = "127.0.0.1"
        self.llama_port: int = 8081
        self.backend_url: str = "http://127.0.0.1:8081"
        self.openrouter_api_key: str = ""
        self.ctx_size: int = 8192
        self.batch_size: int = 256
        self.threads: int = 6
        self.gpu_layers: int = 20
        self.health_timeout_seconds: int = 60
        self.hot_swap_grace_seconds: int = 25
        self.max_concurrent_streams: int = 4
        self.prometheus_port: int = 9000
        self.api_host: str = "127.0.0.1"
        self.api_port: int = 9999
        self.requests_per_second: int = 24
        self.generate_timeout_seconds: int = 300
        self.stream_timeout_seconds: int = 600
        self.health_check_timeout_seconds: int = 90
        self.queue_size: int = 100
        self.queue_timeout_seconds: int = 30

    @classmethod
    def from_env(cls) -> "Config":
        """Create a Config populated from environment variables."""
        cfg = cls()
        if v := os.getenv("MODEL_PATH"):
            cfg.model_path = v
        if v := os.getenv("LLAMA_BIN"):
            cfg.llama_bin = v
        if v := os.getenv("LLAMA_HOST"):
            cfg.llama_host = v
        if v := os.getenv("LLAMA_PORT"):
            cfg.llama_port = int(v)
        if v := os.getenv("BACKEND_URL"):
            cfg.backend_url = v
        if v := os.getenv("OPENROUTER_API_KEY"):
            cfg.openrouter_api_key = v
        if v := os.getenv("API_HOST"):
            cfg.api_host = v
        if v := os.getenv("API_PORT"):
            cfg.api_port = int(v)
        if v := os.getenv("CTX_SIZE"):
            cfg.ctx_size = int(v)
        if v := os.getenv("GPU_LAYERS"):
            cfg.gpu_layers = int(v)
        return cfg

    def __repr__(self) -> str:
        return (
            f"Config(api_host={self.api_host!r}, api_port={self.api_port}, "
            f"model_path={self.model_path!r})"
        )


# ---------------------------------------------------------------------------
# Main Client
# ---------------------------------------------------------------------------

class OfflineIntelligence:
    """
    HTTP client for the Offline Intelligence server.

    Usage::

        from offline_intelligence import Config, OfflineIntelligence

        cfg = Config.from_env()
        client = OfflineIntelligence(cfg)

        print(client.health_check())
        print(client.generate("Hello, world!"))
    """

    def __init__(self, config: Optional[Config] = None):
        self.config = config or Config()
        self.base_url = f"http://{self.config.api_host}:{self.config.api_port}"
        self._session = requests.Session()
        self._session.headers.update({"Content-Type": "application/json"})

    def _get(self, path: str, timeout: Optional[int] = None) -> Any:
        url = f"{self.base_url}{path}"
        try:
            resp = self._session.get(url, timeout=timeout or self.config.health_timeout_seconds)
            resp.raise_for_status()
            return resp.json()
        except requests.HTTPError as e:
            raise OfflineIntelligenceException(str(e), e.response.status_code if e.response else None)
        except requests.RequestException as e:
            raise OfflineIntelligenceException(str(e))

    def _post(self, path: str, body: Optional[Dict] = None, timeout: Optional[int] = None) -> Any:
        url = f"{self.base_url}{path}"
        try:
            resp = self._session.post(url, json=body, timeout=timeout or self.config.generate_timeout_seconds)
            resp.raise_for_status()
            return resp.json()
        except requests.HTTPError as e:
            raise OfflineIntelligenceException(str(e), e.response.status_code if e.response else None)
        except requests.RequestException as e:
            raise OfflineIntelligenceException(str(e))

    def _delete(self, path: str, timeout: Optional[int] = None) -> Any:
        url = f"{self.base_url}{path}"
        try:
            resp = self._session.delete(url, timeout=timeout or self.config.health_timeout_seconds)
            resp.raise_for_status()
            return resp.json()
        except requests.HTTPError as e:
            raise OfflineIntelligenceException(str(e), e.response.status_code if e.response else None)
        except requests.RequestException as e:
            raise OfflineIntelligenceException(str(e))

    # ── Health & Status ────────────────────────────────────────────────────

    def health_check(self) -> Dict:
        """GET /healthz — returns server health status."""
        return self._get("/healthz")

    def get_status(self) -> Dict:
        """GET /admin/status — returns engine/model status."""
        return self._get("/admin/status")

    # ── Model Management ───────────────────────────────────────────────────

    def load_model(self, model_path: str) -> Dict:
        """POST /admin/load — load a model by path."""
        return self._post("/admin/load", {"model_path": model_path})

    def stop_model(self) -> Dict:
        """POST /admin/stop — stop the running model."""
        return self._post("/admin/stop")

    # ── Generation ─────────────────────────────────────────────────────────

    def generate(self, prompt: str, **options: Any) -> Dict:
        """POST /generate — generate a response (non-streaming)."""
        body = {"prompt": prompt, **options}
        return self._post("/generate", body, timeout=self.config.generate_timeout_seconds)

    def generate_stream(self, prompt: str, **options: Any) -> Generator[str, None, None]:
        """
        POST /generate/stream — stream a response via Server-Sent Events.

        Yields each text chunk as a string.

        Example::

            for chunk in client.generate_stream("Tell me a story"):
                print(chunk, end="", flush=True)
        """
        url = f"{self.base_url}/generate/stream"
        body = {"prompt": prompt, **options}
        try:
            with self._session.post(
                url,
                json=body,
                stream=True,
                timeout=self.config.stream_timeout_seconds,
            ) as resp:
                resp.raise_for_status()
                for line in resp.iter_lines(decode_unicode=True):
                    if not line:
                        continue
                    if line.startswith("data: "):
                        data = line[6:]
                        if data.strip() == "[DONE]":
                            return
                        try:
                            payload = json.loads(data)
                            # OpenAI-compatible format
                            if "choices" in payload:
                                delta = payload["choices"][0].get("delta", {})
                                content = delta.get("content", "")
                                if content:
                                    yield content
                            elif "text" in payload:
                                yield payload["text"]
                        except json.JSONDecodeError:
                            yield data
        except requests.HTTPError as e:
            raise OfflineIntelligenceException(str(e), e.response.status_code if e.response else None)
        except requests.RequestException as e:
            raise OfflineIntelligenceException(str(e))

    # ── Conversations ──────────────────────────────────────────────────────

    def get_conversations(self) -> Dict:
        """GET /conversations — list all conversations."""
        return self._get("/conversations")

    def get_conversation(self, conversation_id: str) -> Dict:
        """GET /conversations/{id} — get a single conversation."""
        return self._get(f"/conversations/{conversation_id}")

    def delete_conversation(self, conversation_id: str) -> Dict:
        """DELETE /conversations/{id} — delete a conversation."""
        return self._delete(f"/conversations/{conversation_id}")

    def get_conversation_title(self, conversation_id: str) -> Dict:
        """GET /conversations/{id}/title — get conversation title."""
        return self._get(f"/conversations/{conversation_id}/title")

    def generate_title(self, session_id: str, first_message: str) -> Dict:
        """POST /generate/title — generate a title for a conversation."""
        return self._post("/generate/title", {
            "session_id": session_id,
            "first_message": first_message,
        })

    # ── Memory ─────────────────────────────────────────────────────────────

    def get_memory_stats(self, session_id: str) -> Dict:
        """GET /memory/stats/{session_id} — get memory statistics."""
        return self._get(f"/memory/stats/{session_id}")

    def optimize_memory(self) -> Dict:
        """POST /memory/optimize — trigger memory optimization."""
        return self._post("/memory/optimize")

    def cleanup_memory(self) -> Dict:
        """POST /memory/cleanup — clean up stale memory entries."""
        return self._post("/memory/cleanup")

    # ── Version ────────────────────────────────────────────────────────────

    @staticmethod
    def version() -> str:
        return __version__

    def __repr__(self) -> str:
        return f"OfflineIntelligence(base_url={self.base_url!r})"
