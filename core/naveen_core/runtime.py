from __future__ import annotations

import json
import uuid
from typing import Any

from contracts import CapabilityResult
from orchestrator import Orchestrator


class CoreRuntime:
    def __init__(self, client: Any, orchestrator: Orchestrator) -> None:
        self.client = client
        self.orchestrator = orchestrator
        self._event_counter = 0

    def start(self) -> None:
        self.client.set_event_handler(self.handle_event)
        self._send_status("ready")

    def handle_event(self, event: dict[str, Any]) -> None:
        event_type = event["event_type"]
        payload_raw = event["payload"]
        try:
            payload = json.loads(payload_raw)
        except (TypeError, ValueError, json.JSONDecodeError):
            self._send_error("", "malformed host event payload")
            return

        if event_type == "core.input.text":
            self._handle_text(payload)
        elif event_type == "capability.result":
            return
        else:
            correlation_id = ""
            if isinstance(payload, dict):
                value = payload.get("correlation_id")
                if isinstance(value, str):
                    correlation_id = value
            self._send_error(correlation_id, f"unsupported host event: {event_type}")

    def _handle_text(self, payload: object) -> None:
        if not isinstance(payload, dict):
            self._send_error("", "malformed text input")
            return

        correlation_id = payload.get("correlation_id")
        text = payload.get("text")
        if (
            not isinstance(correlation_id, str)
            or not correlation_id
            or len(correlation_id) > 128
            or not isinstance(text, str)
            or not text.strip()
            or len(text) > 8192
        ):
            self._send_error(
                correlation_id if isinstance(correlation_id, str) else "",
                "malformed text input",
            )
            return

        self._send_status("processing", correlation_id)
        try:
            response = self.orchestrator.handle_text(
                correlation_id,
                text,
                self.request_capability,
            )
            self.client.send_event(
                self._next_id("response"),
                "core.response",
                {
                    "correlation_id": response.correlation_id,
                    "status": "ok",
                    "message": response.message,
                    "data": response.data,
                },
            )
        except Exception:  # noqa: BLE001
            self._send_error(correlation_id, "core request failed")
        finally:
            self._send_status("ready", correlation_id)

    def request_capability(
        self,
        capability_id: str,
        input_data: dict[str, Any],
    ) -> dict[str, Any]:
        request_id = str(input_data.get("request_id") or self._next_id("capability"))
        self.client.send_event(
            self._next_id("capability-request"),
            "capability.request",
            {
                "request_id": request_id,
                "capability_id": capability_id,
                "input": input_data,
            },
        )

        result_event = self.client.wait_for_event(
            "capability.result",
            predicate=lambda event: self._event_request_id(event) == request_id,
            timeout=10,
        )
        payload = json.loads(result_event["payload"])
        if not isinstance(payload, dict):
            raise ValueError("malformed capability result")

        result = CapabilityResult(
            request_id=request_id,
            capability_id=str(payload.get("capability_id", capability_id)),
            ok=payload.get("ok") is True,
            output=payload.get("output") if isinstance(payload.get("output"), dict) else {},
            error=payload.get("error") if isinstance(payload.get("error"), str) else None,
        )
        if not result.ok:
            raise RuntimeError(result.error or "capability denied")
        return result.output

    @staticmethod
    def _event_request_id(event: dict[str, Any]) -> str:
        try:
            value = json.loads(event["payload"])
        except (TypeError, ValueError, json.JSONDecodeError):
            return ""
        if not isinstance(value, dict):
            return ""
        request_id = value.get("request_id")
        return request_id if isinstance(request_id, str) else ""

    def _send_status(self, state: str, correlation_id: str = "") -> None:
        self.client.send_event(
            self._next_id("status"),
            "core.status",
            {"state": state, "correlation_id": correlation_id},
        )

    def _send_error(self, correlation_id: str, message: str) -> None:
        self.client.send_event(
            self._next_id("error"),
            "core.error",
            {"correlation_id": correlation_id, "message": message},
        )

    def _next_id(self, prefix: str) -> str:
        self._event_counter += 1
        return f"{prefix}-{self._event_counter}-{uuid.uuid4().hex[:8]}"
