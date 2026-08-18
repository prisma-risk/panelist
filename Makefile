CARGO ?= cargo
CARGO_DENY_CHECK_FLAGS ?=

.PHONY: all ci check fmt fmt-check lint fix build test doc headers package release-dry-run deny verify-grafana demo demo-down render-examples clean help

all: ci

ci: check headers package deny

check: fmt-check lint build test doc

fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all -- --check

lint:
	$(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

fix:
	$(CARGO) clippy --workspace --all-targets --all-features --locked \
	  --fix --allow-dirty --allow-staged
	$(CARGO) fmt --all

build:
	$(CARGO) build --workspace --all-features --locked

test:
	$(CARGO) test --workspace --all-features --locked

doc:
	RUSTDOCFLAGS="-D warnings -D missing-docs" $(CARGO) doc --workspace --all-features --no-deps --locked

headers:
	python3 scripts/check-panelist-header.py
	cmp LICENSE crates/panelist/LICENSE

package:
	$(CARGO) package -p panelist --allow-dirty --locked

release-dry-run:
	$(CARGO) publish --dry-run -p panelist --locked

deny:
	$(CARGO) deny check $(CARGO_DENY_CHECK_FLAGS)

verify-grafana:
	scripts/verify-grafana.sh

demo:
	scripts/demo.sh up

demo-down:
	scripts/demo.sh down

render-examples:
	scripts/demo.sh render

clean:
	$(CARGO) clean


help:
	@echo "Targets:"
	@echo "  make ci           Full CI parity"
	@echo "  make check        Fast Rust development checks"
	@echo "  make fmt          Reformat the workspace"
	@echo "  make fmt-check    Verify formatting"
	@echo "  make lint         Run Clippy with warnings denied"
	@echo "  make fix          Apply Clippy fixes and format"
	@echo "  make build        Build the locked workspace"
	@echo "  make test         Test the locked workspace"
	@echo "  make doc          Build rustdoc with warnings denied"
	@echo "  make headers      Verify Rust source headers"
	@echo "  make package      Verify the distributable crate tarball"
	@echo "  make release-dry-run  Validate the crates.io upload without publishing"
	@echo "  make deny         Check licenses and advisories"
	@echo "  make verify-grafana  Round-trip golden dashboards through Grafana (needs Docker)"
	@echo "  make demo         Boot the example dashboards on a live Grafana (needs Docker)"
	@echo "  make render-examples  Re-render the gallery screenshots from the demo stack"
	@echo "  make demo-down    Stop the demo stack"
	@echo "  make clean        Remove Cargo build output"
