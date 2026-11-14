import sys; sys.path.insert(0, ".")
from harness import report
cases = []
def add(p, f): cases.append((f"{p}{len(cases)}", f))
V = [2.5, -2.5, 1.005, 0.285, 2.675, 1234.5678, -1234.5678, 0.5, -0.5, 1.5, 0, 1e-12, 123456789.123, 0.1+0.2, 4.35, 1.45, 5.015, -0.049999]
for v in V:
    for d in (-2, -1, 0, 1, 2, 3, 1.7, -1.2):
        add("round", f"=ROUND({v!r},{d})"); add("rup", f"=ROUNDUP({v!r},{d})"); add("rdn", f"=ROUNDDOWN({v!r},{d})")
    add("int", f"=INT({v!r})"); add("trunc", f"=TRUNC({v!r})"); add("trunc2", f"=TRUNC({v!r},2)"); add("sign", f"=SIGN({v!r})")
for v in (7.3, -7.3, 2.5, -2.5, 0, 10, -10, 0.125):
    for s in (1, -1, 0.5, 2, -2, 0, 0.1, 3):
        add("floor", f"=FLOOR({v},{s})"); add("ceil", f"=CEILING({v},{s})"); add("mround", f"=MROUND({v},{s})")
    for d in (3, -3, 2.5, -2.5, 0, 1, 0.1):
        add("mod", f"=MOD({v},{d})")
for v, s in ((0.3, 0.1), (0.7, 0.1), (1.1, 0.1), (2.3, 0.1), (-0.3, -0.1), (0.6, 0.2), (4.35, 0.05), (1e15 + 0.3, 1)):
    add("floorx", f"=FLOOR({v!r},{s!r})"); add("ceilx", f"=CEILING({v!r},{s!r})"); add("mroundx", f"=MROUND({v!r},{s!r})")
for b, e in ((0, 0), (0, -1), (-8, 1/3), (-8, 2), (2, 0.5), (-2, -2), (10, 308), (2, -1074), (1.5, 2.5), (0, 0.5)):
    add("pow", f"=POWER({b},{e!r})"); add("powop", f"={b}^{e!r}" if b >= 0 else f"=({b})^{e!r}")
for v in (0, 1, -1, 0.5, 100, 1e-300, 2.718281828459045):
    add("ln", f"=LN({v})"); add("log10", f"=LOG10({v})"); add("log", f"=LOG({v})"); add("sqrt", f"=SQRT({v})"); add("exp", f"=EXP({v})")
    for base in (2, 10, 0.5, 1, 0, -2):
        add("logb", f"=LOG({v},{base})")
for v in (0, 0.5, 1, -1, 1.0000001, 3.14159, -100, 1e10, 710, -710):
    for fn in ("SIN", "COS", "TAN", "ASIN", "ACOS", "ATAN", "SINH", "COSH", "TANH", "DEGREES", "RADIANS", "ABS"):
        add(fn.lower(), f"={fn}({v})")
for v in (2, 3, 2.5, -2.5, -3, 0, 3.9, -3.9):
    add("iseven", f"=IF(ISEVEN({v}),1,0)"); add("isodd", f"=IF(ISODD({v}),1,0)")
print(len(cases))
report("math", cases, "work")
