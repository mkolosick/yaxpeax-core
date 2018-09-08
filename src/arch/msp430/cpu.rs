use arch::{Decodable, MCU};
use memory::MemoryRepr;
use debug;
use debug::DebugTarget;
use arch::msp430;
use yaxpeax_msp430_mc::{Operand, Opcode, Width};


pub struct MSP430DebugTarget<'a> {
    pub target: &'a mut msp430::cpu::CPU,
    break_conditions: Vec<BreakCondition>,
    watch_targets: Vec<WatchTarget>
}

impl <'a> MSP430DebugTarget<'a> {
    fn check_breakpoints(&self) -> bool {
        for bp in &self.break_conditions {
            match bp {
                BreakCondition::IPValue(ip) => {
                    if &self.target.ip() == ip { return true; }
                },
                BreakCondition::Other(f) => {
                    if f(&self.target) { return true; }
                },
                BreakCondition::MemoryAccess(dest) => {
                    panic!("Memory access breakpoints not yet supported for MSP430");
                },
                BreakCondition::IO => {
                    panic!("IO breakpoints not yet supported for MSP430");
                }
            }
        }
        false
    }
    fn show_watches(&self) {
        for watch in &self.watch_targets {
            println!("WATCH: {}", watch.reify(&self.target));
        }
    }
}

impl WatchTarget {
    /// Builds a pointer from the target of the watch.
    fn pointee(&self, cpu: &msp430::cpu::CPU) -> Option<u16> {
        match self {
            WatchTarget::Pointee(target) => {
                panic!("MSP430 watches not yet supported");
            },
            WatchTarget::MemoryLocation(addr) => {
                panic!("MSP430 watches not yet supported");
            }
        }
    }
    fn reify(&self, cpu: &msp430::cpu::CPU) -> String {
        match self {
            WatchTarget::Pointee(target) => {
                panic!("MSP430 watches not yet supported");
            }
            WatchTarget::MemoryLocation(addr) => {
                panic!("MSP430 watches not yet supported");
            },
        }
    }
}

trait InstructionContext {
//    fn debank(&self, banked: u8) -> u16;
}

impl InstructionContext for msp430::cpu::CPU {
}

trait Contextual<T> {
    fn contextualize(&self, &T) -> String;
}

impl <T> Contextual<T> for yaxpeax_msp430_mc::Instruction
    where T: InstructionContext
{
    fn contextualize(&self, ctx: &T) -> String {
        fn contextualize_op<T: InstructionContext>(op: yaxpeax_msp430_mc::Operand, ctx: &T) -> String {
            panic!("unimplemented");
        }

        let mut result = format!("{}", self.opcode);
        match self.operands[0] {
            Operand::Nothing => { return result; },
            x @ _ => {
                result = format!("{} {}", result, contextualize_op(x, ctx));
            }
        }
        match self.operands[1] {
            Operand::Nothing => { return result; },
            x @ _ => {
                result = format!("{}, {}", result, contextualize_op(x, ctx));
            }
        }
        return result;
    }
}

#[derive(Debug)]
pub enum WatchTarget {
    Pointee(Box<WatchTarget>),
    MemoryLocation(u16),
}
pub enum BreakCondition {
    IPValue(u16),
    Other(fn(&msp430::cpu::CPU) -> bool),
    MemoryAccess(u16),
    IO
}

impl <'a> DebugTarget<'a, msp430::cpu::CPU> for MSP430DebugTarget<'a> {
    type WatchTarget = WatchTarget;
    type BreakCondition = BreakCondition;
    fn attach(cpu: &'a mut msp430::cpu::CPU) -> Self {
        MSP430DebugTarget {
            target: cpu,
            break_conditions: vec![],
            watch_targets: vec![]
        }
    }
    fn single_step(&mut self) -> Result<(), String> {
        self.show_watches();
        self.target.emulate()
    }
    fn run(&mut self) -> debug::RunResult {
        println!("Running...");
        match self.target.emulate() {
            Ok(()) => { },
            Err(msg) => {
                return debug::RunResult::ExecutionError(msg);
            }
        }
        loop {
            if self.check_breakpoints() {
                return debug::RunResult::HitBreakCondition;
            }
            match self.target.emulate() {
                Ok(()) => { },
                Err(msg) => {
                    return debug::RunResult::ExecutionError(msg);
                }
            }
        }
    }
    fn add_watch(&mut self, watch: WatchTarget) -> Result<(), String> {
        self.watch_targets.push(watch);
        Ok(())
    }
    fn add_break_condition(&mut self, break_cond: Self::BreakCondition) -> Result<(), String> {
        self.break_conditions.push(break_cond);
        Ok(())
    }
}

