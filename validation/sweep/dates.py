import sys, itertools; sys.path.insert(0, ".")
from harness import report
D = [(2023,1,30),(2023,1,31),(2023,2,28),(2023,3,31),(2023,4,30),(2023,12,31),(2024,1,1),(2024,1,31),(2024,2,28),(2024,2,29),
     (2024,3,1),(2024,3,30),(2024,3,31),(2024,5,15),(2024,8,31),(2024,12,30),(2024,12,31),(2025,1,1),(2025,2,28),(2025,12,29),(2026,1,3),(2026,1,4),(2027,1,1),(2020,12,31),(2021,1,3)]
d = lambda t: "DATE(%d,%d,%d)" % t
cases = []
def add(prefix, f): cases.append((f"{prefix}{len(cases)}", f))
for t in D:
    for m in (-13,-12,-1,0,1,2,11,12,13,24,-24):
        add("edate", f"=EDATE({d(t)},{m})"); add("eomonth", f"=EOMONTH({d(t)},{m})")
    add("eomfrac", f"=EOMONTH({d(t)},1.7)"); add("edatefrac", f"=EDATE({d(t)},-1.7)")
for a, b in itertools.product(D, D):
    if a <= b:
        for u in ("Y","M","D","MD","YM","YD"):
            add("datedif", f'=DATEDIF({d(a)},{d(b)},"{u}")')
    add("days", f"=DAYS({d(b)},{d(a)})")
    add("netwd", f"=NETWORKDAYS({d(a)},{d(b)})")
for a, b in itertools.product(D[::3], D[::4]):
    add("netwdh", f"=NETWORKDAYS({d(a)},{d(b)},h.d)")
    for basis in (1,2,3,4):
        add("yf", f"=YEARFRAC({d(a)},{d(b)},{basis})")
for t in D:
    for n in (-10,-5,-1,0,1,2,5,10,30,-30):
        add("workday", f"=WORKDAY({d(t)},{n})")
    add("workdayh", f"=WORKDAY({d(t)},5,h.d)")
    for ty in (1,2,3,11,12,13,14,15,16,17):
        add("weekday", f"=WEEKDAY({d(t)},{ty})")
    for ty in (1,2,11,12,13,14,15,16,17,21):
        add("weeknum", f"=WEEKNUM({d(t)},{ty})")
    add("isoweek", f"=ISOWEEKNUM({d(t)})")
# year-end runs for week numbers
import datetime as dt
for y in (2019,2020,2021,2022,2026,2027,2028):
    for k in range(-4, 5):
        day = dt.date(y,12,31) + dt.timedelta(k)
        t = (day.year, day.month, day.day)
        add("isoye", f"=ISOWEEKNUM({d(t)})")
        for ty in (1,2,21):
            add("weeknumye", f"=WEEKNUM({d(t)},{ty})")
# DATE normalisation
for y, m, dd in [(2024,13,1),(2024,0,1),(2024,1,0),(2024,2,30),(2024,-1,15),(2024,1,-31),(2023,2,29),(99,1,1),(1999,25,31),(2024,1,400),(10000,1,1),(2024,12,32)]:
    add("date", f"=DATE({y},{m},{dd})")
for s in (45292.25, 45292.999999, 0.5, 60, 61, 1, 2958465):
    add("ymd", f"=YEAR({s})*10000+MONTH({s})*100+DAY({s})")
for h, mi, se in [(25,0,0),(0,90,0),(0,0,3661),(-1,0,0),(12,30,30),(23,59,59.5),(0,-1,0)]:
    add("time", f"=TIME({h},{mi},{se})")
for v in (0.99999,0.5,0.75,1/3,0.999994,0.0000058,45292.6,2/3):
    add("hms", f"=HOUR({v})*10000+MINUTE({v})*100+SECOND({v})")
print(len(cases))
report("dates", cases, "work", "h:\n  d: [45292, 45293, 45301, 45658]\n")
