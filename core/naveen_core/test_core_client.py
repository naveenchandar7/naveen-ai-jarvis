from __future__ import annotations

import io
import struct
import unittest

from core_client import (
    AUTH_PROTOCOL_VERSION,
    CLIENT_ID,
    CORE_HEALTH_PAYLOAD,
    IPC_PROTOCOL_VERSION,
    CoreClient,
    canonical_request_material,
    challenge_transcript,
    decode_frame,
    encode_frame,
    message_proof,
    session_key,
)


def make_challenge(**overrides):
    value = {
        "kind": "auth_challenge",
        "ipc_protocol_version": IPC_PROTOCOL_VERSION,
        "auth_protocol_version": AUTH_PROTOCOL_VERSION,
        "launch_id": list(range(16)),
        "challenge_id": list(range(16, 32)),
        "nonce": list(range(32)),
        "issued_at_ms": 1_000,
        "expires_at_ms": 31_000,
        "expected_client_id": CLIENT_ID,
        "correlation_id": "auth-1",
    }
    value.update(overrides)
    return value


def read_frame(stream: io.BytesIO) -> bytes:
    header = stream.read(4)
    if len(header) != 4:
        raise AssertionError("missing frame")
    length = struct.unpack(">I", header)[0]
    body = stream.read(length)
    if len(body) != length:
        raise AssertionError("truncated frame")
    return header + body


class CoreClientTests(unittest.TestCase):
    def test_frame_round_trip(self):
        message = {"kind": "request", "protocol_version": 1, "payload": [1, 2, 3]}
        self.assertEqual(decode_frame(encode_frame(message)), message)

    def test_malformed_frame_rejected(self):
        with self.assertRaises(ValueError):
            decode_frame(b"\x00\x00\x00\x10{}")

    def test_challenge_and_session_material_are_deterministic(self):
        c = make_challenge()
        secret = b"S" * 32
        sid = b"I" * 16
        transcript = challenge_transcript(c)
        self.assertTrue(transcript.startswith(b"NAVEEN-AUTH-V1"))
        self.assertEqual(session_key(secret, c, sid), session_key(secret, c, sid))

    def test_message_proof_changes_with_sequence(self):
        key = b"K" * 32
        sid = b"I" * 16
        p1 = message_proof(key, 1, sid, "c", 1, b"digest")
        p2 = message_proof(key, 1, sid, "c", 2, b"digest")
        self.assertNotEqual(p1, p2)

    def test_request_material_binds_method_and_payload(self):
        base = {
            "protocol_version": 1,
            "session_id": list(b"I" * 16),
            "correlation_id": "c",
            "sequence": 1,
            "method": "a",
            "payload": list(b"x"),
        }
        changed = dict(base)
        changed["method"] = "b"
        self.assertNotEqual(canonical_request_material(base), canonical_request_material(changed))
        changed = dict(base)
        changed["payload"] = list(b"y")
        self.assertNotEqual(canonical_request_material(base), canonical_request_material(changed))

    def test_authenticate_and_health_use_live_contract(self):
        c = make_challenge()
        secret = b"S" * 32
        session_id = b"I" * 16
        bootstrap = {
            "kind": "auth_bootstrap",
            "ipc_protocol_version": IPC_PROTOCOL_VERSION,
            "launch_id": c["launch_id"],
            "launch_secret": list(secret),
        }
        auth_session = {
            "kind": "auth_session",
            "ipc_protocol_version": IPC_PROTOCOL_VERSION,
            "auth_protocol_version": AUTH_PROTOCOL_VERSION,
            "session_id": list(session_id),
            "expires_at_ms": 601_000,
            "correlation_id": c["correlation_id"],
        }
        response = {
            "kind": "response",
            "protocol_version": IPC_PROTOCOL_VERSION,
            "correlation_id": "core-health-1",
            "session_id": list(session_id),
            "sequence": 1,
            "status": "ok",
            "error_code": None,
            "payload": list(b"alive"),
            "proof": None,
        }
        reader = io.BytesIO(
            encode_frame(bootstrap)
            + encode_frame(c)
            + encode_frame(auth_session)
            + encode_frame(response)
        )
        writer = io.BytesIO()
        client = CoreClient(reader, writer)
        client.authenticate()
        self.assertEqual(client.session_id, session_id)
        result = client.request_health()
        self.assertEqual(result["payload"], list(b"alive"))

        writer.seek(0)
        outbound_auth = decode_frame(read_frame(writer))
        self.assertEqual(outbound_auth["kind"], "auth_response")
        self.assertEqual(outbound_auth["client_id"], CLIENT_ID)
        self.assertEqual(len(outbound_auth["proof"]), 32)

        outbound_health = decode_frame(read_frame(writer))
        self.assertEqual(outbound_health["method"], "core.health")
        self.assertEqual(outbound_health["payload"], list(CORE_HEALTH_PAYLOAD))
        self.assertEqual(len(outbound_health["proof"]), 32)

    def test_invalid_bootstrap_protocol_is_rejected(self):
        bootstrap = {
            "kind": "auth_bootstrap",
            "ipc_protocol_version": 99,
            "launch_id": list(range(16)),
            "launch_secret": list(b"S" * 32),
        }
        client = CoreClient(io.BytesIO(encode_frame(bootstrap)), io.BytesIO())
        with self.assertRaises(ValueError):
            client.authenticate()

    def test_expired_challenge_shape_is_rejected(self):
        c = make_challenge(expires_at_ms=1_000)
        bootstrap = {
            "kind": "auth_bootstrap",
            "ipc_protocol_version": IPC_PROTOCOL_VERSION,
            "launch_id": c["launch_id"],
            "launch_secret": list(b"S" * 32),
        }
        client = CoreClient(io.BytesIO(encode_frame(bootstrap) + encode_frame(c)), io.BytesIO())
        with self.assertRaises(ValueError):
            client.authenticate()

    def test_response_correlation_mismatch_is_rejected(self):
        client = CoreClient(io.BytesIO(), io.BytesIO())
        client.session_id = b"I" * 16
        with self.assertRaises(ValueError):
            client._validate_response(
                {
                    "kind": "response",
                    "protocol_version": IPC_PROTOCOL_VERSION,
                    "correlation_id": "wrong",
                    "session_id": list(client.session_id),
                    "sequence": 1,
                    "status": "ok",
                    "error_code": None,
                    "payload": list(b"alive"),
                },
                "expected",
                1,
            )


if __name__ == "__main__":
    unittest.main()
