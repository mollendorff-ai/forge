#!/bin/bash
# Forge Prediction Methods - Validation Environment Setup
# Installs FOSS tools for roundtrip validation of prediction methods

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"

echo "=== Forge Validation Environment Setup ==="
echo ""

# Check for Homebrew (macOS)
if command -v brew &> /dev/null; then
    echo "[1/4] Installing R (statistical computing)..."
    brew install r 2>/dev/null || echo "R already installed"

    echo "[2/4] Installing QuantLib (derivatives pricing)..."
    brew install quantlib 2>/dev/null || echo "QuantLib already installed"
else
    echo "WARNING: Homebrew not found. Install R and QuantLib manually."
    echo "  - R: https://cran.r-project.org/"
    echo "  - QuantLib: https://www.quantlib.org/"
fi

# Python venv for scipy/numpy/pgmpy
echo "[3/4] Setting up Python validation environment..."
if [ ! -d "$VENV_DIR" ]; then
    python3 -m venv "$VENV_DIR"
fi
"$VENV_DIR/bin/pip" install --quiet --upgrade pip
"$VENV_DIR/bin/pip" install --quiet scipy numpy pgmpy

# R packages
echo "[4/4] Installing R packages..."
R --quiet -e 'install.packages(c("boot", "sensitivity"), repos="https://cloud.r-project.org", quiet=TRUE)' 2>/dev/null || true

echo ""
echo "=== Validation Environment Ready ==="
echo ""
echo "Installed tools:"
echo "  - R $(R --version 2>/dev/null | head -1 | cut -d' ' -f3)"
echo "  - Python venv: $VENV_DIR"
echo "  - scipy $($VENV_DIR/bin/python -c 'import scipy; print(scipy.__version__)')"
echo "  - numpy $($VENV_DIR/bin/python -c 'import numpy; print(numpy.__version__)')"
echo "  - pgmpy $($VENV_DIR/bin/python -c 'import pgmpy; print(pgmpy.__version__)')"
echo ""
echo "Roundtrip validation tools:"
echo "  - Scenario Analysis: R weighted.mean()"
echo "  - Decision Trees:    scipy/numpy"
echo "  - Real Options:      QuantLib"
echo "  - Tornado Diagrams:  R sensitivity package"
echo "  - Bootstrap:         R boot package"
echo "  - Bayesian Networks: pgmpy"
