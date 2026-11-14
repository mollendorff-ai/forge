import json,re,sys
c=json.load(open(sys.argv[1])); n=int(sys.argv[2]) if len(sys.argv)>2 else 5
pre=lambda x: re.sub(r"\d+$","",x)
for k in ["forge_vs_both","all_differ","engines_differ_forge_gn","engines_differ_forge_lo"]:
    g={}
    for it in c[k]: g.setdefault(pre(it[0]),[]).append(it)
    for p,items in g.items():
        print(f"== {k} {p} ({len(items)})  [formula, forge, other(s)]")
        for it in items[:n]: print("   ",it[1:])
