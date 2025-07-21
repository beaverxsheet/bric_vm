use std::{fs::File, io::Write, path::PathBuf};

use clap::Parser;

/// Assemble a .basm file into a .bvm
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to the .basm file
    #[arg(short, long)]
    in_path: PathBuf,

    /// path to output to
    #[arg(short, long)]
    out_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let input_string = std::fs::read_to_string(args.in_path).expect("unable to read input file");

    let vmdesc = match bric_vm::assembler::run(&input_string) {
        Err(e) => {
            eprintln!("assembly error: {}", e);
            std::process::exit(-1);
        }
        Ok(v) => v,
    };

    {
        let mut file = File::create(args.out_path).expect("cant create output file");
        let out_bytes = match vmdesc.serialize() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("serialization error: {}", e);
                std::process::exit(-1);
            }
        };
        file.write(&out_bytes)
            .expect("unable to write to output path");
    }
    std::process::exit(0);
}
