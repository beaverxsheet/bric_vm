use std::cell::LazyCell;

use regex::Regex;

use crate::{BError, util::number_literal_to_u16, vm::VmDescription};

/// Processes the `\[macro\]` section of a .basm file
/// Reads macros and definitions and copies them to the appropriate locations in the \[text\] section.
pub mod preprocessor {
    use regex::{Captures, Regex};
    use std::cell::LazyCell;
    use std::collections::{HashMap, HashSet};

    use crate::BError;
    use crate::util::number_literal_to_u16;

    // TODO: test macros with multiple arguments

    /// Represents a macro, includes the replacements needed when inserting with arguments
    struct Macro {
        // It would be more efficient to precompute the replacements
        regexes: Vec<Regex>,
        text: String,
    }

    impl Macro {
        /// Makes the regexes for replacing the arguments. The args MUST match `[a-zA-Z\.\_]+`.
        fn new(args: Vec<String>, text: String) -> Self {
            let mut regexes = Vec::new();
            for arg in args {
                let clean = arg.replace(".", "\\.").replace("_", "\\_");
                let pattern = format!("=\\s({clean})(?<right>\\s*[\\s|;])");
                // Should not panic. We sanitize pretty well.
                regexes.push(Regex::new(&pattern).unwrap());
            }
            Self { regexes, text }
        }

        /// Replaces the args
        /// Super dirty
        fn replace_args(&self, args: Vec<String>) -> String {
            let mut out_text = self.text.clone();
            for (arg_idx, rgx) in self.regexes.iter().enumerate() {
                out_text = rgx
                    .replace_all(&out_text, |caps: &Captures| {
                        format!("= {}{}", args[arg_idx], &caps["right"])
                    })
                    .to_string();
            }
            out_text
        }
    }

    /// All allowable register strings
    const REGISTERS: LazyCell<HashSet<String>> = LazyCell::new(|| {
        HashSet::from_iter(
            ["A", "*A", "D", "E", "F", "G", "H"]
                .iter()
                .map(|v| v.to_string()),
        )
    });
    /// Keywords in .basm
    const KEYWORDS: LazyCell<HashSet<String>> = LazyCell::new(|| {
        HashSet::from_iter(["begin", "end", "label"].iter().map(|v| v.to_string()))
    });
    /// Instructions in .basm
    const INSTRUCTIONS: LazyCell<HashSet<String>> = LazyCell::new(|| {
        HashSet::from_iter(
            [
                "and", "or", "xor", "add", "sub", "inc", "dec", "not", "lsl", "lsr", "asr", "rol",
                "ror",
            ]
            .iter()
            .map(|v| v.to_string()),
        )
    });

    /// Matches names for defines, labels and macros
    pub(crate) const RE_NAME: LazyCell<Regex> =
        LazyCell::new(|| Regex::new(r"^[a-zA-Z\.\_]+$").unwrap());
    /// Matches hex, bin, and dec number literals
    pub(crate) const RE_NUMBER_LIT: LazyCell<Regex> =
        LazyCell::new(|| Regex::new(r"^(0x[0-9a-fA-F]+|0b[01]+|[0-9]+)?$").unwrap());

