use std::hash::Hash;

use termion::color;

use SyntaxedRender;
use SyntaxedSSARender;
use yaxpeax_arch::{Arch, LengthedInstruction};
use arch::InstructionSpan;
use arch::msp430;
use arch::msp430::syntaxed_render;
use arch::msp430::{PartialInstructionContext};
use yaxpeax_msp430_mc::{Instruction, Opcode, Operand, Width, MSP430};
use analyses::control_flow::{BasicBlock, ControlFlowGraph};
use analyses::static_single_assignment::cytron::SSA;
use std::collections::HashMap;

impl <T> SyntaxedSSARender<MSP430, T, ()> for yaxpeax_msp430_mc::Instruction where T: msp430::PartialInstructionContext {
    fn render_with_ssa_values(
        &self,
        address: <MSP430 as Arch>::Address,
        context: Option<&T>,
        function_table: &HashMap<<MSP430 as Arch>::Address, ()>,
        ssa: &SSA<MSP430>) -> String {

        use analyses::static_single_assignment::cytron::Direction;
        fn render_operand<T: PartialInstructionContext>(address: <MSP430 as Arch>::Address, operand: &Operand, context: Option<&T>, ssa: &SSA<MSP430>, direction: Direction) -> String {
            fn signed_hex(num: i16) -> String {
                if num >= 0 {
                    format!("+{:#x}", num)
                } else {
                    format!("-{:#x}", -num)
                }
            }
            fn register_name(num: u8) -> &'static str {
                match num {
                    0 => "pc",
                    1 => "sp",
                    2 => "sr",
                    3 => "cg",
                     4 => "r4",   5 => "r5",   6 => "r6",   7 => "r7",
                     8 => "r8",   9 => "r9",  10 => "r10", 11 => "r11",
                    12 => "r12", 13 => "r13", 14 => "r14", 15 => "r15",
                    _ => unreachable!()
                }
            }

            fn numbered_register_name<T: PartialInstructionContext>(address: <MSP430 as Arch>::Address, reg: u8, context: Option<&T>, ssa: &SSA<MSP430>, direction: Direction) -> String {
                format!("{}_{}",
                    register_name(reg),
                    match ssa.values.get(&address).and_then(|addr_values| addr_values.get(&(msp430::Location::Register(reg), direction))) {
                        Some(data) => format!("{}", data.borrow().version()),
                        None => format!("ERR_{:?}", direction)
                    }
                )
            }

            match operand {
                Operand::Register(reg) => { numbered_register_name(address, *reg, context, ssa, direction) },
                Operand::Indexed(reg, offset) => {
                    format!("{}({})", signed_hex(*offset as i16), numbered_register_name(address, *reg, context, ssa, Direction::Read))
                },
                Operand::RegisterIndirect(reg) => {
                    format!("@{}", numbered_register_name(address, *reg, context, ssa, Direction::Read))
                },
                Operand::IndirectAutoinc(reg) => {
                    format!("@{}+", numbered_register_name(address, *reg, context, ssa, Direction::Read))
                },
                Operand::Offset(offset) => {
                    match context.and_then(|ctx| ctx.address()) {
                        Some(address) => {
                            // TODO: Uhhhh.. is this supposed to be instr len, not 2?
                            format!("{:#x}", address.wrapping_add((*offset as u16).wrapping_mul(2)).wrapping_add(2))
                        },
                        None => {
                            format!("{}(pc)", signed_hex(*offset as i16))
                        }
                    }
                },
                Operand::Symbolic(offset) => {
                    match context.and_then(|ctx| ctx.address()) {
                        Some(address) => {
                            format!("{:#x}", address.wrapping_add(*offset))
                        },
                        None => {
                            format!("{}(pc)", signed_hex(*offset as i16))
                        }
                    }
                },
                Operand::Immediate(imm) => {
                    format!("#{:#x}", imm)
                },
                Operand::Absolute(offset) => {
                    format!("&{:#x}", offset)
                },
                Operand::Const4 => {
                    "4".to_owned()
                },
                Operand::Const8 => {
                    "8".to_owned()
                },
                Operand::Const0 => {
                    "0".to_owned()
                },
                Operand::Const1 => {
                    "1".to_owned()
                },
                Operand::Const2 => {
                    "2".to_owned()
                },
                Operand::ConstNeg1 => {
                    "-1".to_owned()
                },
                Operand::Nothing => {
                    "<No Operand>".to_owned()
                }
            }
        }

