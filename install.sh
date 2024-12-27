#!/bin/bash

# 默认安装路径
INSTALL_PATH=${VMPL_INSTALL_PATH:-/usr/local}

# 编译 vmpl-run
cargo build --release

# 创建必要的目录
sudo mkdir -p $INSTALL_PATH/bin
sudo mkdir -p $INSTALL_PATH/lib

# 安装二进制文件
sudo cp target/release/vmpl-run $INSTALL_PATH/bin/

# 安装库文件
sudo cp ../apps/basic/libzphook_basic.so $INSTALL_PATH/lib/
sudo cp ../libdunify.so $INSTALL_PATH/lib/
sudo cp ../libzpoline.so $INSTALL_PATH/lib/
sudo cp ../vmpl-dev/hook/libvmpl_hook.so $INSTALL_PATH/lib/

# 设置权限
sudo chmod 755 $INSTALL_PATH/bin/vmpl-run
sudo chmod 644 $INSTALL_PATH/lib/lib*.so

# 更新动态链接器缓存
sudo ldconfig

echo "Installation completed. You can now use 'vmpl-run' command." 