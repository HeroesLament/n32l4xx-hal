# HAL port sprint

Goal: **full** `n32l4xx-hal` compile for both `n32l403` and `n32l406` against
the svd2rust-0.37.1 PAC. Every module builds -- not just the firmware-critical
subset. Tracks the milestone burn-down from the corrected 656-error baseline
(see PORT_STATUS.md "Re-baselined"; the earlier "717 -> 518" figures were a
grep undercount -- use the json metric described in the HANDOFF below).

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
- [x] **M3 gpio (was 282/303)** -- DONE (commit `589ac3e`). Full STM32-style
  per-pin AF rewrite. gpio now contributes 4 errors (down from ~303), all in
  gpio/dynamic.rs (old CNF/MODE/PULL PinMode leftover, folds into M5 driver
  work). See HANDOFF below for the architecture and the AF-table provenance.
- [ ] **M4 driver Remap removal (the new M3-consequence)** -- the altmap
  rewrite deleted the Remap/RemapIO/Remapper layer, so every driver that was
  generic over `RMP: Remap` no longer compiles. Files + error counts (n32l403,
  json): pwm.rs 61, spi.rs 32, can.rs 18, serial.rs 15, serial/uart_impls.rs
  5, gpio/dynamic.rs 4. Drop the `RMP: Remap` generic + `RMP::remap(afio)`
  calls; change pin bounds `RemapIO<Self,RMP> + Into<...>` -> just `Into<...>`.
  This changes the public constructor signatures (acceptable now -- servo4257
  HAL deps are still commented out). pwm.rs is the heaviest and was NOT in the
  original scope; it consumes the timer pins through Remap.
- [ ] **M5 adc (25)** -- HAL assumes Adc1..Adc4; N32L4 has a single `Adc`.
  Same instance-name + device-shape trim as rcc/enable.rs, plus likely enum
  enrichment for sequence/sample-time fields. (Was 235 in the original
  baseline table; the real current count is 25 -- most of the old "adc" bulk
  was actually shared gpio/pin resolution that the M3 rewrite cleared.)
- [ ] **M6 device-shape remainder: i2c.rs (8), dma/mod.rs (6), sac.rs (1)** --
  i2c.rs references I2c1..I2c4 (n32g4 multi-instance; N32L4 has i2c1/i2c2
  only). Same trim pattern. Not firmware-critical but required for full
  compile.

Note: the M3 gpio rewrite (656 -> 175) was the heart of the port. The
remaining 175 are all in *consumers* of the old model (drivers still on
Remap) plus the multi-instance device-shape trims (adc/i2c/dma). No more
architecture-level surprises are expected -- M4/M5/M6 are mechanical.

## Exit criteria

`cargo build --features n32l403 --target thumbv7em-none-eabihf` AND
`--features n32l406` both succeed with zero errors. Then the firmware can
uncomment its `n32l4` / `n32l4xx-hal` path deps.

## Notes / decisions as they land

### HANDOFF (pick up here) -- M4 driver Remap removal

State: M1 (rcc) + M2 (afio) + **M3 (gpio) DONE**. The full STM32-style GPIO AF
rewrite is complete and committed. Working tree clean.

Current commits on branch `feat/gpio-af-stm-model`:
- `18ec5fd` -- verified N32L40x AF-table extraction tooling (tools/gpio_af/).
- `589ac3e` -- STM32-style per-pin AF altmap + pin! macro rewrite. **HEAD.**
(These sit on top of the earlier rcc/licensing work. The branch has not been
merged to main yet -- decide whether to merge after the build goes green, or
keep stacking M4-M6 on this branch.)

Error count (json metric, verified IDENTICAL on n32l403 AND n32l406):
baseline 656 -> **175** after `589ac3e`. gpio went ~303 -> 4 (the 4 are in
gpio/dynamic.rs, an old-model leftover that folds into M4).

**Read first, in order:** PORT_STATUS.md "Re-baselined", this file's Milestones
+ this HANDOFF, then tools/gpio_af/README.md (AF-table provenance), then
AGENTS.md + DECISIONS.md in ../servo4257-rs.

#### What M3 did (DONE -- the architecture is settled, do not relitigate)

