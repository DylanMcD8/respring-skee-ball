#!/usr/bin/env python3
"""Compile the actual CPU/bridge source in a small bounded-memory harness.

Generated game/runtime dispatch is replaced with misses to isolate ABI veneers
and strict rejection. Instruction semantics, step dispatch and byte guards are
production code. Full generated translation is tested separately in the app.
"""
from pathlib import Path
import argparse, os, shutil, subprocess, tempfile
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--rustc',required=True)
a=p.parse_args()
source=Path(__file__).resolve().parents[1]/'compatibility-core/src/cpu/interpreter'
with tempfile.TemporaryDirectory(prefix='chuzzle-bridge-check-') as directory:
    root=Path(directory);cpu=root/'cpu';cpu.mkdir();out=cpu/'interpreter';out.mkdir()
    for f in ['mod.rs','arm.rs','thumb16.rs','thumb32.rs','vfp.rs','native_bridge.rs']:
        shutil.copyfile(source/f,out/f)
    for name,method in [('native_generated','native_step'),('runtime_generated','runtime_native_step')]:
        (out/(name+'.rs')).write_text('use super::*; impl InterpreterCpu { pub(super) fn '+method+'(&mut self, _: &mut Mem) -> Option<CpuState> { None } }')
    (root/'main.rs').write_text('''#![allow(dead_code)]
macro_rules! echo { ($($args:tt)*) => { eprintln!($($args)*) }; }
mod mem {
    #[derive(Clone, Copy)] pub struct Ptr(u32);
    impl Ptr { pub fn from_bits(x:u32)->Self { Self(x) } }
    pub type ConstVoidPtr=Ptr;
    pub struct Mem(pub Vec<u8>);
    impl Mem {
        pub fn get_bytes_fallible(&self,p:Ptr,n:u32)->Option<&[u8]> {
            self.0.get(p.0 as usize..(p.0 as usize).checked_add(n as usize)?)
        }
        pub fn get_bytes_fallible_mut(&mut self,p:Ptr,n:u32)->Option<&mut [u8]> {
            self.0.get_mut(p.0 as usize..(p.0 as usize).checked_add(n as usize)?)
        }
    }
}
mod cpu {
    #[derive(Debug,PartialEq,Eq)] pub enum CpuState { Normal,Svc(u32),Error(CpuError) }
    #[derive(Debug,PartialEq,Eq)] pub enum CpuError { MemoryError,UndefinedInstruction,Breakpoint }
    pub mod interpreter;
}
fn main() { cpu::interpreter::check_bridge(); }
''')
    with (out/'mod.rs').open('a') as f:
        f.write('''
pub fn check_bridge() {
    let mut checked=0;
    for flags in 0..16u32 {
      for byte in [false,true] {
       for alias in [false,true] {
        let mut mem=Mem(vec![0xa5;0x3000]);
        let mut cpu=InterpreterCpu::new(1);cpu.cpsr=(flags<<28)|CPSR_USER_MODE;
        cpu.regs[1]=0x2000;cpu.regs[2]=0x12345678;cpu.regs[0]=0x12345678;
        let instruction=0xe1010090 | if alias {0} else {2} | if byte {1<<22} else {0};
        let before=cpu.cpsr;
        assert_eq!(cpu.exec_arm(instruction,0x1000,&mut mem),Some(CpuState::Normal));
        assert_eq!(cpu.regs[0],if byte {0xa5}else{0xa5a5a5a5});
        assert_eq!(&mem.0[0x2000..0x2004],if byte {&[0x78,0xa5,0xa5,0xa5]}else{&[0x78,0x56,0x34,0x12]});
        assert_eq!(cpu.cpsr,before);assert_eq!(cpu.regs[15],0x1004);checked+=1;
       }
      }
    }
    for address in [0u32,0x3000,0x2001] {
      let mut mem=Mem(vec![0xa5;0x3000]);let old=mem.0.clone();
      let mut cpu=InterpreterCpu::new(1);cpu.regs[1]=address;cpu.regs[0]=123;
      assert_eq!(cpu.exec_arm(0xe1010092,0x1000,&mut mem),Some(CpuState::Error(CpuError::MemoryError)));
      assert_eq!(mem.0,old);assert_eq!(cpu.regs[0],123);checked+=1;
    }
    let mut mem=Mem(vec![0xa5;0x3000]);let old=mem.0.clone();
    let mut cpu=InterpreterCpu::new(1);cpu.regs[1]=0x2000;cpu.cpsr=CPSR_USER_MODE;
    assert_eq!(cpu.exec_arm(0x01010092,0x1000,&mut mem),Some(CpuState::Normal));
    assert_eq!(mem.0,old);assert_eq!(cpu.regs[0],0);checked+=1;
    for (rotation, expected) in [0x00120034u32,0x00ab00cd,0x00340012,0x00cd00ab].into_iter().enumerate() {
      let mut cpu=InterpreterCpu::new(1);cpu.regs[3]=0xab12cd34;cpu.cpsr=0xf0000010;
      let mut mem=Mem(vec![0;0x3000]);
      assert_eq!(cpu.exec_arm(0xe6cf2073|((rotation as u32)<<10),0x1000,&mut mem),Some(CpuState::Normal));
      assert_eq!(cpu.regs[2],expected);assert_eq!(cpu.cpsr,0xf0000010);checked+=1;
    }
    let mut cpu=InterpreterCpu::new(1);cpu.regs[3]=0xab12cd34;cpu.regs[1]=0xfff0fff0;
    let mut mem=Mem(vec![0;0x3000]);
    assert_eq!(cpu.exec_arm(0xe6c12073,0x1000,&mut mem),Some(CpuState::Normal));
    assert_eq!(cpu.regs[2],0x00020024);checked+=1;
    for (a,b,c,expected) in [(7u32,6u32,50u32,8u32),(0xffffffff,2,1,3),(0x10000,0x10000,123,123)] {
     let mut cpu=InterpreterCpu::new(1);cpu.regs[2]=a;cpu.regs[8]=b;cpu.regs[3]=c;cpu.cpsr=0xf0000010;
     assert_eq!(cpu.exec_arm(0xe0633892,0x1000,&mut mem),Some(CpuState::Normal));
     assert_eq!(cpu.regs[3],expected);assert_eq!(cpu.cpsr,0xf0000010);checked+=1;
    }
    for cancel in 0..4 {
     let mut mem=Mem(vec![0;0x3000]);mem.0[0x2000..0x2004].copy_from_slice(&41u32.to_le_bytes());
     let mut cpu=InterpreterCpu::new(1);cpu.regs[2]=0x2000;cpu.regs[12]=42;cpu.cpsr=0xf0000010;
     assert_eq!(cpu.exec_arm(0xe1924f9f,0x1000,&mut mem),Some(CpuState::Normal));
     assert_eq!(cpu.regs[4],41);
     if cancel==1 {cpu.exec_arm(0xf57ff01f,0x1004,&mut mem);}
     if cancel==2 {let mut ctx=CpuContext::default();cpu.swap_context(&mut ctx);cpu.swap_context(&mut ctx);}
     if cancel==3 {cpu.regs[2]=0x2004;}
     assert_eq!(cpu.exec_arm(0xe1825f9c,0x1008,&mut mem),Some(CpuState::Normal));
     assert_eq!(cpu.regs[5],if cancel==0 {0}else{1});
     assert_eq!(u32::from_le_bytes(mem.0[0x2000..0x2004].try_into().unwrap()),if cancel==0 {42}else{41});
     assert_eq!(cpu.cpsr,0xf0000010);
     assert_eq!(cpu.exec_arm(0xe1825f9c,0x100c,&mut mem),Some(CpuState::Normal));assert_eq!(cpu.regs[5],1);checked+=1;
    }
    for opcode in [0xf5d2f000u32,0xf552f010] {
     let mut cpu=InterpreterCpu::new(1);cpu.regs[2]=0xffffffff;cpu.cpsr=0xf0000010;
     let before=cpu.regs;let old=mem.0.clone();
     assert_eq!(cpu.exec_arm(opcode,0x1000,&mut mem),Some(CpuState::Normal));
     assert_eq!(&cpu.regs[..15],&before[..15]);assert_eq!(cpu.cpsr,0xf0000010);assert_eq!(mem.0,old);checked+=1;
    }
    for (opcode, expected) in [(0xf3e089c0u32,[2.,4.,6.,8.]), (0xf3e081e0,[13.,16.,19.,22.]), (0xf3e085e0,[7.,4.,1.,-2.])] {
     let mut cpu=InterpreterCpu::new(1);cpu.cpsr=0xf0000010;
     cpu.set_sreg(0,2f32.to_bits());cpu.set_sreg(1,3f32.to_bits());
     for k in 0..4 {cpu.set_sreg(32+k,((k+1) as f32).to_bits());cpu.set_sreg(48+k,10f32.to_bits());}
     assert_eq!(cpu.exec_arm(opcode,0x1000,&mut mem),Some(CpuState::Normal));
     for k in 0..4 {assert_eq!(f32::from_bits(cpu.get_sreg(48+k)),expected[k]);}
     assert_eq!(cpu.cpsr,0xf0000010);checked+=1;
    }
    // Scalar aliases the destination: all lanes must use its original value.
    let mut cpu=InterpreterCpu::new(1);cpu.set_sreg(0,2f32.to_bits());cpu.set_sreg(1,3f32.to_bits());
    assert_eq!(cpu.exec_arm(0xf2a00940,0x1000,&mut mem),Some(CpuState::Normal));
    assert_eq!(f32::from_bits(cpu.get_sreg(0)),4.);assert_eq!(f32::from_bits(cpu.get_sreg(1)),6.);checked+=1;
    for subtract in [false,true] {
     let mut cpu=InterpreterCpu::new(1);cpu.cpsr=0xf0000010;
     for k in 0..4 {cpu.set_sreg(16+k,((k+1) as f32).to_bits());cpu.set_sreg(8+k,2f32.to_bits());cpu.set_sreg(24+k,10f32.to_bits());}
     assert_eq!(cpu.exec_arm(0xf208cd54 | if subtract {1<<21} else {0},0x1000,&mut mem),Some(CpuState::Normal));
     for k in 0..4 {let product=2.*((k+1) as f32);assert_eq!(f32::from_bits(cpu.get_sreg(24+k)),if subtract {10.-product} else {10.+product});}
     assert_eq!(cpu.cpsr,0xf0000010);checked+=1;
    }
    for thumb in [false,true] {
     for op in 0..8u32 {
      let mut cpu=InterpreterCpu::new(1);cpu.cpsr=0xf0000010;
      let a=0xff00ff00aa55aa55u64;let b=0x00ffffff12345678u64;let old=0x55555555aaaaaaaa;
      for k in 0..2 {cpu.set_dreg(k,old);cpu.set_dreg(2+k,a);cpu.set_dreg(4+k,b);}
      let opcode=if thumb {0xef020154 | ((op>>2)<<28) | ((op&3)<<20)} else {0xf2020154 | ((op>>2)<<24) | ((op&3)<<20)};
      assert_eq!(cpu.exec_vfp(opcode,0x1000,&mut mem),Some(CpuState::Normal));
      let expected=match op {0=>a&b,1=>a&!b,2=>a|b,3=>a|!b,4=>a^b,5=>(old&a)|(!old&b),6=>(old&!b)|(a&b),_=>(old&b)|(a&!b)};
      assert_eq!(cpu.get_dreg(0),expected);assert_eq!(cpu.get_dreg(1),expected);assert_eq!(cpu.cpsr,0xf0000010);checked+=1;
     }
    }
    for q in 0..2u32 {
      for (imm,expected) in [(1,0x1111111111111111u64),(15,0x8888888888888888),(2,0x2211221122112211),(14,0x8877887788778877),(4,0x4433221144332211),(12,0x8877665588776655)] {
       let mut cpu=InterpreterCpu::new(1);cpu.set_dreg(14,0x8877665544332211);
       assert_eq!(cpu.exec_arm(0xf3b08c0e|(imm<<16)|(q<<6),0x1000,&mut mem),Some(CpuState::Normal));
       assert_eq!(cpu.get_dreg(8),expected);if q==1 {assert_eq!(cpu.get_dreg(9),expected);}checked+=1;
      }
    }
    println!("Passed {checked} SWP/SWPB, UXTB16/UXTAB16, MLS, exclusive-monitor, prefetch, and NEON scalar cases");
}

''')
    for reference in [False,True]:
        cmd=[a.rustc,'--edition=2021','-O',str(root/'main.rs'),'-o',str(root/'check')]
        if reference: cmd+=['--cfg','feature="aot-reference"']
        subprocess.run(cmd,check=True)
        env={k:v for k,v in os.environ.items() if not k.startswith('CHUZZLE_')}
        if not reference:
            # Delivery must ignore every development switch that could request
            # interpretation, disable translation, or enable the oracle.
            env.update(CHUZZLE_EXECUTION='interpreter',CHUZZLE_AOT_STRICT='0',
                       CHUZZLE_AOT_RUNTIME='0',CHUZZLE_AOT_VALIDATE='1')
        subprocess.run([str(root/'check')],check=True,env=env)
