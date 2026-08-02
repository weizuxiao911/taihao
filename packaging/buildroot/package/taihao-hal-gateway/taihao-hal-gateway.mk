################################################################################
#
# taihao-hal-gateway
#
################################################################################

# 源码位于仓库根 src/hal-gateway（BR2_EXTERNAL_TAIHAO_PATH = <仓库>/packaging/buildroot）
TAIHAO_HAL_GATEWAY_VERSION = 0.1.0
TAIHAO_HAL_GATEWAY_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src/hal-gateway
TAIHAO_HAL_GATEWAY_SITE_METHOD = local
TAIHAO_HAL_GATEWAY_LICENSE = Apache-2.0
TAIHAO_HAL_GATEWAY_DEPENDENCIES = host-rustc

TAIHAO_HAL_GATEWAY_CARGO_ENV = \
	CARGO_TARGET_$(call UPPERCASE,$(RUSTC_TARGET_NAME))_LINKER=$(notdir $(TARGET_CROSS))gcc

define TAIHAO_HAL_GATEWAY_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/hal-gateway \
		$(TARGET_DIR)/usr/bin/hal-gateway
	$(INSTALL) -D -m 0644 $(@D)/hal-gateway.toml.example \
		$(TARGET_DIR)/etc/taihao-os/hal-gateway.toml
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_TAIHAO_PATH)/package/taihao-hal-gateway/hal-gateway.service \
		$(TARGET_DIR)/usr/lib/systemd/system/hal-gateway.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/hal-gateway.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/hal-gateway.service
	mkdir -p $(TARGET_DIR)/var/lib/taihao-os/hal-gateway
	mkdir -p $(TARGET_DIR)/var/log/taihao-os
endef

$(eval $(cargo-package))
