import sys, json; sys.path.insert(0, ".")
from harness import report_text
q = lambda s: json.dumps(s, ensure_ascii=False)  # no quotes inside the texts, so no "" escaping
S = ["hello", "", " a  b ", "Hello World", "αβγ", "straße", "x", "ABCdef123", "a*b?c", "1,234.5"]
cases = []
def add(p, f): cases.append((f"{p}{len(cases)}", f))
for s in S:
    for n in (-1, 0, 1, 2, 3, 20, 2.7):
        add("left", f"=LEFT({q(s)},{n})"); add("right", f"=RIGHT({q(s)},{n})")
    add("left1", f"=LEFT({q(s)})"); add("right1", f"=RIGHT({q(s)})")
    for st in (-1, 0, 1, 2, 5, 30, 1.9):
        for n in (0, 1, 3, 2.5, -1):
            add("mid", f"=MID({q(s)},{st},{n})")
    add("len", f"=LEN({q(s)})"); add("up", f"=UPPER({q(s)})"); add("lo", f"=LOWER({q(s)})"); add("trim", f"=TRIM({q(s)})")
    for n in (0, 1, 3, 2.9, -1):
        add("rept", f"=REPT({q(s)},{n})")
    for find in ("l", "L", "", "o W", "?", "*", "b?c", "~*", "z"):
        for st in ("", ",1", ",2", ",0", ",4", ",12", ",-1"):
            add("find", f"=FIND({q(find)},{q(s)}{st})"); add("search", f"=SEARCH({q(find)},{q(s)}{st})")
    for st, n, new in ((1,1,"Z"),(0,1,"Z"),(2,0,"ZZ"),(3,10,""),(20,2,"Q"),(1,-1,"Z"),(2,2.5,"Z")):
        add("repl", f"=REPLACE({q(s)},{st},{n},{q(new)})")
    for old, new in (("l","L"),("","x"),("a","aa"),(" ","")):
        add("subst", f"=SUBSTITUTE({q(s)},{q(old)},{q(new)})")
        for k in (1, 2, 0, -1, 1.5):
            add("substn", f"=SUBSTITUTE({q(s)},{q(old)},{q(new)},{k})")
for a in (("a","b"),("", "x", ""),("1","2","3")):
    add("concat", f"=CONCAT({','.join(map(q,a))})"); add("concatenate", f"=CONCATENATE({','.join(map(q,a))})")
    for d in (",", "", " - "):
        for ign in ("TRUE", "FALSE"):
            add("tj", f"=TEXTJOIN({q(d)},{ign},{','.join(map(q,a))})")
add("concatn", "=CONCAT(1.5,TRUE,2)"); add("concatn", "=CONCATENATE(0.1+0.2)"); add("concatn", "=CONCAT(1E+20)"); add("concatn", "=CONCAT(1/3)")
add("tjcol", '=TEXTJOIN("-",TRUE,d.s)'); add("tjcol", '=TEXTJOIN("-",FALSE,d.s)'); add("concol", "=CONCAT(d.s)")
NUMS = [0, 1234.5, -1234.5, 0.5, 0.125, 2.5, 3.5, -0.5, 1e6, 0.000123, 45292, 45292.75, 1.005, 12345678.9, -0.004]
FMTS = ["0", "0.00", "#,##0", "#,##0.00", "0%", "0.0%", "$#,##0.00", "0.00E+00", "#", "#.##", "00000", "yyyy-mm-dd", "dd/mm/yyyy",
        "mmm d, yyyy", "dddd", "mmmm", "hh:mm", "h:mm AM/PM", "hh:mm:ss", "0.0,", "#,##0,,", "#,##0;(#,##0)", "0.00;-0.00;zero", "General", "@", "?/?", "# ?/?"]
for n in NUMS:
    for fm in FMTS:
        add("text", f"=TEXT({n},{q(fm)})")
for s in ("αβγ", "straße", "ÀÉÎ"):
    for find in ("β", "Β", "γ", "?", "*e", "é"):
        for st in ("", ",1", ",2", ",3", ",4"):
            add("findu", f"=FIND({q(find)},{q(s)}{st})"); add("searchu", f"=SEARCH({q(find)},{q(s)}{st})")
for v in ("inf", "nan", ".5", "5.", "1e", "-$5", "$-5", "1,2,3", "1,234,567.25", "  -1.5e-3 ", "1.5E+2", "+7", "--5", "5-", "1 000", "12:30:15", "25:00", "1:60", "2024-02-30", "50 %", "($5)", "1e400",
          "123", "1,234.5", " 12 ", "1e3", "50%", "$1,000", "abc", "", "-5", "(5)", "2024-01-15", "12:30", "1.2.3", "0x10", "TRUE"):
    add("value", f"=VALUE({q(v)})")
print(len(cases))
report_text("text", cases, "work", "d:\n  s: [\"a\", \"\", \"b\", \"c\"]\n")
