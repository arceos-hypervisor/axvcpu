[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/arceos-hypervisor/axvcpu)

# AxVCpu

The crate implements a virtualization abstraction that enables hypervisors to manage virtual CPUs across different hardware architectures. It provides a unified interface for VCPU creation, execution control, exit handling, and resource management while abstracting away architecture-specific implementation details.

# APIS

```
// Setup the vcpu.
pub fn setup(
        &self,
        entry: GuestPhysAddr,
        ept_root: HostPhysAddr,
        arch_config: A::SetupConfig,
    ) -> AxResult
```

```
// Get the set of physical CPUs who can run this vcpu.
// If `None`, this vcpu has no limitation and can be scheduled on any physical CPU.
pub const fn phys_cpu_set(&self) -> Option<usize>
```

```
// Get whether the vcpu is the BSP. We always assume the first vcpu (vcpu with id #0) is the BSP.
pub const fn is_bsp(&self) -> bool 
```

```
// Set the state of the vcpu.
pub unsafe fn set_state(&self, state: VCpuState) 
```

```
// Run the vcpu.
pub fn run(&self) -> AxResult<AxVCpuExitReason> 
```

```
// Bind the vcpu to the current physical CPU.
pub fn bind(&self) -> AxResult 
```

```
// Unbind the vcpu from the current physical CPU.
pub fn unbind(&self) -> AxResult 
```

```
// Sets the entry address of the vcpu.
pub fn set_entry(&self, entry: GuestPhysAddr) -> AxResult
```

```
// Sets the value of a general-purpose register according to the given index.
pub fn set_gpr(&self, reg: usize, val: usize) 
```

# Examples

### Implementation for `AxVCpuHal` trait.

```
impl AxVCpuHal for AxVCpuHalImpl {
    fn alloc_frame() -> Option<HostPhysAddr> {
        <AxVMHalImpl as AxVMHal>::PagingHandler::alloc_frame()
    }

    fn dealloc_frame(paddr: HostPhysAddr) {
        <AxVMHalImpl as AxVMHal>::PagingHandler::dealloc_frame(paddr)
    }

    #[inline]
    fn phys_to_virt(paddr: HostPhysAddr) -> HostVirtAddr {
        <AxVMHalImpl as AxVMHal>::PagingHandler::phys_to_virt(paddr)
    }

    fn virt_to_phys(vaddr: axaddrspace::HostVirtAddr) -> axaddrspace::HostPhysAddr {
        std::os::arceos::modules::axhal::mem::virt_to_phys(vaddr)
    }

    #[cfg(target_arch = "aarch64")]
    fn irq_fetch() -> usize {
        axhal::irq::fetch_irq()
    }

    #[cfg(target_arch = "aarch64")]
    fn irq_hanlder() {
        let irq_num = axhal::irq::fetch_irq();
        debug!("IRQ handler {irq_num}");
        axhal::irq::handler_irq(irq_num);
    }
}
```

### Boot target vCPU on the specified VM.

```
fn vcpu_on(vm: VMRef, vcpu_id: usize, entry_point: GuestPhysAddr, arg: usize) {
    let vcpu = vm.vcpu_list()[vcpu_id].clone();

    vcpu.set_entry(entry_point)
        .expect("vcpu_on: set_entry failed");
    vcpu.set_gpr(0, arg);

    ...
}

```

### Run a vCPU according to the given vcpu_id in VM.

```
pub fn run_vcpu(&self, vcpu_id: usize) -> AxResult<AxVCpuExitReason> {
        let vcpu = self
            .vcpu(vcpu_id)
            .ok_or_else(|| ax_err_type!(InvalidInput, "Invalid vcpu_id"))?;

        vcpu.bind()?;

        let exit_reason = loop {
            let exit_reason = vcpu.run()?;
            trace!("{exit_reason:#x?}");
            let handled = match &exit_reason {
                AxVCpuExitReason::MmioRead {
                    addr,
                    width,
                    reg,
                    reg_width: _,
                } => {
                    let val = self
                        .get_devices()
                        .handle_mmio_read(*addr, (*width).into())?;
                    vcpu.set_gpr(*reg, val);
                    true
                }
                AxVCpuExitReason::MmioWrite { addr, width, data } => {
                    self.get_devices()
                        .handle_mmio_write(*addr, (*width).into(), *data as usize);
                    true
                }
                AxVCpuExitReason::IoRead { port: _, width: _ } => true,
                AxVCpuExitReason::IoWrite {
                    port: _,
                    width: _,
                    data: _,
                } => true,
                AxVCpuExitReason::NestedPageFault { addr, access_flags } => self
                    .inner_mut
                    .address_space
                    .lock()
                    .handle_page_fault(*addr, *access_flags),
                _ => false,
            };
            if !handled {
                break exit_reason;
            }
        };

        vcpu.unbind()?;
        Ok(exit_reason)
    }
```

More detailed usage in [Axvisor](https://github.com/arceos-hypervisor/axvisor), [AXVM](https://github.com/arceos-hypervisor/axvm).

