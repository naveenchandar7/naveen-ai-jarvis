from __future__ import annotations

from dataclasses import dataclass

SYSTEM_TELEMETRY_READ = "system.telemetry.read"


@dataclass(frozen=True)
class CapabilityDescriptor:
    capability_id: str
    risk: str
    description: str


class CapabilityCatalog:
    def __init__(self) -> None:
        self._items = {
            SYSTEM_TELEMETRY_READ: CapabilityDescriptor(
                capability_id=SYSTEM_TELEMETRY_READ,
                risk="low",
                description="Read current host system telemetry.",
            )
        }

    def get(self, capability_id: str) -> CapabilityDescriptor | None:
        return self._items.get(capability_id)

    def all(self) -> tuple[CapabilityDescriptor, ...]:
        return tuple(self._items.values())
