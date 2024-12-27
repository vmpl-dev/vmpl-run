# vmpl-run

A tool to run programs with VMPLs (Virtual Machine Privilege Levels) support.

## Features

- Run programs with VMPL support
- Enable VMPL for processes and threads
- Support for hotcalls optimization
- Debug mode for troubleshooting
- User mode execution support

## Usage

```
# 基本运行
vmpl-run -r ./my_program

# 启用VMPL
vmpl-run -r -v ./my_program

# 启用VMPL进程支持
vmpl-run -r -v -p ./my_program

# 启用VMPL线程支持
vmpl-run -r -v -t ./my_program

# 在用户模式下运行
vmpl-run -r -v -u ./my_program

# 启用所有功能
vmpl-run -r -v -p -t -u -h -d ./my_program
```

## Installation

```bash
# Install from deb package
sudo dpkg -i vmpl-run_0.1.0_amd64.deb
```

然后安装 cargo-deb 并构建 deb 包：
```bash:vmpl-run/build-deb.sh
# 安装 cargo-deb（如果尚未安装）
cargo install cargo-deb
# 构建发布版本
cargo build --release
# 创建 deb 包
cargo deb
```

然后安装 deb 包：

```bash
sudo dpkg -i target/debian/vmpl-run_0.1.0_amd64.deb
```
