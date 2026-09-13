from __future__ import annotations

import hashlib
import hmac
import json
import struct
import threading
import time
from typing import Any, BinaryIO, Callable

IPC_PROTOCOL_VERSION = 1
AUTH_PROTOCOL_VERSION = 1
AUTH_PROOF_LEN = 32
SESSION_ID_LEN = 16
AUTH_LAUNCH_ID_LEN = 16
AUTH_CHALLENGE_ID_LEN = 16
AUTH_CHALLENGE_NONCE_LEN = 32
AUTH_SECRET_LEN = 32

IPC_MAX_WIRE_BODY_SIZE = 1024 * 1024
IPC_MAX_PAYLOAD_SIZE = 512 * 1024
IPC_MAX_CORRELATION_ID_LEN = 128
IPC_MAX_EVENT_ID_LEN = 128
IPC_MAX_EVENT_TYPE_LEN = 128
IPC_MAX_METHOD_LEN = 128

CLIENT_ID = "naveen-ai-core"
CORE_HEALTH_METHOD = "core.health"
CORE_HEALTH_PAYLOAD = b"heartbeat"

AUTH_DOMAIN = b"NAVEEN-AUTH-V1"
SESSION_DOMAIN = b"NAVEEN-SESSION-V1"
MESSAGE_DOMAIN = b"NAVEEN-MESSAGE-V1"
IPC_DOMAIN = b"NAVEEN-IPC-MESSAGE-V1"


def _u16(value: int) -> bytes:
    return struct.pack(">H", value)


def _u32(value: int) -> bytes:
    return struct.pack(">I", value)


def _u64(value: int) -> bytes:
    return struct.pack(">Q", value)


def _bytes(value: bytes) -> bytes:
    return _u32(len(value)) + value


def _text(value: str) -> bytes:
    return _bytes(value.encode("utf-8"))


def _as_bytes(value: object, expected_length: int, field: str) -> bytes:
    try:
        result = bytes(value)  # type: ignore[arg-type]
    except (TypeError, ValueError):
        raise ValueError(f"malformed {field}") from None
    if len(result) != expected_length:
        raise ValueError(f"malformed {field}")
    return result


def challenge_transcript(challenge: dict[str, Any]) -> bytes:
    out = bytearray(AUTH_DOMAIN)
    out += _u16(challenge["auth_protocol_version"])
    out += _bytes(bytes(challenge["launch_id"]))
    out += _bytes(bytes(challenge["challenge_id"]))
    out += _bytes(bytes(challenge["nonce"]))
    out += _u64(challenge["issued_at_ms"])
    out += _u64(challenge["expires_at_ms"])
    out += _text(challenge["expected_client_id"])
    out += _text(challenge["correlation_id"])
    return bytes(out)


def session_key(secret: bytes, challenge: dict[str, Any], session_id: bytes) -> bytes:
    material = SESSION_DOMAIN + _bytes(challenge_transcript(challenge)) + _bytes(session_id)
    return hmac.new(secret, material, hashlib.sha256).digest()


def canonical_request_material(request: dict[str, Any]) -> bytes:
    out = bytearray(IPC_DOMAIN)
    out.append(1)
    out += _u16(request["protocol_version"])
    out += _bytes(bytes(request["session_id"]))
    out += _text(request["correlation_id"])
    out += _u64(request["sequence"])
    out += _text(request["method"])
    out += _bytes(bytes(request["payload"]))
    return bytes(out)


def message_proof(
    key: bytes,
    protocol_version: int,
    session_id: bytes,
    correlation_id: str,
    sequence: int,
    digest: bytes,
) -> bytes:
    material = bytearray(MESSAGE_DOMAIN)
    material += _u16(protocol_version)
    material += _bytes(session_id)
    material += _text(correlation_id)
    material += _u64(sequence)
    material += _bytes(digest)
    return hmac.new(key, bytes(material), hashlib.sha256).digest()


