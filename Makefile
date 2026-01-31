build:
	cargo build --release

run:
	cargo run --release

test:
	cargo test --release

clean:
	cargo clean

doc:
	cargo doc --release

publish:
	cargo publish

publish-dry:
	cargo publish --dry-run

package-list:
	cargo package --list

release:
	cargo build --release
	cargo run --release
	cargo test --release
	cargo clean
	cargo doc --release
	cargo publish --release
	@echo "Release complete!"