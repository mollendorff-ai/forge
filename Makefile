# Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed build-all install install-user install-system uninstall install-forge install-all cross-forge lint lint-fix format format-check test test-unit test-integration test-e2e test-validate test-calculate test-all test-coverage coverage coverage-report coverage-ci validate-docs validate-yaml validate-diagrams validate-all install-tools clean clean-test pre-build post-build pre-commit check

# ═══════════════════════════════════════════════════════════════════════════════
# OS AND ARCHITECTURE DETECTION
# ═══════════════════════════════════════════════════════════════════════════════

UNAME_S := $(shell uname -s 2>/dev/null || echo Windows)
UNAME_M := $(shell uname -m 2>/dev/null || echo x86_64)

# Normalize architecture names
ifeq ($(UNAME_M),arm64)
    ARCH := aarch64
else ifeq ($(UNAME_M),aarch64)
    ARCH := aarch64
else
    ARCH := x86_64
endif

# Set platform-specific variables
ifeq ($(UNAME_S),Linux)
    PLATFORM := linux
    BUILD_TARGET := $(ARCH)-unknown-linux-musl
    STATIC_BINARY := target/$(BUILD_TARGET)/release/forge
    TARGET_FLAG := --target $(BUILD_TARGET)
    UPX_SUPPORTED := true
else ifeq ($(UNAME_S),Darwin)
    PLATFORM := macos
    BUILD_TARGET := $(ARCH)-apple-darwin
    STATIC_BINARY := target/release/forge
    TARGET_FLAG :=
    # UPX not supported on macOS - breaks code signing
    UPX_SUPPORTED := false
else ifneq (,$(findstring MINGW,$(UNAME_S)))
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else ifneq (,$(findstring MSYS,$(UNAME_S)))
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else ifeq ($(OS),Windows_NT)
    PLATFORM := windows
    BUILD_TARGET := x86_64-pc-windows-msvc
    STATIC_BINARY := target/release/forge.exe
    TARGET_FLAG :=
    UPX_SUPPORTED := true
else
    PLATFORM := unknown
    BUILD_TARGET :=
    STATIC_BINARY := target/release/forge
    TARGET_FLAG :=
    UPX_SUPPORTED := false
endif

# Detect if tools are available
HAS_UPX := $(shell command -v upx 2> /dev/null)
HAS_CROSS := $(shell command -v cross 2> /dev/null)

# Cross-compilation targets (for build-all and new cross-* targets)
CROSS_TARGETS := x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu
CROSS_TARGETS_ALL := aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-pc-windows-gnu

# Detect if cargo-zigbuild is available
HAS_ZIGBUILD := $(shell command -v cargo-zigbuild 2> /dev/null)

