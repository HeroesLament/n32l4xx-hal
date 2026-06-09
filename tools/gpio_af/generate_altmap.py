#!/usr/bin/env python3
"""
Generate src/gpio/alt/altmap.rs for the N32L40x from the verified AF table.

INPUT  : af_table_um.tsv  (signal, pin, af) -- authoritative, UM-derived.
OUTPUT : altmap.generated.rs  (review, then move into src/gpio/alt/altmap.rs)

Design
------
The N32L40x selects pin alternate functions via a per-pin 4-bit AF number in
GPIOx_AFL/AFH (STM32 model). So each peripheral-function pin is represented as
gpio::PXn<Alternate<A, Otype>> with a CONCRETE A from the table. The `pin!`
macro (rewritten in alt.rs) takes per-pin AF numbers.

Every emitted pin entry is traceable to one row of af_table_um.tsv. The only
hand-authored data here is MODULE_SPEC: the mapping from datasheet signal
names to (HAL module, pin-enum name, trait wiring). That spec is small and
auditable; the pin/AF data is never hand-entered.

A signal/pin not present in the table is simply not emitted -- never guessed.
"""
import os, re, sys
from collections import defaultdict, OrderedDict

HERE = os.path.dirname(os.path.abspath(__file__))
TABLE = os.path.join(HERE, "af_table_um.tsv")
OUT = os.path.join(HERE, "altmap.generated.rs")

# ---------------------------------------------------------------------------
# MODULE SPEC  (the only hand-authored mapping; verify against UM + drivers)
#
# Each entry maps a HAL module to:
#   pac:      the PAC peripheral type (via `as PER`)
#   signals:  OrderedDict mapping the datasheet signal-name regex -> the
#             pin-enum name + its default Otype. The regex is matched against
#             the table's `signal` column; the capture/instance is implied by
#             which module owns it (we filter table rows by signal prefix).
#   traits:   list of trait-impl blocks to emit after the pin! block.
#
# Otype defaults follow the existing altmap.rs + UM Table 5-36..5-40 electrical
# guidance: TX/SCK/MOSI/NSS/CK/RTS/OUT = PushPull; RX/MISO/BKIN = Floating/Input;
# I2C SCL/SDA = OpenDrain.
# ---------------------------------------------------------------------------

# Otype constants.
# NOTE: an alternate-function pad on the N32L40x is configured push-pull or
# open-drain at the pad; the "floating/input" character of an RX/MISO line is a
# property of the peripheral's data direction, NOT of the AF mode bits. The
# typestate `Alternate<const A, Otype>` only has PinMode impls for Otype =
# PushPull and OpenDrain (see gpio/convert.rs), so every AF pin MUST use one of
# those two. RX/MISO/BKIN/timer-inputs therefore use PushPull (the pad is
# alternate-push-pull; the peripheral drives direction). Only true open-drain
# buses (I2C SCL/SDA) use OpenDrain.
PP = "PushPull"
OD = "OpenDrain"

def spi_signals():
    # SPIx_I2Sx_<role>  -> enum name
    return OrderedDict([
        ("NSS_WS",   ("Nss",  PP)),
        ("SCLK_CK",  ("Sck",  PP)),
        ("MISO_MCK", ("Miso", PP)),
        ("MOSI_SD",  ("Mosi", PP)),
    ])

def usart_signals():
    return OrderedDict([
        ("CK",  ("Ck",  PP)),
        ("CTS", ("Cts", PP)),
        ("RTS", ("Rts", PP)),
        ("RX",  ("Rx",  PP)),
        ("TX",  ("Tx",  PP)),
    ])

def uart_signals():
    return OrderedDict([
        ("RX", ("Rx", PP)),
        ("TX", ("Tx", PP)),
    ])

def tim_signals(with_n=False, with_etr=True, with_bkin=False, nchan=4):
    d = OrderedDict()
    for c in range(1, nchan+1):
        d[f"CH{c}"] = (f"Ch{c}", PP)
    if with_n:
        for c in range(1, 4):
            d[f"CH{c}N"] = (f"Ch{c}n", PP)
    if with_etr:
        d["ETR"] = ("Etr", PP)
    if with_bkin:
        d["BKIN"] = ("Bkin", PP)
    return d

def i2c_signals():
    return OrderedDict([
        ("SCL",  ("Scl",  OD)),
        ("SDA",  ("Sda",  OD)),
        ("SMBA", ("Smba", PP)),
    ])

def lptim_signals():
    return OrderedDict([
        ("IN1", ("In1", PP)),
        ("IN2", ("In2", PP)),
        ("OUT", ("Out", PP)),
        ("ETR", ("Etr", PP)),
    ])

def can_signals():
    return OrderedDict([
        ("RX", ("Rx", PP)),
        ("TX", ("Tx", PP)),
    ])

