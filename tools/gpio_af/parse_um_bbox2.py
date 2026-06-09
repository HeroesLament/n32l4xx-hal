#!/usr/bin/env python3
"""
Coordinate-aware parser for the N32L40x UM alternate-function remap tables,
v2: contiguous-segmentation assignment.

The UM renders each signal label at the VERTICAL CENTER of its block of
pin-rows. Pure nearest-label or y-partition heuristics fail when a 1-pin
block sits next to a tall multi-pin block (a boundary pin is physically
closer to the neighbor's center than to its own). See _inspect_usart1.py.

Correct model: within one table, the pin-rows (sorted by y) must be split
into exactly N contiguous groups, one per signal label (also sorted by y),
and label_i is the center of group_i. We therefore choose the contiguous
partition minimizing sum |median_y(group_i) - y(label_i)|. This respects
both (a) pins of a signal are contiguous and (b) the label centers its block.
Solved exactly with DP.

Input : um_af_bbox.html  (pdftotext -bbox, physical pp.121-132)
Output: af_table_um.tsv  SIGNAL<TAB>PIN<TAB>AF<TAB>table

Prints diagnostics: per-table label/pin counts, any table where #labels
> #pins (impossible -> parse bug), and the worst per-assignment residual
(large residual = suspced misparse, inspect).
"""
import os, re, html
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
BBOX = os.path.join(HERE, "um_af_bbox.html")

WORD_RE = re.compile(
    r'<word xMin="([\d.]+)" yMin="([\d.]+)" xMax="([\d.]+)" yMax="([\d.]+)">(.*?)</word>'
)
PIN1_RE  = re.compile(r"^P([A-D])(\d{1,2})$")
RANGE_RE = re.compile(r"^P([A-D])(\d{1,2})~P([A-D])(\d{1,2})$")
AF_RE    = re.compile(r"^AF(\d{1,2})$")
# signal label: leftish column, all-caps token. Accept with-underscore
# (TIM1_CH1, SPI1_I2S1_NSS_WS) AND a small allowlist of legitimately-bare
# signals. We must REJECT bare peripheral names (TIM5, SPI1, USART2, ...)
# and column/section furniture (A, B, D, USART, COMP, EXTI, RTC, ...) which
# share the all-caps left-column shape but are section sub-headers, not
# signals. Getting this wrong inflates a table's label count past its pin
# count (impossible) and mis-mux a pin -> a silent hardware fault.
BARE_SIGNAL_RE = re.compile(
    r"^(EVENTOUT|MCO|COM\d+|SEG\d+|JTDI|JTDO|NJTRST)$"
)
def is_signal(t):
    if AF_RE.match(t) or PIN1_RE.match(t) or RANGE_RE.match(t):
        return False
    if "_" in t and re.match(r"^[A-Z][A-Z0-9_]*[A-Z0-9]$", t):
        return True
    if BARE_SIGNAL_RE.match(t):
        return True
    return False

X_SIG_MAX = 235.0   # label column left of this

def load_pages():
    pages, cur = [], None
    for line in open(BBOX, encoding="utf-8"):
        if "<page " in line:
            cur = []; pages.append(cur)
        m = WORD_RE.search(line)
        if m and cur is not None:
            x0, y0, x1, y1, t = m.groups()
            cur.append({"x": float(x0), "y": float(y0), "t": html.unescape(t).strip()})
    return pages

def group_rows(words, ytol=3.0):
    rows = []
    for w in sorted(words, key=lambda w: (w["y"], w["x"])):
        for r in rows:
            if abs(r["y"] - w["y"]) <= ytol:
                r["ws"].append(w); break
        else:
            rows.append({"y": w["y"], "ws": [w]})
    for r in rows:
        r["ws"].sort(key=lambda w: w["x"])
    rows.sort(key=lambda r: r["y"])
    return rows

def segment_assign(labels, pins):
    """labels: [(y, sig)] sorted; pins: [(y, pin, af)] sorted.
    Partition pins into len(labels) contiguous groups minimizing
    sum |group_median_y - label_y|. Returns list parallel to pins of sig.
    DP over (label_index, pin_index)."""
    L = len(labels); M = len(pins)
    if L == 0:
        return [None]*M
    if L == 1:
        return [labels[0][1]]*M
    py = [p[0] for p in pins]
    ly = [l[0] for l in labels]
    # cost[i][j] = cost of assigning pins[i:j] (contiguous) to one label center ly[?]
    # We DP: dp[k][j] = min cost to assign first j pins to first k labels.
    import statistics
    def med(a, b):  # median of py[a:b]
        seg = py[a:b]
        return statistics.median(seg)
    INF = float("inf")
    # dp[k][j]
    dp = [[INF]*(M+1) for _ in range(L+1)]
    back = [[-1]*(M+1) for _ in range(L+1)]
    dp[0][0] = 0.0
    for k in range(1, L+1):
        # k-th label (index k-1) takes pins[i:j], i from previous boundary
        for j in range(k, M+1):           # need >=1 pin per label
            for i in range(k-1, j):       # previous boundary
                if dp[k-1][i] == INF:
                    continue
                c = dp[k-1][i] + abs(med(i, j) - ly[k-1])
                if c < dp[k][j]:
                    dp[k][j] = c; back[k][j] = i
    # reconstruct
    assign = [None]*M
    j = M
    for k in range(L, 0, -1):
        i = back[k][j]
        for p in range(i, j):
            assign[p] = labels[k-1][1]
        j = i
    return assign

