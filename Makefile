# Mouvify Forge - YAML Formula Calculator
# Build and test targets for optimized binary

.PHONY: help build build-static build-compressed test test-unit test-integration test-validate test-calculate test-all clean clean-test

# Detect if upx is available
HAS_UPX := $(shell command -v upx 2> /dev/null)

help:
	@echo "Mouvify Forge - Available Commands"
	@echo ""
	@echo "Build Targets:"
	@echo "  make build              - Standard release build"
	@echo "  make build-static       - Static release build (musl, 1.2MB)"
	@echo "  make build-compressed   - Static + UPX compressed (440KB)"
	@echo ""
	@echo "Test Targets:"
	@echo "  make test               - Run all cargo tests (unit + integration)"
	@echo "  make test-unit          - Run unit tests only"
	@echo "  make test-integration   - Run integration tests only"
	@echo "  make test-validate      - Validate all test-data files"
	@echo "  make test-calculate     - Calculate all test-data files (dry-run)"
	@echo "  make test-all           - Run cargo tests + validate + calculate"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean              - Remove build artifacts"
	@echo "  make clean-test         - Restore test-data to original state"

build:
	@echo "🔨 Building release binary..."
	@cargo build --release
	@echo "✅ Binary: target/release/mouvify-forge"
	@ls -lh target/release/mouvify-forge

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

test-all: test test-validate test-calculate
	@echo ""
	@echo "🎉 All tests passed!"

clean:
	@echo "🧹 Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Clean complete!"

clean-test:
	@echo "🔄 Restoring test-data files to git state..."
	@git checkout test-data/*.yaml
	@echo "✅ Test data restored!"