help:
	@echo "🔥 Forge - Available Commands"
	@echo ""
	@echo "Platform: $(PLATFORM) ($(ARCH))"
	@echo "Target:   $(BUILD_TARGET)"
	@echo ""
	@echo "Build Targets:"
	@echo "  make build              - Standard release build (with pre/post checks)"
	@echo "  make build-static       - Static release build for current platform"
	@echo "  make build-compressed   - Static + UPX compressed (Linux/Windows only)"
	@echo "  make build-all          - Cross-compile for all platforms (requires cross-rs)"
	@echo ""
	@echo "Install Targets (to ~/bin):"
	@echo "  make install-forge      - Build forge + install to ~/bin"
	@echo "  make install-all        - Build all binaries + install to ~/bin"
	@echo ""
	@echo "System Install Targets:"
	@echo "  make install            - Install to /usr/local/bin (system-wide, requires sudo)"
	@echo "  make install-user       - Install to ~/.local/bin (user-only, no sudo)"
	@echo "  make install-system     - Same as install (system-wide)"
	@echo "  make uninstall          - Uninstall from both locations"
	@echo ""
	@echo "Cross-Platform Builds (cargo-zigbuild):"
	@echo "  make cross-forge        - Build forge for all 5 platforms → dist/"
	@echo ""
	@echo "Code Quality:"
	@echo "  make lint               - Run pedantic clippy checks"
	@echo "  make lint-fix           - Auto-fix clippy warnings"
	@echo "  make format             - Format code with rustfmt"
	@echo "  make format-check       - Check formatting without modifying"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test               - Run all cargo tests (unit + inline tests)"
	@echo "  make test-unit          - Run unit tests only (--lib)"
	@echo "  make test-integration   - Run integration tests only"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test-all           - Run ALL unit tests (2,703 tests)"
	@echo ""
	@echo "E2E Tests (separate repository - ADR-027):"
	@echo "  See: https://github.com/mollendorff-ai/forge-e2e"
	@echo ""
	@echo "Coverage Targets (ADR-004: 100% MANDATORY):"
	@echo "  make coverage           - Run coverage, FAIL if < 100%"
	@echo "  make coverage-report    - Generate HTML coverage report"
	@echo "  make coverage-ci        - CI mode: FAIL if < 100% + lcov output"
	@echo ""
	@echo "Documentation:"
	@echo "  make docs-cli           - Generate CLI reference from --help (auto)"
	@echo "  make docs-cli-check     - Verify CLI docs are up to date (CI)"
	@echo "  make validate-docs      - Validate markdown files (markdownlint-cli2)"
	@echo "  make validate-yaml      - Validate YAML files (yamllint)"
	@echo "  make validate-all       - Run ALL validators (docs + yaml)"
	@echo ""
	@echo "Presentation:"
	@echo "  (moved to https://github.com/mollendorff-ai/asimov)"
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
	@echo "3️⃣  Checking CLI docs are up to date..."
	@$(MAKE) -s docs-cli-check
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
	@echo "🔨 Building static release binary..."
	@echo "   Platform: $(PLATFORM) ($(ARCH))"
	@echo "   Target:   $(BUILD_TARGET)"
ifeq ($(PLATFORM),linux)
	@cargo build --release $(TARGET_FLAG)
else ifeq ($(PLATFORM),macos)
	@cargo build --release
else ifeq ($(PLATFORM),windows)
	@cargo build --release
else
	@echo "❌ Unknown platform: $(UNAME_S)"
	@exit 1
endif
	@echo "✅ Binary: $(STATIC_BINARY)"
	@ls -lh $(STATIC_BINARY)

build-compressed: build-static
	@echo ""
ifeq ($(UPX_SUPPORTED),true)
ifdef HAS_UPX
	@echo "📦 BEFORE compression:"
	@ls -lh $(STATIC_BINARY) | tail -1
	@BEFORE=$$(stat -c%s $(STATIC_BINARY) 2>/dev/null || stat -f%z $(STATIC_BINARY)); \
	echo ""; \
	echo "🗜️  Compressing with UPX --best --lzma..."; \
	upx --best --lzma $(STATIC_BINARY); \
	echo ""; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	echo "✨ AFTER compression:"; \
	echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
	ls -lh $(STATIC_BINARY) | tail -1; \
	AFTER=$$(stat -c%s $(STATIC_BINARY) 2>/dev/null || stat -f%z $(STATIC_BINARY)); \
	SAVED=$$(($$BEFORE - $$AFTER)); \
	PERCENT=$$(awk "BEGIN {printf \"%.1f\", ($$SAVED / $$BEFORE) * 100}"); \
	echo ""; \
	echo "🎉 Saved: $$SAVED bytes ($$PERCENT% smaller!)"; \
	echo "📊 From $$(numfmt --to=iec-i --suffix=B $$BEFORE 2>/dev/null || echo $$BEFORE bytes) → $$(numfmt --to=iec-i --suffix=B $$AFTER 2>/dev/null || echo $$AFTER bytes)"
