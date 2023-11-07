ifeq ($(PREFIX),)
    PREFIX := /usr/local
endif

build:
	cargo build --release

clean:
	cargo clean

install: build
	install -Dm755 target/release/satty -t ${PREFIX}/bin/
	install -Dm644 satty.desktop ${PREFIX}/share/applications/satty.desktop
	install -Dm644 assets/satty.svg ${PREFIX}/share/icons/hicolor/scalable/apps/satty.svg

	install -Dm644 LICENSE ${PREFIX}/share/licenses/satty/LICENSE

uninstall:
	rm ${PREFIX}/bin/satty
	rmdir -p ${PREFIX}/bin || true

	rm ${PREFIX}/share/applications/satty.desktop
	rmdir -p ${PREFIX}/share/applications || true

	rm ${PREFIX}/share/icons/hicolor/scalable/apps/satty.svg
	rmdir -p ${PREFIX}/share/icons/hicolor/scalable/apps || true

	rm ${PREFIX}/share/licenses/satty/LICENSE
	rmdir -p ${PREFIX}/share/licenses/satty || true