    /// Do the pre-processing step. This replaces macros and defines in the \[text\] section
    pub fn preprocess(macros_text: &str, code: &str) -> Result<(String, usize), BError> {
        // This is incredibly inefficient because we go through the entire file for each step instead of going through only once or twice.
        // A more efficient lexer might be logos.
        // This only runs on the `\[macros\]` section so it shouldn't be too bad...

        let mut macros: HashMap<String, Macro> = HashMap::new();
        let mut defines = HashMap::<String, u16>::new();

        let mut in_macro = false; // are we currently in a macro definition? if so the below variables are read
        let mut current_macro_name = None;
        let mut current_macro_args = None;
        let mut current_macro_text = None;
        // allows us to efficiently count the number of lines, because this should get optimized away... (hopefully)
        let mut mline = 0;
        for (line_no, line) in macros_text.lines().enumerate() {
            mline = line_no;
            if in_macro {
                // If in_macro is true current_ variables should be Some

                // `end` signifies the end of a macro definition
                if line.trim_start().starts_with("end") {
                    let nmacro =
                        Macro::new(current_macro_args.unwrap(), current_macro_text.unwrap());
                    macros.insert(current_macro_name.unwrap(), nmacro);
                    in_macro = false;
                    current_macro_args = None;
                    current_macro_text = None;
                    current_macro_name = None;
                } else {
                    if let Some(ref mut cmt) = current_macro_text {
                        cmt.push_str(line);
                        cmt.push('\n');
                    }
                }

                continue;
            }

            // tokenize lines
            let mut tokens = line.split_whitespace();
            if let Some(mode) = tokens.next() {
                match mode {
                    // define is built like: `define name value`, where value is a number literal
                    "define" => {
                        let define_name = tokens.next().ok_or(BError::AsmParseError(format!(
                            "error on line {line_no}: {line}\nNo name for define"
                        )))?;
                        let define_value = tokens.next().ok_or(BError::AsmParseError(format!(
                            "error on line {line_no}: {line}\nNo value for define"
                        )))?;

                        if !RE_NAME.is_match(define_name) {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nInvalid define name"
                            )));
                        }

                        if !RE_NUMBER_LIT.is_match(define_value) {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nInvalid number literal"
                            )));
                        }

                        // check whether the name is already taken
                        if REGISTERS.contains(define_name)
                            | KEYWORDS.contains(define_name)
                            | INSTRUCTIONS.contains(define_name.to_lowercase().as_str())
                            | defines.contains_key(define_name)
                        {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nThe name {define_name} is already in use"
                            )));
                        }

                        let number = number_literal_to_u16(define_value).map_err( |_| BError::AsmParseError(
                                format!("error on line {line_no}: {line}\nCan't parse {define_value} to a number."),
                            ))?;

                        defines.insert(define_name.to_string(), number);

                        // error if more tokens in line
                        if let Some(_) = tokens.next() {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nThe line contains unnecessary text"
                            )));
                        }
                    }
                    // begin syntax for beginning a macro `begin name(arg1, ...)`, if only one arg the comma can be left out
                    "begin" => {
                        let macro_name = tokens.next().ok_or(BError::AsmParseError(format!(
                            "error on line {line_no}: {line}\nNo name for macro"
                        )))?;

                        // check whether the name is already taken
                        if REGISTERS.contains(macro_name)
                            | KEYWORDS.contains(macro_name)
                            | INSTRUCTIONS.contains(macro_name.to_lowercase().as_str())
                            | defines.contains_key(macro_name)
                            | macros.contains_key(macro_name)
                        {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nThe name {macro_name} is already in use"
                            )));
                        }

                        let args = tokens.collect::<Vec<&str>>().join("");
                        if !(args.starts_with("(") & args.ends_with(")")) {
                            return Err(BError::AsmParseError(format!(
                                "error on line {line_no}: {line}\nInvalid arguments or unnecessary text"
                            )));
                        }
                        let mut arg_names = Vec::new();
                        for arg in args[1..args.len() - 1].split(",") {
                            arg_names.push(arg.to_string())
                        }
                        current_macro_name = Some(macro_name.to_string());
                        current_macro_args = Some(arg_names);
                        current_macro_text = Some(String::new());
                        in_macro = true;
                    }
                    // comments
                    "#" => {
                        continue;
                    }
                    _ => {
                        return Err(BError::AsmParseError(format!(
                            "error on line {line_no}: {line}\nInvalid Text in `[macros]`"
                        )));
                    }
                }
            }
        }

        let mut out = code.to_string();

        // replace macros
        for (name, mac) in macros {
            let clean = name.replace(".", "\\.").replace("_", "\\_");
            let pattern = format!("(?m)^\\s*{clean}\\s*\\((.+)\\)\\s*$");
            // should be fine. sanitization above
            let rgx = Regex::new(&pattern).unwrap();
            loop {
                // we do this to not have a borrow on out while we modify it
                // the two variables are used. the linter is wrong
                #[allow(unused)]
                let mut range = None;
                #[allow(unused)]
                let mut args = None;
                if let Some(m) = rgx.find(&out) {
                    if let Some(cap) = rgx.captures(m.as_str()) {
                        args = Some(
                            cap[1]
                                .split(",")
                                .map(|v| v.trim().to_string())
                                .collect::<Vec<String>>(),
                        );
                        range = Some(m.range());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
                let repl_text = mac.replace_args(args.unwrap());
                out.replace_range(range.unwrap(), &repl_text);
            }
        }

        // Replaces the defines
        // Super dirty
        for (name, value) in defines {
            let clean = name.replace(".", "\\.").replace("_", "\\_");
            let pattern = format!("=\\s({clean})(?<right>\\s*[\\s|;])");
            // should be fine. sanitization
            let rgx = Regex::new(&pattern).unwrap();
            out = rgx
                .replace_all(&out, |caps: &Captures| {
                    format!("= {}{}", value, &caps["right"])
                })
                .to_string()
        }

        Ok((out, mline))
    }
}

