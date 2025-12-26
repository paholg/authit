serve *args:
    cd web && dx serve {{args}}

test *args:
    cargo nextest run --no-fail-fast {{args}}

up:
    nix flake update
    cargo upgrade -i

fix:
    cargo clippy --fix --allow-staged

lint: fmt-check clippy

fmt-check:
    cargo fmt --all -- --check

clippy:
    cargo clippy -- -D warnings
