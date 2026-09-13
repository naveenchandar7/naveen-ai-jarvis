from __future__ import annotations

import hashlib
import hmac
import json
import struct
from dataclasses import dataclass
from typing import BinaryIO

IPC_PROTOCOL_VERSION = 1
AUTH_PROTOCOL_VERSION = 1
AUTH_PROOF_LEN = 32
SESSION_ID_LEN = 16
MAX_FRAME = 1024 * 1024
MAX_PAYLOAD = 512 * 1024
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


def challenge_transcript(challenge: dict) -> bytes:
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


def session_key(secret: bytes, challenge: dict, session_id: bytes) -> bytes:
    material = SESSION_DOMAIN + _bytes(challenge_transcript(challenge)) + _bytes(session_id)
    return hmac.new(secret, material, hashlib.sha256).digest()


def canonical_request_material(request: dict) -> bytes:
    out = bytearray(IPC_DOMAIN)
    out.append(1)  # request kind, matching Rust's canonical message material
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


def encode_frame(message: dict) -> bytes:
    body = json.dumps(message, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    if not body or len(body) > MAX_FRAME:
        raise ValueError("frame too large")
    return _u32(len(body)) + body


def decode_frame(frame: bytes) -> dict:
    if len(frame) < 4:
        raise ValueError("truncated length prefix")
    length = struct.unpack(">I", frame[:4])[0]
    if length == 0 or length > MAX_FRAME or len(frame) != length + 4:
        raise ValueError("invalid frame length")
    message = json.loads(frame[4:])
    if not isinstance(message, dict):
        raise ValueError("invalid message")
    return message


def _as_bytes(value: object, expected_length: int, field: str) -> bytes:
    try:
        result = bytes(value)  # type: ignore[arg-type]
    except (TypeError, ValueError):
        raise ValueError(f"malformed {field}") from None
    if len(result) != expected_length:
        raise ValueError(f"malformed {field}")
    return result


@dataclass
class CoreClient:
    reader: BinaryIO
    writer: BinaryIO
    secret: bytes | None = None
    launch_id: bytes | None = None
    session_id: bytes | None = None
    session_key_bytes: bytes | None = None
    sequence: int = 0

    def read_message(self) -> dict:
        header = self._read_exact(4)
        length = struct.unpack(">I", header)[0]
        if length == 0 or length > MAX_FRAME:
            raise ValueError("invalid frame length")
        body = self._read_exact(length)
        message = json.loads(body)
        if not isinstance(message, dict):
            raise ValueError("invalid message")
        return message

    def send_message(self, message: dict) -> None:
        self.writer.write(encode_frame(message))
        self.writer.flush()

    def authenticate(self) -> None:
        bootstrap = self.read_message()
        if bootstrap.get("kind") != "auth_bootstrap":
            raise ValueError("expected auth bootstrap")
        if bootstrap.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")

        self.launch_id = _as_bytes(bootstrap.get("launch_id"), 16, "launch_id")
        self.secret = _as_bytes(bootstrap.get("launch_secret"), 32, "launch_secret")

        challenge = self.read_message()
        if challenge.get("kind") != "auth_challenge":
            raise ValueError("expected auth challenge")
        if challenge.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")
        if challenge.get("auth_protocol_version") != AUTH_PROTOCOL_VERSION:
            raise ValueError("unsupported auth protocol")
        if _as_bytes(challenge.get("launch_id"), 16, "launch_id") != self.launch_id:
            raise ValueError("launch identity mismatch")
        challenge_id = _as_bytes(challenge.get("challenge_id"), 16, "challenge_id")
        nonce = _as_bytes(challenge.get("nonce"), 32, "nonce")
        if not isinstance(challenge.get("expected_client_id"), str):
            raise ValueError("malformed client identity")
        if not isinstance(challenge.get("correlation_id"), str) or not challenge["correlation_id"]:
            raise ValueError("malformed correlation")
        if not isinstance(challenge.get("issued_at_ms"), int) or not isinstance(challenge.get("expires_at_ms"), int):
            raise ValueError("malformed challenge timestamps")
        if challenge["expires_at_ms"] <= challenge["issued_at_ms"]:
            raise ValueError("malformed challenge lifetime")

        challenge["challenge_id"] = list(challenge_id)
        challenge["nonce"] = list(nonce)
        proof = hmac.new(self.secret, challenge_transcript(challenge), hashlib.sha256).digest()
        self.send_message({
            "kind": "auth_response",
            "ipc_protocol_version": IPC_PROTOCOL_VERSION,
            "auth_protocol_version": AUTH_PROTOCOL_VERSION,
            "launch_id": list(self.launch_id),
            "challenge_id": list(challenge_id),
            "client_id": CLIENT_ID,
            "correlation_id": challenge["correlation_id"],
            "proof": list(proof),
        })

        session = self.read_message()
        if session.get("kind") != "auth_session":
            raise ValueError("expected auth session")
        if session.get("ipc_protocol_version") != IPC_PROTOCOL_VERSION:
            raise ValueError("unsupported IPC protocol")
        if session.get("auth_protocol_version") != AUTH_PROTOCOL_VERSION:
            raise ValueError("unsupported auth protocol")
        if session.get("correlation_id") != challenge["correlation_id"]:
            raise ValueError("authentication correlation mismatch")

        self.session_id = _as_bytes(session.get("session_id"), SESSION_ID_LEN, "session_id")
        if not isinstance(session.get("expires_at_ms"), int) or session["expires_at_ms"] <= challenge["issued_at_ms"]:
            raise ValueError("malformed session lifetime")
        self.session_key_bytes = session_key(self.secret, challenge, self.session_id)

    def request_health(self) -> dict:
        if self.session_id is None or self.session_key_bytes is None:
            raise RuntimeError("not authenticated")

        self.sequence += 1
        correlation = f"core-health-{self.sequence}"
        request = {
            "kind": "request",
            "protocol_version": IPC_PROTOCOL_VERSION,
            "correlation_id": correlation,
            "session_id": list(self.session_id),
            "sequence": self.sequence,
            "method": CORE_HEALTH_METHOD,
            "payload": list(CORE_HEALTH_PAYLOAD),
        }
        digest = canonical_request_material(request)
        proof = message_proof(
            self.session_key_bytes,
            IPC_PROTOCOL_VERSION,
            self.session_id,
            correlation,
            self.sequence,
            digest,
        )
        request["proof"] = list(proof)
        self.send_message(request)
        response = self.read_message()
        self._validate_response(response, correlation, self.sequence)
        return response

    def _validate_response(self, response: dict, correlation: str, sequence: int) -> None:
        if response.get("kind") != "response":
            raise ValueError("unexpected response kind")
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
        if bytes(response.get("payload", [])) != b"alive":
            raise ValueError("invalid health response")

    def _read_exact(self, count: int) -> bytes:
        chunks = bytearray()
        while len(chunks) < count:
            chunk = self.reader.read(count - len(chunks))
            if not chunk:
                raise EOFError("IPC closed")
            chunks += chunk
        return bytes(chunks)
PY