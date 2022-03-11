#VERSION := $(shell cat VERSION)
RUSTV=stable
DOCKER_TAG=$(VERSION)
GITHUB_TAG=v$(VERSION)
GIT_COMMIT=$(shell git rev-parse HEAD)
TEST_BUILD=$(if $(RELEASE),release,debug)
EXTRA_ARG=

install-fmt:
	rustup component add rustfmt --toolchain $(RUSTV)

check-fmt:
	cargo +$(RUSTV) fmt -- --check

install-clippy:
	rustup component add clippy --toolchain $(RUSTV)

check-clippy:	install-clippy
	cargo +$(RUSTV) clippy --all --all-targets --all-features --tests -- -D warnings

build-all-tests:
	cargo build --tests --all-features

run-all-tests:
	cargo test --all --all-features

run-all-doc-tests:
	cargo test --all --all-features --doc

release:
	cargo build --release
