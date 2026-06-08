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

### HANDOFF (pick up here) -- M3 gpio: register-model rewrite

State: M1 (rcc) + M2 (afio) DONE. gpio phantom-port feature-gating DONE,
committed at **`6904992`** (working tree clean on top of `a1a610a`). PAC at
`fcda93a`, servo4257-rs docs at `bfd9d5e`.

Error count: baseline was 656; after `6904992` it is **484** (verified
IDENTICAL on n32l403 and n32l406). gpio went 303 -> 131. NOTE: gpio's true
count was **303**, not the 282 in the old baseline table below -- the 282
predated a clean json run. Use 484 / 131 as the current numbers.

**Read first, in order:** PORT_STATUS.md "Re-baselined", then this file, then
AGENTS.md + DECISIONS.md in ../servo4257-rs.

#### How to measure errors (DO NOT use grep -c '^error')

`grep -c '^error'` UNDERCOUNTS (misses multi-line blocks). Use the compiler's
json count:
```
cargo build --features n32l403 --target thumbv7em-none-eabihf \
  --message-format=json 2>/dev/null > /tmp/b.json
```
Tally /tmp/b.json: records where `reason==compiler-message` &&
`message.level==error`, skip the one whose message starts with "aborting".
Group by first `src/<mod>` in `spans[].file_name`. Re-measure BOTH devices.
NOTE: the filesystem MCP is sandboxed to ~/src; write scratch analyzers INTO
the repo (e.g. scratch_*.py) and `rm` them before committing -- /tmp is not
writable via filesystem tools but IS readable/writable via tmux/bash.

#### What `6904992` did (DONE)

