import sys, itertools; sys.path.insert(0, ".")
from harness import report
cases = []
def add(p, f): cases.append((f"{p}{len(cases)}", f))
rates = [0, 0.05/12, 0.08, -0.01, 0.5]
for r, n, pv, fv, ty in itertools.product(rates, [1, 12, 360, 0.5], [0, -100000, 2500], [0, 5000], [0, 1]):
    add("pmt", f"=PMT({r},{n},{pv},{fv},{ty})")
    add("fv", f"=FV({r},{n},{pv if pv else -100},{fv},{ty})")
    add("pv", f"=PV({r},{n},-500,{fv},{ty})")
for r, pmt, pv, fv, ty in itertools.product(rates, [-500, 0, 500], [10000, -10000, 0], [0, 20000], [0, 1]):
    add("nper", f"=NPER({r},{pmt},{pv},{fv},{ty})")
for r, n, per, ty in itertools.product([0, 0.06/12, 0.1], [12, 36], [0, 1, 6, 12, 13, 36, 1.5], [0, 1]):
    add("ipmt", f"=IPMT({r},{per},{n},-20000,0,{ty})")
    add("ppmt", f"=PPMT({r},{per},{n},-20000,0,{ty})")
for r, n, s, e, ty in itertools.product([0.09/12, 0], [360, 12], [1, 13, 0, 2], [12, 24, 1, 400], [0, 1, 2]):
    add("cumipmt", f"=CUMIPMT({r},{n},125000,{s},{e},{ty})")
    add("cumprinc", f"=CUMPRINC({r},{n},125000,{s},{e},{ty})")
for n, pmt, pv, fv, ty, g in [(48,-200,8000,0,0,0.1),(10,-1000,5000,0,0,0.1),(12,0,-1000,1500,0,0.1),(5,-100,100,0,0,0.1),(360,-1000,180000,0,1,0.01),(10,100,1000,0,0,0.1),(3,-1,10,0,0,-0.5),(60,-500,25000,-1000,1,0.02)]:
    add("rate", f"=RATE({n},{pmt},{pv},{fv},{ty},{g})")
for c, s, l in itertools.product([10000, 2400, 1], [0, 1000, 10000], [1, 5, 10, 0.5]):
    add("sln", f"=SLN({c},{s},{l})")
    add("syd", f"=SYD({c},{s},{l},1)"); add("syd", f"=SYD({c},{s},{l},{l})")
    for per in (1, 2, 5, 10, 11):
        for m in (12, 6, 1):
            add("db", f"=DB({c},{s},{l},{per},{m})")
        for fct in (2, 1.5, 3):
            add("ddb", f"=DDB({c},{s},{l},{per},{fct})")
for args in ["2400,300,120,0,1","2400,300,120,0,0.875","2400,300,10,0,0.875","2400,300,10,6,18,1.5","2400,300,10,6,18,1.5,TRUE","2400,300,10,0,10","2400,300,10,0,10,2,TRUE","10000,1000,5,1.5,3.5","10000,1000,5,0,5,3","10000,0,4,2,4,2","100,10,3,0,3,1","100,10,3,2.5,3"]:
    add("vdb", f"=VDB({args})")
for r, np_ in itertools.product([0.05, 0.123, 0.0, -0.02], [1, 2, 4, 12, 365, 2.7, 0.5]):
    add("effect", f"=EFFECT({r},{np_})"); add("nominal", f"=NOMINAL({r},{np_})")
for n, pv, fv in [(96,10000,11000),(1,100,110),(10,100,50),(0,100,110),(5,-100,-150),(5,100,-150),(2.5,100,121)]:
    add("rri", f"=RRI({n},{pv},{fv})")
print(len(cases))
extra = """flows:
  a: [-10000, 3000, 4200, 6800]
  b: [-100, 39, 59, 55, 20]
  c: [100, 200, 300]
  d: [-70000, 12000, 15000, 18000, 21000, 26000]
  e: [-1000, 0, 0, 0, 1200]
  dates: [2008-01-01, 2008-03-01, 2008-10-30, 2009-02-15]
  f: [-10000, 2750, 4250, 3250]
"""
for col in "abcde":
    add("irr", f"=IRR(flows.{col})"); add("irrg", f"=IRR(flows.{col}, -0.5)")
    for r in (0.1, 0, -0.05):
        add("npv", f"=NPV({r}, flows.{col})")
    add("mirr", f"=MIRR(flows.{col}, 0.1, 0.12)")
for r in (0.09, 0, -0.5):
    add("xnpv", f"=XNPV({r}, flows.f, flows.dates)")
add("xirr", "=XIRR(flows.f, flows.dates)"); add("xirr", "=XIRR(flows.f, flows.dates, 0.5)")
report("fin", cases, "work", extra)