else
	@echo "⚠️  UPX not found - install with: sudo apt install upx-ucl (Linux) or choco install upx (Windows)"
	@echo "📦 Static binary built (not compressed):"
	@ls -lh $(STATIC_BINARY)
endif
else
	@echo "ℹ️  UPX compression not supported on $(PLATFORM) (breaks code signing)"
	@echo "📦 Static binary built:"
	@ls -lh $(STATIC_BINARY)
endif

# Cross-compile forge for all platforms (requires cross-rs: cargo install cross)
build-all:
	@echo "🌍 Cross-compiling forge for all platforms..."
	@echo ""
ifndef HAS_CROSS
	@echo "❌ cross-rs not found. Install with: cargo install cross"
	@echo "   Also requires Docker to be running."
	@exit 1
endif
	@mkdir -p dist
	@for target in $(CROSS_TARGETS); do \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		echo "🔨 Building forge for $$target..."; \
		echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"; \
		cross build --release --bin forge --target $$target || exit 1; \
		if echo "$$target" | grep -q "windows"; then \
			cp target/$$target/release/forge.exe dist/forge-$$target.exe; \
			ls -lh dist/forge-$$target.exe; \
		else \
			cp target/$$target/release/forge dist/forge-$$target; \
			ls -lh dist/forge-$$target; \
		fi; \
		echo ""; \
	done
	@echo "✅ All builds complete! Binaries in dist/"
	@ls -lh dist/

install-system: clean build-compressed
	@echo "📦 Installing forge to /usr/local/bin (system-wide)..."
ifeq ($(PLATFORM),windows)
	@echo "❌ Use install-user on Windows or copy manually"
	@exit 1
else
	@sudo install -m 755 $(STATIC_BINARY) /usr/local/bin/forge
	@echo "✅ Installed to /usr/local/bin/forge"
	@echo "🔍 Verify with: forge --version"
endif

install-user: clean build-compressed
	@echo "📦 Installing forge to ~/.local/bin (user-only)..."
	@mkdir -p ~/.local/bin
ifeq ($(PLATFORM),windows)
	@copy $(STATIC_BINARY) %USERPROFILE%\.local\bin\forge.exe
else
	@install -m 755 $(STATIC_BINARY) ~/.local/bin/forge
endif
	@echo "✅ Installed to ~/.local/bin/forge"
	@echo "💡 Make sure ~/.local/bin is in your PATH"
	@echo "🔍 Verify with: forge --version"

install: install-system

uninstall:
	@echo "🗑️  Uninstalling forge..."
	@sudo rm -f /usr/local/bin/forge 2>/dev/null || true
	@rm -f ~/.local/bin/forge 2>/dev/null || true
	@echo "✅ Uninstalled from both /usr/local/bin and ~/.local/bin"

# ═══════════════════════════════════════════════════════════════════════════
# INSTALL TO ~/bin TARGETS
# ═══════════════════════════════════════════════════════════════════════════

install-forge:
	@echo "🔨 Building forge..."
	@cargo build --release --bin forge
	@echo ""
	@echo "📦 Installing forge to ~/bin..."
	@mkdir -p ~/bin
	@install -m 755 target/release/forge ~/bin/forge
	@echo "✅ Installed to ~/bin/forge"
	@echo "💡 Make sure ~/bin is in your PATH"
	@echo "🔍 Verify with: forge --version"
	@echo ""
	@echo "📊 Function count:"
	@~/bin/forge functions 2>/dev/null | wc -l | xargs -I{} echo "   {} functions available"

install-all: install-forge
	@echo ""
	@echo "✅ forge installed to ~/bin!"
	@ls -lh ~/bin/forge

# ═══════════════════════════════════════════════════════════════════════════
# CROSS-PLATFORM BUILDS (cargo-zigbuild)
# ═══════════════════════════════════════════════════════════════════════════

cross-forge:
	@echo "🌍 Cross-compiling forge (enterprise) for all platforms..."
	@echo ""
