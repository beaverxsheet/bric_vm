# BDB debugger
The BDB debugger can be called using the following options:
```
Usage: bdb [OPTIONS] --path <PATH>

Options:
  -c, --coredump             from coredump (.bdb file)
  -u, --uart                 use uart, does not work for coredumps
  -p, --path <PATH>          path to the .bvm or .bdb file
  -m, --max-iter <MAX_ITER>  max amount of iterations to continue the CPU for when continuing [default: 65535]
  -h, --help                 Print help
  -V, --version              Print version
```

Coredumps currently do not support serialization of the entire VM state. In particular Memory callbacks, ROM mappings and Memory permissions, as well as execution finalization state. These should not really be problematic to view though.

## Commands
- `q` quit the program
- `c` continue execution for MAX_ITER iterations, or until a breakpoint is hit or until the execution halts
- `s` step one instruction
- `dis` disassemble and display the entire ROM
- `i reg [REG]` display the current value of the register specified by `REG`
- `i mem [beginning] [length]` display the RAM memory in the region `beginning` - `beginning + length`
- `i rom [beginning] [length]` display the ROM in the region `beginning` - `beginning + length`
- `i ci` display a disassembly of the instruction in ROM at the position of the `PC` (program counter)
- `i pc` display the current value of the program counter
- `b [location]` set a breakpoint at `location`
- `rb [location]` remove a breakpoint at `location`
- `u` enter something into the UART. Leave by entering `quit_uart`