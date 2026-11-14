"""Run formula cases through Forge, LibreOffice and Gnumeric, and compare.

Each case (id, formula) becomes a scalar t.<id> = IFERROR(formula, -999999),
so an error on either side is the number -999999. Forge calculates the model;
`forge export` writes it to a workbook; LibreOffice recalculates a copy with
the cached values removed, and Gnumeric recalculates with --recalc. A value
is the same when it agrees to 1e-9, relative. The result, in
<workdir>/<name>_result.json, sorts every case into: all agree; Forge against
both engines; the engines against each other (Forge with one of them); all
three different. A case where Forge is alone is a candidate, not a verdict:
the suite holds Excel checks where Excel agrees with Forge against both.
"""
import csv, datetime as dt, json, os, re, shutil, subprocess, sys, zipfile

FORGE = os.environ.get("FORGE", "forge")
BASE = dt.date(1899, 12, 30)
ERR = -999999.0

def to_num(s):
    s = s.strip().strip('"')
    if s == "": return None
    m = re.fullmatch(r"(\d{4})[-/](\d{2})[-/](\d{2})", s)
    if m:
        return float((dt.date(*map(int, m.groups())) - BASE).days)
    m = re.fullmatch(r"(-?[\d.]+(?:E[-+]?\d+)?)%", s)
    if m: return float(m.group(1)) / 100
    m = re.fullmatch(r"(\d{1,2}):(\d{2}):(\d{2}(?:\.\d+)?)(?: (AM|PM))?", s)
    if m:
        h, mi, se, ap = int(m.group(1)), int(m.group(2)), float(m.group(3)), m.group(4)
        if ap == "PM" and h != 12: h += 12
        if ap == "AM" and h == 12: h = 0
        return (h * 3600 + mi * 60 + se) / 86400
    m = re.fullmatch(r"(\d{2})/(\d{2})/(\d{4,5})", s)
    if m:
        mo, d, y = map(int, m.groups())
        try: return float((dt.date(y, mo, d) - BASE).days)
        except ValueError: return s
    if s.upper() in ("TRUE", "FALSE"): return 1.0 if s.upper() == "TRUE" else 0.0
    try: return float(s)
    except ValueError: return s

def run(name, cases, workdir, extra=""):
    os.makedirs(workdir, exist_ok=True)
    y = os.path.join(workdir, f"{name}.yaml")
    with open(y, "w") as f:
        f.write('_forge_version: "5.4.0"\nt:\n')
        for cid, formula in cases:
            body = formula.lstrip("=")
            f.write(f'  {cid}: {{formula: {json.dumps("=IFERROR(" + body + ", -999999)")}}}\n')
        f.write(extra)
    out = subprocess.run([FORGE, "calculate", y, "--dry-run"], capture_output=True, text=True)
    forge = {}
    for line in out.stdout.splitlines():
        m = re.match(r"\s+t\.(\w+) = (.*)$", line)
        if m: forge[m.group(1)] = to_num(m.group(2))
    if not forge:
        print(out.stdout[-2000:], out.stderr[-2000:]); raise SystemExit("forge failed")
    x = os.path.join(workdir, f"{name}.xlsx")
    r = subprocess.run([FORGE, "export", y, x], capture_output=True, text=True)
    if r.returncode: print(r.stdout, r.stderr); raise SystemExit("export failed")
    # strip cached values so LibreOffice recalculates
    s = os.path.join(workdir, f"{name}_nocache.xlsx")
    with zipfile.ZipFile(x) as zin, zipfile.ZipFile(s, "w", zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename.startswith("xl/worksheets/"):
                data = re.sub(rb"(<c [^>]*>)(<f>[^<]*</f>)<v>[^<]*</v>", rb"\1\2", data)
            zout.writestr(item, data)
    lo_dir = os.path.join(workdir, "lo_" + name); shutil.rmtree(lo_dir, ignore_errors=True)
    subprocess.run(["soffice", "--headless", "--convert-to",
                    "csv:Text - txt - csv (StarCalc):44,34,76,1,,0,false,true,false,false,false,-1",
                    "--outdir", lo_dir, s], capture_output=True)
    gn_dir = os.path.join(workdir, "gn_" + name); shutil.rmtree(gn_dir, ignore_errors=True); os.makedirs(gn_dir)
    subprocess.run(["ssconvert", "-S", "--recalc", x, os.path.join(gn_dir, "%s.csv")], capture_output=True)
    def read(p):
        return list(csv.reader(open(p)))
    lo_rows = read(os.path.join(lo_dir, f"{name}_nocache-t.csv"))
    sheets = sorted(os.listdir(gn_dir))
    gn_rows = max((read(os.path.join(gn_dir, f)) for f in sheets), key=len)
    # LibreOffice CSV loses the Name column header text when values are blank; align by position
    names = [r[0] for r in gn_rows[1:]]
    assert len(names) == len(cases), f"engine CSV has {len(names)} rows for {len(cases)} cases (wrong sheet?)"
    assert len(lo_rows) - 1 == len(cases), f"LibreOffice CSV has {len(lo_rows)-1} rows for {len(cases)} cases"
    lo_vals = [to_num(r[1]) if len(r) > 1 else None for r in lo_rows[1:]]
    gn_vals = [to_num(r[1]) if len(r) > 1 else None for r in gn_rows[1:]]
    res = []
    for i, n in enumerate(names):
        res.append((n, forge.get(n), lo_vals[i] if i < len(lo_vals) else None, gn_vals[i]))
    return res

def same(a, b):
    if a is None or b is None: return a is b
    if isinstance(a, str) or isinstance(b, str): return a == b
    return abs(a - b) <= 1e-9 * max(1.0, abs(a), abs(b)) or (abs(a) < 1e-6 and abs(a - b) <= 5e-15)

def classify(res, formulas):
    out = {"all_agree": [], "forge_vs_both": [], "engines_differ_forge_lo": [], "engines_differ_forge_gn": [], "all_differ": []}
    for n, f, lo, gn in res:
        fl, fg, lg = same(f, lo), same(f, gn), same(lo, gn)
        if fl and fg: out["all_agree"].append(n)
        elif lg: out["forge_vs_both"].append((n, formulas[n], f, lo))
        elif fl: out["engines_differ_forge_lo"].append((n, formulas[n], f, gn))
        elif fg: out["engines_differ_forge_gn"].append((n, formulas[n], f, lo))
        else: out["all_differ"].append((n, formulas[n], f, lo, gn))
    return out

def report(name, cases, workdir, extra=""):
    formulas = dict(cases)
    res = run(name, cases, workdir, extra)
    c = classify(res, formulas)
    print(f"== {name}: {len(res)} cases; all agree {len(c['all_agree'])}; Forge vs both engines {len(c['forge_vs_both'])}; "
          f"engines differ (Forge=LO) {len(c['engines_differ_forge_lo'])}, (Forge=Gnumeric) {len(c['engines_differ_forge_gn'])}; all differ {len(c['all_differ'])}")
    json.dump(c, open(os.path.join(workdir, f"{name}_result.json"), "w"), indent=1, default=str)
    return c