The N32L40x uses the STM32-F4/L4 GPIO model: per-pin 4-bit AF number written
to GPIOx_AFL (pins 0-7) / GPIOx_AFH (pins 8-15). Confirmed three ways: SDK
`GPIO_ConfigPinRemap()` computes `AFL/AFH |= AF << (pin*4)`; the UM register
defs show `AFSEL0[3:0]`..`AFSEL15[3:0]`; and convert.rs::set_mode_bits already
writes those nibbles. The n32g4 Remap/RemapIO/Remapper model (AFIO remap-group
selectors) was the WRONG model and is now gone from altmap.rs.

Landed in `589ac3e`:
- **gpio.rs / convert.rs** already carried `Alternate<const A: u8, Otype =
  PushPull>` and the AFL/AFH-writing `set_mode_bits` (from earlier work). Only
  `Alternate<A, PushPull>` and `Alternate<A, OpenDrain>` have PinMode impls --
  so every AF pin is PushPull or OpenDrain; RX/MISO/timer-inputs are
  alternate-push-pull (the "floating/input" character is the peripheral's
  data direction, not the AF pad mode).
- **pin! macro (gpio/alt.rs):** deleted the dead first arm; the remaining arm
  takes per-pin AF numbers `PXn: AF,` and expands each entry to
  `gpio::PXn<Alternate<AF, Otype>>`.
- **altmap.rs:** fully regenerated (1436 -> 855 lines), RemapIO layer deleted.
  19 modules, full N32L403/406 coverage: spi1/2, usart1-3, uart4/5, lpuart,
  tim1-5/8/9, lptim, i2c1/2, can. 191 pin entries, every AF# traceable to the
  verified table.

#### THE AF-TABLE (the hard-won asset -- reuse it, don't re-derive)

The authoritative per-pin (signal, pin, AF#) map lives in
**tools/gpio_af/af_table_um.tsv** -- 314 triples, 0 conflicts, extracted from
the N32L40x User Manual V2.5.0 Tables 5-6..5-29 and manual-verified for
SPI1/USART1/CAN/TIM1/TIM5/TIM8/I2C1/LPTIM. The UM PDF
(`EN_UM_N32L40x_Series_User_Manual_V2.5.0.pdf`) is gitignored at repo root
(kept locally). altmap.rs is generated from this table by
**tools/gpio_af/generate_altmap.py** (`python3 generate_altmap.py` -> writes
altmap.generated.rs; the committed home is src/gpio/alt/altmap.rs). The only
hand-authored part of the generator is MODULE_SPEC (signal-name -> module /
enum / trait wiring); pin/AF data is never hand-entered. README.md in that dir
has full provenance + the bbox/DP-segmentation parser story.

If altmap needs regen (e.g. adding a signal): edit MODULE_SPEC, run the
generator, `cp altmap.generated.rs ../../src/gpio/alt/altmap.rs`, rebuild.
A pre-rewrite RemapIO backup is at tools/gpio_af/altmap.rs.remapio-backup
(gitignored) for reference.

#### M4 PLAN -- remove the Remap generic from the drivers (pick up here)

The altmap rewrite deleted Remap/RemapIO/Remapper, so the drivers that were
generic over `RMP: Remap` no longer compile. This is the bulk of the current
175 errors. Files (n32l403 json counts): pwm.rs 61, spi.rs 32, can.rs 18,
serial.rs 15, serial/uart_impls.rs 5, gpio/dynamic.rs 4.

Per file, mechanically:
- Drop the `<RMP: Remap, ...>` generic parameter from constructors.
- Delete the `RMP::remap(afio)` calls (the per-pin `.into()`/`into_mode()`
  conversion now does the AFL/AFH muxing -- there is nothing left to remap).
- Change pin bounds `crate::gpio::alt::altmap::RemapIO<Self,RMP> +
  Into<Self::Sck>` -> just `Into<Self::Sck>` (and Miso/Mosi/Nss/Rx/Tx/etc.).
- The `afio` argument to the constructors may become unused -- decide per
  driver whether to drop it (cleaner) or keep it `_afio` for API stability.
  Dropping it changes public signatures; that is ACCEPTABLE NOW because
  servo4257-rs's HAL path deps are still commented out (ideal window for
  breaking API changes -- see DECISIONS.md).
- gpio/dynamic.rs (4 errors) references the old CNF/MODE/PULL PinMode consts;
  re-point to the L4 pmode/pot/pupd model (same as convert.rs already does).

