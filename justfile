set positional-arguments

code-review: check-format build clippy test check-docs

check:
    cargo check --workspace --all-targets --all-features

check-warnings:
    RUSTFLAGS="--deny warnings" cargo check --workspace --all-targets --all-features --exclude "protos"

build:
    cargo build --all-targets --all-features

run:
    cargo run

test:
    cargo test

doc:
    cargo doc

watch:
    cargo watch -c -w src -x "run"

clippy:
    cargo clippy --workspace -- -D warnings -D clippy::all

# Reformats code. Requires nightly because several useful options (e.g. imports_granularity) are
# nightly-only
fmt:
    cargo +nightly fmt

check-format:
    cargo +nightly fmt -- --check

check-docs:
    RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links -D rustdoc::private-intra-doc-links -D rustdoc::bare-urls" cargo doc --all

outdated:
    # Check for outdated dependencies, consider installing cargo-edit and running 'cargo upgrade' if this fails
    cargo outdated --exit-code 1

upgrade:
    cargo upgrade --workspace

machete:
    cargo machete --fix

remove-unused-deps: machete
