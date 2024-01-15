bin:
	cargo build --release
fmt:
	cargo fmt
	git status
test:
	cargo test --release
clean:
	cargo clean
lint:
	cargo clippy --release --fix
	git status

xxx:	fmt lint test
