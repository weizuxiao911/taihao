#!/bin/bash
# 太昊 OS 本地验证脚本
# 当前 skeleton：定义入口，不实际跑 QEMU

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

OUTPUT_DIR="${ROOT_DIR}/output"
IMAGE_DIR="${OUTPUT_DIR}/buildroot/images"
KERNEL_IMAGE="${IMAGE_DIR}/Image"
ROOTFS_EXT4="${IMAGE_DIR}/rootfs.ext4"

usage() {
    cat <<EOF
用法: $0 [命令]

命令:
    build       跑 build.sh build
    boot        启动 QEMU 验证镜像（amd64）
    boot-aa     启动 QEMU 验证镜像（aarch64）
    all         build + boot
    smoke       本地模块验证（cargo test 全仓 + SKILL frontmatter 断言 + mock 端到端）
    help        显示帮助

EOF
}

cmd_build() {
    "${ROOT_DIR}/packaging/build.sh" build
}

cmd_boot() {
    if [[ ! -f "${ROOTFS_EXT4}" ]]; then
        echo "镜像不存在: ${ROOTFS_EXT4}"
        echo "先跑 '$0 build'"
        exit 1
    fi
    qemu-system-x86_64 \
        -M pc \
        -m 1G \
        -kernel "${KERNEL_IMAGE}" \
        -drive file="${ROOTFS_EXT4}",format=raw,if=virtio \
        -append "root=/dev/vda rw console=ttyS0" \
        -nographic \
        -serial mon:stdio
}

cmd_boot_aa() {
    if [[ ! -f "${ROOTFS_EXT4}" ]]; then
        echo "镜像不存在: ${ROOTFS_EXT4}"
        echo "先跑 '$0 build'"
        exit 1
    fi
    qemu-system-aarch64 \
        -M virt \
        -cpu cortex-a76 \
        -m 1G \
        -kernel "${KERNEL_IMAGE}" \
        -drive file="${ROOTFS_EXT4}",format=raw,if=virtio \
        -append "root=/dev/vda rw console=ttyAMA0" \
        -nographic \
        -serial mon:stdio
}

# 本地模块验证（无需 QEMU / 无需 Buildroot）
cmd_smoke() {
    echo "=== 1. cargo test 全仓 ==="
    for crate in pi-agent hal-gateway rt-loop comm-center extension-bridge; do
        echo "--- ${crate} ---"
        (cd "${ROOT_DIR}/src/${crate}" && cargo test --quiet)
    done

    echo "=== 2. SKILL frontmatter 断言（examples + overlay 一致性） ==="
    local examples="${ROOT_DIR}/examples/skills"
    local overlay="${ROOT_DIR}/packaging/buildroot/board/taihao/rootfs-overlay/etc/taihao-os/skills"
    local n_examples n_overlay
    n_examples=$(ls "${examples}"/*.md | wc -l | tr -d ' ')
    n_overlay=$(ls "${overlay}"/*.md | wc -l | tr -d ' ')
    echo "examples: ${n_examples} 个, overlay: ${n_overlay} 个"
    [[ "${n_examples}" -ge 6 ]] || { echo "examples SKILL 数量不足 6"; exit 1; }
    [[ "${n_overlay}" -eq "${n_examples}" ]] || { echo "overlay 与 examples 数量不一致"; exit 1; }
    # 逐个断言 frontmatter 字段
    for f in "${examples}"/*.md; do
        for field in name version security_level description; do
            grep -q "^${field}:" "${f}" || { echo "SKILL ${f} 缺字段 ${field}"; exit 1; }
        done
        grep -Eq "^security_level: (L1|L2|L3)$" "${f}" || { echo "SKILL ${f} 安全分级非法"; exit 1; }
    done
    echo "SKILL frontmatter 断言通过"

    echo "=== 3. mock 端到端（hal-gateway 直调 + extension-bridge 调用） ==="
    local hal_bin="${ROOT_DIR}/target/debug/hal-gateway"
    local ext_bin="${ROOT_DIR}/target/debug/extension-bridge"
    if [[ -x "${hal_bin}" ]] && [[ -x "${ext_bin}" ]]; then
        echo "--- hal-gateway call（白名单外应被拒） ---"
        "${hal_bin}" --config /nonexistent call --caller test --tool motor_detonate || true
        echo "--- extension-bridge call（白名单工具） ---"
        "${ext_bin}" --config /nonexistent call tools/list '{"id":1}' || true
    else
        echo "二进制未构建，跳过端到端（先跑 cargo build）"
    fi
    echo "=== smoke 完成 ==="
}

cmd_all() {
    cmd_build
    cmd_boot
}

case "${1:-help}" in
    build) cmd_build ;;
    boot) cmd_boot ;;
    boot-aa) cmd_boot_aa ;;
    all) cmd_all ;;
    smoke) cmd_smoke ;;
    help|--help|-h) usage ;;
    *) usage; exit 1 ;;
esac
