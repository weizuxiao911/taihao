################################################################################
#
# taihao-pi-agent
#
################################################################################

# 源码位于仓库根 src/pi-agent（BR2_EXTERNAL_TAIHAO_PATH = <仓库>/packaging/buildroot）
TAIHAO_PI_AGENT_VERSION = 0.1.0
TAIHAO_PI_AGENT_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src/pi-agent
TAIHAO_PI_AGENT_SITE_METHOD = local
TAIHAO_PI_AGENT_LICENSE = Apache-2.0
TAIHAO_PI_AGENT_DEPENDENCIES = host-rustc

TAIHAO_PI_AGENT_CARGO_ENV = \
	CARGO_TARGET_$(call UPPERCASE,$(RUSTC_TARGET_NAME))_LINKER=$(notdir $(TARGET_CROSS))gcc

define TAIHAO_PI_AGENT_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 $(@D)/target/$(RUSTC_TARGET_NAME)/release/pi-agent \
		$(TARGET_DIR)/usr/bin/pi-agent
	$(INSTALL) -D -m 0644 $(@D)/pi-agent.toml.example \
		$(TARGET_DIR)/etc/taihao-os/pi-agent.toml
	$(INSTALL) -D -m 0644 $(BR2_EXTERNAL_TAIHAO_PATH)/package/taihao-pi-agent/pi-agent.service \
		$(TARGET_DIR)/usr/lib/systemd/system/pi-agent.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/pi-agent.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/pi-agent.service
	mkdir -p $(TARGET_DIR)/var/lib/taihao-os/pi-agent
endef

$(eval $(cargo-package))
