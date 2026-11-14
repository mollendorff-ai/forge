import sys, itertools; sys.path.insert(0, ".")
from harness import report
cases = []
def add(p, f): cases.append((f"{p}{len(cases)}", f))
for (c, s, l), st, ln, fac, ns in itertools.product([(100,10,3),(2400,300,10),(10000,1000,5),(50000,5000,8),(900,100,4)],
        [0, 0.5, 1, 1.25, 2.5, 2.75, 3.5, 4.1, 6.5], [0.25, 0.5, 1, 1.5, 3], [2, 1.5, 3, 1], ["FALSE", "TRUE"]):
    if st + ln <= l:
        add("vdb", f"=VDB({c},{s},{l},{st},{st+ln},{fac},{ns})")
print(len(cases))
report("vdb", cases, "work")
