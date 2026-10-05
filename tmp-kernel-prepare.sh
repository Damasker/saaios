#!/bin/bash
set -e
ROOT="$HOME/local/debroot"
export PATH="$ROOT/usr/bin:$PATH"
export LD_LIBRARY_PATH="$ROOT/usr/lib/x86_64-linux-gnu:$ROOT/lib/x86_64-linux-gnu:${LD_LIBRARY_PATH:-}"
K=/home/mike/kernel-work/common-bd23337
cd "$K"
sed -i 's/^CONFIG_LOCALVERSION=.*/CONFIG_LOCALVERSION="-android14-11-gbd23337e42e7-ab14791245"/' .config
run_make() {
  proot -b "$ROOT/usr/bin/m4:/usr/bin/m4" \
        -b "$ROOT/usr/bin/flex:/usr/bin/flex" \
        -b "$ROOT/usr/bin/bison:/usr/bin/bison" \
        -b "$ROOT/usr/share/bison:/usr/share/bison" \
        env LD_LIBRARY_PATH="$LD_LIBRARY_PATH" "$@"
}
run_make m4 --version | head -1
run_make make ARCH=arm64 olddefconfig
echo OLDDEF_OK
run_make make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- modules_prepare -j"$(nproc)"
echo PREPARE_OK
cat include/config/kernel.release
ls -la Module.symvers