        // try to recover some of the "emulated" instructions... fall back with a naive render
        match self {
            Instruction { opcode: Opcode::MOV, operands: [Operand::Const0, Operand::Const0], op_width: _ } => {
                format!("{}{}{}", color::Fg(color::Blue), "nop", color::Fg(color::Reset))
            },
            Instruction { opcode: Opcode::MOV, operands: [Operand::Const0, dest], op_width: _ } => {
                let start_color = syntaxed_render::opcode_color(Opcode::MOV);
                format!("{}{}{} {}", start_color, "clr", color::Fg(color::Reset), render_operand(address, &dest, context, ssa, Direction::Write))
            },
            Instruction { opcode: Opcode::MOV, operands: [Operand::IndirectAutoinc(1), Operand::Register(0)], op_width: Width::W } => {
                // this is a pop
                let start_color = syntaxed_render::opcode_color(Opcode::CALL);
                format!("{}{}{}", start_color, "ret", color::Fg(color::Reset))
            },
            Instruction { opcode: Opcode::MOV, operands: [Operand::IndirectAutoinc(1), dest], op_width: Width::W } => {
                // this is a pop
                let start_color = syntaxed_render::opcode_color(Opcode::PUSH);
                format!("{}{}{} {}", start_color, "pop", color::Fg(color::Reset), render_operand(address, &dest, context, ssa, Direction::Write))
            },
            Instruction { opcode: Opcode::MOV, operands: [src, Operand::Register(0)], op_width: Width::W } => {
                // br [src]
                let start_color = syntaxed_render::opcode_color(Opcode::JMP);
                format!("{}{}{} {}", start_color, "br", color::Fg(color::Reset), render_operand(address, &src, context, ssa, Direction::Read))
            }
            x @ _ => {
                let start_color = syntaxed_render::opcode_color(self.opcode);
                let mut result = format!("{}{}{}{}", start_color, self.opcode, match self.op_width {
                    Width::W => "",
                    Width::B => ".b"
                }, color::Fg(color::Reset));

                let (rw0, rw1) = match self.opcode {
                    Opcode::Invalid(_) => { (Direction::Read, Direction::Read) },
                    Opcode::CALL => { (Direction::Read, Direction::Read) },
                    Opcode::RETI => { (Direction::Read, Direction::Read) },
                    Opcode::JNE => { (Direction::Read, Direction::Read) },
                    Opcode::JEQ => { (Direction::Read, Direction::Read) },
                    Opcode::JNC => { (Direction::Read, Direction::Read) },
                    Opcode::JC => { (Direction::Read, Direction::Read) },
                    Opcode::JN => { (Direction::Read, Direction::Read) },
                    Opcode::JGE => { (Direction::Read, Direction::Read) },
                    Opcode::JL => { (Direction::Read, Direction::Read) },
                    Opcode::JMP => { (Direction::Read, Direction::Read) },
                    Opcode::MOV => { (Direction::Read, Direction::Write) },
                    Opcode::RRA => { (Direction::Read, Direction::Read) },
                    Opcode::SXT => { (Direction::Read, Direction::Read) },
                    Opcode::PUSH => { (Direction::Read, Direction::Read) },
                    Opcode::AND => { (Direction::Read, Direction::Read) },
                    Opcode::XOR => { (Direction::Read, Direction::Read) },
                    Opcode::BIT => { (Direction::Read, Direction::Read) },
                    Opcode::BIC => { (Direction::Read, Direction::Read) },
                    Opcode::RRC => { (Direction::Read, Direction::Read) },
                    Opcode::SWPB => { (Direction::Read, Direction::Read) },
                    Opcode::BIS => { (Direction::Read, Direction::Read) },
                    Opcode::ADD => { (Direction::Read, Direction::Read) },
                    Opcode::ADDC => { (Direction::Read, Direction::Read) },
                    Opcode::SUBC => { (Direction::Read, Direction::Read) },
                    Opcode::SUB => { (Direction::Read, Direction::Read) },
                    Opcode::DADD => { (Direction::Read, Direction::Read) },
                    Opcode::CMP => { (Direction::Read, Direction::Read) }
                };
                match self.operands[0] {
                    Operand::Nothing => { return result; },
                    x @ _ => {
                        result.push(' ');
                        result.push_str(&render_operand(address, &x, context, ssa, rw0));
                    }
                };
                match self.operands[1] {
                    Operand::Nothing => { return result; },
                    x @ _ => {
                        result.push(',');
                        result.push(' ');
                        result.push_str(&render_operand(address, &x, context, ssa, rw1));
                    }
                };
                result
            }
        }
    }
}