/// Processes the \[text\] section of a .basm file and returns the logic part of ROM
pub mod text_processor {
    use crate::{
        BError,
        assembler::preprocessor::{RE_NAME, RE_NUMBER_LIT},
        util::number_literal_to_u16,
        vm::{AluInstruction, Instruction, Register},
    };
    use std::collections::HashMap;

    /// Jump conditions
    #[derive(Debug)]
    enum Jumps {
        None,
        Jlt,
        Jeq,
        Jgt,
        Jle,
        Jge,
        Jmp,
        Jne,
    }

    impl Jumps {
        /// Set up the jump bits of the instruction depending on the Jump conditions
        fn set_alu_inst(&self, alu_inst: &mut AluInstruction) {
            match self {
                Self::None => {
                    alu_inst.set_eq(false);
                    alu_inst.set_gt(false);
                    alu_inst.set_lt(false);
                }
                Self::Jlt => {
                    alu_inst.set_eq(false);
                    alu_inst.set_gt(false);
                    alu_inst.set_lt(true);
                }
                Self::Jeq => {
                    alu_inst.set_eq(true);
                    alu_inst.set_gt(false);
                    alu_inst.set_lt(false);
                }
                Self::Jgt => {
                    alu_inst.set_eq(false);
                    alu_inst.set_gt(true);
                    alu_inst.set_lt(false);
                }
                Self::Jle => {
                    alu_inst.set_eq(true);
                    alu_inst.set_gt(false);
                    alu_inst.set_lt(true);
                }
                Self::Jge => {
                    alu_inst.set_eq(true);
                    alu_inst.set_gt(false);
                    alu_inst.set_lt(true);
                }
                Self::Jmp => {
                    alu_inst.set_eq(true);
                    alu_inst.set_gt(true);
                    alu_inst.set_lt(true);
                }
                Self::Jne => {
                    alu_inst.set_eq(false);
                    alu_inst.set_gt(true);
                    alu_inst.set_lt(true);
                }
            }
        }

        fn parse_str(input: &str) -> Option<Self> {
            match input {
                "JLT" => Some(Self::Jlt),
                "JGT" => Some(Self::Jgt),
                "JEQ" => Some(Self::Jeq),
                "JLE" => Some(Self::Jle),
                "JGE" => Some(Self::Jge),
                "JMP" => Some(Self::Jmp),
                "JNE" => Some(Self::Jne),
                _ => None,
            }
        }
    }

    /// inputs for X in the ALU
    enum XOps {
        A,
        MA,
        D,
        E,
        F,
        G,
        H,
        Zero,
    }

    impl XOps {
        fn from_str(input: &str) -> Option<Self> {
            match input {
                "A" => Some(Self::A),
                "*A" => Some(Self::MA),
                "D" => Some(Self::D),
                "E" => Some(Self::E),
                "F" => Some(Self::F),
                "G" => Some(Self::G),
                "H" => Some(Self::H),
                "0" => Some(Self::Zero),
                _ => None,
            }
        }
    }

