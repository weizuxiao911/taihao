################################################################################
# extension-bridge — 太昊 OS 扩展 / MCP 桥
################################################################################

EXTENSION_BRIDGE_VERSION = 0.1.0
EXTENSION_BRIDGE_SITE = $(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src
EXTENSION_BRIDGE_SITE_METHOD = local


EXTENSION_BRIDGE_CARGO_TARGET_DIR = $(BUILD_DIR)/taihao-cargo-target

EXTENSION_BRIDGE_CARGO_ENV = \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(TARGET_CC)" \
	RUSTFLAGS="-C linker=$(TARGET_CC)"

define EXTENSION_BRIDGE_BUILD_CMDS
	cd $(@D) && \
	$(EXTENSION_BRIDGE_CARGO_ENV) \
	/usr/bin/cargo build --release \
		--target aarch64-unknown-linux-gnu \
		--target-dir $(EXTENSION_BRIDGE_CARGO_TARGET_DIR) \
		-p extension-bridge
endef

define EXTENSION_BRIDGE_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(EXTENSION_BRIDGE_CARGO_TARGET_DIR)/aarch64-unknown-linux-gnu/release/extension-bridge \
		$(TARGET_DIR)/usr/bin/extension-bridge
	$(INSTALL) -D -m 0644 \
		$(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src/systemd/extension-bridge.service \
		$(TARGET_DIR)/usr/lib/systemd/system/extension-bridge.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/extension-bridge.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/extension-bridge.service
endef

$(eval $(generic-package))