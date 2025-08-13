use core::cell::{RefCell, UnsafeCell};

use axaddrspace::{GuestPhysAddr, HostPhysAddr};
use axerrno::{AxResult, ax_err};
use axvisor_api::vmm::{VCpuId, VMId};

use super::{AxArchVCpu, AxVCpuExitReason};

/// Immutable configuration data for a virtual CPU.
///
/// This structure contains the constant properties of a vCPU that don't change
/// after creation, such as CPU affinity settings and identifiers.
struct AxVCpuInnerConst {
    /// Unique identifier of the VM this vCPU belongs to
    vm_id: VMId,
    /// Unique identifier of this vCPU within its VM
    vcpu_id: VCpuId,
    /// Physical CPU ID that has priority to run this vCPU
    /// Used for CPU affinity optimization
    favor_phys_cpu: usize,
    /// Bitmask of physical CPUs that can run this vCPU
    /// If `None`, the vCPU can run on any available physical CPU
    /// Similar to Linux CPU_SET functionality
    phys_cpu_set: Option<usize>,
}

/// Represents the current execution state of a virtual CPU.
///
/// The vCPU follows a strict state machine:
/// Created → Free → Ready → Running
///
/// Invalid state is used when errors occur during state transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VCpuState {
    /// Invalid state - indicates an error occurred during state transition
    Invalid = 0,
    /// Initial state after vCPU creation, not yet initialized
    Created = 1,
    /// vCPU is initialized and ready to be bound to a physical CPU
    Free = 2,
    /// vCPU is bound to a physical CPU and ready for execution
    Ready = 3,
    /// vCPU is currently executing on a physical CPU
    Running = 4,
    /// vCPU execution is blocked (waiting for I/O, etc.)
    Blocked = 5,
}

/// Mutable runtime state of a virtual CPU.
///
/// This structure contains data that changes during vCPU execution,
/// protected by RefCell for interior mutability.
pub struct AxVCpuInnerMut {
    /// Current execution state of the vCPU
    state: VCpuState,
}

/// Architecture-independent virtual CPU implementation.
///
/// This is the main vCPU abstraction that provides a unified interface for
/// managing virtual CPUs across different architectures. It delegates
/// architecture-specific operations to implementations of the `AxArchVCpu` trait.
///
/// # Design Principles
///
/// - **Interior Mutability**: Most methods take `&self` and handle mutability internally
/// - **Thread Safety**: Not thread-safe by design - caller must ensure proper synchronization
/// - **State Management**: Enforces strict state transitions to prevent invalid operations
/// - **Architecture Abstraction**: Delegates arch-specific operations to `AxArchVCpu` implementations
///
/// # Safety
///
/// This struct uses `UnsafeCell` for the architecture-specific state to allow
/// interior mutability without runtime checks, as `RefCell` guards cannot be
/// dropped when launching a vCPU.
pub struct AxVCpu<A: AxArchVCpu> {
    /// Immutable vCPU configuration (VM ID, CPU affinity, etc.)
    inner_const: AxVCpuInnerConst,
    /// Mutable vCPU state protected by RefCell for safe interior mutability
    inner_mut: RefCell<AxVCpuInnerMut>,
    /// Architecture-specific vCPU implementation
    ///
    /// Uses UnsafeCell instead of RefCell because RefCell guards cannot be
    /// dropped during vCPU execution (when control is transferred to guest)
    arch_vcpu: UnsafeCell<A>,
}

impl<A: AxArchVCpu> AxVCpu<A> {
    /// Creates a new virtual CPU instance.
    ///
    /// Initializes a vCPU with the given configuration and creates the underlying
    /// architecture-specific implementation. The vCPU starts in the `Created` state.
    ///
    /// # Arguments
    ///
    /// * `vm_id` - Unique identifier of the VM this vCPU belongs to
    /// * `vcpu_id` - Unique identifier for this vCPU within the VM
    /// * `favor_phys_cpu` - Physical CPU ID that should preferentially run this vCPU
    /// * `phys_cpu_set` - Optional bitmask of allowed physical CPUs (None = no restriction)
    /// * `arch_config` - Architecture-specific configuration for vCPU creation
    ///
    /// # Returns
    ///
    /// Returns `Ok(AxVCpu)` on success, or an error if architecture-specific creation fails.
    pub fn new(
        vm_id: VMId,
        vcpu_id: VCpuId,
        favor_phys_cpu: usize,
        phys_cpu_set: Option<usize>,
        arch_config: A::CreateConfig,
    ) -> AxResult<Self> {
        Ok(Self {
            inner_const: AxVCpuInnerConst {
                vm_id,
                vcpu_id,
                favor_phys_cpu,
                phys_cpu_set,
            },
            inner_mut: RefCell::new(AxVCpuInnerMut {
                state: VCpuState::Created,
            }),
            arch_vcpu: UnsafeCell::new(A::new(vm_id, vcpu_id, arch_config)?),
        })
    }

