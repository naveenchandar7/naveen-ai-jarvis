from __future__ import annotations

from dataclasses import dataclass, field
from typing import Mapping


Permission = str
CapabilityId = str
DeviceRequirement = str


@dataclass(frozen=True)
class CapabilityDescriptor:
    """Provider/device-independent metadata for a callable capability."""

    capability_id: CapabilityId
    input_schema: Mapping[str, object] = field(default_factory=dict)
    output_schema: Mapping[str, object] = field(default_factory=dict)
    permissions: tuple[Permission, ...] = ()
    risk: str = "low"
    device_requirements: tuple[DeviceRequirement, ...] = ()
    adapter_id: str = ""
    verification: str = "required"

    def __post_init__(self) -> None:
        if not self.capability_id.strip():
            raise ValueError("capability id must not be empty")
        if not self.risk.strip():
            raise ValueError("capability risk must not be empty")
        if not self.verification.strip():
            raise ValueError("capability verification must not be empty")


class CapabilityRegistry:
    """Registry for capability contracts, independent of execution adapters."""

    def __init__(self) -> None:
        self._items: dict[CapabilityId, CapabilityDescriptor] = {}

    def register(self, descriptor: CapabilityDescriptor) -> None:
        capability_id = descriptor.capability_id.strip()
        if capability_id != descriptor.capability_id:
            raise ValueError("capability id must not contain surrounding whitespace")
        if capability_id in self._items:
            raise ValueError(f"capability already registered: {capability_id}")
        self._items[capability_id] = descriptor

    def get(self, capability_id: CapabilityId) -> CapabilityDescriptor:
        try:
            return self._items[capability_id]
        except KeyError as error:
            raise KeyError(
                f"capability is not registered: {capability_id}"
            ) from error

    def names(self) -> tuple[CapabilityId, ...]:
        return tuple(sorted(self._items))

    def all(self) -> tuple[CapabilityDescriptor, ...]:
        return tuple(self._items[name] for name in self.names())

    def contains(self, capability_id: CapabilityId) -> bool:
        return capability_id in self._items
