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

### HANDOFF (pick up here) -- M3 gpio is next

State: M1 (rcc) and M2 (afio) DONE, committed at `c834090` on the HAL repo,
working tree clean. PAC committed at `fcda93a`, servo4257-rs docs at `bfd9d5e`.
Authoritative error count is **656** (see metric note below). rcc + afio = 0.
Next target by leverage: gpio (282), then adc (235).

**Read these first, in order:** PORT_STATUS.md (the "Re-baselined" section has
the real per-module/per-code numbers), then this file top-to-bottom, then
AGENTS.md + DECISIONS.md in ../servo4257-rs for the architecture invariants.

#### How to measure errors (DO NOT use grep -c '^error')

`grep -c '^error'` UNDERCOUNTS -- it misses multi-line error blocks. Use the
compiler's own count. Authoritative recipe:
```
cargo build --features n32l403 --target thumbv7em-none-eabihf \
  --message-format=json 2>/dev/null > /tmp/b.json
```
Then tally with a tiny python over /tmp/b.json: count records where
`reason==compiler-message`, `message.level==error`, skipping the one whose
`message.message` starts with "aborting". Group by the first `src/<mod>` in
`spans[].file_name`. (That is exactly how the 656 / per-module table was
produced.) Always re-measure both n32l403 AND n32l406 -- they can differ.

#### M3 gpio approach (do NOT chase errors one at a time)

282 errors, mostly E0592 (duplicate defs in src/gpio/alt/altmap.rs) + E0425
(types the N32L403 lacks). ROOT CAUSE is structural: the n32g4 HAL's AF
pin-mux tables enumerate ports/pins/peripherals for the largest package
(PE/PF/PG ports, Spi3, Uart6/7, Adc2-4, etc.) that the N32L403/406 don't
have. Understand the macro structure in alt.rs/altmap.rs FIRST, then trim
structurally:
- The N32L403/406 have GPIOA-D only (no PE/PF/PG). Confirm against the PAC
  `Peripherals` struct (gpioa..gpiod present; check for gpioe+).
- Remove/feature-gate AF entries referencing absent peripherals (Spi3,
  Uart6/7, I2c3/4, Adc2-4).
- The package is LQFP48 for both boards (see AGENTS.md hardware section) --
  only pins on that package exist. Cross-check the real AF mux against the
  vendor SDK / datasheet, do NOT invent AF assignments.
- Same enum-enrichment-at-the-PAC-layer technique applies if gpio matches on
  field enums the SVD lacks (see the recurring-technique note at top).

#### M4 adc (235): the HAL assumes Adc1..Adc4; the N32L4 has ONE adc
(named `Adc` in the PAC). Expect the same instance-name + device-shape trim
as rcc/enable.rs, plus likely enum enrichment for sequence/sample-time fields.

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
