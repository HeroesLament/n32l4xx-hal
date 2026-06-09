#!/usr/bin/env python3
"""
Build the N32L40x (pin, signal, AF#) table by intersecting two
hand-transcribed authoritative sources:

  af_map.txt : peripheral -> AF number   (from SDK header n32l40x_gpio.h)
  pins.tsv   : pin -> signal list         (from datasheet Table 3-1)

For each pin's datasheet signal we derive the peripheral prefix, look up
its AF number(s) in the AF map, and emit a (pin, signal, AF#) row.

Crucially: where a peripheral is published at MORE THAN ONE AF number,
the pin alone does not tell us which -- we FLAG it as ambiguous rather
than guess. A wrong AF nibble silently mis-muxes a pin on real silicon,
so ambiguous rows must be resolved against SDK examples or the reference
manual before they go into the HAL.

Outputs:
  af_table.tsv     : resolved rows  PIN<TAB>SIGNAL<TAB>AF
  af_ambiguous.tsv : pin/signal whose peripheral spans multiple AFs
  af_unmapped.tsv  : signals whose peripheral isn't in the AF map at all
and a summary to stdout.

This is a build-time data tool; it does not touch the HAL source.
"""
import sys, os, re
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))

# --- signal prefix -> AF-map peripheral key normalization ---------------
# Datasheet signal looks like "<PERIPH>_<func>" or bare (e.g. "JTDI").
# We map the datasheet peripheral token to the af_map.txt key.
def peripheral_of(signal):
    s = signal.strip()
    # Debug / SWJ group -> SW_JTAG (AF0)
    if s in ("JTDI", "JTDO_TRACESWO", "NJTRST", "SWDIO_JTMS", "SWCLK_JTCK") \
       or s.startswith("TRACED") or s == "TRACESWO":
        return "SW_JTAG"
    # EVENTOUT -> EVENTOUT (AF3)
    if s == "EVENTOUT":
        return "EVENTOUT"
    # MCO -> MCO (AF8)
    if s == "MCO":
        return "MCO"
    # LCD_SEGxx / LCD_COMx -> LCD  (AF10/AF11 -- itself ambiguous, see below)
    if s.startswith("LCD_"):
        return "LCD"
    # peripheral is the token before the first underscore
    token = s.split("_", 1)[0]
    # I2S shares the SPI block on this device (I2S1<->SPI1, I2S2<->SPI2).
    # The datasheet lists them as distinct signals on the same pins; the AF
    # map only carries SPIn. Map I2Sn -> SPIn so the AF resolves.
    m = re.match(r"I2S(\d)", token)
    if m:
        return "SPI" + m.group(1)
    return token

def load_af_map(path):
    af = defaultdict(set)   # peripheral -> set of AF numbers
    with open(path) as f:
        for line in f:
            line = line.split("#", 1)[0].strip()
            if not line:
                continue
            parts = line.split()
            if len(parts) != 2:
                print(f"WARN: malformed af_map line: {line!r}", file=sys.stderr)
                continue
            aftok, periph = parts
            m = re.match(r"AF(\d+)$", aftok)
            if not m:
                print(f"WARN: bad AF token: {aftok!r}", file=sys.stderr)
                continue
            af[periph].add(int(m.group(1)))
    return af

def load_pins(path):
    pins = []   # (pin, [signals])
    with open(path) as f:
        for line in f:
            raw = line.split("#", 1)[0].rstrip("\n")
            if not raw.strip():
                continue
            if "\t" not in raw:
                print(f"WARN: pin line without tab: {raw!r}", file=sys.stderr)
                continue
            pin, sigs = raw.split("\t", 1)
            pin = pin.strip()
            siglist = [s.strip() for s in sigs.split(",") if s.strip()]
            pins.append((pin, siglist))
    return pins

def main():
    af = load_af_map(os.path.join(HERE, "af_map.txt"))
    pins = load_pins(os.path.join(HERE, "pins.tsv"))

    resolved = []    # (pin, signal, af)
    ambiguous = []   # (pin, signal, peripheral, sorted afs)
    unmapped = []    # (pin, signal, peripheral)

    for pin, sigs in pins:
        for sig in sigs:
            periph = peripheral_of(sig)
            afs = af.get(periph)
            if not afs:
                unmapped.append((pin, sig, periph))
            elif len(afs) == 1:
                resolved.append((pin, sig, next(iter(afs))))
            else:
                ambiguous.append((pin, sig, periph, sorted(afs)))

    with open(os.path.join(HERE, "af_table.tsv"), "w") as f:
        f.write("# pin\tsignal\taf   (resolved, unambiguous)\n")
        for pin, sig, a in resolved:
            f.write(f"{pin}\t{sig}\t{a}\n")
    with open(os.path.join(HERE, "af_ambiguous.tsv"), "w") as f:
        f.write("# pin\tsignal\tperipheral\tcandidate_afs   (NEEDS verification)\n")
        for pin, sig, p, afs in ambiguous:
            f.write(f"{pin}\t{sig}\t{p}\t{','.join(map(str,afs))}\n")
    with open(os.path.join(HERE, "af_unmapped.tsv"), "w") as f:
        f.write("# pin\tsignal\tperipheral   (no AF-map entry; investigate)\n")
        for pin, sig, p in unmapped:
            f.write(f"{pin}\t{sig}\t{p}\n")

    npins = len(pins)
    nsig = sum(len(s) for _, s in pins)
    print(f"pins: {npins}   signals: {nsig}")
    print(f"resolved (unique AF):   {len(resolved)}")
    print(f"ambiguous (multi-AF):   {len(ambiguous)}")
    print(f"unmapped (no AF entry): {len(unmapped)}")
    # which peripherals are the ambiguous ones?
    amb_periphs = defaultdict(set)
    for _, _, p, afs in ambiguous:
        amb_periphs[p].update(afs)
    if amb_periphs:
        print("\nAmbiguous peripherals (peripheral -> candidate AFs):")
        for p in sorted(amb_periphs):
            print(f"  {p:8s} {sorted(amb_periphs[p])}")
    if unmapped:
        ump = sorted({p for _,_,p in unmapped})
        print("\nUnmapped peripherals:", ", ".join(ump))

if __name__ == "__main__":
    main()
