import sys; sys.path.insert(0, ".")
from harness import report, report_text
# How a number reads as text (CONCAT, LEN, a text comparison): Excel keeps 15 significant digits
NUMS = ["1/3", "2/3", "0.1+0.2", "1E+20", "1E+15", "999999999999999", "1234567890123456", "123456789012345678", "1E-5", "0.000001",
        "1E-9", "1E-10", "1.5E-11", "-1/3", "100/3", "1E+100", "-0.0000123456789", "0.1", "45292.75", "PI()", "2^0.5", "1/7*1E+10", "123.456"]
num, txt = [], []
for i, n in enumerate(NUMS):
    num.append((f"len{i}", f"=LEN({n})"))
    num.append((f"lenc{i}", f"=LEN(CONCAT({n}))"))
    txt.append((f"cat{i}", f'=CONCAT("[",{n},"]")'))
num.append(("lencol", "=LEN(CONCAT(d.s))"))
num.append(("lencolt", '=LEN(TEXTJOIN(",",TRUE,d.s))'))
report("numtext", num, "work", 'd:\n  s: ["a", "", "b", "c"]\n')
report_text("numtextt", txt, "work")
