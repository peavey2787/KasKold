"""Canonical firmware feature combinations used by build and lint validation."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class FirmwareBuild:
    features: str
    environment: tuple[tuple[str, str], ...] = ()

    def env_overrides(self) -> dict[str, str]:
        return dict(self.environment)


PSRAM_OCTAL = (("ESP_HAL_CONFIG_PSRAM_MODE", "octal"),)

FEATURE_MATRIX = (
    FirmwareBuild("m5stack"),
    FirmwareBuild("m5stack,silent"),
    FirmwareBuild("m5stack,production"),
    # CoreS3 Lite reuses the CoreS3 adapter but remains non-production until
    # physical HIL qualification is completed.
    FirmwareBuild("m5stack-lite"),
    FirmwareBuild("m5stack-lite,silent"),
    # Developer/QA diagnostics are compile-tested but feature_policy.rs keeps
    # them out of silent/production release images.
    FirmwareBuild("m5stack,sentinel-scan"),
    FirmwareBuild("m5stack,e12-capture"),
    FirmwareBuild("m5stack,rng-probe"),
    FirmwareBuild("m5stack,wdev-capture"),
    FirmwareBuild("m5stack,sha-bench"),
    FirmwareBuild("m5stack,argon2-bench"),
    FirmwareBuild("m5stack,icon-browser"),
    FirmwareBuild("m5stack,boot-kats-full"),
    FirmwareBuild("m5stack,workflow-tests"),
    FirmwareBuild("m5stack-lite,workflow-tests"),
    FirmwareBuild("m5stack,workflow-test-auto"),
    FirmwareBuild("m5stack-lite,workflow-test-auto"),
    FirmwareBuild("qemu"),
    FirmwareBuild("qemu-tests"),
)
