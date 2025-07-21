use bric_vm::{
    BError,
    debugger::Debugger,
    disassembler::{self, disassemble_inst},
    util::number_literal_to_u16,
    vm::{Register, VmDescription},
};
use clap::Parser;
use std::{
    io::{self, Write},
    path::PathBuf,
};

// TODOs
// - Memory editing
// - Memory Breakpoints

/// Runs a BRIC from a .bvm or .bdb in an Assembler
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// from coredump (.bdb file)
    #[arg(short, long, default_value_t = false)]
    coredump: bool,

    /// use uart, does not work for coredumps
    #[arg(short, long, default_value_t = false)]
    uart: bool,

    /// path to the .bvm or .bdb file
    #[arg(short, long)]
    path: PathBuf,

    /// max amount of iterations to continue the CPU for when continuing
    #[arg(short, long, default_value_t = 0xffff)]
    max_iter: usize,
}

fn make_dbg(input: &[u8], coredump: bool, use_uart: bool) -> Result<Debugger, BError> {
    if coredump {
        Debugger::deserialize(&input)
    } else {
        Debugger::new(VmDescription::deserialize(&input)?, vec![], use_uart)
    }
}

fn main() {
    let args = Args::parse();

    let input = std::fs::read(args.path).expect("unable to read input file");

    let mut debugger = match make_dbg(&input, args.coredump, args.uart) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error deserializing: {}", e);
            std::process::exit(-1);
        }
    };

    loop {
        let uout = debugger.get_uart_out();
        if uout.len() != 0 {
            println!("uart>> {:?}", uout);
        }

        let mut user_input = String::new();
        print!("bdb> ");
        let _ = io::stdout().flush();
        io::stdin()
            .read_line(&mut user_input)
            .expect("unable to read stdin");

        match user_input.trim() {
            "q" => {
                std::process::exit(0);
            }
            "c" => {
                debugger.run(args.max_iter);
            }
            "s" => {
                debugger.step();
            }
            "dis" => {
                match disassembler::disassemble(debugger.get_rom(), true) {
                    Ok(s) => println!("{}", s),
                    Err(e) => eprintln!("unable to disassemble {}", e),
                };
            }
            "u" => {
                if !args.uart {
                    eprintln!("UART not activated");
                    continue;
                }
                println!("capturing uart input... enter `quit_uart` to leave");
                loop {
                    print!("uart> ");
                    let mut uart_input = String::new();
                    let _ = io::stdout().flush();
                    io::stdin()
                        .read_line(&mut uart_input)
                        .expect("unable to read stdin");
                    if matches!(uart_input.as_str(), "quit_uart\n") {
                        break;
                    }
                    for c in uart_input.chars() {
                        debugger.write_uart_byte(c as u8);
                    }
                }
            }
            "" => {}
            o => {
                if o.starts_with("i") {
                    let parts: Vec<&str> = o.split_whitespace().collect();
                    if parts.len() < 2 || parts[0] != "i" {
                        eprintln!("unrecognized input");
                        continue;
                    }

                    match parts[1] {
                        "reg" => {
                            if let Some(reg_text) = parts.get(2) {
                                if let Some(reg) = Register::from_str(*&reg_text) {
                                    println!("{} = {:#04x}", reg_text, debugger.inspect_reg(reg));
                                } else {
                                    eprintln!("invalid register name");
                                    continue;
                                }
                            } else {
                                eprintln!("not enough arguments for `i reg`");
                            }
                        }
                        v @ "mem" | v @ "rom" => {
                            if parts.len() != 4 {
                                eprintln!("not enough arguments for `i {}`", v);
                                continue;
                            }
                            match number_literal_to_u16(parts[2]) {
                                Ok(start_addr) => {
                                    match number_literal_to_u16(parts[3]) {
                                        Ok(length) => {
                                            let mem_dump = match v {
                                                "mem" => {
                                                    debugger.inspect_memory(start_addr, length)
                                                }
                                                "rom" => debugger.inspect_rom(start_addr, length),
                                                _ => panic!("unreachable"),
                                            };
                                            for (i, v) in mem_dump.iter().enumerate() {
                                                if i % 16 == 0 {
                                                    // should not panic because we limit the size of inspect memory to the size of RAM.
                                                    print!("\n{:#06x}\t", start_addr + (i as u16));
                                                }
                                                print!("{:#06x} ", v);
                                            }
                                            println!()
                                        }
                                        Err(_) => {
                                            eprintln!("invalid length");
                                        }
                                    }
                                }
                                Err(_) => {
                                    eprintln!("invalid starting address");
                                }
                            }
                        }
                        "ci" => {
                            let pc = debugger.get_pc();
                            if let Some(inst) = debugger.inspect_rom(pc, 1).get(0) {
                                let mut out = String::new();
                                match disassemble_inst(inst, &mut out) {
                                    Ok(_) => {
                                        println!("{}", out);
                                    }
                                    Err(_) => {
                                        eprintln!("unable to decode instruction");
                                    }
                                }
                            } else {
                                eprintln!("PC points outside of valid ROM range");
                            }
                        }
                        "pc" => {
                            println!("PC = {}", debugger.get_pc());
                        }
                        _ => {
                            eprintln!("unrecognized input");
                        }
                    }
                } else if o.starts_with("b") || o.starts_with("rb") {
                    let parts: Vec<&str> = o.split_whitespace().collect();
                    if parts.len() < 2 || parts[0] != "b" {
                        eprintln!("unrecognized input");
                        continue;
                    }

                    match number_literal_to_u16(parts[1]) {
                        Ok(v) => {
                            if parts[0] == "b" {
                                debugger.register_breakpoint(v);
                                println!("registered new breakpoint at {:#04x}", v);
                            } else {
                                if debugger.remove_breakpoint(v) {
                                    println!("removed breakpoint at {:#04x}", v);
                                } else {
                                    eprintln!("that breakpoint does not exist");
                                }
                            }
                        }
                        Err(_) => {
                            eprintln!("unable to parse breakpoint address");
                        }
                    }
                } else {
                    eprintln!("unknown input");
                }
            }
        }
    }
}
