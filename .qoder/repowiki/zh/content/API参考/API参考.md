<cite>
**本文档中引用的文件**
- [vcpu.rs](file://src/vcpu.rs)
- [arch_vcpu.rs](file://src/arch_vcpu.rs)
- [exit.rs](file://src/exit.rs)
- [lib.rs](file://src/lib.rs)
- [Cargo.toml](file://Cargo.toml)
</cite>

# API参考

## 目录
1. [AxVCpu结构体公共方法](#axvcpu结构体公共方法)
2. [AxArchVCpu trait契约规范](#axarchvcpu-trait契约规范)
3. [AxVCpuExitReason枚举变体](#axvcpuexitreason枚举变体)

## AxVCpu结构体公共方法

### new() 方法
创建一个新的虚拟CPU实例。

**签名**
```rust
pub fn new(
    vm_id: VMId,
    vcpu_id: VCpuId,
    favor_phys_cpu: usize,
    phys_cpu_set: Option<usize>,
    arch_config: A::CreateConfig,
) -> AxResult<Self>
```

**参数说明**
- `vm_id`: 所属虚拟机的唯一标识符
- `vcpu_id`: 该虚拟CPU在虚拟机内的唯一标识符
- `favor_phys_cpu`: 优先运行此虚拟CPU的物理CPU ID，用于CPU亲和性优化
- `phys_cpu_set`: 可选的允许运行的物理CPU位掩码（`None`表示无限制）
- `arch_config`: 架构特定的虚拟CPU创建配置

**返回值解释**
成功时返回 `Ok(AxVCpu)`，若架构特定创建失败则返回错误。

**可能抛出的错误类型**
- 架构特定实现可能返回的各种错误

**使用示例**
```rust
let vcpu = AxVCpu::new(vm_id, vcpu_id, 0, None, arch_config)?;
```

**生命周期约束**
创建的虚拟CPU实例遵循严格的状态机：Created → Free → Ready → Running。

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L85-L114)

### setup() 方法
为虚拟CPU执行设置操作。

**签名**
```rust
pub fn setup(
    &self,
    entry: GuestPhysAddr,
    ept_root: HostPhysAddr,
    arch_config: A::SetupConfig,
) -> AxResult
```

**参数说明**
- `entry`: 客户机入口地址（客户机物理地址）
- `ept_root`: 扩展页表（EPT）根地址（主机物理地址）
- `arch_config`: 架构特定的设置配置

**返回值解释**
成功时返回 `Ok(())`，失败时返回错误。

**可能抛出的错误类型**
- `BadState`: 当前状态不是 `Created` 状态
- 架构特定实现可能返回的各种错误

**使用示例**
```rust
vcpu.setup(entry_addr, ept_root_addr, setup_config)?;
```

**不安全操作**
此方法本身是安全的，但要求调用者确保传入的地址有效。

**生命周期约束**
此方法将虚拟CPU从 `Created` 状态转换到 `Free` 状态。

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L116-L137)

### bind() 方法
将虚拟CPU绑定到当前物理CPU。

**签名**
```rust
pub fn bind(&self) -> AxResult
```

**参数说明**
无参数。

**返回值解释**
成功时返回 `Ok(())`，失败时返回错误。

**可能抛出的错误类型**
- `BadState`: 当前状态不是 `Free` 状态
- 架构特定实现可能返回的各种错误

**使用示例**
```rust
vcpu.bind()?;
```

**不安全操作**
此方法本身是安全的，但要求调用者确保在正确的上下文中调用。

**生命周期约束**
此方法将虚拟CPU从 `Free` 状态转换到 `Ready` 状态。

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L290-L298)

### run() 方法
执行虚拟CPU直到发生VM退出。

**签名**
```rust
pub fn run(&self) -> AxResult<AxVCpuExitReason>
```

**参数说明**
无参数。

**返回值解释**
成功时返回 `Ok(AxVCpuExitReason)`，包含导致VM退出的原因；失败时返回错误。

**可能抛出的错误类型**
- `BadState`: 当前状态不是 `Ready` 状态
- 架构特定实现可能返回的各种错误

**使用示例**
```rust
match vcpu.run() {
    Ok(exit_reason) => handle_exit(exit_reason),
    Err(e) => log_error(e),
}
```

**不安全操作**
此方法本身是安全的，但在执行期间会转移控制权给客户机代码。

**生命周期约束**
此方法将虚拟CPU从 `Ready` 状态转换到 `Running` 状态，执行完成后返回到 `Ready` 状态。

**Section sources**
- [vcpu.rs](file://src/vcpu.rs#L279-L288)

## AxArchVCpu trait契约规范

### 关联类型 CreateConfig
架构特定的虚拟CPU创建配置。

**用途**
允许每个架构定义其自身在初始化期间所需的配置参数。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L12-L16)

### 关联类型 SetupConfig
架构特定的虚拟CPU设置配置。

**用途**
允许每个架构指定在基本创建之后、执行之前需要的额外配置参数。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L19-L23)

### new() 方法
创建新的架构特定虚拟CPU实例。

**签名**
```rust
fn new(vm_id: VMId, vcpu_id: VCpuId, config: Self::CreateConfig) -> AxResult<Self>
```

**调用约定**
由 `AxVCpu::new()` 调用，在虚拟CPU创建期间执行。

**预期行为**
返回一个新创建的架构特定虚拟CPU实例。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L25-L27)

### set_entry() 方法
设置客户机入口点。

**签名**
```rust
fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult
```

**调用约定**
保证只被调用一次，且在 `setup()` 调用之前。

**预期行为**
配置虚拟CPU的执行起始地址。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L29-L31)

### set_ept_root() 方法
设置扩展页表（EPT）根。

**签名**
```rust
fn set_ept_root(&mut self, ept_root: HostPhysAddr) -> AxResult
```

**调用约定**
在 `setup()` 调用之前调用。

**预期行为**
设置用于客户机到主机物理地址转换的顶级页表。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L34-L38)

### setup() 方法
完成虚拟CPU初始化并准备执行。

**签名**
```rust
fn setup(&mut self, config: Self::SetupConfig) -> AxResult
```

**调用约定**
在 `set_entry()` 和 `set_ept_root()` 之后调用。

**预期行为**
执行任何最终的架构特定设置，使虚拟CPU准备好执行。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L40-L43)

