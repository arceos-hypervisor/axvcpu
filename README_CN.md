<h1 align="center">axvcpu</h1>

<p align="center">面向 ArceOS Hypervisor 的虚拟 CPU 抽象</p>

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/axvcpu.svg)](https://crates.io/crates/axvcpu)
[![Docs.rs](https://docs.rs/axvcpu/badge.svg)](https://docs.rs/axvcpu)
[![Rust](https://img.shields.io/badge/edition-2024-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/arceos-hypervisor/axvcpu/blob/main/LICENSE)

</div>

[English](README.md) | 中文

# Introduction

`axvcpu` 为 ArceOS hypervisor 提供与体系结构无关的虚拟 CPU 抽象。它将统一的 VCpu 生命周期模型与可插拔的架构后端 trait 结合起来，适合在 `#![no_std]` 环境中构建面向 x86_64、AArch64 与 RISC-V 平台的 hypervisor。

该库导出四个核心公开接口：

- **`AxArchVCpu`** - 由具体架构 VCpu 后端实现的 trait
- **`AxVCpu`** - 与架构无关的主 VCpu 封装及生命周期控制器
- **`VCpuState`** - 用于 VCpu 生命周期管理的状态机
- **`AxVCpuExitReason`** - VCpu 执行返回的统一 VM-exit 原因枚举

该 crate 还重新导出了 `percpu` 模块中的每 CPU 辅助工具。

## Quick Start

### Requirements

- Rust nightly 工具链
- Rust 组件：rust-src、clippy、rustfmt

```bash
# 安装 rustup（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 nightly 工具链与所需组件
rustup install nightly
rustup component add rust-src clippy rustfmt --toolchain nightly
```

### Run Check and Test

```bash
# 1. 进入仓库目录
cd axvcpu

# 2. 代码检查
./scripts/check.sh

# 3. 运行测试
./scripts/test.sh
```

## Integration

### Installation

将以下依赖加入 `Cargo.toml`：

```toml
[dependencies]
axvcpu = "0.3.0"
```

### Example

```rust
extern crate alloc;

use alloc::vec::Vec;
use axaddrspace::{GuestPhysAddr, HostPhysAddr};
use axerrno::AxResult;
use axvcpu::{AxArchVCpu, AxVCpu, AxVCpuExitReason, VCpuState};

#[derive(Debug)]
struct MockArchVCpu {
    entry: Option<GuestPhysAddr>,
    ept_root: Option<HostPhysAddr>,
    is_setup: bool,
    is_bound: bool,
    pending_interrupts: Vec<usize>,
    return_value: usize,
}

#[derive(Debug, Clone)]
struct MockCreateConfig;

#[derive(Debug)]
struct MockSetupConfig;

impl AxArchVCpu for MockArchVCpu {
    type CreateConfig = MockCreateConfig;
    type SetupConfig = MockSetupConfig;

    fn new(_vm_id: usize, _vcpu_id: usize, _config: Self::CreateConfig) -> AxResult<Self> {
        Ok(Self {
            entry: None,
            ept_root: None,
            is_setup: false,
            is_bound: false,
            pending_interrupts: Vec::new(),
            return_value: 0,
        })
    }

    fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult {
        self.entry = Some(entry);
        Ok(())
    }

    fn set_ept_root(&mut self, ept_root: HostPhysAddr) -> AxResult {
        self.ept_root = Some(ept_root);
        Ok(())
    }

    fn setup(&mut self, _config: Self::SetupConfig) -> AxResult {
        self.is_setup = true;
        Ok(())
    }

    fn run(&mut self) -> AxResult<AxVCpuExitReason> {
        Ok(AxVCpuExitReason::Halt)
    }

    fn bind(&mut self) -> AxResult {
        self.is_bound = true;
        Ok(())
    }

    fn unbind(&mut self) -> AxResult {
        self.is_bound = false;
        Ok(())
    }

    fn set_gpr(&mut self, _reg: usize, _val: usize) {}

    fn inject_interrupt(&mut self, vector: usize) -> AxResult {
        self.pending_interrupts.push(vector);
        Ok(())
    }

    fn set_return_value(&mut self, val: usize) {
        self.return_value = val;
    }
}

fn main() {
    let vcpu = AxVCpu::<MockArchVCpu>::new(1, 0, 0, None, MockCreateConfig).unwrap();
    assert_eq!(vcpu.state(), VCpuState::Created);

    vcpu.setup(
        GuestPhysAddr::from(0x1000),
        HostPhysAddr::from(0x2000),
        MockSetupConfig,
    )
    .unwrap();
    assert_eq!(vcpu.state(), VCpuState::Free);

    vcpu.bind().unwrap();
    let exit = vcpu.run().unwrap();
    assert!(matches!(exit, AxVCpuExitReason::Halt));

    vcpu.inject_interrupt(32).unwrap();
    vcpu.set_return_value(0);
}
```

### Documentation

生成并查看 API 文档：

```bash
cargo doc --no-deps --open
```

在线文档： [docs.rs/axvcpu](https://docs.rs/axvcpu)

# Contributing

1. Fork 仓库并创建分支
2. 本地运行检查：`./scripts/check.sh`
3. 本地运行测试：`./scripts/test.sh`
4. 提交 PR 并通过 CI 检查

# License

本项目基于 Apache License 2.0 许可证发布。详见 [LICENSE](LICENSE)。
