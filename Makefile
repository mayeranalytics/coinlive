build:
	RUSTFLAGS='-C target-cpu=native' cargo build

run: 
	RUSTFLAGS='-C target-cpu=native' cargo run

release:
	RUSTFLAGS='-C target-cpu=native -C opt-level=3' cargo build --release

run-release:
	RUSTFLAGS='-C target-cpu=native -C opt-level=3' cargo run --release

doc:
	cargo doc --no-deps --open

kill:
	ps -eaf | grep debug/coinlive|grep -v grep|awk '{print $$2}'|xargs kill

test:
	cargo test


.PHONY: build run release run-release kill doc test
