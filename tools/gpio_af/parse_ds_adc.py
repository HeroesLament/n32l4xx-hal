#!/usr/bin/env python3
"""
Extract ADC_INx -> GPIO pin map from N32L40x Datasheet Table 3-1.

Input : /tmp/ds_t31_bbox.html  (pdftotext -bbox -f 40 -l 46)
Method: per page, the pin-name column (xMin<300) and the ADC_IN column
        (xMin>400). A pin's ADC_INx label is NOT always on the pin-name's
        row -- in tall cells the ADC label sits above the (lower/centered)
        pin name. So we BAND: sort pin names by y; each pin owns the y-span
        bounded by the midpoints to its neighbours. Every ADC_IN is assigned
        to the band (pin) its y falls into. This is the same banding fix
        parse_um_bbox.py uses for vertically-centred labels.
Output: tools/gpio_af/adc_channel_um.tsv  (PIN<TAB>CHANNEL)
"""
import re, sys, html

HERE = __file__.rsplit('/',1)[0]
BBOX = "/tmp/ds_t31_bbox.html"
OUT  = HERE + "/adc_channel_um.tsv"

word_re = re.compile(r'<word xMin="([\d.]+)" yMin="([\d.]+)"[^>]*>(.*?)</word>')
pin_re  = re.compile(r'^(P[A-F]\d{1,2})')
adc_re  = re.compile(r'^ADC_IN(\d{1,2})')

pages = re.split(r'(?=<page\b)', open(BBOX).read())
seen = {}        # channel -> pin
problems = []

for pi, page in enumerate(pages):
    pins, adcs = [], []
    for m in word_re.finditer(page):
        x,y,t = float(m[1]),float(m[2]),html.unescape(m[3])
        if x < 300 and pin_re.match(t): pins.append((y, pin_re.match(t).group(1)))
        elif x > 400 and adc_re.match(t): adcs.append((y, int(adc_re.match(t).group(1))))
    if not pins or not adcs: continue
    pins.sort()
    # band boundaries: midpoints between consecutive pin y's
    bounds = [(-1e9)] + [ (pins[i][0]+pins[i+1][0])/2 for i in range(len(pins)-1) ] + [1e9]
    for (ay, ch) in adcs:
        # find band index: largest i with bounds[i] <= ay < bounds[i+1]
        idx = None
        for i in range(len(pins)):
            if bounds[i] <= ay < bounds[i+1]:
                idx = i; break
        if idx is None:
            problems.append(f"ADC_IN{ch} p{pi} y={ay:.1f} -> no band"); continue
        if ch in seen and seen[ch] != pins[idx][1]:
            problems.append(f"ADC_IN{ch} conflict: {seen[ch]} vs {pins[idx][1]}")
        seen[ch] = pins[idx][1]

with open(OUT,"w") as f:
    f.write("# PIN\tADC_CHANNEL  -- Datasheet Table 3-1, pdftotext -bbox, parse_ds_adc.py (banded)\n")
    for ch in sorted(seen): f.write(f"{seen[ch]}\t{ch}\n")

print(f"channels: {len(seen)} (expect 16)")
print("map:", ", ".join(f"{seen[c]}=IN{c}" for c in sorted(seen)))
if problems:
    print("\n!! PROBLEMS:", file=sys.stderr)
    for p in problems: print("  "+p, file=sys.stderr)
else:
    print("verify: clean, no orphans/conflicts.")