    /// Supported mnemonics in the assembler
    enum Cmds {
        And,
        Or,
        Xor,
        Add,
        Sub,
        Inc,
        Dec,
        Not,
        Lsl,
        Lsr,
        Asr,
        Rol,
        Ror,
    }

    impl Cmds {
        fn from_str(input: &str) -> Option<Self> {
            match input.to_lowercase().as_str() {
                "and" => Some(Self::And),
                "or" => Some(Self::Or),
                "xor" => Some(Self::Xor),
                "add" => Some(Self::Add),
                "sub" => Some(Self::Sub),
                "inc" => Some(Self::Inc),
                "dec" => Some(Self::Dec),
                "not" => Some(Self::Not),
                "lsl" => Some(Self::Lsl),
                "lsr" => Some(Self::Lsr),
                "asr" => Some(Self::Asr),
                "rol" => Some(Self::Ror),
                "ror" => Some(Self::Rol),
                _ => None,
            }
        }

        /// Tells us how many arguments to expect for a given mnemonic
        fn arg_num(&self) -> usize {
            match self {
                Self::And => 2,
                Self::Or => 2,
                Self::Xor => 2,
                Self::Add => 2,
                Self::Sub => 2,
                Self::Inc => 1,
                Self::Dec => 1,
                Self::Not => 1,
                Self::Lsl => 1,
                Self::Lsr => 1,
                Self::Asr => 1,
                Self::Ror => 1,
                Self::Rol => 1,
            }
        }
    }

    /// Intermediate format between first and second pass
    pub struct AssemblerOutput {
        pub rom: Vec<u16>,
        pub label_definitions: HashMap<String, usize>,
        pub label_uses: HashMap<String, Vec<usize>>,
        pub rom_lines: usize,
    }

    /// Parse both operands of a two operand mnemonic into source, switch and zero fields
    fn parse_two(a: &str, b: &str) -> Result<(XOps, bool, bool), ()> {
        if a == "0" {
            if let Some(xop) = XOps::from_str(b) {
                return Ok((xop, true, true));
            }
        }
        if a == "A" {
            if let Some(xop) = XOps::from_str(b) {
                return Ok((xop, true, false));
            }
        }
        if b == "A" {
            if let Some(xop) = XOps::from_str(a) {
                return Ok((xop, false, false));
            }
        }
        Err(())
    }

    /// Set the source in the instruction
    fn set_source(x: XOps, inst: &mut AluInstruction) {
        match x {
            XOps::Zero => inst.set_zx(true),
            XOps::A => inst.set_source(Register::A as u8),
            XOps::MA => inst.set_source(Register::MA as u8),
            XOps::D => inst.set_source(Register::D as u8),
            XOps::E => inst.set_source(Register::E as u8),
            XOps::F => inst.set_source(Register::F as u8),
            XOps::G => inst.set_source(Register::G as u8),
            XOps::H => inst.set_source(Register::H as u8),
        };
    }