ifndef HAS_ZIGBUILD
	@echo "❌ cargo-zigbuild not found. Install with: cargo install cargo-zigbuild"
	@exit 1
endif
	@mkdir -p dist
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge for macOS ARM64 (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge --target aarch64-apple-darwin
	@cp target/aarch64-apple-darwin/release/forge dist/forge-aarch64-apple-darwin
	@ls -lh dist/forge-aarch64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🍎 Building forge for macOS Intel (native)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo build --release --bin forge --target x86_64-apple-darwin
	@cp target/x86_64-apple-darwin/release/forge dist/forge-x86_64-apple-darwin
	@ls -lh dist/forge-x86_64-apple-darwin
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge for Linux x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target x86_64-unknown-linux-musl
	@cp target/x86_64-unknown-linux-musl/release/forge dist/forge-x86_64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-x86_64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-x86_64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🐧 Building forge for Linux ARM64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target aarch64-unknown-linux-musl
	@cp target/aarch64-unknown-linux-musl/release/forge dist/forge-aarch64-unknown-linux-musl
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-aarch64-unknown-linux-musl; \
	fi
	@ls -lh dist/forge-aarch64-unknown-linux-musl
	@echo ""
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "🪟 Building forge for Windows x86_64 (zigbuild)..."
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo zigbuild --release --bin forge --target x86_64-pc-windows-gnu
	@cp target/x86_64-pc-windows-gnu/release/forge.exe dist/forge-x86_64-pc-windows-gnu.exe
	@if command -v upx >/dev/null 2>&1; then \
		echo "🗜️  Compressing with UPX..."; \
		upx --best --lzma dist/forge-x86_64-pc-windows-gnu.exe; \
	fi
	@ls -lh dist/forge-x86_64-pc-windows-gnu.exe
	@echo ""
	@echo "✅ All builds complete! Binaries in dist/"
	@ls -lh dist/forge-*

lint:
	@echo "🔍 Running pedantic clippy checks..."
	@cargo clippy --all-targets -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions \
		-A clippy::float_cmp \
		-A clippy::items_after_statements \
		-A clippy::similar_names \
		-A clippy::unreadable_literal \
		-A clippy::doc_markdown \
		-A clippy::multiple_crate_versions \
		-A clippy::needless_pass_by_value \
		-A clippy::too_many_lines \
		-A clippy::cast_possible_truncation \
		-A clippy::format_push_string \
		-A clippy::match_same_arms \
		-A clippy::must_use_candidate \
		-A clippy::redundant_clone \
		-A clippy::or_fun_call \
		-A clippy::redundant_pub_crate \
		-A clippy::cast_lossless \
		-A clippy::cognitive_complexity \
		-A clippy::option_if_let_else \
		-A clippy::struct_excessive_bools \
		-A clippy::struct_field_names \
		-A clippy::significant_drop_tightening \
		-A clippy::if_not_else
	@echo "✅ Clippy checks passed!"

lint-fix:
	@echo "🔧 Running clippy with auto-fix..."
	@cargo clippy --fix --allow-dirty --allow-staged --all-targets -- \
		-W clippy::pedantic \
		-W clippy::nursery \
		-W clippy::cargo \
		-A clippy::missing_errors_doc \
		-A clippy::missing_panics_doc \
		-A clippy::module_name_repetitions \
		-A clippy::float_cmp \
		-A clippy::items_after_statements \
		-A clippy::similar_names \
		-A clippy::unreadable_literal \
		-A clippy::doc_markdown \
		-A clippy::multiple_crate_versions \
		-A clippy::needless_pass_by_value \
		-A clippy::too_many_lines \
		-A clippy::cast_possible_truncation \
		-A clippy::format_push_string \
		-A clippy::match_same_arms \
		-A clippy::must_use_candidate \
		-A clippy::redundant_clone \
		-A clippy::or_fun_call \
		-A clippy::redundant_pub_crate \
		-A clippy::cast_lossless \
		-A clippy::cognitive_complexity \
		-A clippy::option_if_let_else \
		-A clippy::struct_excessive_bools \
		-A clippy::struct_field_names \
		-A clippy::significant_drop_tightening \
		-A clippy::if_not_else
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

# E2E tests migrated to forge-e2e (ADR-027)
# See: https://github.com/mollendorff-ai/forge-e2e

test-all: test test-validate test-calculate
	@echo ""
	@echo "🎉 All tests passed!"

# Legacy test-coverage target (shows summary only)
test-coverage:
	@echo "📊 Test Coverage Summary (use 'make coverage' for actual coverage)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@echo "See ADR-004: 100% test coverage is MANDATORY"
	@echo "Run 'make coverage' to verify coverage meets 100% requirement"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo test 2>&1 | grep -E "running [0-9]+ tests" | awk '{sum += $$2} END {print "Total tests: " sum}'

# ═══════════════════════════════════════════════════════════════════════════
# COVERAGE TARGETS (ADR-004: 100% REQUIRED)
# ═══════════════════════════════════════════════════════════════════════════

# Coverage: Run tests with coverage, FAIL if < 100%
# ADR-004: 100% coverage is MANDATORY - NO EXCEPTIONS
coverage:
	@echo "📊 Running test coverage (100% REQUIRED - ADR-004)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "❌ cargo-llvm-cov not found. Installing..."; \
		cargo install cargo-llvm-cov; \
	fi
	@cargo llvm-cov --fail-under-lines 100 --ignore-filename-regex '(tests/|test_)' || \
		(echo ""; echo "❌ COVERAGE BELOW 100% - BUILD FAILED (ADR-004)"; echo "Run 'make coverage-report' to see uncovered lines"; exit 1)
	@echo ""
	@echo "✅ 100% coverage verified!"

# Coverage report: Generate detailed HTML report and open in browser
coverage-report:
	@echo "📊 Generating coverage report..."
	@if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
		echo "❌ cargo-llvm-cov not found. Installing..."; \
		cargo install cargo-llvm-cov; \
	fi
	@cargo llvm-cov --html --ignore-filename-regex '(tests/|test_)' --output-dir coverage-report
	@echo "✅ Coverage report generated: coverage-report/html/index.html"
	@if command -v xdg-open >/dev/null 2>&1; then \
		xdg-open coverage-report/html/index.html; \
	elif command -v open >/dev/null 2>&1; then \
		open coverage-report/html/index.html; \
	else \
		echo "Open coverage-report/html/index.html in your browser"; \
	fi

# Coverage CI: Strict 100% enforcement for CI/CD pipeline
# ADR-004: FAIL THE BUILD if < 100% - NO EXCEPTIONS
coverage-ci:
	@echo "📊 CI Coverage Check (100% REQUIRED - ADR-004)"
	@echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
	@cargo llvm-cov --fail-under-lines 100 --ignore-filename-regex '(tests/|test_)' --lcov --output-path lcov.info
	@echo ""
	@echo "✅ 100% coverage verified!"
	@echo "📄 lcov.info generated for coverage upload"

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
		yamllint warmup.yaml sprint.yaml roadmap.yaml 2>/dev/null && \
		echo "✅ YAML validation passed"; \
	else \
		echo "❌ yamllint not found. Run: pip install yamllint"; \
		exit 1; \
	fi

validate-diagrams:
	@echo "🎨 Diagram validation (Mermaid diagrams are validated by GitHub)"
	@echo "✅ Mermaid diagrams embedded in markdown - no validation needed"
	@if [ -d "diagrams" ] && find diagrams -name "*.puml" -o -name "*.plantuml" 2>/dev/null | grep -q .; then \
		echo "⚠️  Warning: Found old PlantUML files in diagrams/ - consider removing"; \
	fi

validate-all: validate-docs validate-yaml validate-diagrams
	@echo ""
	@echo "✅ All validation checks completed!"

# ═══════════════════════════════════════════════════════════════════════════
# DOCUMENTATION GENERATION
# ═══════════════════════════════════════════════════════════════════════════

# Generate CLI reference documentation from actual --help output
docs-cli:
	@echo "📚 Generating CLI documentation from --help..."
	@mkdir -p docs/cli
	@echo "# Forge CLI Reference" > docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo "> Auto-generated from \`forge --help\`. Do not edit manually." >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo "## Main Help" >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@echo '```' >> docs/cli/README.md
	@./target/release/forge --help >> docs/cli/README.md
	@echo '```' >> docs/cli/README.md
	@echo "" >> docs/cli/README.md
	@for cmd in calculate validate audit export import watch compare variance sensitivity goal-seek break-even update functions upgrade simulate scenarios decision-tree real-options tornado bootstrap bayesian; do \
		echo "## $$cmd" >> docs/cli/README.md; \
		echo "" >> docs/cli/README.md; \
		echo '```' >> docs/cli/README.md; \
		./target/release/forge $$cmd --help >> docs/cli/README.md 2>/dev/null || true; \
		echo '```' >> docs/cli/README.md; \
		echo "" >> docs/cli/README.md; \
	done
	@echo "✅ Generated docs/cli/README.md"

# Verify CLI docs are up to date (for CI)
docs-cli-check:
	@echo "🔍 Checking CLI documentation is up to date..."
	@$(MAKE) -s docs-cli
	@if git diff --quiet docs/cli/README.md; then \
		echo "✅ CLI documentation is up to date"; \
	else \
		echo "❌ CLI documentation is out of date!"; \
		echo "Run 'make docs-cli' to regenerate"; \
		exit 1; \
	fi

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
	@echo "4. Marp CLI (presentation generation)"
	@echo "   npm install -g @marp-team/marp-cli"
	@echo ""
	@echo "5. PlantUML (diagram validation - optional)"
	@echo "   Using public server: https://www.plantuml.com/plantuml"
	@echo ""
	@echo "Current status:"
	@command -v cargo >/dev/null 2>&1 && echo "  ✅ Rust/Cargo installed" || echo "  ❌ Rust/Cargo not found"
	@command -v markdownlint-cli2 >/dev/null 2>&1 && echo "  ✅ markdownlint-cli2 installed" || echo "  ❌ markdownlint-cli2 not found"
	@command -v yamllint >/dev/null 2>&1 && echo "  ✅ yamllint installed" || echo "  ❌ yamllint not found"
	@command -v marp >/dev/null 2>&1 && echo "  ✅ Marp CLI installed" || echo "  ❌ Marp CLI not found"
	@curl -s --head --max-time 5 https://www.plantuml.com/plantuml/png/ >/dev/null 2>&1 && echo "  ✅ PlantUML server accessible" || echo "  ⚠️  PlantUML server unreachable"
	@echo ""

# ═══════════════════════════════════════════════════════════════════════════
# WORKFLOW TARGETS
# ═══════════════════════════════════════════════════════════════════════════

# Full pre-commit check (what CI would run)
# ADR-004: 100% coverage is MANDATORY - NO EXCEPTIONS
pre-commit: format-check lint test coverage docs-cli-check validate-all
	@echo ""
	@echo "✅ Pre-commit checks passed! Safe to commit."

# Quick check during development (faster than pre-commit)
check: format-check lint test-unit validate-docs
	@echo ""
	@echo "✅ Quick checks passed!"

# ═══════════════════════════════════════════════════════════════════════════
# PRESENTATION
# ═══════════════════════════════════════════════════════════════════════════
# Presentation deck moved to: https://github.com/mollendorff-ai/asimov
# See: docs/PRESENTATION.md for redirect info
