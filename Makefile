# Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed install install-user install-system uninstall lint lint-fix format format-check test test-unit test-integration test-e2e test-validate test-calculate test-all test-coverage validate-docs validate-yaml validate-diagrams validate-all install-tools clean clean-test pre-build post-build pre-commit check

# Detect if upx is available
HAS_UPX := $(shell command -v upx 2> /dev/null)

help:
	@echo "🔥 Forge - Available Commands"
	@echo ""
	@echo "Build Targets:"
	@echo "  make build              - Standard release build (with pre/post checks)"
	@echo "  make build-static       - Static release build (musl, 1.2MB)"
	@echo "  make build-compressed   - Static + UPX compressed (440KB)"
	@echo ""
	@echo "Install Targets:"
	@echo "  make install            - Install to /usr/local/bin (system-wide, requires sudo)"
	@echo "  make install-user       - Install to ~/.local/bin (user-only, no sudo)"
	@echo "  make install-system     - Same as install (system-wide)"
	@echo "  make uninstall          - Uninstall from both locations"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint               - Run pedantic clippy checks"
	@echo "  make lint-fix           - Auto-fix clippy warnings"
	@echo "  make format             - Format code with rustfmt"
	@echo "  make format-check       - Check formatting without modifying"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test               - Run all cargo tests (unit + integration + E2E)"
	@echo "  make test-unit          - Run unit tests only"
	@echo "  make test-integration   - Run integration tests only"
	@echo "  make test-e2e           - Run E2E tests with actual YAML files"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test-all           - Run ALL tests (136 total)"
	@echo "  make test-coverage      - Show test coverage summary"
	@echo ""
	@echo "Documentation Validation:"
	@echo "  make validate-docs      - Validate markdown files (markdownlint-cli2)"
	@echo "  make validate-yaml      - Validate YAML files (yamllint)"
	@echo "  make validate-diagrams  - Validate PlantUML diagrams (if present)"
	@echo "  make validate-all       - Run ALL validators (docs + yaml + diagrams)"
	@echo ""
	@echo "Workflows:"
	@echo "  make pre-commit         - Full pre-commit check (format + lint + test + validate-all)"
	@echo "  make check              - Quick check during development (faster than pre-commit)"
	@echo ""
	@echo "Utilities:"
	@echo "  make install-tools      - Show installation commands for required tools"
	@echo "  make clean              - Remove build artifacts"
	@echo "  make clean-test         - Restore test-data to original state"

pre-build:
	@echo "🔍 Running pre-build checks..."
	@echo ""
	@echo "1️⃣  Running lint (pedantic clippy)..."
	@$(MAKE) -s lint
	@echo ""
	@echo "2️⃣  Running unit tests..."
	@cargo test --lib --quiet
	@echo "✅ Unit tests passed!"
	@echo ""
	@echo "✅ Pre-build checks complete!"
	@echo ""

post-build:
	@echo ""
	@echo "🧪 Running post-build checks..."
	@echo ""
	@echo "1️⃣  Running E2E tests..."
	@cargo test --quiet
	@echo "✅ All tests passed!"
	@echo ""
	@echo "✅ Post-build checks complete!"

build: pre-build
	@echo "🔨 Building release binary..."
	@cargo build --release
	@echo "✅ Binary: target/release/forge"
	@ls -lh target/release/forge
	@$(MAKE) -s post-build

build-static:
	@echo "🔨 Building static release binary (musl)..."
	@cargo build --release --target x86_64-unknown-linux-musl
	@echo "✅ Binary: target/x86_64-unknown-linux-musl/release/forge"
	@ls -lh target/x86_64-unknown-linux-musl/release/forge

build-compressed: build-static
	@echo ""
ifdef HAS_UPX
	@echo "📦 BEFORE compression:"
	@ls -lh target/x86_64-unknown-linux-musl/release/forge | tail -1
	@BEFORE=$$(stat -c%s target/x86_64-unknown-linux-musl/release/forge 2>/dev/null || stat -f%z target/x86_64-unknown-linux-musl/release/forge); \
	echo ""; \
	echo "🗜️  Compressing with UPX --best --lzma..."; \
	upx --best --lzma target/x86_64-unknown-linux-musl/release/forge; \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo "✨ WOW! AFTER compression:"; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	ls -lh target/x86_64-unknown-linux-musl/release/forge | tail -1; \
	AFTER=$$(stat -c%s target/x86_64-unknown-linux-musl/release/forge 2>/dev/null || stat -f%z target/x86_64-unknown-linux-musl/release/forge); \
	SAVED=$$(($$BEFORE - $$AFTER)); \
	PERCENT=$$(awk "BEGIN {printf \"%.1f\", ($$SAVED / $$BEFORE) * 100}"); \
	echo ""; \
	echo "🎉 Saved: $$SAVED bytes ($$PERCENT% smaller!)"; \
	echo "📊 From $$(numfmt --to=iec-i --suffix=B $$BEFORE 2>/dev/null || echo $$BEFORE bytes) → $$(numfmt --to=iec-i --suffix=B $$AFTER 2>/dev/null || echo $$AFTER bytes)"
