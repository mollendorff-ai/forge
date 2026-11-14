import json, re, sys
from collections import Counter, defaultdict
c = json.load(open(sys.argv[1]))
pre = lambda n: re.sub(r"\d+$", "", n)
tot = Counter()
for k, v in c.items():
    for item in v:
        n = item if isinstance(item, str) else item[0]
        tot[(pre(n), k)] += 1
fns = sorted(set(p for p, _ in tot))
keys = ["all_agree", "forge_vs_both", "engines_differ_forge_lo", "engines_differ_forge_gn", "all_differ"]
print("%-11s" % "fn", *["%9s" % k[:9] for k in ["agree", "F≠both", "F=LO≠Gn", "F=Gn≠LO", "alldiff"]])
for p in fns:
    print("%-11s" % p, *["%9d" % tot[(p, k)] for k in keys])
