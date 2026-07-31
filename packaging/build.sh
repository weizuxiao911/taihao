#!/bin/bash
# 太昊 OS 构建入口
# 当前 skeleton：定义入口，不实际跑 Buildroot

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

BUILDROOT_DEFCONFIG="${ROOT_DIR}/packaging/buildroot/configs/taihao_defconfig"
BUILDROOT_DIR="${BUILDROOT_DIR:-${HOME}/buildroot}"
OUTPUT_DIR="${ROOT_DIR}/output"

usage() {
    cat <<EOF
用法: $0 [命令]

命令:
    fetch   拉取 Buildroot 源码（若本地不存在）
    build   跑 Buildroot 构建（需先 fetch）
    clean   清理 output/
    help    显示本帮助

环境变量:
    BUILDROOT_DIR  Buildroot 源码目录（默认: \$HOME/buildroot）
    JOBS           并行编译数（默认: \$(nproc)）

EOF
}

cmd_fetch() {
    if [[ -d "${BUILDROOT_DIR}" ]]; then
        echo "Buildroot 已存在: ${BUILDROOT_DIR}"
        return 0
    fi
    echo "克隆 Buildroot (2024.05 LTS):"
    git clone --depth=1 --branch=2024.05 \
        https://github.com/buildroot/buildroot.git "${BUILDROOT_DIR}"
}

cmd_build() {
    if [[ ! -d "${BUILDROOT_DIR}" ]]; then
        echo "Buildroot 不存在，先跑 '$0 fetch'"
        exit 1
    fi
    if [[ ! -f "${BUILDROOT_DEFCONFIG}" ]]; then
        echo "找不到 defconfig: ${BUILDROOT_DEFCONFIG}"
        exit 1
    fi
    mkdir -p "${OUTPUT_DIR}"
    cd "${BUILDROOT_DIR}"
    make O="${OUTPUT_DIR}/buildroot" BR2_DEFCONFIG="${BUILDROOT_DEFCONFIG}" defconfig
    make O="${OUTPUT_DIR}/buildroot" -j"${JOBS:-$(nproc)}"
    echo "构建产物: ${OUTPUT_DIR}/buildroot/images/"
}

cmd_clean() {
    rm -rf "${OUTPUT_DIR}"
    echo "清理完毕: ${OUTPUT_DIR}"
}

case "${1:-help}" in
    fetch) cmd_fetch ;;
    build) cmd_build ;;
    clean) cmd_clean ;;
    help|--help|-h) usage ;;
    *) usage; exit 1 ;;
esac
