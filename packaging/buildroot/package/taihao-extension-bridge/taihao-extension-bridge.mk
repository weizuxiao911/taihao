################################################################################
#
# taihao-extension-bridge
#
################################################################################

# 源码位于仓库根 src/extension-bridge（BR2_EXTERNAL_TAIHAO_PATH = <仓库>/packaging/buildroot）
TAIHAO_EXTENSION_BRIDGE_VERSION = 0.1.0
TAIHAO_EXTENSION_BRIDGE_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src/extension-bridge
TAIHAO_EXTENSION_BRIDGE_SITE_METHOD = local
TAIHAO_EXTENSION_BRIDGE_LICENSE = Apache-2.0
TAIHAO_EXTENSION_BRIDGE_DEPENDENCIES = host-rustc

TAIHAO_EXTENSION_BRIDGE_CARGO_ENV = \
	CARGO_TARGET_$(call UPPERCASE,$(RUSTC_TARGET_NAME))_LINKER=$(notdir $(TARGET_CROSS))gcc

define TAIHAO_EXTENSION_BRIDGE_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/extension-bridge \
		$(TARGET_DIR)/usr/bin/extension-bridge
	$(INSTALL) -D -m 0644 $(@D)/extension-bridge.toml.example \
		$(TARGET_DIR)/etc/taihao-os/extension-bridge.toml
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_TAIHAO_PATH)/package/taihao-extension-bridge/extension-bridge.service \
		$(TARGET_DIR)/usr/lib/systemd/system/extension-bridge.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/extension-bridge.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/extension-bridge.service
	mkdir -p $(TARGET_DIR)/var/lib/taihao-os/extension-bridge
endef

$(eval $(cargo-package))