    /// Sets up the vCPU for execution.
    ///
    /// Configures the vCPU's entry point, memory management (EPT root), and any
    /// architecture-specific setup. Transitions the vCPU from `Created` to `Free` state.
    pub fn setup(
        &self,
        entry: GuestPhysAddr,
        ept_root: HostPhysAddr,
        arch_config: A::SetupConfig,
    ) -> AxResult {
        self.manipulate_arch_vcpu(VCpuState::Created, VCpuState::Free, |arch_vcpu| {
            arch_vcpu.set_entry(entry)?;
            arch_vcpu.set_ept_root(ept_root)?;
            arch_vcpu.setup(arch_config)?;
            Ok(())
        })
    }

    /// Returns the unique identifier of this vCPU.
    pub const fn id(&self) -> VCpuId {
        self.inner_const.vcpu_id
    }

    /// Returns the preferred physical CPU for this vCPU.
    ///
    /// This is used for CPU affinity optimization - the scheduler should
    /// preferentially run this vCPU on the returned physical CPU ID.
    ///
    /// # Note
    ///
    /// Currently unused in the implementation but reserved for future
    /// scheduler optimizations.
    pub const fn favor_phys_cpu(&self) -> usize {
        self.inner_const.favor_phys_cpu
    }

    /// Returns the set of physical CPUs that can run this vCPU.
    pub const fn phys_cpu_set(&self) -> Option<usize> {
        self.inner_const.phys_cpu_set
    }

    /// Checks if this vCPU is the Bootstrap Processor (BSP).
    ///
    /// By convention, the vCPU with ID 0 is always considered the BSP,
    /// which is responsible for system initialization in multi-core VMs.
    pub const fn is_bsp(&self) -> bool {
        self.inner_const.vcpu_id == 0
    }

    /// Gets the current execution state of the vCPU.
    pub fn state(&self) -> VCpuState {
        self.inner_mut.borrow().state
    }

    /// Set the state of the vcpu.
    /// # Safety
    /// This method is unsafe because it may break the state transition model.
    /// Use it with caution.
    pub unsafe fn set_state(&self, state: VCpuState) {
        self.inner_mut.borrow_mut().state = state;
    }

    /// Execute a block with the state of the vcpu transitioned from `from` to `to`. If the current state is not `from`, return an error.
    ///
    /// The state will be set to [`VCpuState::Invalid`] if an error occurs (including the case that the current state is not `from`).
    ///
    /// The state will be set to `to` if the block is executed successfully.
    pub fn with_state_transition<F, T>(&self, from: VCpuState, to: VCpuState, f: F) -> AxResult<T>
    where
        F: FnOnce() -> AxResult<T>,
    {
        let mut inner_mut = self.inner_mut.borrow_mut();
        if inner_mut.state != from {
            inner_mut.state = VCpuState::Invalid;
            ax_err!(
                BadState,
                format!("VCpu state is not {:?}, but {:?}", from, inner_mut.state)
            )
        } else {
            let result = f();
            inner_mut.state = if result.is_err() {
                VCpuState::Invalid
            } else {
                to
            };
            result
        }
    }