Start with pwm.rs (heaviest) or spi.rs (firmware-critical / encoder path) --
your call. Commit each driver green and separately. Re-measure BOTH devices
after each (they have tracked identically all along).

NOT YET SPOT-CHECKED: the generated timer/i2c/lptim modules were verified
against the UM for a sample (TIM1/5/8, I2C1, LPTIM all correct); tim2/3/4/9
and i2c2 came from the same parser but were eyeballed less. Low risk (0
parser conflicts, SPI/USART/CAN all exact), but worth a glance if a timer pin
misbehaves on hardware later.

#### After M4: M5 adc (25), M6 i2c/dma/sac (15)

M5 adc: HAL assumes Adc1..Adc4; N32L4 has a single `Adc`. Same instance-name
+ device-shape trim as rcc/enable.rs, plus likely enum enrichment for
sequence/sample-time fields (PAC-layer svdtools, SDK-verified values -- see
the recurring technique at the top of this file).
M6: i2c.rs references I2c1..I2c4 (n32g4 multi-instance); trim to i2c1/i2c2.
dma/mod.rs (6), sac.rs (1) similar device-shape trims.

Exit criteria unchanged: both `--features n32l403` and `--features n32l406`
build with zero errors, then servo4257-rs can uncomment its path deps.

#### How to measure errors (DO NOT use grep -c '^error')

`grep -c '^error'` UNDERCOUNTS (misses multi-line blocks). Use the compiler's
json count. Keep tool calls SHORT (see MCP warning below): kick the build with
its own `tmux wait-for -S <chan>` + redirect, read/parse in a SEPARATE call.
```
cargo build --features n32l403 --target thumbv7em-none-eabihf \
  --message-format=json 2>/dev/null | python3 -c '...'
```
Tally records where `reason==compiler-message` && `message.level==error`
(skip the one whose message starts with "aborting"); group by the primary
span's `file_name`. Re-measure BOTH devices (they have tracked identically:
656 then 175 on each). The filesystem MCP is sandboxed to ~/src and /tmp is
NOT readable by it (use tmux/bash for /tmp, or write scratch into the repo and
rm before committing).

#### REFERENCE (historical) -- stm32f4xx-hal, the model we mirrored for M3

[HISTORICAL: M3 is DONE. This section recorded the stm32-rs pattern we used to
shape the GPIO rewrite. Kept for context. Note we ended up generating altmap
from the verified AF table rather than hand-writing a gpio!/IntoAf macro --
the per-pin AF correctness lives in the table + generator, not an IntoAf
bound. The register-mapping table below is still a useful reference.]

The N32L40x GPIO is the STM32-F4 register model. `stm32f4xx-hal/src/gpio.rs`
(github stm32-rs/stm32f4xx-hal, MIT/Apache -- MIRROR THE PATTERN, do not paste
verbatim; our HAL is 0BSD) is the proven implementation to transcribe. Their
registers map 1:1 to ours:
  moder->pmode | otyper->pot | ospeedr->sr | pupdr->pupd | afrl->afl |
  afrh->afh | bsrr->pbsc | odr->pod | idr->pid
Their PAC uses indexed accessors `w.ospeedr(N)`; OUR PAC uses numbered fields
`w.sr{N}()`, `w.pmode{N}()`, `w.pot{N}()`, `w.pupd{N}()`, `afl/afh.afsel{N}()`
(afh indexes afsel0..7 for pins 8..15).

Key patterns to adopt from their gpio.rs:
1. Type-state: `pub struct Alternate<const A: u8, Otype = PushPull>(...)`;
   `pub type Debugger = Alternate<0, PushPull>`. All marker impls become
   `impl<const A: u8, Otype> marker::X for Alternate<A, Otype>`.
2. `af!` macro generating `pub type AF0..AF15<Otype=PushPull> = Alternate<n,
   Otype>` aliases.
3. `marker::IntoAf<const A: u8>` trait. The gpio! macro emits, PER PIN, one
   `impl<MODE> marker::IntoAf<$A> for PXi<MODE> {}` for each AF number that
   pin legally supports. THIS is how correct muxing is enforced by the type
   system: a peripheral pin trait is bounded `IntoAf<4>` etc., so only pins
   that expose that function at that AF# satisfy it. <-- this is exactly where
   the DATASHEET per-pin AF column plugs in (the [$($A:literal),*] list in the
   gpio! macro per-pin row).
