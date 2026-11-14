"""Generated cases: each function called with arguments drawn at random from a
seed, weighted toward the values where spreadsheets go wrong, then run through
Forge, LibreOffice and Gnumeric by harness.py.

    FORGE=../../target/release/forge python3 -I generate.py --seed 7 --per-function 20
    python3 -I generate.py --seed 7 --per-function 20 --only DATEDIF   # replay one function
    python3 -I generate.py --seed 1 --per-function 20 --fail           # as CI runs it

Every case is judged against LibreOffice and Gnumeric (or LibreOffice alone,
where JUDGES says why). Forge alone against the judges, or all three
different, is a failure unless a rule in known.py covers it (ADR-062).

The same seed and the same file give the same cases, in the same order, with
the same data columns: a case's id is <function>_<seed>_<n>.
"""
import argparse, datetime as dt, json, os, random, subprocess, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from harness import ERR, run, run_text, same
import known

BASE = dt.date(1899, 12, 30)
q = lambda s: json.dumps(s, ensure_ascii=False)

# ── values ────────────────────────────────────────────────────────────────────

EDGE_NUMBERS = [0, 1, -1, 0.5, -0.5, 2.5, -2.5, 1.005, 0.285, 0.1, 1e-12, -1e-12, 1e12, 1e15, 0.999999999, 3, 7, 12, 100,
                -100, 1234.5678, 0.0001, 45292, 2.675, 1/3]

def number(r):
    if r.random() < 0.4:
        return r.choice(EDGE_NUMBERS)
    scale = r.choice([1, 10, 1000, 1e6, 0.01])
    v = r.uniform(-1, 1) * scale
    return round(v, r.choice([0, 1, 2, 3, 6, 12]))

def positive(r):
    return abs(number(r)) or 1

def integer(r, lo, hi):
    """Mostly whole numbers in [lo, hi], sometimes a fraction or one just outside."""
    x = r.random()
    if x < 0.08: return lo - 1
    if x < 0.16: return hi + 1
    if x < 0.28: return round(r.uniform(lo, hi), 1)
    return r.randint(lo, hi)

def rate(r):
    return r.choice([0, 0.05, 0.1, -0.05, 1e-9, 0.5, 0.0125, 1, -0.99, round(r.uniform(-0.2, 0.3), 4)])

EDGE_DAYS = [(2, 28), (2, 29), (3, 1), (1, 31), (12, 31), (1, 1), (4, 30), (8, 31), (3, 31), (6, 30), (2, 27)]

def date_tuple(r):
    y = r.choice([1900, 1904, 1999, 2000, 2020, 2023, 2024, 2025, 2026, 2027, 2028, 2100, r.randint(1950, 2060)])
    if r.random() < 0.5:
        m, d = r.choice(EDGE_DAYS)
    else:
        m, d = r.randint(1, 12), r.randint(1, 31)
    while True:
        try:
            dt.date(y, m, d); return (y, m, d)
        except ValueError:
            d -= 1

def date(r):
    return "DATE(%d,%d,%d)" % date_tuple(r)

TEXTS = ["hello", "", " a  b ", "Hello World", "αβγ", "straße", "x", "ABCdef123", "a*b?c", "1,234.5", "50%", "  12 ",
         "apple pie", "Banana", "ÀÉÎ", "2024-01-15", "tab\tx", "-5", "TRUE", "~", "日本語"]

def text(r):
    return r.choice(TEXTS)

PATTERNS = ["b*", "*e", "?pple", "a*", "~*", "*", "?", "apple", "APPLE", "<>b*", "=apple", ">c", "<=b", "", "<>", ">2", "<0", "=0", "3",
            ">=1000", "<>3", "*a*"]

# ── data columns, drawn once per seed ─────────────────────────────────────────

def columns(r):
    """A table g of six rows: numbers with empty cells, dates, texts, a sorted
    and a descending column; another table h of the same with eight rows."""
    def nums(n, empty=True):
        return [None if empty and r.random() < 0.15 else number(r) for _ in range(n)]
    def dates(n):
        return ["%04d-%02d-%02d" % date_tuple(r) for _ in range(n)]
    def texts(n):
        return [r.choice(TEXTS[:15] + ["apple", "cherry", "date"]) for _ in range(n)]
    out = {}
    for name, n in (("g", 6), ("h", 8)):
        a = nums(n)
        s = sorted(r.sample(range(-20, 200), n))
        out[name] = {"a": a, "b": nums(n), "p": [positive(r) for _ in range(n)], "s": s, "sd": s[::-1],
                     "d": sorted(dates(n)), "t": texts(n), "cf": [round(r.uniform(-1, 1) * 1000, 2) for _ in range(n)]}
        out[name]["cf"][0] = -abs(out[name]["cf"][0]) - 100
    return out

