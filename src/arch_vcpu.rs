use page_table_multiarch::{MappingFlags, PageSize};

use axaddrspace::{GuestPhysAddr, GuestVirtAddr, HostPhysAddr};
use axerrno::AxResult;

use crate::exit::AxVCpuExitReason;

/// A trait for architecture-specific vcpu.
///
/// This trait is an abstraction for virtual CPUs of different architectures.
pub trait AxArchVCpu: Sized + AxVcpuAccessGuestState {
    /// The configuration for creating a new [`AxArchVCpu`]. Used by [`AxArchVCpu::new`].
    type CreateConfig;
    /// The configuration for setting up a created [`AxArchVCpu`]. Used by [`AxArchVCpu::setup`].
    type SetupConfig;
    /// The configuration for setting up a new [`AxArchVCpu`] for host VM. Used by [`AxArchVCpu::setup_from_context`] in type 1.5 scenario.
    type HostContext;

    /// Create a new `AxArchVCpu`.
    fn new(config: Self::CreateConfig) -> AxResult<Self>;

    /// Load current vcpu state into a pre-constructed `HostContext` structure.
    fn load_context(&self, config: &mut Self::HostContext) -> AxResult;

    /// Set the entry point of the vcpu.
    ///
    /// It's guaranteed that this function is called only once, before [`AxArchVCpu::setup`] being called.
    fn set_entry(&mut self, entry: GuestPhysAddr) -> AxResult;

    /// Set the EPT root of the vcpu.
    ///
    /// It's guaranteed that this function is called only once, before [`AxArchVCpu::setup`] being called.
    fn set_ept_root(&mut self, ept_root: HostPhysAddr) -> AxResult;

    /// Setup the vcpu.
    ///
    /// It's guaranteed that this function is called only once, after [`AxArchVCpu::set_entry`] and [`AxArchVCpu::set_ept_root`] being called.
    fn setup(&mut self, config: Self::SetupConfig) -> AxResult;

    /// Setup the vcpu from a pre-constructed `HostContext` structure.
    fn setup_from_context(&mut self, config: Self::HostContext) -> AxResult;

    /// Run the vcpu until a vm-exit occurs.
    fn run(&mut self) -> AxResult<AxVCpuExitReason>;

    /// Bind the vcpu to the current physical CPU.
    fn bind(&mut self) -> AxResult;

    /// Unbind the vcpu from the current physical CPU.
    fn unbind(&mut self) -> AxResult;

    /// Set the value of a general-purpose register according to the given index.
    fn set_gpr(&mut self, reg: usize, val: usize);
}

pub trait AxVcpuAccessGuestState {
    /// The type of the general-purpose registers.
    /// This type should be a struct that contains the general-purpose registers of the architecture.
    /// TODO: maybe we can seperate this into a independent crate.
    type GeneralRegisters;

    fn regs(&self) -> &Self::GeneralRegisters;
    fn regs_mut(&mut self) -> &mut Self::GeneralRegisters;

    fn read_gpr(&self, reg: usize) -> usize;
    fn write_gpr(&mut self, reg: usize, val: usize);

    fn instr_pointer(&self) -> usize;
    fn set_instr_pointer(&mut self, val: usize);

    fn stack_pointer(&self) -> usize;
    fn set_stack_pointer(&mut self, val: usize);

    fn frame_pointer(&self) -> usize;
    fn set_frame_pointer(&mut self, val: usize);

    fn return_value(&self) -> usize;
    fn set_return_value(&mut self, val: usize);

    fn guest_is_privileged(&self) -> bool;
    fn guest_page_table_query(
        &self,
        gva: GuestVirtAddr,
    ) -> Option<(GuestPhysAddr, MappingFlags, PageSize)>;

    fn append_eptp_list(&mut self, idx: usize, eptp: HostPhysAddr) -> AxResult;
    fn remove_eptp_list_entry(&mut self, idx: usize) -> AxResult;
    fn get_eptp_list_entry(&self, idx: usize) -> AxResult<HostPhysAddr>;
}
