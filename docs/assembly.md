# Assembly for BRIC
## Numbers
Numbers can be written in one of three formats:
- 0xab01: Hexadecimal, supports both capital and lowercase letters
- 0b1101: Binary
- 112: Decimal
## Comments
Comments are always in their own line. A comment is signified by a `#` being the first character.
## Whitespace
More than one whitespace is interpreted by the assembler as one whitespace. Whitespace at the beginning of a line is ignored.
## Names
Certain objects can be named for the pre-processor. Names can contain `a-z`,`A-Z`,`.` and `_`, but can not be the same name used for something else.
## Sections
There are multiple sections that can be defined `[macros]` for macros, `[text]` the section that is supposed to be interpreted and `[consts]`, a section of the code that is used for constants.

The `[consts]` section is written in the assembly as `[code ADDR]`, to let the assembler know where constants will be mounted in RAM.

Not all sections are needed, but at the very least one should have a `[text]` section.
## The `[macros]` section
In the `[macros]` section macros can be defined that run on the rest of the source.
### Defines
A value can be defined using `define NAME VALUE`. They will be copied into the correct place by the pre-processor. Currently only number literals can be assigned to defines. If more flexibility is needed use _macros_.

Defines can only be defined in the `[marcos]` section.
### Macros
Macros can be defined between a `begin macro` and an `end macro`. In order to give a macro a name the name is put behind the `begin macro` part in the same line. After this come the arguments. The arguments are put in parentheses `()`, separated by commas and are also named. Arguments names can not be the same as any names used anywhere else except in the arguments of other macros.
A macro might look like this:
```
begin abc.def (ghi, jkl)
    # Code here
end
```

Macros can only be defined in the `[macros]` section

## Labels
Labels specify a position in the code and are later translated by the pre-processor into memory positions. The syntax for labels is `label NAME:`. A label is the only object in its line (except for comments).
## Computations
Any instruction for the CPU is a computation looking like follows:
```
TARGET = CALCULATION; JUMP-COND
```
- `TARGET` is any register including `*A` which is memory at the address pointed to by the `A` register.
- `CALCULATION` is any of the calculations found in the _Calculations_ section. The calculations are always between the `A` register and another register like `TARGET`.
- `JUMP-COND` is one of
    - `JLT`: jump less than
    - `JEQ`: jump equal
    - `JGT`: jump greater than
    - `JLE`: jump less than or equal
    - `JGE`: jump greater than or equal
    - `JMP`: jump unconditionally
    - `JNE`: jump not equal
In addition any of the parts of the computation can be left out except for the calculation. If the target is left out, the `=` is left out with it. This causes the data to not be written anywhere after the calculation. If the jump is left out the `;` is left out with it. This causes no jump after the computation.

In the case of an unconditional jump `JMP` all other operands can be left out;
## Calculations
All _registers_ include `A`, `*A` and `D`-`H` or none.

A calculation is between one of the _registers_ (part A) and A. The first operand of the operation can always be set to 0.

Parts A and B can be on either side of the computation.

Operands include:
| Symbol | Operation              | Operands | 
| ------ | ---------------------- | -------- |
| `and`  | And                    | 2        |
| `or`   | Or                     | 2        |
| `xor`  | Xor                    | 2        |
| `add`  | Plus                   | 2        |
| `sub`  | Minus                  | 2        |
| `inc`  | Increment              | 1        |
| `dec`  | Decrement              | 1        |
| `not`  | Not                    | 1        |
| `lsl`  | Logical shift left     | 1        |
| `lsr`  | Logical shift right    | 1        |
| `asr`  | Arithmetic shift right | 1        |
| `rol`  | Roll left              | 1        |
| `ror`  | Roll right             | 1        |

For some combinations of operands and symbols the operands can be left out (e.g. if the operand acts only on one part or if the result is supposed to only be determined by one part).

Valid computations might be
- `add, A, D`
- `sub, A, D`
- `inc, A`
- `dec, A`
- `sub, 0, D`
- `not, A`
- `dec, 0`
- `add, 0, D`
- `add, 0, A`

TODO: Add mnemonics for common computations.

## Assignments
One can assign a number (up to 0x7fff) to `A` using `A = NUMBER`. Assignments can not have operators or jump conditions.

## Constants
Constants can only be defined in the `[constants ADDR]` section using the syntax:
```
label CONSTANT_NAME:
M = 0xffff
```
Only numbers up to 0xffff can be written this way. Writing `M =` here allows the user to write arbitrary numbers into memory.

The constants section is mounted into RAM at the address pointed to by `ADDR`.

## File Names
Human readable assembly files commonly have the `.basm` extension. Assembled binaries have the `.bexe` file extension.

## Notes
Because it is difficulat to access memory addresses > 0x7fff in RAM it is advisable to put dynamic program memory (e.g. the stack or the heap) there.