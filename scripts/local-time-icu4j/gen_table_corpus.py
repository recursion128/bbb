#!/usr/bin/env python3
"""Emit corpora whose golden outputs become the generated locale tables."""
import sys

T_Q = {1: 1771059600000, 2: 1778835600000, 3: 1786784400000, 4: 1794733200000}
# Quarter-representative instants (mid-Feb/May/Aug/Nov 2026, 09:00 UTC).

locales = [line.strip() for line in open(sys.argv[1]) if line.strip()]
langs = sorted({l.split("_")[0] for l in locales if "_" in l})
all_ids = [""] + locales + [l for l in langs if l not in locales]

mode = sys.argv[2]
if mode == "quarters":
    for lid in all_ids:
        for pat in ["QQQ", "QQQQ", "QQQQQ", "qqq", "qqqq", "qqqqq"]:
            for q in (1, 2, 3, 4):
                print(f"{pat}\t{lid}\tAsia/Shanghai\t{T_Q[q]}")
elif mode == "gmtzero":
    for lid in all_ids:
        print(f"OOOO\t{lid}\tUTC\t{T_Q[3]}")
        print(f"O\t{lid}\tUTC\t{T_Q[3]}")
elif mode == "unknowncity":
    for lid in all_ids:
        print(f"VVV\t{lid}\tGMT+3\t{T_Q[3]}")
elif mode == "dayperiods":
    # Every minute of one UTC day, per width: the flexible day-period name
    # changes exactly at ICU4J's rule boundaries, so the sweep yields both the
    # boundaries and the names without reimplementing CLDR dayPeriodRules.
    T_DAY = 1785888000000  # 2026-08-05 00:00 UTC
    for lid in all_ids:
        for width in ("B", "BBBB", "BBBBB"):
            for minute in range(1440):
                print(f"{width}\t{lid}\tUTC\t{T_DAY + minute * 60000}")
