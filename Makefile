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

deb:
	cargo deb
	@echo "Deb package created in target/debian/"

install-deb:
	sudo dpkg -i target/debian/filecast_*.deb

release:
	cargo build --release
	cargo deb
	@echo "Release complete! Binary: target/release/filecast, Deb: target/debian/"

prepare:
	cargo build --release
	cargo deb
	mkdir -p releases
	cp target/debian/filecast_*.deb releases/
	cp target/release/filecast releases/
	@echo "Release files ready in releases/"
	@ls -la releases/