################################################################################
#
# taihao-rt-loop
#
################################################################################

# 源码位于仓库根 src/rt-loop（BR2_EXTERNAL_TAIHAO_PATH = <仓库>/packaging/buildroot）
TAIHAO_RT_LOOP_VERSION = 0.1.0
TAIHAO_RT_LOOP_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src/rt-loop
TAIHAO_RT_LOOP_SITE_METHOD = local
TAIHAO_RT_LOOP_LICENSE = Apache-2.0
TAIHAO_RT_LOOP_DEPENDENCIES = host-rustc

TAIHAO_RT_LOOP_CARGO_ENV = \
	CARGO_TARGET_$(call UPPERCASE,$(RUSTC_TARGET_NAME))_LINKER=$(notdir $(TARGET_CROSS))gcc

define TAIHAO_RT_LOOP_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/rt-loop \
		$(TARGET_DIR)/usr/bin/rt-loop
	$(INSTALL) -D -m 0644 $(@D)/rt-loop.toml.example \
		$(TARGET_DIR)/etc/taihao-os/rt-loop.toml
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_TAIHAO_PATH)/package/taihao-rt-loop/rt-loop.service \
		$(TARGET_DIR)/usr/lib/systemd/system/rt-loop.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/rt-loop.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/rt-loop.service
	mkdir -p $(TARGET_DIR)/var/lib/taihao-os/rt-loop
endef

$(eval $(cargo-package))