# trait emitters: given the set of enum names present, produce Rust impl text.
def spi_traits(present):
    return f"""    impl SpiCommon for PER {{
        type Sck = Sck;
        type Miso = Miso;
        type Mosi = Mosi;
        type Nss = Nss;
    }}
"""

def serial_traits(present):
    out = []
    out.append("""    impl SerialAsync for PER {
        type Rx<Itype> = Rx<Input<Itype>>;
        type Tx<Otype> = Tx<Otype>;
    }
""")
    if "Ck" in present:
        out.append("""    impl SerialSync for PER {
        type Rx<Itype> = Rx<Input<Itype>>;
        type Tx<Otype> = Tx<Otype>;
        type Ck = Ck;
    }
""")
    if "Cts" in present and "Rts" in present:
        out.append("""    impl SerialRs232 for PER {
        type Rx<Itype> = Rx<Input<Itype>>;
        type Tx<Otype> = Tx<Otype>;
        type Cts = Cts;
        type Rts = Rts;
    }
""")
    return "".join(out)

def tim_traits(present):
    out = []
    chmap = {"Ch1":0,"Ch2":1,"Ch3":2,"Ch4":3}
    for name, c in chmap.items():
        if name in present:
            out.append(f"""    impl TimCPin<{c}> for PER {{
        type Ch<Otype> = {name}<Otype>;
    }}
""")
    ncmap = {"Ch1n":0,"Ch2n":1,"Ch3n":2}
    for name, c in ncmap.items():
        if name in present:
            out.append(f"""    impl TimNCPin<{c}> for PER {{
        type ChN<Otype> = {name}<Otype>;
    }}
""")
    if "Bkin" in present:
        out.append("""    impl TimBkin for PER {
        type Bkin = Bkin;
    }
""")
    if "Etr" in present:
        out.append("""    impl TimEtr for PER {
        type Etr = Etr;
    }
""")
    return "".join(out)

def i2c_traits(present):
    return """    impl I2cCommon for PER {
        type Scl = Scl;
        type Sda = Sda;
        type Smba = Smba;
    }
"""

def can_traits(present):
    return """    impl CanCommon for PER {
        type Rx = Rx;
        type Tx = Tx;
    }
"""

def lptim_traits(present):
    return ""  # no common trait yet; pins exposed for future driver use

# module -> spec
def M(pac, signals, traits_fn, sig_prefix):
    return {"pac": pac, "signals": signals, "traits": traits_fn, "prefix": sig_prefix}

MODULE_SPEC = OrderedDict([
    # SPI / I2S
    ("spi1",  M("Spi1",  spi_signals(),  spi_traits,    "SPI1_I2S1_")),
    ("spi2",  M("Spi2",  spi_signals(),  spi_traits,    "SPI2_I2S2_")),
    # USART (full sync/async/rs232)
    ("usart1", M("Usart1", usart_signals(), serial_traits, "USART1_")),
    ("usart2", M("Usart2", usart_signals(), serial_traits, "USART2_")),
    ("usart3", M("Usart3", usart_signals(), serial_traits, "USART3_")),
    # UART (async only)
    ("uart4", M("Uart4", uart_signals(), serial_traits, "UART4_")),
    ("uart5", M("Uart5", uart_signals(), serial_traits, "UART5_")),
    # LPUART -> treat as serial (async + rs232 cts/rts)
    ("lpuart", M("Lpuart", usart_signals(), serial_traits, "LPUART_")),
    # Timers
    ("tim1", M("Tim1", tim_signals(with_n=True,  with_etr=True, with_bkin=True), tim_traits, "TIM1_")),
    ("tim2", M("Tim2", tim_signals(with_n=False, with_etr=True), tim_traits, "TIM2_")),
    ("tim3", M("Tim3", tim_signals(with_n=False, with_etr=True), tim_traits, "TIM3_")),
    ("tim4", M("Tim4", tim_signals(with_n=False, with_etr=False), tim_traits, "TIM4_")),
    ("tim5", M("Tim5", tim_signals(with_n=False, with_etr=False), tim_traits, "TIM5_")),
    ("tim8", M("Tim8", tim_signals(with_n=True,  with_etr=True, with_bkin=True), tim_traits, "TIM8_")),
    ("tim9", M("Tim9", tim_signals(with_n=False, with_etr=True), tim_traits, "TIM9_")),
    ("lptim", M("Lptim", lptim_signals(), lptim_traits, "LPTIM_")),
    # I2C
    ("i2c1", M("I2c1", i2c_signals(), i2c_traits, "I2C1_")),
    ("i2c2", M("I2c2", i2c_signals(), i2c_traits, "I2C2_")),
    # CAN
    ("can",  M("Can", can_signals(), can_traits, "CAN_")),
])