Feature-gated the phantom PE/PF/PG references in src/gpio/alt/altmap.rs using
the SAME positive-cfg idiom already in gpio.rs:
`#[cfg(any(feature = "n32g451", "n32g452", "n32g455", "n32g457", "n32g4fr"))]`.
The N32L4 Cargo.toml never sets those features, so the code compiles out while
preserving upstream family shape. Gated 3 whole phantom modules (spi3, uart6,
uart7 -- peripherals absent from the PAC) + scattered phantom PE pin rows in
spi1/spi2/uart4/uart5/tim1 (both the RemapIO impls and the pin! array entries;
the pin! macro supports per-entry #[cfg] by design).

#### THE BIG FINDING -- the remaining 131 gpio errors are a WRONG-ARCHITECTURE
#### problem, not enum enrichment. DO NOT try Option-A PAC enrichment here.

The n32g4 HAL is written for the **STM32-F1 / n32g4 GPIO model**:
`pl_cfg`/`ph_cfg` registers with 2-bit CNF+MODE per pin, and "alternate
function" = a CNF code with NO af-number, plus an AFIO peripheral-REMAP scheme
(`rmp_cfg`/`rmp_cfg3` + per-peripheral `*_rmp` selector bits).

The **N32L40x is the STM32-F4/L4 model** (verified against the vendor SDK,
which is authoritative -- do NOT copy n32g4):
- PAC gpioa block exposes: `pmode{N}` (2-bit mode: 00 in / 01 out / 10 ALT /
  11 analog), `pot{N}` (1-bit push-pull/open-drain), `pupd{N}` (2-bit
  none/pu/pd), `sr{N}` (1-bit slew), `afl`/`afh` each with `afsel0..afsel7`
  (4-bit AF selector per pin; afl = pins 0-7, afh = pins 8-15), plus
  pod/pid/pbsc/pbc/plock. There is NO pl_cfg/ph_cfg.
- AF selection is per-pin: write a 4-bit AF code into afsel{N} of afl (N<8)
  or afh (N>=8). Confirmed by SDK `GPIO_ConfigPinRemap()` in
  n32l40-sdk/firmware/n32l40x_std_periph_driver/src/n32l40x_gpio.c (~line 571).
- The AFIO `RMP_CFG` register is NOT a peripheral remap selector. Its only
  real fields (SDK n32l40x.h ~line 7266): SPI1_NSS(b11) SPI2_NSS(b10)
  ADC_ETRI(b9) ADC_ETRR(b8) EXTI_ETRI(b4..7) EXTI_ETRR(b0..3). There is NO
  RMP_CFG3 and NO `*_rmp` peripheral selectors anywhere on this silicon.
  Adding them to the PAC would be INVENTING registers -- forbidden.

AF-number assignment table (peripheral -> AF#) is in the SDK at
`n32l40-sdk/firmware/n32l40x_std_periph_driver/inc/n32l40x_gpio.h` as
`#define GPIO_AF<n>_<PERIPHERAL>`. Key ones (a peripheral can have several AFs
-- THAT is how the L4 "remaps", by choosing a different AF# on a different
pin, not via a remap register):
  SPI1: AF0,1,4,5,6 | SPI2: AF0,1,5 | CAN: AF1,5
  TIM1: AF2,5,7 | TIM2: AF2,5 | TIM3: AF2,4 | TIM8: AF0,6,7
  USART1: AF0,1,4 | USART2: AF0,4,6 | USART3: AF0,4,5,7
  UART4: AF6 | UART5: AF6,7 | LPUART: AF2,4,6,7
  I2C1: AF4,7 | I2C2: AF1,5,6 | COMP1/2: AF7,8(,9) | MCO: AF8 | EVENTOUT: AF3
NOTE: this gives peripheral->AF#, but the COMPLETE mux also needs the
per-PIN AF column (which physical pin exposes a function at which AF#) from
the DATASHEET pin-mux table -- the SDK header does not encode pin identity.
For firmware-critical signals (encoder SPI, TIM PWM ch, CAN) cross-check those
specific pins; a full all-pins alt layer needs the datasheet table.

#### THE PLAN (agreed: adopt the adjacent-MCU / stm32-rs paradigm verbatim)

Keep alt.rs's trait system + pin! macro + type-state -- they are correct and
device-agnostic (SpiCommon/SerialAsync/TimCPin<C>/TimNCPin<C>/TimBkin/etc.).
DELETE the Remap/RemapIO/Remapper layer in altmap.rs (models nonexistent hw).
Do it as SMALL, INDIVIDUALLY-COMPILING commits (intermediate states won't
build until the chain completes, so don't leave the tree broken across a
hang):

(a) Type-state: change `Alternate<Otype>` -> `Alternate<const A: u8, Otype =
    PushPull>` in gpio.rs (the const A carries the AF number, stm32-rs style).
    Update the marker impls (Interruptible/Readable/OutputSpeed/Active for
    Alternate) and `pub type Debugger`. This ripples into the pin! macro in
    alt.rs and every `Alternate<...>` reference.
(b) Rewrite convert.rs `mode()` (3 copies: Pin, ErasedPin, PartiallyErasedPin)
    to the L4 registers: pmode{N}/pot{N}/pupd{N} for mode/otype/pull, and for
    Alternate<A,_> ALSO write A into afl.afsel{N} (N<8) / afh.afsel{N-8}
    (N>=8). Replace the PinMode CNF/MODE consts with L4 pmode/pot/pupd values.
    Add `into_alternate::<const A: u8>()`.
(c) Fix gpio.rs `set_speed` (uses pl_cfg/ph_cfg -> use sr{N}) and confirm
    _set_high/_set_low/_is_* already use pbsc/pbc/pod/pid (they do).
(d) Rebuild altmap.rs peripheral tables around (pin, AF#) pairs from the table
    above; drop the Remapper/Remap machinery. Keep SpiCommon/Serial*/Tim*
    impls. The AFIO RMP_CFG real fields (SPI NSS mode, ADC ETR trigger, EXTI
    mux) are separate small helpers, not part of pin AF muxing.
Re-measure both devices after each step; commit each green.

#### M4 adc (now 222, was 235): HAL assumes Adc1..Adc4; N32L4 has ONE `Adc`.
Same instance-name + device-shape trim as rcc/enable.rs, plus likely enum
enrichment for sequence/sample-time fields.

#### REFERENCE TO APE: stm32f4xx-hal (the canonical F4/L4-model HAL)

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
