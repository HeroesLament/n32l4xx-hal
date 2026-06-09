# N32L40x GPIO alternate-function (AF) table extraction

This directory derives the authoritative per-pin alternate-function map for the
NSING N32L40x and is the data source for the STM32-style GPIO `pin!` rewrite in
`src/gpio/alt/altmap.rs`.

## Why this exists

The N32L40x selects pin alternate functions via a per-pin 4-bit AF number
written to `GPIOx_AFL` (pins 0–7) / `GPIOx_AFH` (pins 8–15) — the STM32 model,
confirmed three ways:
- the vendor SDK `GPIO_ConfigPinRemap()` computes `AFL/AFH |= AF << (pin*4)`;
- the User Manual register defs show `AFSEL0[3:0]`..`AFSEL15[3:0]`;
- `src/gpio/convert.rs::set_mode_bits` already writes those nibbles.

The HAL therefore needs `(pin, signal, AF#)` triples. The datasheet pin table
lists each pin's *functions* but not their AF *numbers*; the SDK header lists
AF# per *peripheral* but not per *pin*. Only the **User Manual** Tables 5-6..5-29
("`<periph>` alternate function remapping") give the exact per-pin AF number.

## Authoritative source

`EN_UM_N32L40x_Series_User_Manual_V2.5.0.pdf` (from nationstech.com; not
committed — see root `.gitignore`). Section 5.2.5, printed pp.97-108
(physical PDF pp.121-132).

## Pipeline (kept files)

1. `um_af_remap_raw.txt` — `pdftotext -layout` slice of the remap tables
   (human-readable reference; not parsed directly).
2. `parse_um_bbox2.py` — the real parser. Consumes a `pdftotext -bbox`
   coordinate dump (regenerate with:
   `pdftotext -bbox -f 121 -l 132 EN_UM_..._V2.5.0.pdf um_af_bbox.html`)
   and emits `af_table_um.tsv`.
   - The UM renders each signal label at the VERTICAL CENTER of its pin block,
     so line-order parsing mis-assigns. The parser groups words by (x,y) and
     uses a DP contiguous-segmentation to assign pins to the nearest label
     block. Section sub-headers (bare `TIM5`, `SPI1`, ...) are rejected via a
     header-row context check + a bare-signal allowlist.
3. `af_table_um.tsv` — OUTPUT: `signal<TAB>pin<TAB>af<TAB>source_table`.
   314 triples, 62 pins, 102 signals, 0 conflicts. **Manual-verified** against
   the UM for the firmware-critical peripherals (SPI1, USART1, CAN, TIM1).

`af_map.txt` (peripheral→AF# from the SDK header) and `pins.tsv` (datasheet
pin→function) drove the earlier intersection approach (`build_af_table.py`);
they are retained as a cross-check but `af_table_um.tsv` is the source of truth
because it is per-pin exact.

## Trust note

A wrong AF nibble silently mis-muxes a pin on real hardware. Any pin/signal not
present in `af_table_um.tsv` must be treated as unimplemented (compile error if
used), never guessed.