def main():
    pages = load_pages()
    triples = []
    diag = []   # (table, nlabels, npins, worst_resid)

    cur_table = None
    labels, pins = [], []

    def flush():
        nonlocal labels, pins, cur_table
        if cur_table and pins:
            assign = segment_assign(sorted(labels), sorted(pins))
            for (py, pin, af), sig in zip(sorted(pins), assign):
                triples.append((sig, pin, af, cur_table))
            diag.append((cur_table, len(labels), len(pins)))
        labels, pins = [], []

    for page in pages:
        for r in group_rows(page):
            txt = " ".join(w["t"] for w in r["ws"])
            mt = re.search(r"Table\s+5-(\d+)", txt)
            if mt:
                flush(); cur_table = "5-" + mt.group(1); continue
            if "I/O configuration of peripherals" in txt:
                flush(); cur_table = None; continue
            if cur_table is None:
                continue
            row_pin = row_af = row_sig = None
            # A section sub-header row reads "<PERIPH> alternate function
            # remapping" -- its leading token shares the signal column's
            # shape but is NOT a signal. Detect such rows by context and
            # suppress label capture on them. (x-position alone fails: long
            # SPI signal names like SPI1_I2S1_MISO_MCK start at x~89,
            # overlapping the header indent.)
            is_header_row = ("alternate" in txt and "remapping" in txt) or \
                            ("Alternate" in txt and "function" in txt)
            for w in r["ws"]:
                t = w["t"]; tc = re.sub(r"\(\d+\)$", "", t)
                if AF_RE.match(t):
                    row_af = int(AF_RE.match(t).group(1))
                elif PIN1_RE.match(tc) or RANGE_RE.match(tc):
                    row_pin = tc
                elif is_signal(t) and w["x"] < X_SIG_MAX and not is_header_row:
                    row_sig = t
            if row_sig:
                labels.append((r["y"], row_sig))
            if row_pin and row_af is not None:
                mr = RANGE_RE.match(row_pin)
                if mr and mr.group(1) == mr.group(3):
                    # a pin RANGE on one row, all same AF, all same signal:
                    # emit each but tag with the row y so segmentation keeps them together
                    for n in range(int(mr.group(2)), int(mr.group(4))+1):
                        pins.append((r["y"], f"P{mr.group(1)}{n}", row_af))
                else:
                    pins.append((r["y"], row_pin, row_af))
    flush()

    # de-dup
    seen=set(); out=[]
    for s,p,a,t in triples:
        if (s,p,a) in seen: continue
        seen.add((s,p,a)); out.append((s,p,a,t))

    with open(os.path.join(HERE,"af_table_um.tsv"),"w") as f:
        f.write("# signal\tpin\taf\tsource_table  (N32L40x UM V2.5.0, DP-segmented)\n")
        for s,p,a,t in sorted(out,key=lambda r:(r[1],r[0] or "")):
            f.write(f"{s}\t{p}\t{a}\tTable {t}\n")

    print(f"triples: {len(out)}  pins: {len(set(p for _,p,_,_ in out))}  signals: {len(set(s for s,_,_,_ in out))}")
    # impossible tables
    bad = [(t,nl,npn) for (t,nl,npn) in diag if nl>npn]
    if bad:
        print("TABLES with #labels>#pins (parse bug):")
        for t,nl,npn in bad: print(f"  {t}: {nl} labels, {npn} pins")
    sp=defaultdict(set)
    for s,p,a,_ in out: sp[(s,p)].add(a)
    conf={k:v for k,v in sp.items() if len(v)>1}
    print("conflicts:", len(conf))
    for (s,p),v in sorted(conf.items()): print(f"  {s} {p}: {sorted(v)}")
    none_sig=[r for r in out if r[0] is None]
    print("None-signal rows:", len(none_sig))

if __name__ == "__main__":
    main()