### run() 方法
执行虚拟CPU直到VM退出。

**签名**
```rust
fn run(&mut self) -> AxResult<AxVCpuExitReason>
```

**调用约定**
在虚拟CPU处于 `Running` 状态时调用。

**预期行为**
转移控制权给客户机虚拟CPU，运行直到触发需要虚拟机监控器干预的VM退出条件。

**Section sources**
- [arch_vcpu.rs](file://src/arch_vcpu.rs#L45-L49)

## AxVCpuExitReason枚举变体

### Halt
虚拟CPU已执行停机指令并进入空闲状态。

**触发条件**
当客户机操作系统没有工作可做并等待中断或其他事件唤醒时。

**携带的数据字段**
无数据字段。

**Section sources**
- [exit.rs](file://src/exit.rs#L218-L220)

### Io
虚拟CPU执行了I/O操作。

**触发条件**
当客户机访问设备寄存器或其他需要虚拟机监控器模拟的硬件映射内存区域时。

**携带的数据字段**
- `IoRead`: 包含端口号和访问宽度
- `IoWrite`: 包含端口号、访问宽度和写入数据

**Section sources**
- [exit.rs](file://src/exit.rs#L157-L182)

### Interrupt
外部中断被传递给虚拟CPU。

**触发条件**
当来自外部设备的硬件中断需要由客户机或虚拟机监控器处理时。

**携带的数据字段**
- `ExternalInterrupt`: 包含硬件中断向量号

**Section sources**
- [exit.rs](file://src/exit.rs#L138-L145)

### Hypercall
客户机指令触发了对虚拟机监控器的超级调用。

**触发条件**
当客户机操作系统请求虚拟机监控器服务时，类似于传统操作系统中的系统调用。

**携带的数据字段**
- `nr`: 标识请求服务的超级调用编号
- `args`: 传递给超级调用的参数（最多6个）

**Section sources**
- [exit.rs](file://src/exit.rs#L25-L33)

### MmioRead
客户机执行了内存映射I/O读取操作。

**触发条件**
当客户机访问设备寄存器或其他需要虚拟机监控器模拟的硬件映射内存区域时。

**携带的数据字段**
- `addr`: 正在读取的客户机物理地址
- `width`: 内存访问的宽度/大小
- `reg`: 将接收读取值的客户机寄存器索引
- `reg_width`: 目标寄存器的宽度
- `signed_ext`: 是否将读取值符号扩展以填充寄存器

**Section sources**
- [exit.rs](file://src/exit.rs#L35-L50)

### MmioWrite
客户机执行了内存映射I/O写入操作。

**触发条件**
当客户机写入设备寄存器或其他需要虚拟机监控器模拟的硬件映射内存区域时。

**携带的数据字段**
- `addr`: 正在写入的客户机物理地址
- `width`: 内存访问的宽度/大小
- `data`: 正在写入内存位置的数据

**Section sources**
- [exit.rs](file://src/exit.rs#L52-L63)

### SysRegRead
客户机执行了系统寄存器读取操作。

**触发条件**
当客户机读取架构特定的控制和状态寄存器时。

**携带的数据字段**
- `addr`: 正在读取的系统寄存器地址/标识符
- `reg`: 将接收读取值的客户机寄存器索引

**Section sources**
- [exit.rs](file://src/exit.rs#L65-L81)

### SysRegWrite
客户机执行了系统寄存器写入操作。

**触发条件**
当客户机写入架构特定的控制和状态寄存器时。

**携带的数据字段**
- `addr`: 正在写入的系统寄存器地址/标识符
- `value`: 正在写入系统寄存器的数据

**Section sources**
- [exit.rs](file://src/exit.rs#L83-L97)

### NestedPageFault
在客户机内存访问期间发生嵌套页错误。

**触发条件**
当客户机访问未映射的内存区域或违反访问权限时。

**携带的数据字段**
- `addr`: 导致故障的客户机物理地址
- `access_flags`: 尝试的访问类型（读/写/执行）

**Section sources**
- [exit.rs](file://src/exit.rs#L118-L130)

### CpuUp
请求启动辅助CPU核心。

**触发条件**
在多核虚拟机引导过程中，当主CPU请求启动辅助CPU时。

**携带的数据字段**
- `target_cpu`: 要启动的目标CPU标识符
- `entry_point`: 辅助CPU应开始执行的客户机物理地址
- `arg`: 传递给辅助CPU的参数

**Section sources**
- [exit.rs](file://src/exit.rs#L184-L202)

### CpuDown
虚拟CPU已被关闭。

**触发条件**
当虚拟CPU执行了关机指令或超级调用并且应该被暂停时。

**携带的数据字段**
- `_state`: 电源状态信息（当前未使用）

**Section sources**
- [exit.rs](file://src/exit.rs#L204-L210)

### SystemDown
客户机请求系统范围关机。

**触发条件**
当整个虚拟机应该被关闭时，而不仅仅是当前虚拟CPU。

**携带的数据字段**
无数据字段。

**Section sources**
- [exit.rs](file://src/exit.rs#L212-L216)

### Nothing
无需特殊处理。

**触发条件**
虚拟CPU内部处理了退出，提供机会让虚拟机监控器检查虚拟设备状态、处理待处理中断等。

**携带的数据字段**
无数据字段。

**Section sources**
- [exit.rs](file://src/exit.rs#L222-L232)

### FailEntry
VM条目因无效虚拟CPU状态或配置而失败。

**触发条件**
当硬件虚拟化层无法成功进入客户机时。

**携带的数据字段**
- `hardware_entry_failure_reason`: 硬件特定的失败原因代码

**Section sources**
- [exit.rs](file://src/exit.rs#L234-L243)

### SendIPI
客户机正在尝试发送处理器间中断（IPI）。

**触发条件**
在多核系统中用于处理器间通信。

**携带的数据字段**
- `target_cpu`: 接收IPI的目标CPU标识符
- `target_cpu_aux`: 复杂目标CPU规范的辅助字段
- `send_to_all`: 是否向除发送者外的所有CPU广播IPI
- `send_to_self`: 是否向当前CPU发送IPI（自IPI）
- `vector`: 要传递的IPI向量/中断号

**Section sources**
- [exit.rs](file://src/exit.rs#L245-L259)