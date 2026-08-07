################################################################################
# hal-gateway — 太昊 OS 硬件访问网关
################################################################################

HAL_GATEWAY_VERSION = 0.1.0
HAL_GATEWAY_SITE = $(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src
HAL_GATEWAY_SITE_METHOD = local


HAL_GATEWAY_CARGO_TARGET_DIR = $(BUILD_DIR)/taihao-cargo-target

HAL_GATEWAY_CARGO_ENV = \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(TARGET_CC)" \
	RUSTFLAGS="-C linker=$(TARGET_CC)"

define HAL_GATEWAY_BUILD_CMDS
	cd $(@D) && \
	$(HAL_GATEWAY_CARGO_ENV) \
	/usr/bin/cargo build --release \
		--target aarch64-unknown-linux-gnu \
		--target-dir $(HAL_GATEWAY_CARGO_TARGET_DIR) \
		-p hal-gateway
endef

define HAL_GATEWAY_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(HAL_GATEWAY_CARGO_TARGET_DIR)/aarch64-unknown-linux-gnu/release/hal-gateway \
		$(TARGET_DIR)/usr/bin/hal-gateway
	$(INSTALL) -D -m 0644 \
		$(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src/systemd/hal-gateway.service \
		$(TARGET_DIR)/usr/lib/systemd/system/hal-gateway.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/hal-gateway.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/hal-gateway.service
endef

$(eval $(generic-package))