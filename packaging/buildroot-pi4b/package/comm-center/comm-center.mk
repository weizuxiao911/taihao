################################################################################
# comm-center — 太昊 OS 调度通信中心
################################################################################

COMM_CENTER_VERSION = 0.1.0
COMM_CENTER_SITE = $(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src
COMM_CENTER_SITE_METHOD = local


COMM_CENTER_CARGO_TARGET_DIR = $(BUILD_DIR)/taihao-cargo-target

COMM_CENTER_CARGO_ENV = \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(TARGET_CC)" \
	RUSTFLAGS="-C linker=$(TARGET_CC)"

define COMM_CENTER_BUILD_CMDS
	cd $(@D) && \
	$(COMM_CENTER_CARGO_ENV) \
	/usr/bin/cargo build --release \
		--target aarch64-unknown-linux-gnu \
		--target-dir $(COMM_CENTER_CARGO_TARGET_DIR) \
		-p comm-center
endef

define COMM_CENTER_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(COMM_CENTER_CARGO_TARGET_DIR)/aarch64-unknown-linux-gnu/release/comm-center \
		$(TARGET_DIR)/usr/bin/comm-center
	$(INSTALL) -D -m 0644 \
		$(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src/systemd/comm-center.service \
		$(TARGET_DIR)/usr/lib/systemd/system/comm-center.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/comm-center.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/comm-center.service
endef

$(eval $(generic-package))