    /// Assemble the text section
    /// not very efficient but okay
    /// TODO: we don't check the label names in this function against the list of keywords and registers
    pub fn assemble(code: String, code_offset: usize) -> Result<AssemblerOutput, BError> {
        let mut label_definitions = HashMap::new(); // where the labels are defined
        let mut label_uses: HashMap<String, Vec<usize>> = HashMap::new(); // where the labels are used (if we know yet)
        let mut mem = Vec::new(); // output memory
        // allows us to efficiently count the lines, as this should get optimized away... (hopefully)
        let mut cline = 0;
        for (code_idx, line) in code.lines().enumerate() {
            cline = code_idx;
            let trline = line.trim();
            // comment
            if trline.starts_with("#") {
                continue;
            }
            // empty line
            if matches!(trline, "") {
                continue;
            }
            // label for jumps
            if trline.starts_with("label") {
                let terr = Err(BError::AsmParseError(format!(
                    "error on line {}: {}\nincorrect label",
                    code_idx + code_offset,
                    line
                )));

                if !trline.ends_with(":") {
                    return terr;
                }

                let label = trline[5..trline.len() - 1].trim();
                // check label format
                if !RE_NAME.is_match(label) {
                    return terr;
                }
                if label_definitions.contains_key(label) {
                    return Err(BError::AsmParseError(format!(
                        "error on line {}: {}\nlabel already in use",
                        code_idx + code_offset,
                        line
                    )));
                }
                // this is okay as we always add an extra instruction to the end
                label_definitions.insert(label.to_string(), mem.len());
                continue;
            }
            if trline == "JMP" {
                // Always Jump
                mem.push(Instruction::Alu(AluInstruction(0b0000000000000111)).to_u16());
                continue;
            }

            let mut seen_eq = false;
            let mut seen_sc = false;
            let mut parts = Vec::new();
            let mut current_start = 0;
            // tokenize and find out which type of line this is
            for (idx, c) in trline.chars().enumerate() {
                match c {
                    '=' => {
                        if seen_eq | seen_sc {
                            return Err(BError::AsmParseError(format!(
                                "error on line {}: {}\nsections wrong",
                                code_idx + code_offset,
                                line
                            )));
                        }
                        parts.push(&trline[current_start..idx]);
                        current_start = idx + 1;
                        seen_eq = true;
                    }
                    ';' => {
                        if seen_sc {
                            return Err(BError::AsmParseError(format!(
                                "error on line {}: {}\nsections wrong",
                                code_idx + code_offset,
                                line
                            )));
                        }
                        parts.push(&trline[current_start..idx]);
                        current_start = idx + 1;
                        seen_sc = true;
                    }
                    _ => {}
                }
            }

            parts.push(&trline[current_start..]);

            let mut parts_slice = &parts[..];

            // parse target
            let target = if seen_eq {
                let tgt_str = parts_slice[0];
                parts_slice = &parts_slice[1..];
                Register::from_str(tgt_str.trim()).ok_or(BError::AsmParseError(format!(
                    "error on line {}: {}\nimproper target",
                    code_idx + code_offset,
                    line
                )))?
            } else {
                Register::None
            };

            // parse jump condition
            let jump = if seen_sc {
                let jmp_str = parts_slice
                    .get(1)
                    .ok_or(BError::AsmParseError(format!(
                        "error on line {}: {}\nconditional jump without computation",
                        code_idx + code_offset,
                        line
                    )))?
                    .trim();
                parts_slice = &parts_slice[..parts_slice.len() - 1];
                Jumps::parse_str(&jmp_str).ok_or(BError::AsmParseError(format!(
                    "error on line {}: {}\nimproper jump",
                    code_idx + code_offset,
                    line
                )))?
            } else {
                Jumps::None
            };

            // parse operation
            let operation = {
                if parts_slice.len() != 1 {
                    return Err(BError::AsmParseError(format!(
                        "error on line {}: {}\nno operation",
                        code_idx + code_offset,
                        line
                    )));
                }
                let mut operands = parts_slice[0].split(",");
                let cmd_or_lit = operands
                    .next()
                    .ok_or(BError::AsmParseError(format!(
                        "error on line {}: {}\nno operation or number",
                        code_idx + code_offset,
                        line
                    )))?
                    .trim();

                if let Some(cmd) = Cmds::from_str(cmd_or_lit) {
                    let inputs: Vec<&str> = operands.map(|v| v.trim()).collect();
                    // check whether we have enough operands
                    if !inputs.len() == cmd.arg_num() {
                        return Err(BError::AsmParseError(format!(
                            "error on line {}: {}\nnot enough arguments for operation",
                            code_idx + code_offset,
                            line
                        )));
                    };
                    // create our ALU instruction
                    let mut inst = AluInstruction(0);
                    match cmd {
                        h @ Cmds::Add | h @ Cmds::Sub => {
                            let (x, sw, zx) = parse_two(inputs[0], inputs[1]).map_err(|_| {
                                BError::AsmParseError(format!(
                                    "error on line {}: {}\none or both operands invalid",
                                    code_idx + code_offset,
                                    line
                                ))
                            })?;

                            if matches!(x, XOps::Zero) {
                                Err(BError::AsmParseError(format!(
                                    "error on line {}: {}\nright operand may not be zero here",
                                    code_idx + code_offset,
                                    line
                                )))?;
                            }
                            inst.set_sw(sw);
                            set_source(x, &mut inst);
                            inst.set_zx(zx);
                            inst.set_op(match h {
                                Cmds::Add => 0b000,
                                Cmds::Sub => 0b001,
                                // can't be reached
                                _ => 0b000,
                            });

                            jump.set_alu_inst(&mut inst);
                            inst.set_target(target as u8);
                            inst.set_u(true);
                            Instruction::Alu(inst)
                        }
                        h @ Cmds::Asr | h @ Cmds::Inc | h @ Cmds::Dec => {
                            let x =
                                XOps::from_str(inputs[0]).ok_or(BError::AsmParseError(format!(
                                    "error on line {}: {}\ninvalid operand {}",
                                    code_idx + code_offset,
                                    line,
                                    inputs[0]
                                )))?;
                            set_source(x, &mut inst);
                            jump.set_alu_inst(&mut inst);
                            inst.set_op(match h {
                                Cmds::Asr => 0b100,
                                Cmds::Inc => 0b010,
                                Cmds::Dec => 0b011,
                                // can't be reached
                                _ => 0b000,
                            });
                            inst.set_u(true);
                            inst.set_target(target as u8);
                            Instruction::Alu(inst)
                        }
                        h @ Cmds::And | h @ Cmds::Or | h @ Cmds::Xor => {
                            let (x, sw, zx) = parse_two(inputs[0], inputs[1]).map_err(|_| {
                                BError::AsmParseError(format!(
                                    "error on line {}: {}\none or both operands invalid",
                                    code_idx + code_offset,
                                    line
                                ))
                            })?;

                            if matches!(x, XOps::Zero) {
                                Err(BError::AsmParseError(format!(
                                    "error on line {}: {}\nright operand may not be zero here",
                                    code_idx + code_offset,
                                    line
                                )))?;
                            }
                            inst.set_sw(sw);
                            set_source(x, &mut inst);
                            inst.set_zx(zx);
                            match h {
                                Cmds::And => inst.set_op(0b000),
                                Cmds::Or => inst.set_op(0b001),
                                Cmds::Xor => inst.set_op(0b010),
                                _ => {}
                            }
                            jump.set_alu_inst(&mut inst);
                            inst.set_target(target as u8);
                            inst.set_u(false);
                            Instruction::Alu(inst)
                        }
                        h @ Cmds::Not
                        | h @ Cmds::Lsl
                        | h @ Cmds::Lsr
                        | h @ Cmds::Rol
                        | h @ Cmds::Ror => {
                            let x =
                                XOps::from_str(inputs[0]).ok_or(BError::AsmParseError(format!(
                                    "error on line {}: {}\ninvalid operand {}",
                                    code_idx + code_offset,
                                    line,
                                    inputs[0]
                                )))?;
                            set_source(x, &mut inst);
                            jump.set_alu_inst(&mut inst);
                            inst.set_op(match h {
                                Cmds::Not => 0b011,
                                Cmds::Lsl => 0b100,
                                Cmds::Lsr => 0b101,
                                Cmds::Rol => 0b110,
                                Cmds::Ror => 0b111,
                                _ => 0, // impossible to reach
                            });
                            inst.set_u(false);
                            inst.set_target(target as u8);
                            Instruction::Alu(inst)
                        }
                    }
                } else if RE_NUMBER_LIT.is_match(cmd_or_lit) {
                    // we have a number here -> literal to put in A
                    let value = number_literal_to_u16(cmd_or_lit).map_err(|_| {
                        BError::AsmParseError(format!(
                            "error on line {}: {}\nunable to parse {} as a number",
                            code_idx + code_offset,
                            line,
                            cmd_or_lit
                        ))
                    })?;
                    if value > 0x7fff {
                        Err(BError::AsmParseError(format!(
                            "error on line {}: {}\n{} is to large",
                            code_idx + code_offset,
                            line,
                            value
                        )))?;
                    }
                    Instruction::Data(value)
                } else {
                    // is the element a label?
                    if let Some(uselist) = label_uses.get_mut(cmd_or_lit) {
                        uselist.push(mem.len());
                    } else {
                        if !RE_NAME.is_match(cmd_or_lit) {
                            Err(BError::AsmParseError(format!(
                                "error on line {}: {}\n cant parse {}.",
                                code_idx + code_offset,
                                line,
                                cmd_or_lit
                            )))?;
                        }
                        label_uses.insert(cmd_or_lit.to_string(), vec![mem.len()]);
                    }
                    Instruction::Data(0)
                }
            };
            let v = operation.to_u16();
            mem.push(v);
        }
        // make sure there is always a last instruction incase there is a label at the very end
        mem.push(Instruction::Data(0).to_u16());

        // there will be problems after a length of 0x7fff
        if mem.len() > 0xffff {
            return Err(BError::AsmParseError(format!(
                "your program is to large: {} words",
                mem.len()
            )));
        }

        Ok(AssemblerOutput {
            rom: mem,
            label_definitions,
            label_uses,
            rom_lines: cline,
        })
    }
}

