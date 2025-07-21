use std::{fs::File, io::Write, path::PathBuf};

use bric_vm::vm::VmDescription;
use clap::Parser;

/// Disassemble a .bvm file into as .basm file
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to the .bvm file
    #[arg(short, long)]
    in_path: PathBuf,

    /// path to output .basm to
    #[arg(short, long)]
    out_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let bvm_file = std::fs::read(args.in_path).expect("unable to read input file");
    let vm_desc = match VmDescription::deserialize(&bvm_file) {
        Err(e) => {
            eprintln!("bad input file: {}", e);
            std::process::exit(-1);
        }
        Ok(v) => v,
    };

    match bric_vm::disassembler::disassemble(&vm_desc.rom, false) {
        Ok(s) => {
            let mut file = File::create(args.out_path).expect("cant create outptu file");
            file.write_all(s.as_bytes())
                .expect("unable to write to output path");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("error disassembling: {}", e);
            std::process::exit(-1);
        }
    }
}
