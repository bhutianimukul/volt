.PHONY: docs

BUILD_MISC_DIR = misc
DOCS_DIR = docs
TARGET = volt
TARGET_DIR = target/release
TARGET_DIR_DEBIAN = target/debian
TARGET_DIR_OSX = $(TARGET_DIR)/osx
RELEASE_DIR = release

APP_NAME = Volt.app
APP_TEMPLATE = $(BUILD_MISC_DIR)/osx/$(APP_NAME)
APP_BINARY = $(TARGET_DIR)/$(TARGET)
APP_BINARY_DIR = $(TARGET_DIR_OSX)/$(APP_NAME)/Contents/MacOS
APP_EXTRAS_DIR = $(TARGET_DIR_OSX)/$(APP_NAME)/Contents/Resources

all: install run

run:
	cargo run -p volt --release

dev:
	MTL_HUD_ENABLED=1 cargo run -p volt

dev-debug:
	MTL_HUD_ENABLED=1 VOLT_LOG_LEVEL=debug make dev

install:
	cargo fetch

build: install
	RUSTFLAGS='-C link-arg=-s' cargo build --release

$(TARGET)-universal:
	RUSTFLAGS='-C link-arg=-s' MACOSX_DEPLOYMENT_TARGET="10.15" cargo build --release --target=x86_64-apple-darwin
	RUSTFLAGS='-C link-arg=-s' MACOSX_DEPLOYMENT_TARGET="11.0" cargo build --release --target=aarch64-apple-darwin
	@lipo target/{x86_64,aarch64}-apple-darwin/release/$(TARGET) -create -output $(APP_BINARY)

app-universal: $(APP_NAME)-universal ## Create a universal Volt.app
$(APP_NAME)-%: $(TARGET)-%
	@mkdir -p $(APP_BINARY_DIR)
	@mkdir -p $(APP_EXTRAS_DIR)
	@cp -fRp $(APP_TEMPLATE) $(TARGET_DIR_OSX)
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@touch -r "$(APP_BINARY)" "$(TARGET_DIR_OSX)/$(APP_NAME)"

release-macos: app-universal
	@codesign --remove-signature "$(TARGET_DIR_OSX)/$(APP_NAME)"
	@codesign --force --deep --sign - "$(TARGET_DIR_OSX)/$(APP_NAME)"
	@echo "Created '$(APP_NAME)' in '$(TARGET_DIR_OSX)'"
	mkdir -p $(RELEASE_DIR)
	cp -rf ./target/release/osx/* ./release/
	cd ./release && zip -r ./macos-unsigned.zip ./*

install-macos: release-macos
	rm -rf /Applications/$(APP_NAME)
	mv ./release/$(APP_NAME) /Applications/

lint:
	cargo fmt -- --check --color always
	cargo clippy --all-targets --all-features -- -D warnings

test:
	make lint
	RUST_BACKTRACE=full cargo test --release
