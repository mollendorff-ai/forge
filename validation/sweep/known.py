"""Known differences for generate.py (ADR-062, decision 4).

Each rule picks out one class of input where Forge differs from the judges on
purpose, or where the right answer waits for Excel. It names the functions it
covers, a test on the case, the reason and where the question is tracked. A
rule is never a whole function. The weekly report lists the rules that matched
nothing, so a fixed difference does not stay excused.

A test gets the case: fn (the function), args (the argument texts as drawn),
forge, lo, gn (the three values; -999999 for an error, text for text).
"""
import re

ERR = -999999.0


def num(a):
    """An argument as a number, or None (a date, a text, a column)."""
    try:
        return float(a)
    except (TypeError, ValueError):
        return None


def ymd(a):
    m = re.fullmatch(r"DATE\((-?\d+),(-?\d+),(-?[\d.]+)\)", a or "")
    return tuple(float(x) for x in m.groups()) if m else None


def whole(x):
    return x is not None and float(x).is_integer()


def before_march_1900(a):
    d = ymd(a)
    return d is not None and (d[0] < 1900 or (d[0] == 1900 and d[1] < 3))


class Rule:
    def __init__(self, rid, functions, test, reason, tracked):
        self.id, self.functions, self.test = rid, set(functions), test
        self.reason, self.tracked = reason, tracked

    def covers(self, case):
        return case["fn"] in self.functions and bool(self.test(case))