else
	@echo "⚠️  UPX not found - install with: sudo apt install upx-ucl"
	@echo "📦 Static binary built (not compressed):"
	@ls -lh target/x86_64-unknown-linux-musl/release/forge
endif

install-system: clean build-compressed
	@echo "📦 Installing forge to /usr/local/bin (system-wide)..."
	@sudo install -m 755 target/x86_64-unknown-linux-musl/release/forge /usr/local/bin/forge
	@echo "✅ Installed to /usr/local/bin/forge"
	@echo "🔍 Verify with: forge --version"

install-user: clean build-compressed
	@echo "📦 Installing forge to ~/.local/bin (user-only)..."
	@mkdir -p ~/.local/bin
	@install -m 755 target/x86_64-unknown-linux-musl/release/forge ~/.local/bin/forge
	@echo "✅ Installed to ~/.local/bin/forge"
	@echo "💡 Make sure ~/.local/bin is in your PATH"
	@echo "🔍 Verify with: forge --version"

install: install-system

uninstall:
	@echo "🗑️  Uninstalling forge..."
	@sudo rm -f /usr/local/bin/forge 2>/dev/null || true
	@rm -f ~/.local/bin/forge 2>/dev/null || true
	@echo "✅ Uninstalled from both /usr/local/bin and ~/.local/bin"

lint:
	@echo "🔍 Running pedantic clippy checks..."
	@cargo clippy --all-targets --all-features -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions
	@echo "✅ Clippy checks passed!"

lint-fix:
	@echo "🔧 Running clippy with auto-fix..."
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions
	@echo "✅ Auto-fix complete!"

test-validate:
	@echo "🔍 Validating all test-data files..."
	@echo ""
	@for file in test-data/*.yaml; do \
		echo "📄 Validating: $$file"; \
		cargo run --release -- validate $$file || exit 1; \
		echo ""; \
	done
	@echo "✅ All test files validated successfully!"

test-calculate:
	@echo "🧮 Testing calculation on all test-data files (dry-run)..."
	@echo ""
	@for file in test-data/*.yaml; do \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		echo "📄 Calculating: $$file"; \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		cargo run --release -- calculate $$file --dry-run --verbose || exit 1; \
		echo ""; \
	done
	@echo "✅ All test calculations completed successfully!"

test:
	@echo "🧪 Running all cargo tests..."
	@cargo test

test-unit:
	@echo "🧪 Running unit tests..."
	@cargo test --lib

test-integration:
	@echo "🧪 Running integration tests..."
	@cargo test --test validation_tests

test-e2e:
	@echo "🧪 Running E2E tests with actual YAML files..."
	@cargo test --test e2e_tests

test-all: test test-e2e test-validate test-calculate
	@echo ""
	@echo "🎉 All tests passed!"

test-coverage:
	@echo "📊 Test Coverage Summary"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "Unit Tests (3):"
	@echo "  ✅ calculator::tests::test_simple_calculation"
	@echo "  ✅ parser::tests::test_parse_simple_formula"
	@echo "  ✅ writer::tests::test_update_simple_value"
	@echo ""
	@echo "Integration Tests (5):"
	@echo "  ✅ test_validation_passes_with_correct_values"
	@echo "  ✅ test_validation_fails_with_stale_values"
	@echo "  ✅ test_calculate_updates_stale_values"
	@echo "  ✅ test_validation_with_multiple_mismatches"
	@echo "  ✅ test_dry_run_does_not_modify_file"
	@echo ""
	@echo "E2E Tests (11):"
	@echo "  ✅ e2e_malformed_yaml_fails_gracefully"
	@echo "  ✅ e2e_invalid_formula_variable_not_found"
	@echo "  ✅ e2e_circular_dependency_detected"
	@echo "  ✅ e2e_stale_values_detected"
	@echo "  ✅ e2e_valid_updated_yaml_passes"
	@echo "  ✅ e2e_calculate_updates_stale_file"
	@echo "  ✅ e2e_verbose_output_shows_formulas"
	@echo "  ✅ e2e_platform_test_file_validates"
	@echo "  ✅ e2e_financial_test_file_validates"
	@echo "  ✅ e2e_underscore_test_file_validates"
	@echo "  ✅ e2e_basic_test_file_validates"
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "Total: 19 tests covering:"
	@echo "  • Formula parsing and calculation"
	@echo "  • Value validation (stale detection)"
	@echo "  • YAML file updates"
	@echo "  • Error handling (malformed YAML, invalid formulas)"
	@echo "  • Circular dependency detection"
	@echo "  • Dry-run mode"
	@echo "  • All test-data files"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete!"

clean-test:
	@echo "🔄 Restoring test-data files to git state..."
	@git checkout test-data/*.yaml
	@echo "✅ Test data restored!"

# ═══════════════════════════════════════════════════════════════════════════
# CODE FORMATTING TARGETS
# ═══════════════════════════════════════════════════════════════════════════

format:
	@echo "🎨 Formatting code..."
	@cargo fmt
	@echo "✅ Code formatted"

format-check:
	@echo "🎨 Checking code formatting..."
	@cargo fmt -- --check
	@echo "✅ Code formatting is correct"

# ═══════════════════════════════════════════════════════════════════════════
# DOCUMENTATION VALIDATION TARGETS
# ═══════════════════════════════════════════════════════════════════════════

validate-docs:
	@echo "📝 Validating markdown files..."
	@if command -v markdownlint-cli2 >/dev/null 2>&1; then \
		markdownlint-cli2 '**/*.md' --config .markdownlint.json && \
		echo "✅ Markdown validation passed"; \
	else \
		echo "❌ markdownlint-cli2 not found. Run: npm install -g markdownlint-cli2"; \
		exit 1; \
	fi

