.PHONY: build release test install setup all clean

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

install:
	cargo install --path . --force
	@echo '\nRun: source ~/.zshrc'

setup: install
	ukrop setup --force

all: test release

clean:
	cargo clean
