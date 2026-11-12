use crate::vm::opcode::RegisterOperand;
use std::{cmp::Reverse, collections::BinaryHeap, mem::forget};

bitflags::bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    struct RegisterFlags: u8 {
        /// Whether the register is still in use (not deallocated).
        const USED       = 0b0000_0001;

        const PERSISTENT = 0b0000_0010;
    }
}

impl RegisterFlags {
    fn is_used(self) -> bool {
        self.contains(Self::USED)
    }
    fn is_persistent(self) -> bool {
        self.contains(Self::PERSISTENT)
    }
}

/// An entry in the [`RegisterAllocator`].
#[derive(Debug, Default)]
pub(crate) struct RegisterEntry {
    flags: RegisterFlags,
}

/// Represent a VM register.
///
/// This is intended to be passed by reference or to be moved, dropping this is a bug,
/// it should only be dropped though the [`RegisterAllocator::dealloc()`] method.
/// This doesn't apply to persistent registers.
///
/// A [`Register`] is index into the register allocator,
/// as well as an index into the per-frame register file on each [`CallFrame`](crate::vm::CallFrame).
#[derive(Debug)]
pub(crate) struct Register {
    index: u32,
    flags: RegisterFlags,
}

impl Register {
    /// The index of the [`Register`].
    pub(crate) fn index(&self) -> u32 {
        self.index
    }

    /// The index of the [`Register`] as a [`RegisterOperand`].
    pub(crate) fn variable(&self) -> RegisterOperand {
        self.index.into()
    }

    pub(crate) fn persistent(index: u32) -> Self {
        Self {
            index,
            flags: RegisterFlags::PERSISTENT,
        }
    }
}

impl Drop for Register {
    /// This method should never be called.
    /// It is used to detect when a register has not been deallocated.
    fn drop(&mut self) {
        if self.flags.is_persistent() {
            return;
        }

        // Prevent double panic.
        if std::thread::panicking() {
            return;
        }

        unreachable!("forgot to deallocate a register!")
    }
}

#[derive(Debug, Default)]
pub(crate) struct RegisterAllocator {
    registers: Vec<RegisterEntry>,
    /// Indices of the unused registers. Popping yields the lowest one, the register a scan from
    /// index 0 would find. Only [`Self::dealloc`] clears `USED`, so it is the only producer.
    free: BinaryHeap<Reverse<u32>>,
}

impl RegisterAllocator {
    pub(crate) fn alloc(&mut self) -> Register {
        if let Some(Reverse(i)) = self.free.pop() {
            let register = &mut self.registers[i as usize];
            assert!(!register.flags.is_persistent());

            register.flags |= RegisterFlags::USED;
            return Register {
                index: i,
                flags: register.flags,
            };
        }

        let flags = RegisterFlags::USED;

        let index = self.registers.len() as u32;
        self.registers.push(RegisterEntry { flags });

        Register { index, flags }
    }

    pub(crate) fn alloc_persistent(&mut self) -> Register {
        let mut reg = self.alloc();

        let index = reg.index();

        let register = &mut self.registers[index as usize];

        register.flags |= RegisterFlags::PERSISTENT;

        reg.flags = register.flags;
        reg
    }

    #[track_caller]
    pub(crate) fn dealloc(&mut self, reg: Register) {
        assert!(
            !reg.flags.is_persistent(),
            "Trying to deallocate a persistent register"
        );

        let register = &mut self.registers[reg.index as usize];

        assert!(
            register.flags.is_used(),
            "Cannot deallocate unused variable"
        );
        register.flags.set(RegisterFlags::USED, false);
        self.free.push(Reverse(reg.index));

        // NOTE: We should not drop it since, dropping it used to detect bugs.
        forget(reg);
    }

    pub(crate) fn finish(self) -> u32 {
        for register in &self.registers {
            debug_assert!(
                !register.flags.is_used()
                    || (register.flags.is_used() && register.flags.is_persistent())
            );
        }

        self.registers.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::RegisterAllocator;

    /// Allocation hands out the lowest unused index; the compiled bytecode depends on it.
    #[test]
    fn alloc_returns_the_lowest_unused_index() {
        let mut allocator = RegisterAllocator::default();
        let [r0, r1, r2, r3, r4] = [(); 5].map(|()| allocator.alloc());
        let persistent = allocator.alloc_persistent();
        allocator.dealloc(r3);
        allocator.dealloc(r1);
        allocator.dealloc(r4);

        let reused = [(); 4].map(|()| allocator.alloc());
        assert_eq!(reused.each_ref().map(super::Register::index), [1, 3, 4, 6]);
        assert_eq!(persistent.index(), 5);

        allocator.dealloc(r0);
        allocator.dealloc(r2);
        for reg in reused {
            allocator.dealloc(reg);
        }
        assert_eq!(allocator.finish(), 7);
    }
}