    /// Execute a block with the current vcpu set to `&self`.
    pub fn with_current_cpu_set<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        if get_current_vcpu::<A>().is_some() {
            panic!("Nested vcpu operation is not allowed!");
        } else {
            unsafe {
                set_current_vcpu(self);
            }
            let result = f();
            unsafe {
                clear_current_vcpu::<A>();
            }
            result
        }
    }

    /// Execute an operation on the architecture-specific vcpu, with the state transitioned from `from` to `to` and the current vcpu set to `&self`.
    ///
    /// This method is a combination of [`AxVCpu::with_state_transition`] and [`AxVCpu::with_current_cpu_set`].
    pub fn manipulate_arch_vcpu<F, T>(&self, from: VCpuState, to: VCpuState, f: F) -> AxResult<T>
    where
        F: FnOnce(&mut A) -> AxResult<T>,
    {
        self.with_state_transition(from, to, || {
            self.with_current_cpu_set(|| f(self.get_arch_vcpu()))
        })
    }

    /// Transition the state of the vcpu. If the current state is not `from`, return an error.
    pub fn transition_state(&self, from: VCpuState, to: VCpuState) -> AxResult {
        self.with_state_transition(from, to, || Ok(()))
    }

    /// Get the architecture-specific vcpu.
    #[allow(clippy::mut_from_ref)]
    pub fn get_arch_vcpu(&self) -> &mut A {
        unsafe { &mut *self.arch_vcpu.get() }
    }

    /// Run the vcpu.
    pub fn run(&self) -> AxResult<AxVCpuExitReason> {
        self.transition_state(VCpuState::Ready, VCpuState::Running)?;
        self.manipulate_arch_vcpu(VCpuState::Running, VCpuState::Ready, |arch_vcpu| {
            arch_vcpu.run()
        })
    }

    /// Bind the vcpu to the current physical CPU.
    pub fn bind(&self) -> AxResult {
        self.manipulate_arch_vcpu(VCpuState::Free, VCpuState::Ready, |arch_vcpu| {
            arch_vcpu.bind()
        })
    }

    /// Unbind the vcpu from the current physical CPU.
    pub fn unbind(&self) -> AxResult {
        self.manipulate_arch_vcpu(VCpuState::Ready, VCpuState::Free, |arch_vcpu| {
            arch_vcpu.unbind()
        })
    }

    /// Sets the entry address of the vcpu.
    pub fn set_entry(&self, entry: GuestPhysAddr) -> AxResult {
        self.get_arch_vcpu().set_entry(entry)
    }

    /// Sets the value of a general-purpose register according to the given index.
    pub fn set_gpr(&self, reg: usize, val: usize) {
        self.get_arch_vcpu().set_gpr(reg, val);
    }

    /// Inject an interrupt to the vcpu.
    pub fn inject_interrupt(&self, vector: usize) -> AxResult {
        self.get_arch_vcpu().inject_interrupt(vector)
    }

    /// Sets the return value of the vcpu.
    pub fn set_return_value(&self, val: usize) {
        self.get_arch_vcpu().set_return_value(val);
    }
}

#[percpu::def_percpu]
static mut CURRENT_VCPU: Option<*mut u8> = None;

/// Get the current vcpu on the current physical CPU.
///
/// It's guaranteed that each time before a method of [`AxArchVCpu`] is called, the current vcpu is set to the corresponding [`AxVCpu`].
/// So methods of [`AxArchVCpu`] can always get the [`AxVCpu`] containing itself by calling this method.
pub fn get_current_vcpu<'a, A: AxArchVCpu>() -> Option<&'a AxVCpu<A>> {
    unsafe {
        CURRENT_VCPU
            .current_ref_raw()
            .as_ref()
            .copied()
            .and_then(|p| (p as *const AxVCpu<A>).as_ref())
    }
}

/// Get a mutable reference to the current vcpu on the current physical CPU.
///
/// See [`get_current_vcpu`] for more details.
pub fn get_current_vcpu_mut<'a, A: AxArchVCpu>() -> Option<&'a mut AxVCpu<A>> {
    unsafe {
        CURRENT_VCPU
            .current_ref_mut_raw()
            .as_mut()
            .copied()
            .and_then(|p| (p as *mut AxVCpu<A>).as_mut())
    }
}

/// Set the current vcpu on the current physical CPU.
///
/// # Safety
/// This method is marked as unsafe because it may result in unexpected behavior if not used properly.
/// Do not call this method unless you know what you are doing.
pub unsafe fn set_current_vcpu<A: AxArchVCpu>(vcpu: &AxVCpu<A>) {
    unsafe {
        CURRENT_VCPU
            .current_ref_mut_raw()
            .replace(vcpu as *const _ as *mut u8);
    }
}

/// Clear the current vcpu on the current physical CPU.
///
/// # Safety
/// This method is marked as unsafe because it may result in unexpected behavior if not used properly.
/// Do not call this method unless you know what you are doing.    
pub unsafe fn clear_current_vcpu<A: AxArchVCpu>() {
    unsafe {
        CURRENT_VCPU.current_ref_mut_raw().take();
    }
}
