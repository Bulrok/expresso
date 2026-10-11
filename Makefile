# Expresso: a shot of eval, anywhere.
# Linux: GNOME Shell extension (GJS).  Windows: Rust app.  Shared: spec/cases.json.

UUID      := expresso@bulrok
EXT_DIR   := linux/extension/$(UUID)
WIN_DIR   := windows
WIN_TARGET:= x86_64-pc-windows-gnu
DIST      := dist

.PHONY: test test-linux test-windows test-nested lint dist dist-linux dist-windows install uninstall check-version clean

test: test-linux test-windows        ## Shared spec + unit tests on both implementations

test-linux:
	gjs -m linux/tests/calc-spec.js

test-windows:                         ## Runs on any OS: the evaluator and layout are portable
	cd $(WIN_DIR) && cargo test

test-nested:                          ## Headless nested GNOME Shell end-to-end test (needs GNOME 50)
	linux/tests/nested/run.sh

lint:
	cd $(WIN_DIR) && cargo fmt --check && cargo clippy --all-targets -- -D warnings

dist: dist-linux dist-windows

dist-linux: check-version             ## Extension zip in the extensions.gnome.org layout
	@mkdir -p $(DIST)
	rm -f $(DIST)/$(UUID).shell-extension.zip
	cd $(EXT_DIR) && zip -q -X ../../../$(DIST)/$(UUID).shell-extension.zip extension.js calc.js metadata.json stylesheet.css
	@echo "$(DIST)/$(UUID).shell-extension.zip"

dist-windows: check-version           ## Cross-compiled from Linux (needs mingw64-gcc + the windows-gnu rust-std)
	@mkdir -p $(DIST)
	cd $(WIN_DIR) && cargo build --release --target $(WIN_TARGET)
	cp $(WIN_DIR)/target/$(WIN_TARGET)/release/expresso.exe $(DIST)/expresso-windows-x86_64.exe
	@echo "$(DIST)/expresso-windows-x86_64.exe"

install:
	linux/install.sh

uninstall:
	linux/install.sh --uninstall

check-version:
	@scripts/check-version.sh

clean:
	rm -rf $(DIST) linux/tests/nested/out
	cd $(WIN_DIR) && cargo clean
