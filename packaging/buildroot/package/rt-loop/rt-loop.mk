################################################################################
# rt-loop — 太昊 OS 执行层回路
################################################################################

RT_LOOP_VERSION = 0.1.0
RT_LOOP_SITE = $(BR2_EXTERNAL_TAIHAO_PATH)/../../src
RT_LOOP_SITE_METHOD = local


RT_LOOP_CARGO_TARGET_DIR = $(BUILD_DIR)/taihao-cargo-target

RT_LOOP_CARGO_ENV = \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(TARGET_CC)" \
	RUSTFLAGS="-C linker=$(TARGET_CC)"

define RT_LOOP_BUILD_CMDS
	cd $(@D) && \
	$(RT_LOOP_CARGO_ENV) \
	/usr/bin/cargo build --release \
		--target aarch64-unknown-linux-gnu \
		--target-dir $(RT_LOOP_CARGO_TARGET_DIR) \
		-p rt-loop
endef

define RT_LOOP_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(RT_LOOP_CARGO_TARGET_DIR)/aarch64-unknown-linux-gnu/release/rt-loop \
		$(TARGET_DIR)/usr/bin/rt-loop
	$(INSTALL) -D -m 0644 \
		$(BR2_EXTERNAL_TAIHAO_PATH)/../../src/systemd/rt-loop.service \
		$(TARGET_DIR)/usr/lib/systemd/system/rt-loop.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/rt-loop.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/rt-loop.service
endef

$(eval $(generic-package))