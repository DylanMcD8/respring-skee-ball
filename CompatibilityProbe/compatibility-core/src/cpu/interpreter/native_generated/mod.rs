// Diagnostic probe: no Chuzzle translations may execute against Skee-Ball.
use super::{InterpreterCpu,Mem,CpuState};
impl InterpreterCpu {pub(super) fn native_step(&mut self,_mem:&mut Mem)->Option<CpuState>{None}}
