use axerrno::AxResult;

pub trait AxVMArchPerCpu: Sized {
    /// Create a new per-CPU state.
    fn new(cpu_id: usize) -> AxResult<Self>;
    /// Whether hardware virtualization is enabled on the current CPU.
    fn is_enabled(&self) -> bool;
    /// Enable hardware virtualization on the current CPU.
    fn hardware_enable(&mut self) -> AxResult;
    /// Disable hardware virtualization on the current CPU.
    fn hardware_disable(&mut self) -> AxResult;
}
