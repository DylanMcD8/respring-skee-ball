/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CPU facade using ahead-of-time translated Chuzzle code with an ARM fallback.
//! No executable memory is generated at runtime.
use crate::abi::GuestFunction;
use crate::mem::{GuestUSize, Mem};
mod interpreter;
pub use interpreter::CpuContext;

pub struct Cpu { inner: Box<interpreter::InterpreterCpu> }
#[derive(Debug, PartialEq, Eq)]
pub enum CpuState { Normal, Svc(u32), Error(CpuError) }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuError { MemoryError, UndefinedInstruction, Breakpoint }

impl Cpu {
    pub const SP: usize = 13;
    pub const LR: usize = 14;
    pub const PC: usize = 15;
    pub const CPSR_THUMB: u32 = 0x20;
    pub const CPSR_USER_MODE: u32 = 0x10;
    pub fn new(mem: Option<&mut Mem>) -> Self {
        let pages = mem.map_or(0, |m| m.null_segment_size() / 0x1000);
        Self { inner: interpreter::InterpreterCpu::new(pages) }
    }
    pub fn regs(&self) -> &[u32; 16] { self.inner.regs() }
    pub fn regs_mut(&mut self) -> &mut [u32; 16] { self.inner.regs_mut() }
    pub fn cpsr(&self) -> u32 { self.inner.cpsr() }
    pub fn set_cpsr(&mut self, v: u32) { self.inner.set_cpsr(v); }
    pub fn swap_context(&mut self, ctx: &mut CpuContext) { self.inner.swap_context(ctx); }
    pub fn pc_with_thumb_bit(&self) -> GuestFunction {
        GuestFunction::from_addr_and_thumb_flag(self.regs()[Self::PC], self.cpsr() & Self::CPSR_THUMB != 0)
    }
    pub fn branch(&mut self, pc: GuestFunction) {
        self.regs_mut()[Self::PC] = pc.addr_without_thumb_bit();
        self.set_cpsr((self.cpsr() & !Self::CPSR_THUMB) | if pc.is_thumb() { Self::CPSR_THUMB } else { 0 });
    }
    pub fn branch_with_link(&mut self, pc: GuestFunction, lr: GuestFunction) -> (GuestFunction, GuestFunction) {
        let previous = (self.pc_with_thumb_bit(), GuestFunction::from_addr_with_thumb_bit(self.regs()[Self::LR]));
        self.branch(pc); self.regs_mut()[Self::LR] = lr.addr_with_thumb_bit(); previous
    }
    pub fn invalidate_cache_range(&mut self, base: u32, size: GuestUSize) { self.inner.invalidate_cache_range(base, size); }
    pub fn run_or_step(&mut self, mem: &mut Mem, ticks: Option<&mut u64>) -> CpuState { self.inner.run_or_step(mem, ticks) }
    pub fn dump_regs(&self) { Self::echo_regs(self.regs()); }
    pub fn echo_regs(regs: &[u32; 16]) {
        for row in 0..4 {
            let start = row*4;
            echo!("R{}={:#010x} R{}={:#010x} R{}={:#010x} R{}={:#010x}", start, regs[start], start+1, regs[start+1], start+2, regs[start+2], start+3, regs[start+3]);
        }
    }
}
