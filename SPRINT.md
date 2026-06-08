# HAL port sprint

Goal: **full** `n32l4xx-hal` compile for both `n32l403` and `n32l406` against
the svd2rust-0.37.1 PAC. Every module builds -- not just the firmware-critical
subset. Tracks the milestone burn-down from the 518-error post-PAC-bump
baseline (see PORT_STATUS.md for how 717 -> 518 happened).

Recurring technique: where the HAL matches on PAC field enums the raw SVD
lacks, enrich at the PAC layer (svdtools stage 2), NOT raw-bits in the HAL --
same as rcc. Enum VALUES come from the vendor SDK headers
(`n32l40-sdk/.../inc/`), names from the n32g4 PAC; the two do NOT always agree
(see the SCLKSW divergence), so cross-check every field, never blind-copy.

## Baseline (start of sprint) -- CORRECTED METRIC

656 true errors (compiler `--message-format=json` count; earlier grep-based
"518" was an undercount). By module:
gpio 282, adc 235, pwm 58, serial 39, spi 16, can 14, dma 6, i2c 3, fmc 2,
sac 1. rcc 0 (DONE), afio 0 (cleared by the naming bump).
By code: E0592 238 (gpio AF dup-defs), E0425 200 (HAL assumes bigger device),
E0599 172, E0308 22, E0432 13, E0433 11.

## Milestones (revised against the corrected baseline)

- [x] **M1 rcc** -- DONE. SYSCLK_MAX const added for n32l403/n32l406 (64 MHz);
  enable.rs + mod.rs now error-free (only cosmetic n32g4 cfg-value warnings).
- [x] **M2 afio** -- DONE with no work; the svd2rust 0.37.1 naming bump
  cleared it.
- [ ] **M3 gpio (282)** -- the big one. Bulk is E0592 dup-defs in
  alt.rs/altmap.rs plus E0425 for ports/pins the N32L403 lacks (PE/PF). FULL
  port: every package pin the device actually has; remove AF entries for
  pins/peripherals not present (Spi3, Uart6/7, etc.).
- [ ] **M4 adc (235)** -- larger than first thought. HAL assumes Adc1..Adc4;
  N32L4 has a single ADC. Trim to the real device + enum-enrich as needed.
- [ ] **M5 cold-path: pwm (58), spi (16), can (14)** -- firmware-critical.
  Enum pattern recurs (timer modes, spi cpol/baud, can bit timing).
- [ ] **M6 remaining: serial (39), dma (6), i2c (3), fmc (2), sac (1), usb**
  -- not firmware-used but required for a full HAL compile.

Note: gpio + adc = 517 of 656. Both stem from the HAL assuming a larger
device than N32L403/406. That device-shape trim is the heart of the port.

## Exit criteria

`cargo build --features n32l403 --target thumbv7em-none-eabihf` AND
`--features n32l406` both succeed with zero errors. Then the firmware can
uncomment its `n32l4` / `n32l4xx-hal` path deps.

## Notes / decisions as they land

(append here)