pub fn render_frame<T>(
    addr: u16,
    instr: &<MSP430 as Arch>::Instruction,
    bytes: &[u8],
    ctx: Option<&T>,
    function_table: &HashMap<u16, ()>
) where T: msp430::PartialInstructionContext {
    if let Some(comment) = ctx.and_then(|x| x.comment()) {
        println!("{:04x}: {}{}{}",
            addr,
            color::Fg(&color::Blue as &color::Color),
            comment,
            color::Fg(&color::Reset as &color::Color)
        );
    }
    if let Some(fn_dec) = function_table.get(&addr) {
        println!("      {}{}{}",
            color::Fg(&color::LightYellow as &color::Color),
            "___",
//                        fn_dec.decl_string(),
            color::Fg(&color::Reset as &color::Color)
        );
    }
    print!(
        "{:04x}: {}{} {}{} {}{}: |{}|",
        addr,
        bytes.get(0).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        bytes.get(1).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        bytes.get(2).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        bytes.get(3).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        bytes.get(4).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        bytes.get(5).map(|x| format!("{:02x}", x)).unwrap_or("  ".to_owned()),
        ctx.map(|c| c.indicator_tag()).unwrap_or(" ")
    );
}

pub fn render_instruction<T>(
    instr: &<MSP430 as Arch>::Instruction,
    ctx: Option<&T>,
    function_table: &HashMap<u16, ()>
) where T: msp430::PartialInstructionContext {
    println!(" {}", instr.render(ctx, &function_table))
}

use analyses::static_single_assignment::cytron::SSAValues;
pub fn render_instruction_with_ssa_values<T>(
    address: <MSP430 as Arch>::Address,
    instr: &<MSP430 as Arch>::Instruction,
    ctx: Option<&T>,
    function_table: &HashMap<u16, ()>,
    ssa: &SSA<MSP430>
) where
    T: msp430::PartialInstructionContext,
    <MSP430 as SSAValues>::Location: Eq + Hash,
    <MSP430 as Arch>::Address: Eq + Hash,
    <MSP430 as Arch>::Instruction: SyntaxedSSARender<MSP430, T, ()> {
    println!(" {}", instr.render_with_ssa_values(address, ctx, &function_table, ssa))
}

pub fn show_linear_with_blocks(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    cfg: &ControlFlowGraph<<MSP430 as Arch>::Address>,
    start_addr: <MSP430 as Arch>::Address,
    end_addr: <MSP430 as Arch>::Address) {
    let mut continuation = start_addr;
    while continuation < end_addr {
        // Do we have a block here?
        let block = cfg.get_block(continuation);
        // now, get_block doesn't consult if it's something we've explored
        // in the cfg or just free unused space, so let's check that...
        if cfg.graph.contains_node(block.start) {
        }

        let end = if block.end < end_addr {
            block.end
        } else {
            end_addr
        };
        // haha actually we don't do anything one way or the other.
        // so just use the end of this block as an indication of where
        // to stop linear disassembly here
        //
        // start at continuation because this linear disassembly
        // might start at the middle of a preexisting block
        show_linear(data, user_infos, continuation, end);

        // and continue on right after this block
        if block.end == 0xffff {
            break;
        }
        continuation = block.end + <MSP430 as Arch>::Address::from(1u16);
    }
}

pub fn show_linear(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    start_addr: <MSP430 as Arch>::Address,
    end_addr: <MSP430 as Arch>::Address) {
    let mut continuation = start_addr;
    while continuation < end_addr {
        let mut invalid: yaxpeax_msp430_mc::Instruction = Instruction::blank();
        let mut iter = data.instructions_spanning::<yaxpeax_msp430_mc::Instruction>(continuation, end_addr);
        let mut cont = true;
        loop {
            let (address, instr) = match iter.next() {
                Some((address, instr)) => {
                    (address, instr)
                },
                None => {
                    invalid = yaxpeax_msp430_mc::Instruction {
                        opcode: Opcode::Invalid(
                            (data[(continuation as usize)] as u16) |
                            ((data[(continuation as usize) + 1] as u16) << 8)
                        ),
                        op_width: Width::W,
                        operands: [Operand::Nothing, Operand::Nothing]
                    };
                    continuation += invalid.len() as u16;
                    break; // ... the iterator doesn't distinguish
                           // between None and Invalid ...
                    cont = false;
                    (continuation, &invalid)
                }
            };

            let mut computed = msp430::ComputedContext {
                address: Some(address),
                comment: None
            };
            render_frame(
                address,
                instr,
                &data[(address as usize)..(address as usize + instr.len() as usize)],
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new()
            );
            render_instruction(
                instr,
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new()
            );
            continuation += instr.len() as u16;
            if !cont { break; }
        }
    }
}