4. set_speed / set_internal_resistor become CLEAN one-liners against sr/pupd
   (no 16-arm match). Ours currently has the F1 16-arm pl_cfg/ph_cfg match in
   gpio.rs + convert.rs -- replace with the one-liner form.
5. convert.rs mode(): could NOT fetch their convert.rs (only gpio.rs). But the
   contract is clear -- mode() writes pmode{N} (00 in/01 out/10 alt/11 analog)
   + pot{N} + pupd{N}, and for Alternate<A,_> writes A into afl/afh.afsel{N}.
   Our existing convert.rs mode() skeleton (3 copies: Pin/Erased/PartiallyEr.)
   stays; just re-point the register writes. Fetch their convert.rs next
   session if the mode() body needs more detail:
   https://github.com/stm32-rs/stm32f4xx-hal/blob/master/src/gpio/convert.rs

   AF-WRITE ALGORITHM (from the ST C HAL HAL_GPIO_Init, authoritative):
     reg   = (N >> 3)        // 0 => AFL (pins 0-7), 1 => AFH (pins 8-15)
     nib   = (N & 7) * 4     // bit offset of this pin's 4-bit field
     AFR[reg] = (AFR[reg] & ~(0xF << nib)) | (A << nib)
   In OUR svd2rust PAC the per-field accessors do the mask/shift for us, so
   this collapses to (for Alternate<A,_>):
     if N < 8 { gpio.afl().modify(|_,w| unsafe { w.afsel{N}().bits(A) }) }
     else     { gpio.afh().modify(|_,w| unsafe { w.afsel{N-8}().bits(A) }) }
   and the mode nibble is just `gpio.pmode().modify(|_,w| unsafe {
   w.pmode{N}().bits(0b10) })` (0b10 = alternate), plus pot{N}/pupd{N}.
   So mode() is a 16-arm match (or a helper indexed by N) writing pmode+pot+
   pupd, and additionally afsel for the Alternate case. No mystery remains;
   the convert.rs fetch is optional polish, not a blocker.

NOTE the gpio! macro row format changes to carry the AF list:
  `PXi: (pxi, N, [AF list] $(, $MODE)?),` -- the [$($A:literal),*] is the set
  of AF numbers valid for that pin, sourced from the DATASHEET pin-mux table.
  (SDK gives peripheral->AF#; datasheet gives pin->AF#. Need both. For
  firmware-only scope, fill the AF lists for just the pins the firmware uses.)

#### CRITICAL WORKFLOW WARNING -- MCP server instability

The tmux/filesystem MCP server hung 3x during the previous session, each on
heavier command chains (long `cargo build 2>f; grep ...` one-liners). Two
mitigations, both REQUIRED:
1. Keep tool calls SHORT and SINGLE-PURPOSE. Kick off a build with its own
   `tmux wait-for -S <chan>` channel + redirect to a file; read the file in a
   SEPARATE call. Do not chain build+grep+parse in one send_keys.
2. A server hang once applied a file edit that was NEVER confirmed by a tool
   result (an untracked phantom change to src/rcc/enable.rs). Therefore:
   **ALWAYS `git diff` every file before staging, and never commit a change
   you cannot trace to a specific tool call you made.** On a 282-error module
   like gpio this matters a lot -- a phantom edit is easy to miss among many
   intended ones. Consider committing in small, reviewable batches.
If the user hasn't yet investigated the MCP server's resource/timeout issue,
flag it before doing bulk gpio edits.

#### Metric-correction note (already applied, for context)

The "717 -> 518" progression in commits fcda93a / 00a9e17 / bfd9d5e was
grep-based and undercounts; left as historical record per the user's call.
PORT_STATUS.md carries the correction. Use only the json metric going forward.

#### Reproducible PAC regen (if you touch the PAC again)

The PAC pipeline + install is scripted: `n32l4-pac/tools/regen_device.sh
<N32L403|N32L406> <n32l403|n32l406>`. svd2rust is pinned to **0.37.1** (NOT
0.31.5 -- that was the bug; 0.32+ default ident theme is what matches the
n32g4 shape the HAL wants). Enum enrichment lives in
`n32l4-pac/tools/svd_patch.yaml` (RCC.CFG done; add spi/timer/can/adc fields
there as those modules need them, SDK-verified values).
