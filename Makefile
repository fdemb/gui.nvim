TARGET = gui-nvim

ASSETS_DIR = extra
RELEASE_DIR = target/release

APP_NAME = gui.nvim.app
APP_TEMPLATE = $(ASSETS_DIR)/macos/$(APP_NAME)
APP_DIR = $(RELEASE_DIR)/macos
APP_BINARY = $(RELEASE_DIR)/$(TARGET)
APP_BINARY_DIR = $(APP_DIR)/$(APP_NAME)/Contents/MacOS
APP_RESOURCES_DIR = $(APP_DIR)/$(APP_NAME)/Contents/Resources

DMG_NAME = gui.nvim.dmg
DMG_DIR = $(RELEASE_DIR)/macos

# Minimum macOS version (11.0 = Big Sur, required for wgpu/Metal)
MACOSX_DEPLOYMENT_TARGET = 11.0

vpath $(TARGET) $(RELEASE_DIR)
vpath $(APP_NAME) $(APP_DIR)
vpath $(DMG_NAME) $(APP_DIR)

all: help

help: ## Print this help message
	@grep -E '^[a-zA-Z._-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

binary: $(TARGET)-native ## Build a release binary
binary-universal: $(TARGET)-universal ## Build a universal release binary

$(TARGET)-native:
	MACOSX_DEPLOYMENT_TARGET="$(MACOSX_DEPLOYMENT_TARGET)" cargo build --release

$(TARGET)-universal:
	MACOSX_DEPLOYMENT_TARGET="$(MACOSX_DEPLOYMENT_TARGET)" cargo build --release --target=x86_64-apple-darwin
	MACOSX_DEPLOYMENT_TARGET="$(MACOSX_DEPLOYMENT_TARGET)" cargo build --release --target=aarch64-apple-darwin
	@lipo target/{x86_64,aarch64}-apple-darwin/release/$(TARGET) -create -output $(APP_BINARY)

app: $(APP_NAME)-native ## Create a gui.nvim.app
app-universal: $(APP_NAME)-universal ## Create a universal gui.nvim.app

$(APP_NAME)-%: $(TARGET)-%
	@mkdir -p "$(APP_BINARY_DIR)"
	@mkdir -p "$(APP_RESOURCES_DIR)"
	@cp -fRp $(APP_TEMPLATE) $(APP_DIR)
	@cp -fp $(APP_BINARY) $(APP_BINARY_DIR)
	@touch -r "$(APP_BINARY)" "$(APP_DIR)/$(APP_NAME)"
	@codesign --remove-signature "$(APP_DIR)/$(APP_NAME)" 2>/dev/null || true
	@codesign --force --deep --sign - "$(APP_DIR)/$(APP_NAME)"
	@echo "Created '$(APP_NAME)' in '$(APP_DIR)'"

dmg: $(DMG_NAME)-native ## Create a gui.nvim.dmg
dmg-universal: $(DMG_NAME)-universal ## Create a universal gui.nvim.dmg

$(DMG_NAME)-%: $(APP_NAME)-%
	@echo "Packing disk image..."
	@ln -sf /Applications $(DMG_DIR)/Applications
	@hdiutil create $(DMG_DIR)/$(DMG_NAME) \
		-volname "gui.nvim" \
		-fs HFS+ \
		-srcfolder $(APP_DIR) \
		-ov -format UDZO
	@echo "Created '$(DMG_NAME)' in '$(DMG_DIR)'"

install: install-native ## Install the app to /Applications
install-universal: $(APP_NAME)-universal
	@cp -fRp "$(APP_DIR)/$(APP_NAME)" /Applications/
	@echo "Installed to /Applications/$(APP_NAME)"

install-native: $(APP_NAME)-native
	@cp -fRp "$(APP_DIR)/$(APP_NAME)" /Applications/
	@echo "Installed to /Applications/$(APP_NAME)"

.PHONY: all help binary binary-universal app app-universal dmg dmg-universal install install-native install-universal clean

clean: ## Remove all build artifacts
	@cargo clean
	@rm -rf $(APP_DIR)
