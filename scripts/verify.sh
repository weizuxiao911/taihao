#!/bin/bash
# 太昊 OS 本地验证脚本
# 当前 skeleton：定义入口，不实际跑 QEMU

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

OUTPUT_DIR="${ROOT_DIR}/output"
IMAGE="${OUTPUT_DIR}/buildroot/images/rootfs.ext4"

usage() {
    cat <<EOF
用法: $0 [命令]

命令:
    build    跑 build.sh build
    boot     启动 QEMU 验证镜像（amd64）
    boot-aa  启动 QEMU 验证镜像（aarch64）
    all      build + boot
    help     显示帮助

EOF
}

cmd_build() {
    "${ROOT_DIR}/packaging/build.sh" build
}

cmd_boot() {
    if [[ ! -f "${IMAGE}" ]]; then
        echo "镜像不存在: ${IMAGE}"
        echo "先跑 '$0 build'"
        exit 1
    fi
    qemu-system-x86_64 \
        -M pc \
        -m 1G \
        -kernel "${IMAGE}" \
        -append "root=/dev/sda console=ttyS0" \
        -nographic \
        -serial mon:stdio
}

cmd_boot_aa() {
    if [[ ! -f "${IMAGE}" ]]; then
        echo "镜像不存在: ${IMAGE}"
        echo "先跑 '$0 build'"
        exit 1
    fi
    qemu-system-aarch64 \
        -M virt \
        -cpu cortex-a76 \
        -m 1G \
        -kernel "${IMAGE}" \
        -append "root=/dev/vda console=ttyAMA0" \
        -nographic \
        -serial mon:stdio
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
    help|--help|-h) usage ;;
    *) usage; exit 1 ;;
esac