def encode_frame(message: dict[str, Any]) -> bytes:
    body = json.dumps(message, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    if not body or len(body) > IPC_MAX_WIRE_BODY_SIZE:
        raise ValueError("frame too large")
    return _u32(len(body)) + body


def decode_frame(frame: bytes) -> dict[str, Any]:
    if len(frame) < 4:
        raise ValueError("truncated length prefix")
    length = struct.unpack(">I", frame[:4])[0]
    if length == 0 or length > IPC_MAX_WIRE_BODY_SIZE or len(frame) != length + 4:
        raise ValueError("invalid frame length")
    message = json.loads(frame[4:])
    if not isinstance(message, dict):
        raise ValueError("invalid message")
    return message


class CoreClient:
    def __init__(self, reader: BinaryIO, writer: BinaryIO) -> None:
        self.reader = reader
        self.writer = writer
        self.secret: bytes | None = None
        self.launch_id: bytes | None = None
        self.session_id: bytes | None = None
        self.session_key_bytes: bytes | None = None
        self.sequence = 0
        self.event_sequence = 0
        self._write_lock = threading.Lock()
        self._event_handler: Callable[[dict[str, Any]], None] | None = None

    def set_event_handler(self, handler: Callable[[dict[str, Any]], None]) -> None:
        self._event_handler = handler

    def read_message(self) -> dict[str, Any]:
        header = self._read_exact(4)
        length = struct.unpack(">I", header)[0]
        if length == 0 or length > IPC_MAX_WIRE_BODY_SIZE:
            raise ValueError("invalid frame length")
        body = self._read_exact(length)
        message = json.loads(body)
        if not isinstance(message, dict):
            raise ValueError("invalid message")
        return message

    def send_message(self, message: dict[str, Any]) -> None:
        frame = encode_frame(message)
        with self._write_lock:
            self.writer.write(frame)
            self.writer.flush()

    def authenticate(self) -> None:
        bootstrap = self.read_message()
        if bootstrap.get("kind") != "auth_bootstrap":
            raise ValueError("expected auth bootstrap")
        if bootstrap.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")

        self.launch_id = _as_bytes(bootstrap.get("launch_id"), AUTH_LAUNCH_ID_LEN, "launch_id")
        self.secret = _as_bytes(bootstrap.get("launch_secret"), AUTH_SECRET_LEN, "launch_secret")

        challenge = self.read_message()
        if challenge.get("kind") != "auth_challenge":
            raise ValueError("expected auth challenge")
        if challenge.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")
        if challenge.get("auth_protocol_version") != AUTH_PROTOCOL_VERSION:
            raise ValueError("unsupported auth protocol")
        if _as_bytes(challenge.get("launch_id"), AUTH_LAUNCH_ID_LEN, "launch_id") != self.launch_id:
            raise ValueError("launch identity mismatch")

        challenge_id = _as_bytes(
            challenge.get("challenge_id"), AUTH_CHALLENGE_ID_LEN, "challenge_id"
        )
        nonce = _as_bytes(
            challenge.get("nonce"), AUTH_CHALLENGE_NONCE_LEN, "nonce"
        )

        client_identity = challenge.get("expected_client_id")
        if not isinstance(client_identity, str) or not client_identity:
            raise ValueError("malformed client identity")
        correlation_id = challenge.get("correlation_id")
        if (
            not isinstance(correlation_id, str)
            or not correlation_id
            or len(correlation_id) > IPC_MAX_CORRELATION_ID_LEN
        ):
            raise ValueError("malformed correlation")

        issued_at = challenge.get("issued_at_ms")
        expires_at = challenge.get("expires_at_ms")
        if not isinstance(issued_at, int) or not isinstance(expires_at, int):
            raise ValueError("malformed challenge timestamps")
        if expires_at <= issued_at:
            raise ValueError("malformed challenge lifetime")

        challenge["challenge_id"] = list(challenge_id)
        challenge["nonce"] = list(nonce)
        proof = hmac.new(self.secret, challenge_transcript(challenge), hashlib.sha256).digest()
        self.send_message(
            {
                "kind": "auth_response",
                "ipc_protocol_version": IPC_PROTOCOL_VERSION,
                "auth_protocol_version": AUTH_PROTOCOL_VERSION,
                "launch_id": list(self.launch_id),
                "challenge_id": list(challenge_id),
                "client_id": CLIENT_ID,
                "correlation_id": correlation_id,
                "proof": list(proof),
            }
        )

        session = self.read_message()
        if session.get("kind") != "auth_session":
            raise ValueError("expected auth session")
        if session.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")
        if session.get("auth_protocol_version") != AUTH_PROTOCOL_VERSION:
            raise ValueError("unsupported auth protocol")
        if session.get("correlation_id") != correlation_id:
            raise ValueError("authentication correlation mismatch")

        self.session_id = _as_bytes(session.get("session_id"), SESSION_ID_LEN, "session_id")
        session_expires = session.get("expires_at_ms")
        if not isinstance(session_expires, int) or session_expires <= issued_at:
            raise ValueError("malformed session lifetime")
        self.session_key_bytes = session_key(self.secret, challenge, self.session_id)

    def send_request(self, correlation_id: str, method: str, payload: bytes) -> None:
        if self.session_id is None or self.session_key_bytes is None:
            raise RuntimeError("not authenticated")
        if not correlation_id or len(correlation_id) > IPC_MAX_CORRELATION_ID_LEN:
            raise ValueError("malformed correlation")
        if not method or len(method) > IPC_MAX_METHOD_LEN:
            raise ValueError("malformed method")
        if len(payload) > IPC_MAX_PAYLOAD_SIZE:
            raise ValueError("payload too large")

        self.sequence += 1
        request = {
            "protocol_version": IPC_PROTOCOL_VERSION,
            "correlation_id": correlation_id,
            "session_id": list(self.session_id),
            "sequence": self.sequence,
            "method": method,
            "payload": list(payload),
        }
        digest = canonical_request_material(request)
        proof = message_proof(
            self.session_key_bytes,
            IPC_PROTOCOL_VERSION,
            self.session_id,
            correlation_id,
            self.sequence,
            digest,
        )
        self.send_message(
            {
                "kind": "Request",
                "message": {**request, "proof": list(proof)},
            }
        )

    def send_event(
        self,
        event_id: str,
        event_type: str,
        payload: dict[str, Any] | None = None,
    ) -> None:
        if self.session_id is None:
            raise RuntimeError("not authenticated")
        if not event_id or len(event_id) > IPC_MAX_EVENT_ID_LEN:
            raise ValueError("malformed event id")
        if not event_type or len(event_type) > IPC_MAX_EVENT_TYPE_LEN:
            raise ValueError("malformed event type")

        event_payload = json.dumps(
            payload or {}, separators=(",", ":"), ensure_ascii=False
        ).encode("utf-8")
        if len(event_payload) > IPC_MAX_PAYLOAD_SIZE:
            raise ValueError("event payload too large")

        self.event_sequence += 1
        self.send_message(
            {
                "kind": "Event",
                "message": {
                    "protocol_version": IPC_PROTOCOL_VERSION,
                    "event_id": event_id,
                    "session_id": list(self.session_id),
                    "sequence": self.event_sequence,
                    "event_type": event_type,
                    "payload": list(event_payload),
                    "proof": None,
                },
            }
        )

    def request_health(self) -> dict[str, Any]:
        correlation = f"core-health-{self.sequence + 1}"
        self.send_request(correlation, CORE_HEALTH_METHOD, CORE_HEALTH_PAYLOAD)

        while True:
            message = self.read_message()
            kind = message.get("kind")
            if kind == "Response":
                response = message.get("message")
                if not isinstance(response, dict):
                    raise ValueError("malformed response")
                self._validate_response(response, correlation, self.sequence)
                return response
            if kind == "Event":
                self._dispatch_event(message.get("message"))
                continue
            raise ValueError("unexpected IPC message")

    def wait_for_event(
        self,
        event_type: str,
        *,
        predicate: Callable[[dict[str, Any]], bool] | None = None,
        timeout: float = 10.0,
    ) -> dict[str, Any]:
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"timed out waiting for {event_type}")

            message = self.read_message()
            kind = message.get("kind")
            if kind == "Event":
                event = message.get("message")
                if isinstance(event, dict) and event.get("event_type") == event_type:
                    if predicate is None or predicate(event):
                        self._validate_event(event)
                        return event
                    self._dispatch_event(event)
                else:
                    self._dispatch_event(event)
                continue
            if kind == "Response":
                raise ValueError("unexpected response while waiting for event")
            raise ValueError("unexpected IPC message")

    def _dispatch_event(self, event: object) -> None:
        self._validate_event(event)
        assert isinstance(event, dict)
        payload = bytes(event["payload"])
        envelope = {
            "event_id": event["event_id"],
            "event_type": event["event_type"],
            "sequence": event["sequence"],
            "payload": payload,
        }
        if self._event_handler is not None:
            self._event_handler(envelope)

    def _validate_event(self, event: object) -> None:
        if not isinstance(event, dict):
            raise ValueError("malformed event")
        if self.session_id is None:
            raise RuntimeError("not authenticated")
        if _as_bytes(event.get("session_id"), SESSION_ID_LEN, "session_id") != self.session_id:
            raise ValueError("event session mismatch")

        event_type = event.get("event_type")
        event_id = event.get("event_id")
        sequence = event.get("sequence")
        payload = event.get("payload")
        proof = event.get("proof")

        if not isinstance(event_type, str) or not event_type:
            raise ValueError("malformed event type")
        if not isinstance(event_id, str) or not event_id:
            raise ValueError("malformed event id")
        if not isinstance(sequence, int) or sequence <= 0:
            raise ValueError("malformed event sequence")
        if not isinstance(payload, list):
            raise ValueError("malformed event payload")
        if proof is not None:
            _as_bytes(proof, AUTH_PROOF_LEN, "proof")

        try:
            payload_bytes = bytes(payload)
        except (TypeError, ValueError):
            raise ValueError("malformed event payload") from None
        if len(payload_bytes) > IPC_MAX_PAYLOAD_SIZE:
            raise ValueError("event payload too large")

    def _validate_response(
        self,
        response: dict[str, Any],
        correlation: str,
        sequence: int,
    ) -> None:
        if response.get("protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")
        if _as_bytes(response.get("session_id"), SESSION_ID_LEN, "session_id") != self.session_id:
            raise ValueError("response session mismatch")
        if response.get("correlation_id") != correlation:
            raise ValueError("response correlation mismatch")
        if response.get("sequence") != sequence:
            raise ValueError("response sequence mismatch")
        if response.get("status") != "ok":
            raise ValueError("health request rejected")
        if bytes(response.get("payload") or []) != b"alive":
            raise ValueError("invalid health response")

    def _read_exact(self, count: int) -> bytes:
        chunks = bytearray()
        while len(chunks) < count:
            chunk = self.reader.read(count - len(chunks))
            if not chunk:
                raise EOFError("IPC closed")
            chunks.extend(chunk)
        return bytes(chunks)