RULES = [
    Rule("date-year-below-1900", ["DATE"],
         lambda c: num(c["args"][0]) is not None and 0 <= num(c["args"][0]) <= 1899,
         "DATE with a year from 0 to 1899 adds 1900: Excel gives 3799-12-31 for DATE(1899,12,31), as Forge does and as Microsoft's DATE page says; both engines use the year as it is",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("before-march-1900", ["YEARFRAC", "DAYS", "DATEDIF", "EDATE", "EOMONTH", "WEEKDAY", "WEEKNUM", "NETWORKDAYS", "WORKDAY",
                                "ACCRINT", "PRICEDISC", "YIELDDISC", "YEAR", "MONTH", "DAY", "ISOWEEKNUM"],
         lambda c: any(before_march_1900(a) for a in c["args"]),
         "A date before 1900-03-01: Excel's day numbers count the 29 February 1900 that never was, and the engines and Forge each place those dates differently (ADR-054)",
         "ADR-054"),
    Rule("value-true", ["VALUE"],
         lambda c: c["args"][0].strip('"').upper() in ("TRUE", "FALSE"),
         "VALUE(\"TRUE\"): Excel gives #VALUE!, as Forge does; both engines give 1",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("discount-30-360-month-end", ["PRICEDISC", "YIELDDISC"],
         lambda c: (len(c["args"]) < 5 or (num(c["args"][4]) is not None and int(num(c["args"][4])) in (0, 4))) and any(
             (d := ymd(a)) and (d[2] >= 30 or (d[1] == 2 and d[2] >= 28)) for a in c["args"][:2]),
         "PRICEDISC and YIELDDISC on 30/360 with a month-end date: Excel counts the days as Forge does (the NASD rule; PRICEDISC from 28 February 95.8333, YIELDDISC 0.03711), and both engines do not",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("discount-actual-actual-over-a-year", ["PRICEDISC", "YIELDDISC"],
         lambda c: len(c["args"]) > 4 and num(c["args"][4]) is not None and int(num(c["args"][4])) == 1
             and (a := ymd(c["args"][0])) and (b := ymd(c["args"][1])) and abs(b[0] - a[0]) >= 1,
         "PRICEDISC and YIELDDISC on actual/actual over more than a year: Excel divides by the average length of the years spanned, as Forge does (-4.33217); both engines divide by 365 or 366",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("accrint-coupon-periods", ["ACCRINT"],
         lambda c: len(c["args"]) < 7 or (num(c["args"][6]) is not None and int(num(c["args"][6])) in (0, 1, 4)),
         "ACCRINT on a 30/360 or actual/actual basis: Excel sums each coupon period on its own, as Forge does (83.3333 over month ends); both engines count the whole span at once",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("time-fractions", ["TIME"],
         lambda c: any(num(a) is not None and not whole(num(a)) for a in c["args"]),
         "TIME with a fraction of an hour, minute or second: Excel cuts each to a whole number, as Forge does (TIME(23,59,59.5) is 0.999988); both engines keep the fraction",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("syd-past-the-life", ["SYD"],
         lambda c: num(c["args"][2]) is not None and num(c["args"][3]) is not None
             and (num(c["args"][3]) > num(c["args"][2]) or num(c["args"][2]) < 1 or num(c["args"][3]) < 1),
         "SYD with a period after the life, or a life or period below 1: Excel gives #NUM!, as Forge does; both engines give a number",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("negative-time", ["HOUR", "MINUTE", "SECOND"],
         lambda c: num(c["args"][0]) is not None and num(c["args"][0]) < 0,
         "HOUR, MINUTE and SECOND of a negative number: Excel gives #NUM!, as Forge does; both engines count back from midnight",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("nper-near-zero-rate", ["NPER"],
         lambda c: num(c["args"][0]) is not None and 0 < abs(num(c["args"][0])) < 1e-6,
         "NPER at a rate near 0: Forge's answer is the exact one to 15 digits (checked with 50-digit decimal arithmetic, 2026-11-14); both engines lose digits",
         "the engines"),
    Rule("rate-without-a-near-root", ["RATE"],
         lambda c: any(v == ERR or (isinstance(v, float) and v < -0.3) for v in (c["forge"], c["lo"], c["gn"])),
         "RATE where an answer is a loss of 30% or more a period, or Forge or an engine finds none: the equation has no root near the guess of 10%, and each solver stops somewhere else",
         "T1-EXCEL-CHECKS"),
    Rule("ifs-pattern-on-numbers", ["MAXIFS", "MINIFS"],
         lambda c: any(re.search(r'"[^"]*[*?][^"]*"', a) for a in c["args"][2:]),
         "MAXIFS and MINIFS with a pattern (* or ?): Excel matches text only and gives 0 over numbers, as Forge does; LibreOffice matches numbers with it, Gnumeric gives an error",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("pattern-on-dates", ["COUNTIF", "COUNTIFS", "SUMIF", "SUMIFS", "AVERAGEIF", "AVERAGEIFS"],
         lambda c: any(re.fullmatch(r"[gh]\.d", a) for a in c["args"]) and any(re.search(r'"[^"]*[*?][^"]*"', a) for a in c["args"]),
         "A pattern (* or ?) over a column of dates: a date is a number in the workbook, and Excel matches a pattern with text only (MAXIFS with \"*\" over numbers is 0 in Excel, checked 2026-11-14), as Forge does; both engines match dates with it. COUNTIF over dates itself is not yet checked in Excel",
         "T1-EXCEL-CHECKS"),
    Rule("cumulative-fractions-and-extreme-rates", ["CUMIPMT", "CUMPRINC"],
         lambda c: any(num(a) is not None and not whole(num(a)) for a in c["args"][1:2] + c["args"][3:5])
             or (num(c["args"][0]) is not None and (num(c["args"][0]) >= 0.5 or num(c["args"][0]) < 1e-6)),
         "CUMIPMT and CUMPRINC with a fractional number of periods, start or end: Excel cuts them, as Forge does (CUMIPMT(0.01,10.5,1000,1,5,0) is -40.3457, checked in Excel 2026-11-14), LibreOffice does not. A rate of 50% or more or below 1E-6: the engines lose digits there, and Forge's CUMPRINC, a closed form, is the exact one (checked with fractions, 2026-11-14)",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml); the engines"),
    Rule("weeknum-year-end-types-11-17", ["WEEKNUM"],
         lambda c: len(c["args"]) > 1 and c["args"][1] in [str(t) for t in range(11, 18)]
             and (d := ymd(c["args"][0])) is not None and d[1] == 12 and d[2] >= 25,
         "WEEKNUM with types 11 to 17 in the last week of December: Excel gives 53 (54 for type 14), as Forge and the calendar do; LibreOffice gives 1",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("text-date-before-march-1900", ["TEXT"],
         lambda c: num(c["args"][0]) is not None and num(c["args"][0]) < 61 and re.search(r"[ymd]", c["args"][1]),
         "TEXT of a day number before 61 (1900-03-01) in a date format: Forge #VALUE! below 0; the engines and Forge each place the days before March 1900 differently (ADR-054)",
         "ADR-054"),
    Rule("double-precision-limit", ["FORECAST", "SLOPE", "INTERCEPT", "RSQ", "CORREL", "COVARIANCE.P", "COVARIANCE.S",
                                    "CUMIPMT", "CUMPRINC", "IRR", "MIRR", "RATE", "NPER", "PMT", "PV", "FV", "IPMT", "PPMT"],
         lambda c: c.get("verdict") == "all-different" and all(isinstance(v, float) and v != ERR for v in (c["forge"], c["lo"], c["gn"]))
             and max(c["forge"], c["lo"], c["gn"]) - min(c["forge"], c["lo"], c["gn"]) <= 1e-5 * max(1.0, abs(c["lo"])),
         "All three differ, within 1E-5 of each other: the answer is a difference of large numbers, so 64-bit arithmetic leaves only a few digits (FORECAST on columns from 1 to 1E+15: about six), and Gnumeric's 80-bit arithmetic keeps more. No engine is a reference there",
         "the engines"),
    Rule("datedif-yd-from-february-29", ["DATEDIF"],
         lambda c: (d := ymd(c["args"][0])) is not None and d[1] == 2 and d[2] == 29 and c["args"][2] == '"YD"',
         "DATEDIF \"YD\" from 29 February: Excel counts from 28 February in a year without one (304), as Forge does; LibreOffice from 1 March (303)",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("vdb-fractional-start", ["VDB"],
         lambda c: num(c["args"][3]) is not None and not whole(num(c["args"][3])),
         "VDB with a fractional start period: Excel gives 22.22 for VDB(100,10,3,0.5,1), as Forge does; both engines give 33.33",
         "Excel, 2026-11-14 (validation/gnumeric/tests/edge/edge_excel_checks.yaml)"),
    Rule("db-fractions", ["DB"],
         lambda c: any(num(a) is not None and not whole(num(a)) for a in c["args"][2:4]),
         "DB with a fractional life or period: Forge, LibreOffice and Gnumeric each read the fraction differently, and Microsoft's page does not say. (A fractional month is cut, in Excel as in Forge and LibreOffice: checked 2026-11-14)",
         "T1-EXCEL-CHECKS"),
]


def explain(case):
    """The first rule that covers the case, or None."""
    for rule in RULES:
        if rule.covers(case):
            return rule
    return None