# Which Otype "default:" keyword each Otype maps to in the pin! macro form.
# All our enums use the `<Name> default: <Otype> for no:NoPin, [ ... ]` form,
# where pins carry per-pin AF: `PXn: AF,`.

def load_table():
    rows = []
    for line in open(TABLE):
        if line.startswith("#") or not line.strip():
            continue
        parts = line.rstrip("\n").split("\t")
        if len(parts) < 3:
            continue
        sig, pin, af = parts[0], parts[1], int(parts[2])
        rows.append((sig, pin, af))
    return rows

def main():
    rows = load_table()
    # index: (module, enum_name) -> list of (pin, af), preserving table order
    # We match a row to a module if its signal starts with the module prefix,
    # then strip the prefix and match the remainder to a signal key.
    mod_enum_pins = defaultdict(list)   # (mod, enum) -> [(pin, af)]
    mod_present = defaultdict(set)      # mod -> set(enum names)
    used_rows = 0
    unmatched = []

    for sig, pin, af in rows:
        matched = False
        for mod, spec in MODULE_SPEC.items():
            pfx = spec["prefix"]
            if not sig.startswith(pfx):
                continue
            tail = sig[len(pfx):]
            # find the signal key whose name matches the tail
            sigs = spec["signals"]
            if tail in sigs:
                enum_name, otype = sigs[tail]
                mod_enum_pins[(mod, enum_name)].append((pin, af))
                mod_present[mod].add(enum_name)
                used_rows += 1
                matched = True
                break
        # signals we intentionally don't emit (EVENTOUT, LCD COM/SEG, RTC,
        # COMP, MCO, debug) just fall through unmatched -- that's expected.

    # emit
    lines = []
    lines.append("// @generated by tools/gpio_af/generate_altmap.py from")
    lines.append("// af_table_um.tsv (N32L40x UM V2.5.0). DO NOT EDIT BY HAND.")
    lines.append("// Every pin entry's AF number is traceable to that table.")
    lines.append("//")
    lines.append("// N32L40x selects pin AF via per-pin AFL/AFH nibble (STM32 model);")
    lines.append("// each pin is gpio::PXn<Alternate<A, Otype>> with concrete A.")
    lines.append("")
    lines.append("use super::*;")
    lines.append("use core::marker::PhantomData;")
    lines.append("use crate::gpio::NoPin;")
    lines.append("")

    # Otype keyword per enum (from spec); need to recover otype per enum name.
    # Build (mod, enum)->otype map.
    enum_otype = {}
    for mod, spec in MODULE_SPEC.items():
        for tail,(en,ot) in spec["signals"].items():
            enum_otype[(mod,en)] = ot

    for mod, spec in MODULE_SPEC.items():
        present = mod_present.get(mod)
        if not present:
            # no pins for this module on this device -- skip (e.g. instance absent)
            lines.append(f"// (module {mod}: no pins in table; skipped)")
            lines.append("")
            continue
        pac = spec["pac"]
        lines.append(f"pub mod {mod} {{")
        lines.append("    use super::*;")
        lines.append("    use crate::gpio::{self, Input, PushPull, OpenDrain};")
        lines.append(f"    use crate::{{gpio::alt::altmap::pin, pac::{pac} as PER}};")
        lines.append("")
        lines.append("    pin! {")
        # emit each enum in the spec's signal order
        seen_enums = []
        for tail,(en,ot) in spec["signals"].items():
            pins = mod_enum_pins.get((mod, en))
            if not pins:
                continue
            seen_enums.append(en)
            otype = enum_otype[(mod,en)]
            default_kw = otype
            lines.append(f"        <{en}> default: {default_kw} for no:NoPin, [")
            # de-dup pins (same pin can appear once per enum); keep table order
            seen_pin = set()
            for pin, af in pins:
                if pin in seen_pin:
                    continue
                seen_pin.add(pin)
                lines.append(f"            {pin}: {af},")
            lines.append("        ],")
            lines.append("")
        lines.append("    }")
        lines.append("")
        # traits
        tr = spec["traits"](present)
        if tr:
            lines.append(tr.rstrip("\n"))
        lines.append("}")
        lines.append("")

    with open(OUT, "w") as f:
        f.write("\n".join(lines) + "\n")

    # diagnostics
    print(f"table rows: {len(rows)}   matched into modules: {used_rows}")
    print(f"modules emitted: {sum(1 for m in MODULE_SPEC if mod_present.get(m))}")
    for mod in MODULE_SPEC:
        if mod_present.get(mod):
            npins = sum(len({p for p,_ in mod_enum_pins[(mod,e)]}) for e in mod_present[mod])
            print(f"  {mod:8s} enums={sorted(mod_present[mod])} pins={npins}")
        else:
            print(f"  {mod:8s} (empty)")

if __name__ == "__main__":
    main()
