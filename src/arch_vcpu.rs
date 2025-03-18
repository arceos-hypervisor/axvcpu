use axaddrspace::{GuestPhysAddr, HostPhysAddr};
use axerrno::AxResult;

use crate::exit::AxVCpuExitReason;

/// A trait for architecture-specific vcpu.
///
/// This trait is an abstraction for virtual CPUs of different architectures.
pub trait AxArchVCpu: Sized {
    /// The configuration for creating a new [`AxArchVCpu`]. Used by [`AxArchVCpu::new`].
    type CreateConfig;
    /// The configuration for setting up a created [`AxArchVCpu`]. Used by [`AxArchVCpu::setup`].
    type SetupConfig;
    /// The configuration for creating a new [`AxArchVCpu`] for host VM. Used by [`AxArchVCpu::new_host`] in type 1.5 scenario.
    type HostConfig;

    /// Create a new `AxArchVCpu`.
    fn new(config: Self::CreateConfig) -> AxResult<Self>;

    /// Create a new `AxArchVCpu` for host VM.
    fn new_host(config: Self::HostConfig) -> AxResult<Self>;

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

    /// Run the vcpu until a vm-exit occurs.
    fn run(&mut self) -> AxResult<AxVCpuExitReason>;

    /// Bind the vcpu to the current physical CPU.
    fn bind(&mut self) -> AxResult;

    /// Unbind the vcpu from the current physical CPU.
    fn unbind(&mut self) -> AxResult;

    /// Set the value of a general-purpose register according to the given index.
    fn set_gpr(&mut self, reg: usize, val: usize);
}