/// Process the \[const\] section of a .basm file
pub mod const_processor {
    use crate::{
        BError,
        assembler::{
            preprocessor::{RE_NAME, RE_NUMBER_LIT},
            text_processor::AssemblerOutput,
        },
        util::number_literal_to_u16,
        vm::VmDescription,
    };

    /// build the const section in ROM,
    /// replace labels to consts and then build a VmDescription which maps consts to the `mount_position`
    /// `const_offset` is the line number of the \[const\] label
    /// TODO: we don't check label names here either
    pub fn find_and_place(
        asm: AssemblerOutput,
        constants: &str,
        const_offset: usize,
        mount_position: u16,
    ) -> Result<VmDescription, BError> {
        let mut label_definitions = asm.label_definitions;
        let mut mem = asm.rom;
        // Compute the amount of bytes we need to align the memory to the next 16 byte boundary
        let more = 0xf - (mem.len() % 0x10);
        mem.append(&mut vec![0u16; more]);

        let consts_start = mem.len();
        let mut consts_amount = 0;
        for (line_idx, line) in constants.lines().enumerate() {
            match line.trim() {
                // label
                s if s.starts_with("label") => {
                    let terr = Err(BError::AsmParseError(format!(
                        "error on line {}: {}\nincorrect label",
                        line_idx + const_offset,
                        line
                    )));

                    if !s.ends_with(":") {
                        return terr;
                    }

                    let label = s[5..s.len() - 1].trim();
                    if !RE_NAME.is_match(label) {
                        return terr;
                    }
                    if label_definitions.contains_key(label) {
                        return Err(BError::AsmParseError(format!(
                            "error on line {}: {}\nlabel already in use",
                            line_idx + const_offset,
                            line
                        )));
                    }
                    // this is okay as we always add an extra instruction to the end
                    label_definitions
                        .insert(label.to_string(), mount_position as usize + consts_amount);
                }
                // const memory
                s if s.starts_with("M") => {
                    let mut parts = s.split("=");
                    // We know there is at least one element in the split
                    parts.next();
                    if let Some(number) = parts.next() {
                        let tnum = number.trim();
                        if !RE_NUMBER_LIT.is_match(tnum) {
                            return Err(BError::AsmParseError(format!(
                                "error on line {}: {}\ninvalid number {}",
                                line_idx + const_offset,
                                line,
                                tnum
                            )));
                        }
                        let value = number_literal_to_u16(tnum).map_err(|_| {
                            BError::AsmParseError(format!(
                                "error on line {}: {}\ninvalid number {}",
                                line_idx + const_offset,
                                line,
                                tnum
                            ))
                        })?;
                        mem.push(value);
                        consts_amount += 1;
                    }
                }
                // comment
                s if s.starts_with("#") => {}
                "" => {}
                _ => {
                    return Err(BError::AsmParseError(format!(
                        "error on line {}: {}\nonly comments, labels and memory allowed",
                        line_idx + const_offset,
                        line
                    )));
                }
            }
        }

        let memlen = mem.len();
        // as said before we already get problems if memlen > 0x7fff
        if memlen > 0xffff {
            return Err(BError::AsmParseError(format!(
                "your program is to large: {} words",
                memlen
            )));
        }

        // second pass
        for (name, positions) in asm.label_uses {
            // we make sure that label_definitions contains our name during construction
            let value = label_definitions[&name];
            for pos in positions.iter() {
                if value > 0x7fff {
                    Err(BError::AsmParseError(format!(
                        "error when inserting labels: {} is to large, you may have to long of a program",
                        value
                    )))?;
                }
                // we make sure the memory has appropriate length before
                let mpos = mem.get_mut(*pos).unwrap();
                *mpos |= value as u16;
            }
        }

        Ok(VmDescription {
            rom: mem,
            rom_mappings: vec![(
                consts_start as u16,
                (memlen - consts_start) as u16,
                mount_position,
            )],
            ..Default::default()
        })
    }
}