def yaml_tables(cols):
    lines = []
    for name, cs in cols.items():
        lines.append(f"{name}:")
        for c, vals in cs.items():
            if c == "d":
                lines.append(f"  {c}: [{', '.join(vals)}]")
            elif c == "t":
                lines.append(f"  {c}: [{', '.join(q(v) for v in vals)}]")
            else:
                lines.append(f"  {c}: [{', '.join('null' if v is None else repr(v) for v in vals)}]")
        # A formula column, so the table is a table to every reader
        lines.append('  k: "=s * 1"')
    return "\n".join(lines) + "\n"

# ── the catalogue: what each argument is ─────────────────────────────────────
# n number, p positive, r rate, i:lo:hi integer, d date, t text, pat a criterion,
# col / pcol / scol / sdcol / dcol / tcol a column of g or h, c:a|b|c a choice,
# and ? before a kind for an argument that may be left out.

NUM, TXT = "num", "text"
CATALOGUE = {
    # dates
    "DATE": (NUM, "i:1899:2101 i:-2:14 i:-1:32"), "EDATE": (NUM, "d i:-25:25"), "EOMONTH": (NUM, "d i:-25:25"),
    "DATEDIF": (NUM, "d d c:\"Y\"|\"M\"|\"D\"|\"MD\"|\"YM\"|\"YD\""), "DAYS": (NUM, "d d"),
    "NETWORKDAYS": (NUM, "d d ?dcol"), "WORKDAY": (NUM, "d i:-40:40 ?dcol"), "YEARFRAC": (NUM, "d d ?i:0:4"),
    "WEEKDAY": (NUM, "d ?c:1|2|3|11|12|13|14|15|16|17"), "WEEKNUM": (NUM, "d ?c:1|2|11|12|13|14|15|16|17|21"),
    "ISOWEEKNUM": (NUM, "d"), "YEAR": (NUM, "d"), "MONTH": (NUM, "d"), "DAY": (NUM, "d"),
    "TIME": (NUM, "i:-1:30 i:-1:130 i:-1:130"), "HOUR": (NUM, "n"), "MINUTE": (NUM, "n"), "SECOND": (NUM, "n"),
    # financial
    "PMT": (NUM, "r i:1:360 n ?n ?c:0|1"), "PV": (NUM, "r i:1:360 n ?n ?c:0|1"), "FV": (NUM, "r i:1:360 n ?n ?c:0|1"),
    "NPER": (NUM, "r n n ?n ?c:0|1"), "RATE": (NUM, "i:1:360 n n ?n ?c:0|1"),
    "IPMT": (NUM, "r i:0:13 i:1:12 n ?n ?c:0|1"), "PPMT": (NUM, "r i:0:13 i:1:12 n ?n ?c:0|1"),
    "CUMIPMT": (NUM, "r i:1:60 p i:0:61 i:0:61 c:0|1"), "CUMPRINC": (NUM, "r i:1:60 p i:0:61 i:0:61 c:0|1"),
    "DB": (NUM, "p p i:1:20 i:0:21 ?i:1:12"), "DDB": (NUM, "p p i:1:20 i:0:21 ?c:1|1.5|2|3"), "SLN": (NUM, "n n i:0:20"),
    "SYD": (NUM, "p p i:1:20 i:0:21"), "VDB": (NUM, "p p i:1:20 i:0:20 i:0:21 ?c:1|2|3 ?c:TRUE|FALSE"),
    "EFFECT": (NUM, "r i:0:13"), "NOMINAL": (NUM, "r i:0:13"), "RRI": (NUM, "i:0:30 n n"),
    "NPV": (NUM, "r col"), "IRR": (NUM, "cfcol"), "MIRR": (NUM, "cfcol r r"),
    "PRICEDISC": (NUM, "d d r p ?i:0:4"), "YIELDDISC": (NUM, "d d p p ?i:0:4"), "ACCRINT": (NUM, "d d d r p c:1|2|4 ?i:0:4"),
    # statistical and aggregation
    "MEDIAN": (NUM, "col"), "VAR.S": (NUM, "col"), "VAR.P": (NUM, "col"), "STDEV.S": (NUM, "col"), "STDEV.P": (NUM, "col"),
    "MODE": (NUM, "col"), "GEOMEAN": (NUM, "pcol"), "SUM": (NUM, "col"), "AVERAGE": (NUM, "col"), "MIN": (NUM, "col"),
    "MAX": (NUM, "col"), "COUNT": (NUM, "col"), "COUNTA": (NUM, "col"), "PRODUCT": (NUM, "col"), "COUNTBLANK": (NUM, "col"),
    "PERCENTILE.INC": (NUM, "col c:0|0.1|0.25|0.5|0.75|0.9|1|-0.1|1.1"), "PERCENTILE.EXC": (NUM, "col c:0|0.1|0.25|0.5|0.75|0.9|1"),
    "QUARTILE.INC": (NUM, "col i:0:4"), "QUARTILE.EXC": (NUM, "col i:0:4"), "LARGE": (NUM, "col i:0:7"), "SMALL": (NUM, "col i:0:7"),
    "RANK.EQ": (NUM, "n col ?c:0|1"), "SLOPE": (NUM, "col col2"), "INTERCEPT": (NUM, "col col2"), "RSQ": (NUM, "col col2"),
    "CORREL": (NUM, "col col2"), "COVARIANCE.P": (NUM, "col col2"), "COVARIANCE.S": (NUM, "col col2"),
    "FORECAST": (NUM, "n col col2"), "SUMPRODUCT": (NUM, "col col2"),
    "NORM.DIST": (NUM, "n n p c:TRUE|FALSE"), "NORM.INV": (NUM, "c:0|0.001|0.025|0.5|0.975|0.999|1|0.3 n p"),
    "NORM.S.DIST": (NUM, "n c:TRUE|FALSE"), "NORM.S.INV": (NUM, "c:0|0.001|0.025|0.5|0.975|0.999|1|0.3"),
    # conditional
    "SUMIF": (NUM, "col pat ?col2"), "COUNTIF": (NUM, "anycol pat"), "AVERAGEIF": (NUM, "col pat ?col2"),
    "SUMIFS": (NUM, "col col2 pat"), "COUNTIFS": (NUM, "col pat col2 pat"), "AVERAGEIFS": (NUM, "col col2 pat"),
    "MAXIFS": (NUM, "col col2 pat"), "MINIFS": (NUM, "col col2 pat"),
    # math
    "ROUND": (NUM, "n i:-3:6"), "ROUNDUP": (NUM, "n i:-3:6"), "ROUNDDOWN": (NUM, "n i:-3:6"), "TRUNC": (NUM, "n ?i:-3:6"),
    "INT": (NUM, "n"), "MOD": (NUM, "n n"), "FLOOR": (NUM, "n n"), "CEILING": (NUM, "n n"), "MROUND": (NUM, "n n"),
    "POWER": (NUM, "n c:0|0.5|2|-1|3|0.333|-0.5"), "SQRT": (NUM, "n"), "EXP": (NUM, "c:0|1|-1|10|700|710|-745"),
    "LN": (NUM, "n"), "LOG10": (NUM, "n"), "LOG": (NUM, "n ?c:2|10|0.5|1|0"), "ABS": (NUM, "n"), "SIGN": (NUM, "n"),
    "SIN": (NUM, "n"), "COS": (NUM, "n"), "TAN": (NUM, "n"), "ASIN": (NUM, "n"), "ACOS": (NUM, "n"), "ATAN": (NUM, "n"),
    "SINH": (NUM, "n"), "COSH": (NUM, "n"), "TANH": (NUM, "n"),
    # text, measured as text
    "LEFT": (TXT, "t ?i:0:8"), "RIGHT": (TXT, "t ?i:0:8"), "MID": (TXT, "t i:0:9 i:0:6"), "LEN": (NUM, "t"),
    "UPPER": (TXT, "t"), "LOWER": (TXT, "t"), "TRIM": (TXT, "t"), "REPT": (TXT, "t i:0:3"),
    "FIND": (NUM, "c:\"l\"|\"a\"|\"\"|\"β\"|\"o␠W\"|\"*\" t ?i:0:12"), "SEARCH": (NUM, "c:\"l\"|\"A\"|\"\"|\"?\"|\"*e\"|\"~*\"|\"b?c\" t ?i:0:12"),
    "REPLACE": (TXT, "t i:0:9 i:0:4 t"), "SUBSTITUTE": (TXT, "t c:\"l\"|\"a\"|\"␠\"|\"\" t ?i:0:3"),
    "CONCAT": (TXT, "t n"), "TEXTJOIN": (TXT, "c:\",\"|\"\" c:TRUE|FALSE tcol"), "VALUE": (NUM, "t"),
    "TEXT": (TXT, "n c:\"0\"|\"0.00\"|\"#,##0\"|\"#,##0.00\"|\"0%\"|\"0.0%\"|\"$#,##0.00\"|\"0.00E+00\"|\"#,##0;(#,##0)\"|\"0.0,,\"|\"yyyy-mm-dd\"|\"mmm␠d,␠yyyy\""),
    # lookup
    "MATCH": (NUM, "sn scol c:0|1"), "XMATCH": (NUM, "sn anysorted ?c:0|-1|1 ?c:1|-1"),
    "XLOOKUP": (NUM, "sn scol col ?c:-1 ?c:0|-1|1 ?c:1|-1"), "INDEX": (NUM, "col i:0:9"), "CHOOSE": (NUM, "i:0:4 n n n"),
    "MATCHT": (NUM, "tpat tcol c:0"),
    # information
    "ISEVEN": (NUM, "n"), "ISODD": (NUM, "n"),
}
# Formulas whose name is not the function: MATCHT is MATCH of a text pattern
ALIAS = {"MATCHT": "MATCH"}

