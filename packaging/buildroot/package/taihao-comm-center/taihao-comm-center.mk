################################################################################
#
# taihao-comm-center
#
################################################################################

# 源码位于仓库根 src/comm-center（BR2_EXTERNAL_TAIHAO_PATH = <仓库>/packaging/buildroot）
TAIHAO_COMM_CENTER_VERSION = 0.1.0
TAIHAO_COMM_CENTER_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src/comm-center
TAIHAO_COMM_CENTER_SITE_METHOD = local
TAIHAO_COMM_CENTER_LICENSE = Apache-2.0
TAIHAO_COMM_CENTER_DEPENDENCIES = host-rustc

TAIHAO_COMM_CENTER_CARGO_ENV = \
	CARGO_TARGET_$(call UPPERCASE,$(RUSTC_TARGET_NAME))_LINKER=$(notdir $(TARGET_CROSS))gcc

define TAIHAO_COMM_CENTER_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/comm-center \
		$(TARGET_DIR)/usr/bin/comm-center
	$(INSTALL) -D -m 0644 $(@D)/comm-center.toml.example \
		$(TARGET_DIR)/etc/taihao-os/comm-center.toml
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_TAIHAO_PATH)/package/taihao-comm-center/comm-center.service \
		$(TARGET_DIR)/usr/lib/systemd/system/comm-center.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/comm-center.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/comm-center.service
	mkdir -p $(TARGET_DIR)/var/lib/taihao-os/comm-center
endef

$(eval $(cargo-package))