// regex to match section labels
const RE_MACROS: LazyCell<Regex> = LazyCell::new(|| Regex::new(r"(?m)^\s*\[macros\]\s*$").unwrap());
const RE_TEXT: LazyCell<Regex> = LazyCell::new(|| Regex::new(r"(?m)^\s*\[text\]\s*$").unwrap());
const RE_CONSTS: LazyCell<Regex> = LazyCell::new(|| {
    Regex::new(r"(?m)^\s*\[consts\s+(?<number>0x[0-9a-fA-F]+|0b[01]+|[0-9]+)?\]\s*$").unwrap()
});

/// Runs the entire assembler chain, resulting in a VmDescription
pub fn run(assembly: &str) -> Result<VmDescription, BError> {
    // find the ranges of each section
    let macros_start = match RE_MACROS.find(assembly) {
        Some(macros_match) => macros_match.end(),
        None => 0,
    };
    let (macros_end, text_start) = match RE_TEXT.find(&assembly[macros_start..]) {
        Some(text_match) => (
            macros_start + text_match.start(),
            macros_start + text_match.end(),
        ),
        None => (macros_start, macros_start),
    };
    let consts_end = assembly.len();
    // find the consts range and mount point
    let (text_end, consts_start, consts_mount) = match RE_CONSTS.find(&assembly[text_start..]) {
        Some(consts_match) => {
            let n_str = &RE_CONSTS
                .captures_at(&assembly[text_start..], consts_match.start())
                .unwrap()["number"];
            // doesn't fail because we already found it
            let number = number_literal_to_u16(n_str).map_err(|_| {
                BError::AsmParseError(format!(
                    "error parsing consts section. The number {n_str} isn't good.",
                ))
            })?;
            (
                text_start + consts_match.start(),
                text_start + consts_match.end(),
                number,
            )
        }
        None => (text_start, text_start, 0xfff0),
    };
    if !((macros_start <= text_start) & (text_start < consts_start)) {
        return Err(BError::AsmParseError(
            "bad section ordering or `[text]` section is missing".to_string(),
        ));
    }
    // run the assembler in sequence
    let (preprocessed, t_offset) = preprocessor::preprocess(
        &assembly[macros_start..macros_end],
        &assembly[text_start..text_end],
    )?;
    let assembled = text_processor::assemble(preprocessed, t_offset)?;
    let const_offset = t_offset + assembled.rom_lines;
    const_processor::find_and_place(
        assembled,
        &assembly[consts_start..consts_end],
        const_offset,
        consts_mount,
    )
}
