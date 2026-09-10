check: lint test

serve *args:
    cd web && dx serve {{args}}

build *args:
    cd web && dx build {{args}}

test *args:
    cargo nextest run --no-fail-fast --no-tests=pass {{args}}

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
