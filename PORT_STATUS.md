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
- **M1 rcc DONE** (SYSCLK_MAX 64 MHz for n32l403/406; mod.rs + enable.rs clean).
- **M2 afio DONE** (cleared by the svd2rust 0.37.1 naming bump).
- **M3 gpio DONE** -- full STM32-style per-pin AF rewrite (commit `589ac3e` on
  branch `feat/gpio-af-stm-model`). See SPRINT.md HANDOFF for the architecture.
  Licensing also settled: both repos dual Apache-2.0 OR MIT.
- **M4 driver port DONE** -- Remap model removed across pwm/spi/can/serial/
  uart_impls/dynamic; DMA ported to the clustered `dma::Ch` model. PAC
  `ef67950`, HAL `5eebce8`.
- **PORT COMPLETE: 656 -> 0.** All drivers (gpio, spi, can, serial, adc, pwm,
  i2c, fmc, dma) build clean with **0 errors and 0 clippy warnings** on both
  `--features n32l403` and `--features n32l406`. Five genuine vendor-SVD bugs
  and a pile of silently-wrong inherited pin/channel maps were fixed along the
  way, every value checked against the UM/Datasheet rather than guessed.
- **ADC trigger enums corrected** -- `EXTRSEL` (regular) and `EXTJSEL`
  (injected) were a single inherited stm32f4 4-bit `ExternalTrigger` enum
  written via raw `.bits()`: wrong event map for codes 100-111 and conflating
  the two distinct N32L40x trigger domains. Replaced with two UM-verified PAC
  field enums (`RegularTrigger`/`InjectedTrigger`, re-exported, written via
  `.variant()`). Values from UM Tables 17-5 / 17-6. See the ADC section below.

## Current state (authoritative)

**Port complete.** `cargo build` and `cargo clippy`, both devices
(`--features n32l403` / `--features n32l406`, `--target thumbv7em-none-eabihf`):
**0 errors, 0 clippy warnings.** Working trees clean.

The library layers are done; the next frontier is firmware bring-up in
`servo4257-rs` (its HAL/PAC path deps are still commented out). See HANDOFF.md
for the cross-repo state and the M5 plan.

## Baseline build error surface (HISTORICAL -- counts unreliable, see note)

> METRIC CORRECTION: the counts in this section and the first "current state"
> figures were produced with `grep -c '^error'` on the build log, which
> UNDERCOUNTS -- it misses multi-line error blocks and miscounts the summary
> line. The authoritative count is the compiler's own
> `--message-format=json` tally (or the "aborting due to N errors" line). The
> "717" and "518" numbers below and in commits fcda93a/00a9e17/bfd9d5e are
> therefore approximate/low. The QUALITATIVE findings (rcc cleared, names now
> resolve, error mix shifted to gpio/adc) hold; only the integers were wrong.
> See "Re-baselined" below for the real figures.

Early grep-based snapshot (kept for narrative continuity, not accuracy):
717 -> 518 across the svd2rust 0.31.5 -> 0.37.1 PAC bump.

## Re-baselined (authoritative, cargo --message-format=json)

After the PAC svd2rust 0.37.1 bump + RCC.CFG enum enrichment + SYSCLK_MAX
const for n32l403/n32l406:

`cargo build --features n32l403 --target thumbv7em-none-eabihf`:
**656 true errors** (compiler count).

By module:
| module | errors |
|--------|-------:|
| gpio   |    282 |
| adc    |    235 |
| pwm    |     58 |
| serial |     39 |
| spi    |     16 |
| can    |     14 |
| dma    |      6 |
| i2c    |      3 |
| fmc    |      2 |
| sac    |      1 |
| **rcc**  | **0** (DONE) |
| **afio** | **0** (cleared by the naming bump) |

By error code: 238 E0592 (duplicate defs -- the gpio AF-table dup-defs),
200 E0425 (name resolution -- HAL assumes a bigger device: Adc2/3/4, Spi3,
Uart6/7, PE/PF pins that N32L403 lacks), 172 E0599 (no method), 22 E0308,
13 E0432, 11 E0433.

ROOT CAUSE recap: the bulk of the original errors were the PAC identifier
shape, not missing enums. The HAL targets the n32g4 shape
(`Sclksw`/`SclkswR`/`Rcc`) = svd2rust 0.32+ default theme; the PAC had been
0.31.5 (legacy `SCLKSW_A`/`RCC`). Bumping to 0.37.1 fixed naming fleet-wide
and gave PascalCase peripheral names (the `RCC`->`Rcc` casing item resolved
with no patch). rcc and afio are now fully clear.

The remaining work is dominated by **gpio (282) + adc (235) = 517 of 656**,
both driven by the HAL assuming a larger device than the N32L403/406 (more
ADCs, more SPIs, more GPIO ports/pins). That is the core porting task: trim
the HAL's device assumptions to match L4 silicon. Note adc is much bigger
than the early grep suggested (235, not ~131), while i2c/dma/serial are much
smaller -- the "stub the unused peripherals" idea is a minor win, not a major
one.

Milestone status: M1 rcc DONE. M2 afio DONE (no work needed). Next targets
by leverage: gpio and adc.

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
- `SYSCLK_MAX` const: defined for n32l403/n32l406 at 64 MHz
  (rcc/mod.rs:307). DONE.

## adc.rs -- external trigger source enums (EXTRSEL / EXTJSEL)

The N32L40x ADC has TWO distinct external-trigger source maps:
- **EXTRSEL** (CTRL2[19:17]) selects the *regular* sequence trigger -- UM
  Table 17-5.
- **EXTJSEL** (CTRL2[14:12]) selects the *injected* sequence trigger -- UM
  Table 17-6.

The SAME 3-bit code means a DIFFERENT timer event in each table (code 0 =
TIM1_CC1 for regular but TIM1_TRGO for injected, etc.), so a single shared
enum cannot be correct for both. The vendor SVD defines no `<enumeratedValues>`
for either field, so svd2rust emitted bare 3-bit `FieldWriter`s and the HAL
carried ONE hand-written `ExternalTrigger` enum -- inherited verbatim from
stm32f4xx-hal, a 4-bit EXTSEL layout whose codes 100-111 named the WRONG
events for this part -- written via raw `.bits()`. It compiled clean and would
have mis-triggered (or never triggered) the ADC on silicon: exactly the
"silently-wrong inherited map" class.

Fixed the same way as every other enum gap: enrich the SVD
(`tools/svd_patch.yaml`, ADC CTRL2) with UM-verified `enumeratedValues` for
both fields, regen, and have the HAL re-export the generated PAC enums
(`config::RegularTrigger` = `Extrsel`, `config::InjectedTrigger` = `Extjsel`)
and write them via `.variant()`. The value->event mapping now lives in the
SVD; a wrong choice is a nonexistent variant (compile error), not a bad int.
Also fixed a latent bug: the injected setter used to clobber the regular
trigger slot in `AdcConfig`; it no longer touches config (only the regular
trigger is auto-applied by `apply_config`).
