################################################################################
# pi-agent — 太昊 OS 认知决策层服务
# 源码:仓库 src/(workspace),构建 pi-agent crate
################################################################################

PI_AGENT_VERSION = 0.1.0
PI_AGENT_SITE = $(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src
PI_AGENT_SITE_METHOD = local


# 共享 cargo target 目录,避免 5 包重复编译
PI_AGENT_CARGO_TARGET_DIR = $(BUILD_DIR)/taihao-cargo-target

# 交叉编译:rustc 目标 + linker 指向 Buildroot 工具链
PI_AGENT_RUSTFLAGS = \
	-C linker=$(TARGET_CC)

PI_AGENT_CARGO_ENV = \
	CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$(TARGET_CC)" \
	RUSTFLAGS="$(PI_AGENT_RUSTFLAGS)"

define PI_AGENT_BUILD_CMDS
	cd $(@D) && \
	$(PI_AGENT_CARGO_ENV) \
	/usr/bin/cargo build --release \
		--target aarch64-unknown-linux-gnu \
		--target-dir $(PI_AGENT_CARGO_TARGET_DIR) \
		-p pi-agent
endef

define PI_AGENT_INSTALL_TARGET_CMDS
	$(INSTALL) -D -m 0755 \
		$(PI_AGENT_CARGO_TARGET_DIR)/aarch64-unknown-linux-gnu/release/pi-agent \
		$(TARGET_DIR)/usr/bin/pi-agent
	$(INSTALL) -D -m 0644 \
		$(BR2_EXTERNAL_TAIHAO_PI4B_PATH)/../../src/systemd/pi-agent.service \
		$(TARGET_DIR)/usr/lib/systemd/system/pi-agent.service
	mkdir -p $(TARGET_DIR)/etc/systemd/system/multi-user.target.wants
	ln -sf /usr/lib/systemd/system/pi-agent.service \
		$(TARGET_DIR)/etc/systemd/system/multi-user.target.wants/pi-agent.service
endef

$(eval $(generic-package))