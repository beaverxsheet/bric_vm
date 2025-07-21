use std::{
    io,
    path::PathBuf,
    sync::mpsc::{self, TryRecvError},
    thread,
};

use bric_vm::{
    BError,
    vm::{Vm, VmDescription},
};
use clap::Parser;

// TODO IO

/// Runs a BRIC from a .bvm file. Does not support UART yet.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// path to the .bvm file
    #[arg(short, long)]
    path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let bvm_file = std::fs::read(args.path).expect("unable to read input file");
    let vm_desc = match VmDescription::deserialize(&bvm_file) {
        Err(e) => {
            eprintln!("bad input file: {}", e);
            std::process::exit(-1);
        }
        Ok(v) => v,
    };

    let mut vm = match Vm::new(vm_desc) {
        Err(e) => {
            eprintln!("error during vm instantiation: {}", e);
            std::process::exit(-1);
        }
        Ok(v) => v,
    };

    let (tx, rx) = mpsc::channel::<String>();
    thread::spawn(move || {
        loop {
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            tx.send(buffer).unwrap();
        }
    });

    loop {
        match rx.try_recv() {
            Ok(line) => {
                match line.as_str() {
                    "q" => {
                        std::process::exit(0);
                    }
                    "u" => {
                        // change to UART state here
                    }
                    _ => {}
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                std::process::exit(-1);
            }
        }

        match vm.cycle() {
            Err(BError::ExecutionHaltedError { value: _ }) => {
                println!("Execution halted");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("error during execution: {}", e);
                std::process::exit(-1);
            }
            Ok(_) => {}
        }
    }
}