def arg(r, kind, cols):
    tab = cols["__table__"]
    rows = len(cols[tab]["s"])
    if kind == "n": return repr(number(r))
    if kind == "p": return repr(positive(r))
    if kind == "r": return repr(rate(r))
    if kind == "d": return date(r)
    if kind == "t": return q(text(r))
    if kind == "pat": return q(r.choice(PATTERNS))
    if kind == "tpat": return q(r.choice(["b*", "*e", "?pple", "apple", "APPLE", "~*", "c?erry", "zzz", "*"]))
    if kind == "sn": return repr(r.choice([r.randint(-25, 210), number(r)]))
    if kind.startswith("i:"):
        lo, hi = map(int, kind[2:].split(":")); return repr(integer(r, lo, hi))
    if kind.startswith("c:"):
        return r.choice(kind[2:].split("|")).replace("␠", " ")   # ␠: a space inside a choice
    if kind in ("col", "col2"): return f"{tab}." + r.choice(["a", "b", "p", "cf"])
    if kind == "pcol": return f"{tab}.p"
    if kind == "cfcol": return f"{tab}.cf"
    if kind == "scol": return f"{tab}.s"
    if kind == "anysorted": return f"{tab}." + r.choice(["s", "sd"])
    if kind == "dcol": return f"{tab}.d"
    if kind == "tcol": return f"{tab}.t"
    if kind == "anycol": return f"{tab}." + r.choice(["a", "t", "d", "p"])
    raise ValueError(kind)

