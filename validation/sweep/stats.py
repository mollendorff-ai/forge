import sys; sys.path.insert(0, ".")
from harness import report
# data sets as table columns of one table each (same length within a table)
SETS = {
    "da": [5, 1, 3, 3, 8],
    "db": [2.5, -1, 0, 7.25, 3, 3, -4, 10],
    "dc": [4],
    "de": [1, 2],
    "dg": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110],
    "dh": [1, 1, 2, 2, 3],
    "dp": [1.5, 2.25, 9, 0.75, 4, 16],
}
PAIRS = {"x": [1, 2, 3, 4, 5, 6], "y": [2.1, 3.9, 6.2, 7.8, 10.1, 12.2], "z": [5, 5, 5, 5, 5, 5], "w": [-3, 1, 0, 8, -2, 4]}
extra = "".join(f"{k}:\n  v: {v}\n" for k, v in SETS.items()) + "pr:\n" + "".join(f"  {k}: {v}\n" for k, v in PAIRS.items())
cases = []
def add(p, f): cases.append((f"{p}{len(cases)}", f))
for k in SETS:
    r = f"{k}.v"
    for fn in ("MEDIAN", "VAR.S", "VARP", "STDEV.S", "STDEVP", "VAR", "VAR.P", "STDEV", "STDEV.P", "MODE", "MODE.SNGL", "GEOMEAN",
               "SUM", "AVERAGE", "MIN", "MAX", "COUNT", "COUNTA", "PRODUCT", "COUNTUNIQUE"):
        add(fn.replace(".", "_").lower(), f"={fn}({r})")
    for kk in (0, 0.1, 0.25, 0.3333, 0.5, 0.9, 0.99, 1, -0.1, 1.1):
        for fn in ("PERCENTILE.INC", "PERCENTILE.EXC", "PERCENTILE"):
            add("pct", f"={fn}({r},{kk})")
    for qq in (0, 1, 2, 3, 4, 1.5, 5, -1):
        for fn in ("QUARTILE.INC", "QUARTILE.EXC", "QUARTILE"):
            add("qrt", f"={fn}({r},{qq})")
    for n in (0, 1, 2, 3, 5, 1.5, 9):
        add("large", f"=LARGE({r},{n})"); add("small", f"=SMALL({r},{n})")
    for v in (3, 1, 8, 2.5, 99, 5, 10):
        add("rank", f"=RANK.EQ({v},{r})"); add("ranka", f"=RANK.EQ({v},{r},1)")
    for crit in ('">2"', '"<3"', '"=3"', '"<>3"', '3', '">=2.5"', '"<=-1"', '""', '"3"'):
        add("sumif", f"=SUMIF({r},{crit})"); add("countif", f"=COUNTIF({r},{crit})"); add("avgif", f"=AVERAGEIF({r},{crit})")
for a, b in (("y", "x"), ("x", "y"), ("z", "x"), ("x", "z"), ("w", "x"), ("y", "w")):
    A, B = f"pr.{a}", f"pr.{b}"
    for fn in ("SLOPE", "INTERCEPT", "RSQ", "COVARIANCE.P", "COVARIANCE.S", "CORREL", "SUMPRODUCT"):
        add(fn.replace(".", "_").lower(), f"={fn}({A},{B})")
    for x0 in (0, 3.5, -2, 10):
        add("fc", f"=FORECAST({x0},{A},{B})"); add("fcl", f"=FORECAST.LINEAR({x0},{A},{B})")
    for crit in ('">3"', '"<=2"', '5'):
        add("sumifs", f"=SUMIFS({A},{B},{crit})"); add("countifs", f"=COUNTIFS({A},{crit},{B},{crit})")
        add("avgifs", f"=AVERAGEIFS({A},{B},{crit})"); add("maxifs", f"=MAXIFS({A},{B},{crit})"); add("minifs", f"=MINIFS({A},{B},{crit})")
for x in (-3, -1.5, 0, 0.5, 1, 2.326, 6, 40):
    for m, s in ((0, 1), (1, 2), (-1, 0.5), (0, 0), (0, -1)):
        for cum in ("TRUE", "FALSE"):
            add("nd", f"=NORM.DIST({x},{m},{s},{cum})")
    for cum in ("TRUE", "FALSE"):
        add("nsd", f"=NORM.S.DIST({x},{cum})")
for p in (0, 1e-10, 0.001, 0.025, 0.1, 0.5, 0.75, 0.975, 0.999999, 1, -0.1):
    add("nsi", f"=NORM.S.INV({p})")
    for m, s in ((0, 1), (10, 3), (0, 0), (0, -1)):
        add("ni", f"=NORM.INV({p},{m},{s})")
print(len(cases))
report("stats", cases, "work", extra)
