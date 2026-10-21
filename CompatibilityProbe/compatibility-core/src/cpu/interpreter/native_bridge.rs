// Native implementation of the two-word ABI veneers emitted by dyld.rs.
// These are host-call records, not arbitrary ARM instructions. The return is
// accepted only beside an unchanged SVC record. All other code must have a
// generated, address-and-byte-guarded translation.
use super::*;

impl InterpreterCpu {
    pub(super) fn native_bridge_step(&mut self, mem: &mut Mem) -> Option<CpuState> {
        if self.is_thumb() { return None; }
        let pc = self.regs[PC];
        let word = self.read_code_u32(mem, pc)?;
        if word & 0xff00_0000 == 0xef00_0000 {
            // environment::handle_cpu_state uses PC-4 to identify this record.
            // The linked host API runs there, after returning from the CPU.
            self.regs[PC] = pc.wrapping_add(4);
            return Some(CpuState::Svc(word & 0x00ff_ffff));
        }
        if word == 0xe12f_ff1e && pc >= 4 {
            let previous = self.read_code_u32(mem, pc - 4)?;
            if previous & 0xff00_0000 == 0xef00_0000 {
                // Preserve ARM/Thumb interworking and flags on the host return.
                self.bx_write_pc(self.regs[14]);
                return Some(CpuState::Normal);
            }
        }
        None
    }
}
