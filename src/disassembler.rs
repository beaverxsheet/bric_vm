use crate::{
    BError,
    vm::{AluInstruction, BIT_15, Register},
};
use std::fmt::Write;

/// Disassembles an instruction into a string
pub fn disassemble_inst(instruction: &u16, output: &mut String) -> Result<(), BError> {
    // if the highest bit is 0 we have an ALU instruction
    if BIT_15 & instruction == 0 {
        let alu_inst = AluInstruction(*instruction);
        // parse the instruction into control bits
        let op = alu_inst.get_op();
        let u = alu_inst.get_u();
        let eq = alu_inst.get_eq();
        let lt = alu_inst.get_lt();
        let gt = alu_inst.get_gt();
        let source = Register::try_from(alu_inst.get_source()).unwrap();
        let target = Register::try_from(alu_inst.get_target()).unwrap();
        let zx = alu_inst.get_zx();
        let sw = alu_inst.get_sw();

        // write output
        match target {
            Register::None => {}
            other => write!(output, "{} = ", other).map_err(|e| BError::IoError(e.to_string()))?,
        }

        // write source + switch
        let (mut x, y) = if sw {
            ("A".to_string(), source.to_string())
        } else {
            (source.to_string(), "A".to_string())
        };

        // write zero
        if zx {
            x = "0".to_string()
        };

        // write instruction mnemonic
        let op = op | if u { 0b1000 } else { 0 };
        match op {
            0 => write!(output, "and, {y}, {x}"),
            1 => write!(output, "or, {y}, {x}"),
            2 => write!(output, "xor, {y}, {x}"),
            3 => write!(output, "not, {x}"),
            4 => write!(output, "lsl, {x}"),
            5 => write!(output, "lsr, {x}"),
            6 => write!(output, "rol, {x}"),
            7 => write!(output, "ror, {x}"),
            8 => write!(output, "add, {x}, {y}"),
            9 => write!(output, "sub, {x}, {y}"),
            10 => write!(output, "inc, {x}"),
            11 => write!(output, "dec, {x}"),
            12 => write!(output, "asr, {x}"),
            _ => Ok(()),
        }
        .map_err(|e| BError::IoError(e.to_string()))?;

        match if lt { 0b100 } else { 0 } | if eq { 0b10 } else { 0 } | if gt { 1 } else { 0 } {
            0b111 => write!(output, "; JMP"),
            0b110 => write!(output, "; JLE"),
            0b011 => write!(output, "; JGE"),
            0b001 => write!(output, "; JGT"),
            0b010 => write!(output, "; JEQ"),
            0b100 => write!(output, "; JLT"),
            0b101 => write!(output, "; JNE"),
            _ => Ok(()),
        }
        .map_err(|e| BError::IoError(e.to_string()))?;
    } else {
        write!(output, "A = {}", instruction & (!BIT_15))
            .map_err(|e| BError::IoError(e.to_string()))?;
    }
    Ok(())
}

/// Disassembles every word of the ROM (including consts)
/// if `lines` is `true` the disassembly includes the address of the instructions
pub fn disassemble(input: &[u16], lines: bool) -> Result<String, BError> {
    let mut out = String::new();
    for (idx, instruction) in input.iter().enumerate() {
        if lines {
            write!(&mut out, "{:#06x}:\t", idx).map_err(|e| BError::IoError(e.to_string()))?;
        }
        disassemble_inst(instruction, &mut out)?;
        write!(&mut out, "\n").map_err(|e| BError::IoError(e.to_string()))?;
    }
    Ok(out)
}
