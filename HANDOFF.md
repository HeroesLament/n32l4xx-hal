# Pass-on — servo4257-rs N32L40x bring-up

_Written at the close of the session that took the HAL port from 656 -> 0._

## TL;DR of where things stand

The two **library** layers are DONE and green:

- `n32l4-pac` @ `main` `ef67950` — svd2rust PAC, builds clean for
  `n32l403` and `n32l406`.
- `n32l4xx-hal` @ `feat/gpio-af-stm-model` `5eebce8` — **0 errors AND 0
  clippy warnings** on both `--features n32l403` and `--features n32l406`.
  Every driver ported: gpio (M3), spi, can, serial, adc, pwm, i2c, fmc,
  dma (M4).

Both working trees are clean. Nothing in flight, no WIP to recover.

The **firmware** (`servo4257-rs`) has NOT yet been pointed at the finished
HAL — its `n32l4`/`n32l4xx-hal` path deps are still commented out in
Cargo.toml (L16-17). That is the next frontier.

## Branch / push state (decide before building on top)

- PAC `main` is **4 commits ahead of `origin/main`** (943337b SPI2,
  8aa329d STPB, dec8ca4 ADC bugs, ef67950 DMA cluster) — unpushed.
- HAL work lives on `feat/gpio-af-stm-model`, **not merged** to the main
  line. The whole 656->0 port is on this branch.
- Don't assume these are published. If the next task is firmware bring-up
  against them, path deps are fine (that's how the HAL eats the PAC:
  `n32l4 = { path = "../n32l4-pac" }`). If anything wants the *published*
  crates, they need pushing/tagging first — confirm with the user.

## The ONE hard-won architectural fact to not relitigate

The N32L40x DMA is the **STM32 F4/L4 channel model**, and the vendor SVD
declared the 8 channels as 40 *flat, distinct* register types
(CHCFG1..8, TXNUM1..8, PADDR1..8, MADDR1..8, CHSEL1..8). That cannot back
an STM32-style HAL stream accessor — the per-channel config registers had
8 distinct types, so a channel-generic `st()` had no single return type.

The fix (PAC `ef67950`): an svdtools `_cluster: "CH[%s]"` directive in
`tools/svd_patch.yaml` collapses the five register families into one
uniform `dma::Ch { chcfg, txnum, paddr, maddr, chsel }` cluster, exposed
as `ch(n: usize) -> &Ch` — exactly how ST's own DMA SVDs are shaped. The
HAL's `DMAChannel::st()` returns one `&dma::Ch` for all channels, and the
call sites in spi.rs/serial.rs stayed byte-for-byte identical
(`.st().chcfg().modify(...)`). If a future DMA need arises, work *with*
the cluster; do not try to reintroduce per-channel register types.

## Recommended next milestone: firmware bring-up (M5)

1. **Re-enable the deps** in `servo4257-rs/Cargo.toml` (uncomment the two
   path lines) and pick the target feature (n32l403 vs n32l406 — confirm
   which board first).
2. Build the firmware. Expect a *new* class of errors: not PAC/HAL shape
   mismatches (those are solved) but firmware-level API drift — places
   where servo4257-rs called HAL APIs that changed during the port
   (the GPIO `Alternate<const A, Otype>` model from M3, the collapsed
   single-instance ADC/CAN, Remap removal, the DMA `st()`/Ch model).
3. **Keep the discipline that worked this whole port:** watch the TOTAL
   error count (not per-file), fix root causes not symptoms (one
   device-shape fix often cascade-clears many E0592/E0119), and verify
   every register/pin/channel fact against the UM/Datasheet PDFs rather
   than trusting inherited n32g4 assumptions. The vendor SVD and inherited
   maps have been wrong >5 times (silently, not just absent).
4. Maintain n32l403/n32l406 lockstep if the firmware is meant to support
   both.

## Method notes that paid off (carry forward)

- Build metric (the metric): pipe `cargo build --features <dev>
  --message-format=json` through a python error-counter; group primary
  spans by file. Watch the TOTAL.
- PDFs are source-of-truth: `EN_UM_N32L40x_..._V2.5.0.pdf` (User Manual)
  and `EN_DS_..._V2.1.0.pdf` (Datasheet, Table 3-1 pins) in the HAL repo
  root (untracked). Extracted text at /tmp/um_full.txt, /tmp/ds_full.txt
  via `pdftotext -layout`. The PAC is repeatedly UNTRUSTWORTHY for
  register/field names — go to the UM.
- PAC enrichment pattern: edit `n32l4-pac/tools/svd_patch.yaml`
  (svdtools _modify/_derive/_cluster), then
  `bash tools/regen_device.sh N32L403 n32l403` (and N32L406) — invoke via
  `bash`, it is not +x. Field-enum gaps (no enumeratedValues in vendor
  SVD) are the STPB/CHCFG pattern: add variants, values UM-verified.
- This PAC's `.modify()/.write()` return u32, not () — so match-arm /
  block-tail statements need `;` (the recurring missing-semicolon E0308,
  "expected `()`, found `u32`"). Seen in gpio, pwm, adc, uart, dma.
- tmux discipline: heredoc a script, THEN run it as a separate send (a
  heredoc + command on one send_keys line garbles the trailing command).
  Multi-line git messages -> write to /tmp/msg.txt, commit -F. The tmux
  MCP dropped a few times on the hotspot; recover via list_panes (pane
  %0, pid 86082) and `git status` for phantom edits — it always came
  back clean.

## Loose ends / things to double-check (none are blocking)

- PORT_STATUS.md was stale (described M4/rcc as in-progress). The
  SYSCLK_MAX OPEN item it listed is actually CLOSED — rcc/mod.rs:307
  defines `SYSCLK_MAX = 64_000_000` for n32l4. Updating that doc to
  "656 -> 0, all drivers green" is a nice-to-have.
- `chmap.rs` (DMA channel mux) is `#[cfg]`-gated to n32g4 only, so it is
  NOT compiled for n32l4. If the firmware needs DMA request mux on
  N32L40x, that is unbuilt territory — check the UM for the N32L40x DMA
  request-mapping mechanism (it differs from n32g4's CHMAPEN).
- The `.DS_Store` files in n32l4-pac are macOS noise, intentionally left
  unstaged. A `.gitignore` entry would be tidy.
