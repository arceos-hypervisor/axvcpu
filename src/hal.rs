use axaddrspace::{HostPhysAddr, HostVirtAddr};

/// The interfaces which the underlying software (kernel or hypervisor) must implement.
pub trait AxVCpuHal {
    type EPTTranslator: axaddrspace::EPTTranslator;
    type PagingHandler: page_table_multiarch::PagingHandler;

    /// Converts a host virtual address to a host physical address.
    ///
    /// # Parameters
    ///
    /// * `vaddr` - The virtual address to convert.
    ///
    /// # Returns
    ///
    /// * `HostPhysAddr` - The corresponding physical address.
    fn virt_to_phys(vaddr: HostVirtAddr) -> HostPhysAddr;

    /// Fetches current interrupt (IRQ) number.
    ///
    /// # Returns
    ///
    /// * `usize` - The current IRQ number.
    fn irq_fetch() -> usize {
        0
    }

    /// Dispatch an interrupt request (IRQ) to the underlying host OS.
    fn irq_hanlder() {
        unimplemented!("irq_handler is not implemented");
    }
}
