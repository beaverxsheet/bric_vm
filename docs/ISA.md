# BRIC
Bens Reduced Instruction set Computer
## Architecture
Inspired by [NandGame](nandgame.com) but with more registers and space for more instructions. Implements both arithmetic and logical left and right shifts.

The computer has a separate ROM and RAM (called memory here) for now.
### Registers
- __A__: Accumulator, can also be used to point to a space in memory to be read out / written to
- __D__ - __H__: General purpose registers. Can only interact with the __A__ register or each other.

Registers are numbered and can be specified as either _source_ or _target_ of an operation (see below). The numbering is as follows:

| Register Number | Register |
| --------------- | -------- |
| 000             | None     |
| 001             | A        |
| 010             | *A       |
| 011             | D        |
| 100             | E        |
| 101             | F        |
| 110             | G        |
| 111             | H        |

### Operations
Operations always take place between the A register (or none) and a GP register or *A (the memory in the address pointed to by A), this is the _source_. The output of an operation is stored in either *A or a GP register (or none) this is the _target_.

Operations are one of the following:

| Operation bits | Operation |
| -------------- | --------- |
| 0000           | X & Y     |
| 0001           | X | Y     |
| 0010           | X ^ Y     |
| 0011           | ~X        |
| 0100           | X LSL 1   |
| 0101           | X LSR 1   |
| 0110           | X RSL 1   |
| 0111           | X RSR 1   |
| 1000           | X + Y     |
| 1001           | X - Y     |
| 1010           | X + 1     |
| 1011           | X - 1     |
| 1100           | X ASR 1   |
| ...            | -         |

There are also two flags that can be set for each operation: `zx` and `sw`.
- `zx` zeros the X input
- `sw` switches the X and Y inputs

When `zx` and `sw` are both 0 the X input comes from _source_ and the Y input comes from __A__.

### Jumps
Each instruction that is not a data instruction can also include a jump. The jump is conditional on the result of the computation done in the operation. After the computation the result (even if it is not stored) is tested and the flags are set appropriately. The jump can be conditional on a combination of the flags.

A jump will lead to the program execution continuing at the instruction pointed to by __A__ in ROM.

The condition of jump can be set using the following flags: `lt`, `eq`, `gt` (less than, equal, greater than). Any combination is possible.

### Data Instructions
Immigrates can have 15 bits. They can only be loaded into the __A__ register.

### Instruction Architecture
| `ci` | source | operation   | target | jump            |
| ---- | ------ | ----------- | ------ | --------------- |
| b    | n      | n `sw` `zx` | n      | `lt` ` eq` `gt` |

- When `ci` is 1 it is a data instruction otherwise it is a normal instruction.
- See _Registers_ for more information about source and target.
- See _Operations_ for more information about operations.
- See _Jumps_ for more information about jumps.