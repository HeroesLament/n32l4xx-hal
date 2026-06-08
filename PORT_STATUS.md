# n32l4xx-hal port status

Living document tracking the n32g4xx-hal -> n32l4xx-hal port. Update as
modules land.

## Done

- Fork created from `n32-rs` (guineawheek n32g4xx-hal, 0BSD).
- `Cargo.toml` retargeted: name -> `n32l4xx-hal`, `n32g4` dep -> `n32l4`
  path dep, device features -> `n32l403` / `n32l406`.
- `lib.rs` PAC re-export arms point at `n32l4::n32l40x`; `bkp` module gated
  off pending port; zero `n32g4` references remain.
- Toolchain pinned to `nightly-2026-06-07` (rust-toolchain.toml).

## Baseline build error surface

`cargo build --features n32l403 --target thumbv7em-none-eabihf`: **717 errors.**

By module:
| module                | errors | notes |
|-----------------------|-------:|-------|
| gpio/alt + alt/altmap |    366 | chip-specific AF pin-mux tables (>half the total) |
| adc                   |    135 | |
| rcc/enable            |    117 | `bus!` macro: peripheral->bus->enable-bit maps |
| afio                  |     89 | |
| gpio/convert + gpio   |     92 | |
| pwm                   |     40 | |
| i2c                   |     33 | not needed by firmware |
| rcc/mod               |     28 | |
| spi                   |     27 | |
| timer                 |     22 | |
| serial               |     21 | not needed by firmware |
| can                   |     11 | |
| (dma/pwr/usb/sac/...) | small  | not needed by firmware |

By error code: 432 E0425 (name resolution), 147 E0592 (dup defs, mostly
gpio/alt), 60 E0599 (no method), 44 E0432 + 33 E0433 (unresolved paths).

## Current state (after PAC svd2rust 0.37.1 bump + RCC.CFG enums)

`cargo build --features n32l403 --target thumbv7em-none-eabihf`: **518 errors**
(down from 717).

ROOT CAUSE of the bulk of the baseline errors was the PAC identifier shape, not
just missing enums. The HAL targets the n32g4 shape (`Sclksw`/`SclkswR`/`Rcc`),
which is the svd2rust **0.32+** default theme; the PAC had been generated with
0.31.5, which emits the legacy `SCLKSW_A`/`SCLKSW_R`/`RCC` shape. Bumping the
PAC generator to 0.37.1 fixed the naming fleet-wide AND gave PascalCase
peripheral names (so the `RCC`->`Rcc` casing item below is resolved with no
separate patch). The RCC.CFG enum enrichment (Sclksw/Ahbpres/Apb1pres) landed
in the same PAC pass.

Error-code shift (the healthy kind): E0425 432->209 (naming fixed), E0432
44->13, E0433 33->11. E0599 60->148 and a new 23x E0308 appeared -- expected:
names now resolve, so the compiler reaches method-call/type checking and
surfaces the real per-module API porting work.

By module (error-line references, counts overlap): gpio 396 (the AF-table
bulk), adc 131, afio 84, **rcc 69** (down from 145 = enable 117 + mod 28), pwm
34, i2c 28, serial 19, spi 16, can 14, dma 12.

Next: rcc/mod + rcc/enable residual (the `bus!` enable-bit maps and method
diffs -- no longer an enum problem), then the gpio AF-table trim, then the
cold-path peripherals.

## Port plan (dependency order)

1. **rcc** (mod + enable) — dependency root for everything else. IN PROGRESS.
2. **spi / can / timer / pwm / adc** — the firmware cold-path peripherals.
3. **gpio** — needed, but AF tables (`alt.rs`/`altmap.rs`, the 366-error bulk)
   can be trimmed to just the pins the firmware muxes (encoder SPI, bridge
   TIM channels, CAN) rather than every package pin.
4. **Defer / leave stubbed:** afio (beyond what gpio needs), i2c, serial, usb,
   sac (crypto), dma. Firmware does not use these.

## Key finding (RESOLVED): the HAL assumes an enum-enriched PAC in the n32g4 shape

The n32g4 HAL matches on PAC field enums (`Sclksw::Pll`, `Ahbpres::Div2`,
etc.). The N32L4 vendor SVD has NO `<enumeratedValues>`, so svd2rust emits
bare field accessors and those enums don't exist.

**Resolved by enriching the SVD at the PAC layer (Option 1)**, not raw-bits in
the HAL. Keeps the HAL upstream-shaped. This recurs across modules (timer
modes, spi cpol/baud, can bit timing) -- handle it the same way each time.

**Enum values are SDK-sourced, not n32g4-copied.** The variant *names* come
from the n32g4 PAC but the *values* are taken from the vendor SDK
(`n32l40-sdk/.../inc/n32l40x_rcc.h`), because the encodings are NOT always
identical:
- AHBPRES does match: SDK `RCC_SYSCLK_DIV2=0x80` >>4 => field 8 == n32g4
  `Ahbpres::Div2=8`. APB1PRES likewise.
- SCLKSW does NOT: N32L4 is `Msi=0/Hsi=1/Hse=2/Pll=3` (it has an MSI source);
  n32g4 is `Hsi=0/Hse=1/Pll=2`. Copying n32g4 values would have made the HAL
  select the wrong clock source on silicon. The earlier "N32L4 and N32G4 share
  the clock-register layout, copy n32g4 values" assumption was WRONG for this
  field.

## rcc/mod.rs — remaining error families (after enum enrichment)

- Field enums: `Sclksw`, `Ahbpres`, `Apb1pres` (APB2PRES derivedFrom APB1PRES)
  — DONE via the PAC svdtools enrichment.
- Peripheral-name casing (`Rcc`/`Flash`/`Tim1`): DONE -- svd2rust 0.37.1
  emits PascalCase peripheral names natively, so no casing patch was needed.
- `SYSCLK_MAX` const: HAL value defined only under n32g4 cfg arms. Still needs
  an n32l403/n32l406 definition (~64 MHz per N32L40x spec). OPEN.