def cases_for(seed, per_function, only=None):
    r = random.Random(seed)
    cols = columns(r)
    out = []
    for name in sorted(CATALOGUE):
        if only and name not in only and ALIAS.get(name) not in only:
            continue
        kind, spec = CATALOGUE[name]
        fr = random.Random(f"{seed}/{name}")   # a function's cases do not move when another is added
        for n in range(per_function):
            cols["__table__"] = fr.choice(["g", "h"])
            args = []
            for a in spec.split():
                if a.startswith("?"):
                    if fr.random() < 0.35: break
                    a = a[1:]
                args.append(arg(fr, a, cols))
            fn = ALIAS.get(name, name)
            cid = f"{name.replace('.', '_').lower()}_{seed}_{n}"
            formula = f"{fn}({','.join(args)})"
            case = {"id": cid, "fn": fn, "name": name, "args": args, "kind": kind, "formula": "=" + formula}
            if kind == TXT and any(a.startswith(("g.", "h.")) for a in args):
                # A text case runs in a one-row table, which cannot read
                # another table's column row by row: measure it as a scalar
                case.update(kind=NUM, formula=f"=LEN({formula})")
            out.append(case)
    del cols["__table__"]
    return out, cols

# Who judges a function (ADR-062, decision 3): both engines unless one cannot
JUDGES = {
    "DATEDIF": ("lo", "Gnumeric 1.12 cannot read DATEDIF from an xlsx file"),
    "XMATCH": ("lo", "Gnumeric 1.12 has no XMATCH"),
    "XLOOKUP": ("lo", "Gnumeric 1.12 has no XLOOKUP"),
}