enum IOCause {
    UART,
    PORT
}

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct CPU {
    pub registers: [u16; 16],
    pub memory: Vec<u8>,
    disable: bool
}

impl CPU {
    pub fn new() -> Self {
        let mut cpu = CPU {
            registers: [0u16; 16],
            memory: vec![0; 0x10000],
            disable: false
        };

        cpu
    }
    pub fn ip(&self) -> u16 {
        self.registers[0]
    }
    pub fn set_ip(&mut self, newval: u16) {
        self.registers[0] = newval;
    }
    fn push(&mut self, value: u32) -> Result<(), String> {
        panic!("push??? you think we can push????");
    }
    fn pop(&mut self) -> Result<u32, String> {
        panic!("pop??? you think we can pop????");
    }

    /*
    #[allow(non_snake_case)]
    fn would_IO(&self) -> Option<IOCause> {
    }
    */
    pub fn get_byte(&mut self, addr: u16) -> Result<u8, String> {
        self.get_byte_noupdate(addr)
    }
    pub fn get_byte_noupdate(&self, addr: u16) -> Result<u8, String> {
        panic!("MSP430 memory not emulated yet");
    }
    pub fn set_byte_noupdate(&mut self, addr: u16, what: u8) -> Result<(), String> {
        panic!("MSP430 memory not emulated yet");
    }
    pub fn set_byte(&mut self, addr: u16, what: u8) -> Result<(), String> {
        self.set_byte_noupdate(addr, what)
    }
    pub fn describe(&self) {
        println!("msp430: ");
        println!("ip=0x{:x}", self.ip());
        match self.decode() {
            Ok(instr) => println!("instruction: {}", instr.contextualize(self)),
            Err(e) => println!("[invalid: {}]", e)
        };
    }
    pub fn program(&mut self, program: MemoryRepr) -> Result<(), String> {
        match program.sections.get(&0) {
            Some(data) => {
                if data.len() > self.memory.len() {
                    return Err(
                        format!(
                            "Data is larger than the chip: 0x{:x} bytes of memory but 0x{:x} available",
                            data.len(),
                            self.memory.len()
                        )
                    );
                }
                println!("DEBUG: writing 0x{:x} bytes of program...", data.len());
                for i in 0..data.len() {
                    self.memory[i] = data[i];
                }
            },
            None => {
                println!("WARN: Provided program includes no code.");
            }
        };

        let initial_ip = ((self.memory[0xffff] as u16) << 8) | (self.memory[0xfffe] as u16);
        if initial_ip == 0xffff {
            self.disable = true;
        } else {
            self.set_ip(initial_ip);
        }

        Ok(())
    }
}

impl MCU for CPU {
    type Addr = u16;
    type Instruction = yaxpeax_msp430_mc::Instruction;
    fn emulate(&mut self) -> Result<(), String> {
        if self.disable {
            return Ok(());
        }

        match self.decode() {
            Ok(instr) => {
                panic!("MSP430 emulation not yet supported");
            },
            Err(msg) => { panic!(msg); }
        };
    }

    fn decode(&self) -> Result<Self::Instruction, String> {
        let mut result = yaxpeax_msp430_mc::Instruction {
            opcode: Opcode::Invalid(0xffff),
            op_width: Width::W,
            operands: [Operand::Nothing, Operand::Nothing]
        };
        match result.decode_into(&self.memory[(self.ip() as usize)..]) {
            Some(()) => Ok(result),
            None => {
                Err(
                    format!(
                        "Unable to decode bytes at 0x{:x}: {:x?}",
                        self.ip(),
                        self.memory[(self.ip() as usize)..((self.ip() + 4) as usize)].iter().collect::<Vec<&u8>>()
                    )
                )
            }
        }
    }
}
