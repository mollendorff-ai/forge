"""Check that budget.yaml, exported, recalculates outside Forge to Forge's numbers.

    python3 -I check_export.py path/to/forge

forge export writes each formula with Forge's result stored beside it.
LibreOffice is run with a profile of its own that recalculates every formula
when the file is loaded (by default it shows the stored results, which are
Forge's), and every number it calculates is compared with Forge's.
Needs LibreOffice and openpyxl.
"""

import subprocess
import sys
import tempfile
from pathlib import Path

from openpyxl import load_workbook

HERE = Path(__file__).resolve().parent

# The same setting validation/gnumeric uses: recalculate on load, always
RECALC_ON_LOAD = """<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>
"""


def numbers(path):
    """Every number in the workbook, by (sheet, cell)."""
    book = load_workbook(path, data_only=True)
    return {
        (sheet.title, cell.coordinate): cell.value
        for sheet in book
        for row in sheet.iter_rows()
        for cell in row
        if isinstance(cell.value, (int, float)) and not isinstance(cell.value, bool)
    }


def formulas(path):
    book = load_workbook(path)
    return sum(
        1
        for sheet in book
        for row in sheet.iter_rows()
        for cell in row
        if isinstance(cell.value, str) and cell.value.startswith("=")
    )


def main(forge):
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        exported = tmp / "budget.xlsx"
        subprocess.run([forge, "export", str(HERE / "budget.yaml"), str(exported)], check=True)
        profile = tmp / "profile" / "user"
        profile.mkdir(parents=True)
        (profile / "registrymodifications.xcu").write_text(RECALC_ON_LOAD)
        out = tmp / "recalculated"
        subprocess.run(
            [
                "soffice",
                f"-env:UserInstallation=file://{profile.parent}",
                "--headless",
                "--convert-to",
                "xlsx",
                "--outdir",
                str(out),
                str(exported),
            ],
            check=True,
            capture_output=True,
        )
        forge_numbers = numbers(exported)
        calc_numbers = numbers(out / "budget.xlsx")
        differ = [
            (place, value, calc_numbers.get(place))
            for place, value in forge_numbers.items()
            if calc_numbers.get(place) is None
            or abs(value - calc_numbers[place]) > 1e-9 * max(abs(value), 1.0)
        ]
        for place, value, other in differ:
            print(f"{place[0]}!{place[1]}: Forge {value}, LibreOffice {other}")
        print(
            f"{len(forge_numbers)} numbers, {formulas(exported)} formulas recalculated "
            f"by LibreOffice, {len(differ)} different"
        )
        return 1 if differ else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "forge"))
