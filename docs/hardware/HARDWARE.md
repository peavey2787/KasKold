[KasKold](../../README.md) › [Documentation](../README.md) › Hardware

# KasKold Hardware

KasKold Hardware uses the M5Stack CoreS3 family. Retired board ports are not shipped as supported targets.

| Target | Build support | Production qualification |
|---|---|---|
| M5Stack CoreS3 | Yes | **Hardware-qualified** |
| M5Stack CoreS3 Lite | Yes; shared CoreS3 adapter/profile | **Pending physical HIL qualification** |

CoreS3 Lite intentionally shares the CoreS3 hardware implementation through a declarative board profile rather than duplicating peripheral code. Until a physical CoreS3 Lite completes the qualification checklist, production/release tooling must not represent it as a qualified release target.

## Qualification checklist

Physical qualification covers boot, display, touch, camera/QR scanning, QR display, SD, RNG/entropy inputs, seed generation, signing, backup/restore, Secure Boot/provisioning paths, power/reboot behavior, and firmware update.

## References

- [M5Stack CoreS3 documentation](https://docs.m5stack.com/en/core/CoreS3)
- [M5Stack CoreS3 Lite documentation](https://docs.m5stack.com/en/core/CoreS3-Lite)
