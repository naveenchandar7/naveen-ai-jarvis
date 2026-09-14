from __future__ import annotations

import unittest

from capability_registry import CapabilityDescriptor, CapabilityRegistry


class CapabilityRegistryTests(unittest.TestCase):
    def test_register_and_get_descriptor(self):
        descriptor = CapabilityDescriptor(
            capability_id="demo.read",
            input_schema={"type": "object"},
            output_schema={"type": "string"},
            permissions=("filesystem.read",),
            risk="medium",
            device_requirements=("filesystem",),
            adapter_id="host.filesystem.read_text",
            verification="content-returned",
        )
        registry = CapabilityRegistry()

        registry.register(descriptor)

        self.assertIs(registry.get("demo.read"), descriptor)
        self.assertTrue(registry.contains("demo.read"))
        self.assertEqual(registry.names(), ("demo.read",))
        self.assertEqual(registry.all(), (descriptor,))

    def test_duplicate_capability_is_rejected(self):
        registry = CapabilityRegistry()
        registry.register(CapabilityDescriptor(capability_id="same"))

        with self.assertRaises(ValueError):
            registry.register(CapabilityDescriptor(capability_id="same"))

    def test_missing_capability_is_rejected(self):
        registry = CapabilityRegistry()

        with self.assertRaises(KeyError):
            registry.get("missing")

    def test_empty_metadata_is_rejected(self):
        with self.assertRaises(ValueError):
            CapabilityDescriptor(capability_id="   ")

        with self.assertRaises(ValueError):
            CapabilityDescriptor(capability_id="demo", risk="   ")

        with self.assertRaises(ValueError):
            CapabilityDescriptor(capability_id="demo", verification="   ")

    def test_capability_id_whitespace_is_rejected(self):
        registry = CapabilityRegistry()

        with self.assertRaises(ValueError):
            registry.register(CapabilityDescriptor(capability_id=" demo "))

    def test_names_are_sorted_and_all_is_deterministic(self):
        registry = CapabilityRegistry()
        registry.register(CapabilityDescriptor(capability_id="z.capability"))
        registry.register(CapabilityDescriptor(capability_id="a.capability"))

        self.assertEqual(
            registry.names(),
            ("a.capability", "z.capability"),
        )
        self.assertEqual(
            tuple(item.capability_id for item in registry.all()),
            ("a.capability", "z.capability"),
        )


if __name__ == "__main__":
    unittest.main()
