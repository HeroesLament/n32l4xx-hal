# `n32l4xx-hal`

HAL for the NSING (Nationstech) **N32L40x** family — specifically the
**N32L403** (MKS SERVO42D MCU) and **N32L406** (MKS SERVO57D MCU).

Ported from [`guineawheek/n32g4xx-hal`](https://github.com/guineawheek/n32g4xx-hal)
(0BSD), retargeted onto the `n32l4` PAC. This is the firmware-side HAL for the
[`servo4257-rs`](../servo4257-rs) closed-loop stepper driver project.

## Status: PORT IN PROGRESS — does not yet build

The fork is retargeted (Cargo.toml, PAC re-export, device features, toolchain)
and the full error surface is characterized, but peripheral modules still
reference the n32g4 register/enum API and are being ported module by module.
See `PORT_STATUS.md` for the live breakdown and plan.

## Devices

Select exactly one device feature (mutually exclusive register maps):

- `n32l403` — SERVO42D (N32L403KBQ7, LQFP48)
- `n32l406` — SERVO57D (N32L406CBL7, LQFP48; adds RS485 + opto I/O)

```
cargo build --features n32l403 --target thumbv7em-none-eabihf
cargo build --features n32l406 --target thumbv7em-none-eabihf
```

## Toolchain

**Nightly required.** The HAL uses 8 unstable features (`adt_const_params`,
`min_specialization`, `impl_trait_in_assoc_type`, `associated_type_defaults`,
`macro_metavar_expr`, `more_qualified_paths`, `negative_impls`,
`trivial_bounds`). Pinned via `rust-toolchain.toml` to `nightly-2026-06-07`,
matching the `n32l4` PAC and the `servo4257-rs` firmware.

## Relationship to the PAC

Depends on the `n32l4` PAC (`../n32l4-pac`) as a path dependency. That PAC is
normalized (svdtools Stage 2) to present the same register dialect as the
n32g4 family PAC, so this HAL's peripheral code stays close to upstream. Where
the vendor SVD lacks field `<enumeratedValues>`, the enums the HAL relies on
are being added at the PAC layer (see the PAC repo), NOT worked around here —
this keeps the HAL upstream-shaped.

## License

0BSD (inherited from the upstream n32g4xx-hal).