def verdict(case):
    """agreed, forge-alone, judges-split or all-different (ADR-062, decision 3)."""
    f, lo, gn = case["forge"], case["lo"], case["gn"]
    if case["fn"] in JUDGES:
        return "agreed" if same(f, lo) else "forge-alone"
    if same(f, lo) and same(f, gn):
        return "agreed"
    if same(lo, gn):
        return "forge-alone"
    if same(f, lo) or same(f, gn):
        return "judges-split"
    return "all-different"

def show(v):
    return "an error" if v == ERR or v == "#ERR" else repr(v)

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--seed", type=int, required=True)
    p.add_argument("--per-function", type=int, default=20)
    p.add_argument("--only", nargs="*", help="function names to replay (as in the catalogue)")
    p.add_argument("--work", default="work")
    p.add_argument("--fail", action="store_true", help="exit 1 when a case fails (CI)")
    a = p.parse_args()
    cases, cols = cases_for(a.seed, a.per_function, set(a.only) if a.only else None)
    extra = yaml_tables(cols)
    for kind, runner, suffix in ((NUM, run, ""), (TXT, run_text, "t")):
        part = [c for c in cases if c["kind"] == kind]
        if not part:
            continue
        values = {}
        for n, f, lo, gn in runner(f"gen{a.seed}{suffix}", [(c["id"], c["formula"]) for c in part], a.work, extra):
            values[n] = (f, lo, gn)
        for c in part:
            c["forge"], c["lo"], c["gn"] = values[c["id"]]
    counts = {"agreed": 0, "forge-alone": 0, "judges-split": 0, "all-different": 0}
    failures, explained = [], {r.id: 0 for r in known.RULES}
    for c in cases:
        v = verdict(c)
        c["verdict"] = v
        counts[v] += 1
        if v in ("forge-alone", "all-different"):
            rule = known.explain(c)
            if rule:
                explained[rule.id] += 1
            else:
                failures.append((v, c))
    commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True).stdout.strip()
    print(f"seed {a.seed}, {a.per_function} a function, {len(cases)} cases (commit {commit or 'unknown'}): "
          f"{counts['agreed']} agreed, {counts['judges-split']} with the engines split (reported, not failed), "
          f"{counts['forge-alone'] + counts['all-different'] - len(failures)} covered by a known difference, {len(failures)} failed")
    for rule in known.RULES:
        if explained[rule.id]:
            print(f"  known {rule.id}: {explained[rule.id]}  ({rule.tracked})")
    unused = [r.id for r in known.RULES if not explained[r.id]]
    if unused and not a.only:
        print(f"  known differences that matched nothing this run: {', '.join(unused)}")
    by_fn = {}
    for v, c in failures:
        by_fn.setdefault(c["name"], []).append((v, c))
    for name, items in sorted(by_fn.items()):
        print(f"FAIL {name}: {len(items)} case(s); replay: python3 -I generate.py --seed {a.seed} --per-function {a.per_function} --only {name}")
        for v, c in items[:5]:
            judges = "LibreOffice" if c["fn"] in JUDGES else "LibreOffice, Gnumeric"
            print(f"     {c['id']}: {c['formula']}  Forge {show(c['forge'])}; {judges}: {show(c['lo'])}"
                  + ("" if c["fn"] in JUDGES else f", {show(c['gn'])}") + f"  [{v}]")
    json.dump(cases, open(os.path.join(a.work, f"gen{a.seed}_cases.json"), "w"), indent=1, default=str)
    # The workbooks and engine outputs of a large run fill the disk
    for f in os.listdir(a.work):
        if f.startswith(f"gen{a.seed}") and f.endswith(".xlsx"):
            os.remove(os.path.join(a.work, f))
    if a.fail and failures:
        sys.exit(1)

if __name__ == "__main__":
    main()
