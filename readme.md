# Ben's Reduced Instruction set Computer (BRIC) Virtual Machine (VM)
This repo contains the source code and specifications for a VM that simulates the BRIC instruction set. This project is mainly academic and I haven't gotten around to writing proper tests yet, so there is a high likelihood of bugs being present.

The instruction set is detailed in [ISA](docs/ISA.md) and the save formats in [vmformat](docs/vmformat.md).

## Building
__Prerequisites__: You need to have `git` and a rust toolchain installed.

1. Get the source using `git` and the move into the directory you just cloned.
2. Run `cargo build --release`
3. Your executables should be in `./target/release`

## Running
1. Write some `basm` code. You can look at the [example](basm_examples/example.basm), which doesn't do much but showcases some of the assembler features. You can also look at the assembly [docs](docs/assembly.md).
2. Assemble your `basm` code into a `bvm` file using the `basm` executable.
3. Run your code in the debugger using the `bdb` executable. See [bdb](docs/debugger.md) for help.

## Project Outline
This project is far from finished. Here are some features that are yet to be implemented:
1. Graphics MMIO. The idea is to implement a very basic graphics API and display the result using a custom display crate built using the `WGPU` crate.
2. Network MMIO for network communication.
3. Implementation of the computer on an FPGA using verilog
4. Extend the assembler to include some simplifying mnemonics
5. Implement the instruction set as a LLVM backend

Furthermore here are some niceties for existing features that are missing:
- Serialization and deserialization of memory mappings and memory blocks
- UART support in the VM
- Debugger memory editing
- Debugger memory breakpoints
- Comprehensive tests. Currently there exist limited tests for the VM but more comprehensive tests would be nice and testing the assembler would also be important.