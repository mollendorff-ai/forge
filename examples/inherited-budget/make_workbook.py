"""Write budget.xlsx: the workbook an FP&A analyst inherits.

Written with openpyxl, never by forge export, so that importing it checks
Forge against a workbook it did not write. openpyxl stores formulas without
their values; LibreOffice then recalculates the file and stores them, as a
spreadsheet would have:

    python3 make_workbook.py budget_formulas.xlsx
    soffice --headless --convert-to xlsx --outdir out budget_formulas.xlsx
    mv out/budget_formulas.xlsx budget.xlsx

What makes it messy, on purpose: title rows above each table, a merged
header over the month table, the driver table's labels in sentences, a
blank row between the half-years, a total row under the months, a named
range, number formats, and one month's staff cost typed in where every
other month reads the assumption.
"""

import sys
from datetime import date

from openpyxl import Workbook
from openpyxl.styles import Alignment, Font
from openpyxl.workbook.defined_name import DefinedName

MONTHS = [date(2027, m, 1) for m in range(1, 13)]
BOLD = Font(bold=True)
MONEY = "#,##0"
PRICE = "$#,##0.00"
PERCENT = "0.0%"


def assumptions(wb):
    ws = wb.active
    ws.title = "Assumptions"
    ws["A1"] = "Kestrel Coffee Roasters: FY2027 budget"
    ws["A1"].font = Font(bold=True, size=14)
    ws.merge_cells("A1:D1")
    ws["A2"] = "Drivers. Change the blue cells only."
    ws.append([])
    ws.append(["Driver", "Value", "Unit", "Comment"])
    for cell in ws[4]:
        cell.font = BOLD
    rows = [
        ("Opening volume", 4200, "kg a month", "January, from the 2026 run rate", MONEY),
        ("Volume growth", 0.015, "a month", "", PERCENT),
        ("Price", 38.5, "$ per kg", "wholesale list price", PRICE),
        ("Green coffee", 14.2, "$ per kg", "", PRICE),
        ("Roasting and packing", 3.1, "$ per kg", "", PRICE),
        ("Rent", 6500, "$ a month", "", MONEY),
        ("Staff", 21000, "$ a month", "", MONEY),
        ("Price rise from July", 0.04, "", "agreed with the two big accounts", PERCENT),
        ("Tax rate", 0.25, "", "", PERCENT),
    ]
    for label, value, unit, comment, fmt in rows:
        ws.append([label, value, unit, comment])
        ws.cell(ws.max_row, 2).number_format = fmt
    ws.column_dimensions["A"].width = 24
    # The price is a named range, as someone once set it up
    wb.defined_names["list_price"] = DefinedName("list_price", attr_text="Assumptions!$B$7")


def monthly(wb):
    ws = wb.create_sheet("Monthly")
    ws["A1"] = "Monthly P&L"
    ws["A1"].font = Font(bold=True, size=14)
    # A merged header over the column groups
    ws["B2"] = "Sales"
    ws.merge_cells("B2:D2")
    ws["E2"] = "Costs"
    ws.merge_cells("E2:H2")
    for ref in ("B2", "E2"):
        ws[ref].alignment = Alignment(horizontal="center")
        ws[ref].font = BOLD
    header = ["Month", "Volume (kg)", "Price", "Revenue", "COGS", "Gross margin", "Rent", "Staff", "EBITDA"]
    for col, text in enumerate(header, start=1):
        ws.cell(3, col, text).font = BOLD
    # Jan-Jun on rows 4-9, a blank row 10, Jul-Dec on rows 11-16
    rows = list(range(4, 10)) + list(range(11, 17))
    for i, (row, month) in enumerate(zip(rows, MONTHS)):
        ws.cell(row, 1, month).number_format = "mmm yy"
        if i == 0:
            volume = "=Assumptions!$B$5"
        else:
            above = rows[i - 1]
            volume = f"=B{above}*(1+Assumptions!$B$6)"
        ws.cell(row, 2, volume).number_format = MONEY
        price = "=list_price" if i < 6 else "=list_price*(1+Assumptions!$B$12)"
        ws.cell(row, 3, price).number_format = PRICE
        ws.cell(row, 4, f"=B{row}*C{row}")
        ws.cell(row, 5, f"=B{row}*(Assumptions!$B$8+Assumptions!$B$9)")
        ws.cell(row, 6, f"=D{row}-E{row}")
        ws.cell(row, 7, "=Assumptions!$B$10")
        # October's staff cost was typed in, once, and never put back
        ws.cell(row, 8, 21000 if month.month == 10 else "=Assumptions!$B$11")
        ws.cell(row, 9, f"=F{row}-G{row}-H{row}")
        for col in range(4, 10):
            ws.cell(row, col).number_format = MONEY
    ws.cell(17, 1, "Total").font = BOLD
    for col in "BDEFGHI":
        ws[f"{col}17"] = f"=SUM({col}4:{col}16)"
        ws[f"{col}17"].number_format = MONEY
        ws[f"{col}17"].font = BOLD
    ws["C17"] = "=D17/B17"
    ws["C17"].number_format = PRICE


def summary(wb):
    ws = wb.create_sheet("Summary")
    ws["A1"] = "FY2027 summary"
    ws["A1"].font = Font(bold=True, size=14)
    rows = [
        ("Revenue", "=Monthly!D17", MONEY),
        ("Gross margin", "=Monthly!F17", MONEY),
        ("Gross margin %", "=B4/B3", PERCENT),
        ("EBITDA", "=Monthly!I17", MONEY),
        ("Tax", "=MAX(0,B6*Assumptions!B13)", MONEY),
        ("Net income", "=B6-B7", MONEY),
        ("H2 revenue over H1", "=SUM(Monthly!D11:D16)/SUM(Monthly!D4:D9)-1", PERCENT),
    ]
    for i, (label, formula, fmt) in enumerate(rows, start=3):
        ws.cell(i, 1, label)
        ws.cell(i, 2, formula).number_format = fmt


def main(path):
    wb = Workbook()
    assumptions(wb)
    monthly(wb)
    summary(wb)
    wb.save(path)


if __name__ == "__main__":
    main(sys.argv[1])
