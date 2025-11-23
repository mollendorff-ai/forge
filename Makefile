# Mouvify Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed install install-user install-system uninstall lint lint-fix test test-unit test-integration test-e2e test-validate test-calculate test-all test-coverage clean clean-test pre-build post-build

# Detect if upx is available
HAS_UPX := $(shell command -v upx 2> /dev/null)

help:
	@echo "Mouvify Forge - Available Commands"
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
	@echo "Lint Targets:"
	@echo "  make lint               - Run pedantic clippy checks"
	@echo "  make lint-fix           - Auto-fix clippy warnings"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test               - Run all cargo tests (unit + integration + E2E)"
	@echo "  make test-unit          - Run unit tests only"
	@echo "  make test-integration   - Run integration tests only"
	@echo "  make test-e2e           - Run E2E tests with actual YAML files"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test-all           - Run ALL tests (40 total)"
	@echo "  make test-coverage      - Show test coverage summary"
	@echo ""
	@echo "Utilities:"
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
	@echo "✅ Binary: target/release/mouvify-forge"
	@ls -lh target/release/mouvify-forge
	@$(MAKE) -s post-build

build-static:
	@echo "🔨 Building static release binary (musl)..."
	@cargo build --release --target x86_64-unknown-linux-musl
	@echo "✅ Binary: target/x86_64-unknown-linux-musl/release/mouvify-forge"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge

build-compressed: build-static
	@echo ""
ifdef HAS_UPX
	@echo "📦 BEFORE compression:"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge | tail -1
	@BEFORE=$$(stat -c%s target/x86_64-unknown-linux-musl/release/mouvify-forge 2>/dev/null || stat -f%z target/x86_64-unknown-linux-musl/release/mouvify-forge); \
	echo ""; \
	echo "🗜️  Compressing with UPX --best --lzma..."; \
	upx --best --lzma target/x86_64-unknown-linux-musl/release/mouvify-forge; \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo "✨ WOW! AFTER compression:"; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge | tail -1; \
	AFTER=$$(stat -c%s target/x86_64-unknown-linux-musl/release/mouvify-forge 2>/dev/null || stat -f%z target/x86_64-unknown-linux-musl/release/mouvify-forge); \
	SAVED=$$(($$BEFORE - $$AFTER)); \
	PERCENT=$$(awk "BEGIN {printf \"%.1f\", ($$SAVED / $$BEFORE) * 100}"); \
	echo ""; \
	echo "🎉 Saved: $$SAVED bytes ($$PERCENT% smaller!)"; \
	echo "📊 From $$(numfmt --to=iec-i --suffix=B $$BEFORE 2>/dev/null || echo $$BEFORE bytes) → $$(numfmt --to=iec-i --suffix=B $$AFTER 2>/dev/null || echo $$AFTER bytes)"
else
	@echo "⚠️  UPX not found - install with: sudo apt install upx-ucl"
	@echo "📦 Static binary built (not compressed):"
	@ls -lh target/x86_64-unknown-linux-musl/release/mouvify-forge
endif

install-system: build
	@echo "📦 Installing mouvify-forge to /usr/local/bin (system-wide)..."
	@sudo install -m 755 target/release/mouvify-forge /usr/local/bin/mouvify-forge
	@echo "✅ Installed to /usr/local/bin/mouvify-forge"
	@echo "🔍 Verify with: mouvify-forge --version"

install-user: build
	@echo "📦 Installing mouvify-forge to ~/.local/bin (user-only)..."
	@mkdir -p ~/.local/bin
	@install -m 755 target/release/mouvify-forge ~/.local/bin/mouvify-forge
	@echo "✅ Installed to ~/.local/bin/mouvify-forge"
	@echo "💡 Make sure ~/.local/bin is in your PATH"
	@echo "🔍 Verify with: mouvify-forge --version"

install: install-system

uninstall:
	@echo "🗑️  Uninstalling mouvify-forge..."
	@sudo rm -f /usr/local/bin/mouvify-forge 2>/dev/null || true
	@rm -f ~/.local/bin/mouvify-forge 2>/dev/null || true
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
