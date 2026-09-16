#!/bin/sh
set -eu

# Builds the bionic Vulkan helper used by the static-musl saai-displayd.
# Google's Mali UMD is an Android/bionic binary; keeping it in this small
# child process avoids mixing two libc implementations in the Wayland server.

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
ndk_root=${ANDROID_NDK_ROOT:?set ANDROID_NDK_ROOT to an Android NDK}
host_tag=${ANDROID_NDK_HOST_TAG:-linux-x86_64}
api=${ANDROID_API:-30}
toolchain="$ndk_root/toolchains/llvm/prebuilt/$host_tag"
cc=${ANDROID_CLANG:-"$toolchain/bin/aarch64-linux-android${api}-clang"}
strip=${ANDROID_STRIP:-"$toolchain/bin/llvm-strip"}
output=${OUTPUT:-"$repo_root/target/aarch64-unknown-linux-musl/pixel7/saai-gpu-compositor"}

mkdir -p "$(dirname -- "$output")"
"$cc" -Wall -Wextra -Werror -O2 -fPIE -pie \
    -I"$toolchain/sysroot/usr/include/drm" \
    "$script_dir/src/gpu-compositor.c" -ldl -o "$output"
"$strip" "$output"
printf '%s\n' "$output"
