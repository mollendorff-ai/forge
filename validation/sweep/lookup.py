import sys, json; sys.path.insert(0, ".")
from harness import report, report_text
q = json.dumps
# ls: sorted ascending; ld: descending; lu: unsorted with duplicates; names: text
extra = ("lk:\n  ls: [1, 3, 5, 7, 9, 11]\n  ld: [11, 9, 7, 5, 3, 1]\n  lu: [4, 2, 9, 2, 7, 4]\n"
         "  nm: [\"apple\", \"Banana\", \"cherry\", \"apple pie\", \"date\", \"b*\"]\n  pr: [10, 20, 30, 40, 50, 60]\n")
num, txt = [], []
def add(L, p, f): L.append((f"{p}{len(L)}", f))
for v in (0, 1, 2, 3, 4, 6, 9, 10, 11, 12, 2.5):
    for t in (1, 0, -1):
        for col in ("ls", "ld", "lu"):
            add(num, "match", f"=MATCH({v},lk.{col},{t})")
    add(num, "match2", f"=MATCH({v},lk.ls)")
    for t in (2, 0.5, -0.5, -3):
        add(num, "matchtype", f"=MATCH({v},lk.ls,{t})")
    for col in ("ls", "ld"):
        add(num, "vl", f"=VLOOKUP({v},lk.{col},1,{'TRUE' if col == 'ls' else 'FALSE'})")
    for mm in (0, -1, 1):
        for sm in (1, -1, 2, -2):
            for col in ("ls", "ld", "lu"):
                add(num, "xmatch", f"=XMATCH({v},lk.{col},{mm},{sm})")
    for mm in (0, -1, 1):
        add(num, "xlookup", f"=XLOOKUP({v},lk.lu,lk.pr,-1,{mm})")
        add(num, "xlookupr", f"=XLOOKUP({v},lk.lu,lk.pr,-1,{mm},-1)")
    add(num, "idxm", f"=INDEX(lk.pr,MATCH({v},lk.ls,1))")
for name in ("apple", "APPLE", "b*", "ban*", "*e", "?pple", "apple*", "~*", "b~*", "zzz", "", "c?erry", "date"):
    add(num, "matcht", f"=MATCH({q(name)},lk.nm,0)")
    add(num, "xmatcht", f"=XMATCH({q(name)},lk.nm,0)")
    add(num, "xmatchw", f"=XMATCH({q(name)},lk.nm,2)")
    add(num, "xlookupw", f"=XLOOKUP({q(name)},lk.nm,lk.pr,-1,2)")
    add(num, "countifw", f"=COUNTIF(lk.nm,{q(name)})")
    add(num, "countifn", f"=COUNTIF(lk.nm,{q('<>' + name)})")
    add(num, "countife", f"=COUNTIF(lk.nm,{q('=' + name)})")
    add(num, "countifsw", f"=COUNTIFS(lk.nm,{q(name)},lk.pr,\">15\")")
    add(num, "avgifw", f"=AVERAGEIF(lk.nm,{q(name)},lk.pr)")
    add(num, "maxifsw", f"=MAXIFS(lk.pr,lk.nm,{q(name)})")
    add(txt, "vlw", f"=VLOOKUP({q(name)},lk.nm,1,FALSE)")
    add(num, "sumifw", f"=SUMIF(lk.nm,{q(name)},lk.pr)")
for i in (-1, 0, 1, 2, 6, 7, 2.9):
    add(num, "index", f"=INDEX(lk.pr,{i})")
    add(num, "choose", f"=CHOOSE({i},10,20,30)")
    add(txt, "indext", f"=INDEX(lk.nm,{i})")
for r, c, a, ref in ((1, 1, 1, "TRUE"), (5, 28, 1, "TRUE"), (3, 3, 2, "TRUE"), (3, 3, 3, "TRUE"), (3, 3, 4, "TRUE"), (3, 3, 1, "FALSE"),
                     (1, 16384, 4, "TRUE"), (1048576, 1, 1, "TRUE"), (0, 1, 1, "TRUE"), (2, 702, 1, "TRUE"), (2, 703, 4, "TRUE"), (2, 26, 5, "TRUE")):
    add(txt, "address", f"=ADDRESS({r},{c},{a},{ref})")
for s in ("1", "5", "2.5", "\"x\""):
    add(txt, "switch", f"=SWITCH({s},1,\"one\",5,\"five\",\"other\")")
    add(txt, "ifs", f"=IFS({s}=1,\"one\",{s}=5,\"five\")")
print(len(num), len(txt))
report("lookup", num, "work", extra)
report_text("lookupt", txt, "work", extra)