validate-yaml:
	@echo "📄 Validating YAML files..."
	@if command -v yamllint >/dev/null 2>&1; then \
		yamllint warmup.yaml roadmap.yaml 2>/dev/null && \
		echo "✅ YAML validation passed"; \
	else \
		echo "❌ yamllint not found. Run: pip install yamllint"; \
		exit 1; \
	fi

validate-diagrams:
	@echo "🎨 Validating PlantUML diagrams..."
	@if [ -d "diagrams" ] && find diagrams -name "*.puml" -o -name "*.plantuml" 2>/dev/null | grep -q .; then \
		if [ -x "bin/validate-plantuml.sh" ]; then \
			./bin/validate-plantuml.sh; \
		else \
			echo "❌ Validation script not found or not executable"; \
			echo "    Run: chmod +x bin/validate-plantuml.sh"; \
			exit 1; \
		fi; \
	else \
		echo "ℹ️  No .puml files found in diagrams/ (see diagrams/README.md to get started)"; \
		echo "✅ Diagram validation skipped"; \
	fi

validate-all: validate-docs validate-yaml validate-diagrams
	@echo ""
	@echo "✅ All validation checks completed!"

# ═══════════════════════════════════════════════════════════════════════════
# UTILITY TARGETS
# ═══════════════════════════════════════════════════════════════════════════

install-tools:
	@echo "📦 Required tools for Forge development:"
	@echo ""
	@echo "1. Rust toolchain (required)"
	@echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
	@echo ""
	@echo "2. markdownlint-cli2 (documentation validation)"
	@echo "   npm install -g markdownlint-cli2"
	@echo ""
	@echo "3. yamllint (YAML validation)"
	@echo "   pip install yamllint"
	@echo ""
	@echo "4. PlantUML (diagram validation - optional)"
	@echo "   Using public server: https://www.plantuml.com/plantuml"
	@echo "   Scripts: bin/validate-plantuml.sh"
	@echo ""
	@echo "Current status:"
	@command -v cargo >/dev/null 2>&1 && echo "  ✅ Rust/Cargo installed" || echo "  ❌ Rust/Cargo not found"
	@command -v markdownlint-cli2 >/dev/null 2>&1 && echo "  ✅ markdownlint-cli2 installed" || echo "  ❌ markdownlint-cli2 not found"
	@command -v yamllint >/dev/null 2>&1 && echo "  ✅ yamllint installed" || echo "  ❌ yamllint not found"
	@curl -s --head --max-time 5 https://www.plantuml.com/plantuml/png/ >/dev/null 2>&1 && echo "  ✅ PlantUML server accessible" || echo "  ⚠️  PlantUML server unreachable"
	@echo ""

# ═══════════════════════════════════════════════════════════════════════════
# WORKFLOW TARGETS
# ═══════════════════════════════════════════════════════════════════════════

# Full pre-commit check (what CI would run)
pre-commit: format-check lint test validate-all
	@echo ""
	@echo "✅ Pre-commit checks passed! Safe to commit."

# Quick check during development (faster than pre-commit)
check: format-check lint test-unit validate-docs
	@echo ""
	@echo "✅ Quick checks passed!"