pub fn show_functions(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    cfg: &ControlFlowGraph<<MSP430 as Arch>::Address>,
    addr: <MSP430 as Arch>::Address) {

}

pub fn show_function_by_ssa(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    cfg: &ControlFlowGraph<<MSP430 as Arch>::Address>,
    addr: <MSP430 as Arch>::Address,
    ssa: &SSA<MSP430>) {

    let fn_graph = cfg.get_function::<()>(addr, &HashMap::new());

    let mut blocks: Vec<<MSP430 as Arch>::Address> = fn_graph.blocks.iter().map(|x| x.start).collect();
    blocks.sort();

    for blockaddr in blocks.iter() {
        let block = cfg.get_block(*blockaddr);
        if block.start == 0x00 { continue; }
        /*
        println!("Basic block --\n  start: {:#x}\n  end:   {:#x}", block.start, block.end);
        println!("  next:");
        for neighbor in cfg.graph.neighbors(block.start) {
            println!("    {:#x}", neighbor);
        }
        */

        if ssa.phi.contains_key(&block.start) {
            println!("Phi: {:?}", ssa.phi[&block.start].keys());
        }

        let mut iter = data.instructions_spanning::<yaxpeax_msp430_mc::Instruction>(block.start, block.end);
//                println!("Block: {:#04x}", next);
//                println!("{:#04x}", block.start);
        while let Some((address, instr)) = iter.next() {
            let mut computed = msp430::ComputedContext {
                address: Some(address),
                comment: None
            };
            render_frame(
                address,
                instr,
                &data[(address as usize)..(address as usize + instr.len() as usize)],
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new()
            );
            render_instruction_with_ssa_values(
                address,
                instr,
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new(),
                ssa
            );
            if ssa.values.contains_key(&address) {
                // println!("  values: {:?}", ssa.values[&address]);
            }
           //println!("{:#04x}: {}", address, instr);
        }
//        println!("------------------------------");
    }
}

pub fn show_function(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    cfg: &ControlFlowGraph<<MSP430 as Arch>::Address>,
    addr: <MSP430 as Arch>::Address) {

    let fn_graph = cfg.get_function::<()>(addr, &HashMap::new());

    let mut blocks: Vec<<MSP430 as Arch>::Address> = fn_graph.blocks.iter().map(|x| x.start).collect();
    blocks.sort();

    for blockaddr in blocks.iter() {
        let block = cfg.get_block(*blockaddr);
        if block.start == 0x00 { continue; }
//        println!("Showing block: {:#x}-{:#x} for {:#x}", block.start, block.end, *blockaddr);
//        continue;
        let mut iter = data.instructions_spanning::<yaxpeax_msp430_mc::Instruction>(block.start, block.end);
//                println!("Block: {:#04x}", next);
//                println!("{:#04x}", block.start);
        while let Some((address, instr)) = iter.next() {
            let mut computed = msp430::ComputedContext {
                address: Some(address),
                comment: None
            };
            render_frame(
                address,
                instr,
                &data[(address as usize)..(address as usize + instr.len() as usize)],
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new()
            );
            render_instruction(
                instr,
                Some(&msp430::MergedContext {
                    user: user_infos.get(&address),
                    computed: Some(&computed)
                }),
                &HashMap::new()
            );
           //println!("{:#04x}: {}", address, instr);
        }
    }
}

pub fn show_block(
    data: &[u8],
    user_infos: &HashMap<<MSP430 as Arch>::Address, msp430::PartialContext>,
    cfg: &ControlFlowGraph<<MSP430 as Arch>::Address>,
    block: &BasicBlock<<MSP430 as Arch>::Address>) {

    println!("Basic block --\n  start: {:#x}\n  end:   {:#x}", block.start, block.end);
    println!("  next:");
    for neighbor in cfg.graph.neighbors(block.start) {
        println!("    {:#x}", neighbor);
    }
    let mut iter = data.instructions_spanning::<yaxpeax_msp430_mc::Instruction>(block.start, block.end);
    while let Some((address, instr)) = iter.next() {
        let mut computed = msp430::ComputedContext {
            address: Some(address),
            comment: None
        };
        render_frame(
            address,
            instr,
            &data[(address as usize)..(address as usize + instr.len() as usize)],
            Some(&msp430::MergedContext {
                user: user_infos.get(&address),
                computed: Some(&computed)
            }),
            &HashMap::new()
        );
        render_instruction(
            instr,
            Some(&msp430::MergedContext {
                user: user_infos.get(&address),
                computed: Some(&computed)
            }),
            &HashMap::new()
        );
        use analyses::control_flow::Determinant;
        println!("Control flow: {:?}", instr.control_flow(user_infos.get(&address)));
    }
}
