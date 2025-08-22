help:
        just --list

develop:
        just initialize-venv
        .venv/bin/python -m maturin develop

initialize-venv:
        python -m venv .venv
        .venv/bin/pip install maturin

lint:
        ruff format --check --diff
        mypy --check
        cargo fmt --check
        cargo clippy --all-targets --all-features -- --deny warnings

format:
        ruff format
        cargo fmt

test:
        just develop
        just lint
        cargo test
        python -m unittest discover python/
