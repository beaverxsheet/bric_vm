# `.bvm` and `.bdb` formats
These formats are subject to change, so old versions of these file formats might not be compatible with newer versions of BRIC_VM or BDB.
## `.bvm` format for serializing VMs
The purpose of this format is to completely describe the state of a BRIC VM. To this purpose we serialize the `bric_vm::vm::VmDescription` struct.

We do not serialize memory `callbacks`.

Encodings are in big endian.

### Structure
The file has three sections separated by labels:
- Header
- Rom Mappings
- Rom
- Ram

### Header
The header contains information smaller variables and magic.
- Magic (4 bytes): The file is labeled by 0x42, 0x56, 0x4d, 0x00. ("BVM" in ASCII)
- Program Counter (3 bytes): current value of the program counter + 0x00 byte
- Registers (12 bytes): A, D-H registers in alphabetical order + 0x00
### Rom Mappings
- Magic (4 bytes): section is labeled by 0x52, 0x4d, 0x50, 0x00 ("RMP" in ASCII)
- Mapping number (3 bytes): amount of rom mappings (encoded big endian in two bytes). The section after this position will have 7*n + 1 bytes. Finally an extra 0x00 is added to the end
- Mappings: Each mapping is encoded rom_addr, length, ram_addr. Each big-endian. After each mapping a 0x00 byte is encoded
### Rom
- Magic (4 bytes): section is labeled by 0x52, 0x4f, 0x4d, 0x00 ("ROM" in ASCII)
- Field amount (3 bytes): Amount of fields in the ROM field last byte is 0x00
- A dump of ROM, each field is encoded in big endian.
- 0x00 end
### Ram
- Magic (4 bytes): section is labeled by 0x52, 0x41, 0x4d, 0x00 ("RAM" in ASCII)
- A dump of RAM 65536 * 2 bytes


## `.bdb` format for serializing the debugger
The purpose of this format is to allow for serialization of a debugger

### Structure
The file consists of three sections
- BDB Header
- Breakpoints
- BVM file

### BDB header
- Magic (4 bytes) 0x42, 044, 0x42, 0x00. ("BDB" in ASCII)

### Breakpoints
- Magic (4 bytes) 0x42, 0x50, 0x53, 0x00 ("BPS" in ASCII)
- The amount of breakpoints in two bytes big endian followed by 0x00 (3 bytes)
- The breakpoints, each two bytes big endian
- trailing 0x00

### BVM file
See above