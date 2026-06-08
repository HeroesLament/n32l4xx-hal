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

## Port plan (dependency order)

1. **rcc** (mod + enable) — dependency root for everything else. IN PROGRESS.
2. **spi / can / timer / pwm / adc** — the firmware cold-path peripherals.
3. **gpio** — needed, but AF tables (`alt.rs`/`altmap.rs`, the 366-error bulk)
   can be trimmed to just the pins the firmware muxes (encoder SPI, bridge
   TIM channels, CAN) rather than every package pin.
4. **Defer / leave stubbed:** afio (beyond what gpio needs), i2c, serial, usb,
   sac (crypto), dma. Firmware does not use these.

## Key finding: the HAL assumes an enum-enriched PAC

The n32g4 HAL matches on PAC field enums (`Sclksw::Pll`, `Ahbpres::Div2`,
etc.). The N32L4 vendor SVD has NO `<enumeratedValues>`, so svd2rust emits
bare `_R`/`_W` field accessors and those enums don't exist.

**Decision: enrich the SVD at the PAC layer (Option 1), not raw-bits in the
HAL.** Keeps the HAL upstream-shaped (consistent with the Path-A PAC
normalization decision). This recurs across modules (timer modes, spi
cpol/baud, can bit timing) — handle it the same way each time.

**De-risked:** the N32L4 and N32G4 share this clock-register layout. Enum
values verified to match between TWO ground-truth sources:
- vendor SDK `n32l40-sdk/.../inc/n32l40x_rcc.h` (e.g. RCC_SYSCLK_DIV2=0x80,
  DIV4=0x90 => AHBPRES field 8, 9 ...)
- n32g4 PAC enums (`Sclksw{Hsi=0,Hse=1,Pll=2}`, `Ahbpres{Div1=0,Div2=8,...}`)

So the SVD enums can be authored with the n32g4 PAC's exact variant names +
values, datasheet-verified, zero invention, zero HAL divergence. Source the
enum values from the n32g4 PAC `rcc/cfg.rs` (and per-peripheral as modules
come up) cross-checked against the vendor SDK headers.

## rcc/mod.rs — remaining error families (after enum enrichment)

- Field enums: `Sclksw`, `Ahbpres`, `Apb1pres` (+ cfg2/cfg3 fields) — fixed by
  the SVD enrichment above.
- Peripheral-name casing: HAL uses `pac::Rcc`, `pac::Flash`, `pac::Tim1/Tim8`;
  our PAC has `RCC`, `FLASH`, `TIM1`, `TIM8` (ALL-CAPS). svd2rust uses the SVD
  peripheral name verbatim. Fix globally (affects many modules): either an
  svdtools `_modify` to PascalCase peripheral names, or HAL-side aliases.
  Decide once; applies fleet-wide.
- `SYSCLK_MAX` const: HAL value defined only under n32g4 cfg arms. Add an
  n32l403/n32l406 definition (~64 MHz per N32L40x spec).
