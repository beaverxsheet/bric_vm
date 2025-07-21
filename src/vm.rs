use crate::util::{BError, Region, RegionMap};
use core::fmt;
use num_enum::TryFromPrimitive;
use std::{collections::HashMap, convert::TryFrom, num::Wrapping};

/// The length of RAM for the BRIC
pub const RAM_LEN: usize = 65536;
pub(crate) const BIT_15: u16 = 0b1000000000000000;

fn new_parse_error(value: u8) -> BError {
    BError::InstParseError {
        value: value as u16,
        message: format!("A register field can only hold 3 bits."),
    }
}

/// Label registers according to the ISA
#[derive(Debug, Eq, PartialEq, TryFromPrimitive, Clone, Copy)]
#[num_enum(error_type(name = BError, constructor = new_parse_error))]
#[repr(u8)]
pub enum Register {
    None = 0b000,
    A = 0b001,
    MA = 0b010,
    D = 0b011,
    E = 0b100,
    F = 0b101,
    G = 0b110,
    H = 0b111,
}

impl Register {
    pub fn from_str(input: &str) -> Option<Self> {
        match input {
            "A" => Some(Self::A),
            "*A" => Some(Self::MA),
            "D" => Some(Self::D),
            "E" => Some(Self::E),
            "F" => Some(Self::F),
            "G" => Some(Self::G),
            "H" => Some(Self::H),
            _ => None,
        }
    }
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::A => write!(f, "A"),
            Self::MA => write!(f, "*A"),
            Self::D => write!(f, "D"),
            Self::E => write!(f, "E"),
            Self::F => write!(f, "F"),
            Self::G => write!(f, "G"),
            Self::H => write!(f, "H"),
        }
    }
}

/// Label the level of access the CPU can have to a region of memory
#[derive(Debug, Clone)]
pub enum AccessLevels {
    ReadWrite,
    Read,
    None,
}

/// Represents the RAM of the VM. This includes the MMIO. Callbacks can be registered using [`Ram::register_callback()`]
pub struct Ram {
    ram: [u16; RAM_LEN],
    write_callbacks: HashMap<u16, Box<dyn FnMut(u16)>>,
    memory_regions: RegionMap<u16, AccessLevels>,
}

impl fmt::Debug for Ram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ram")
            .field("ram", &self.ram)
            .field("write_callbacks", &self.write_callbacks.keys())
            .field("memory_regions", &self.memory_regions)
            .finish()
    }
}

impl Ram {
    fn read_ram(&self, position: u16) -> u16 {
        match self
            .memory_regions
            .find_region(position)
            .unwrap_or(&AccessLevels::ReadWrite)
        {
            AccessLevels::None => 0,
            // doesn't panic because 16 < RAM_LEN
            _ => self.ram[position as usize],
        }
    }

    fn write_ram(&mut self, position: u16, value: u16) {
        match self
            .memory_regions
            .find_region(position)
            .unwrap_or(&AccessLevels::ReadWrite)
        {
            AccessLevels::ReadWrite => {
                self.set_ram(position, value);

                if let Some(cb) = self.write_callbacks.get_mut(&position) {
                    (*cb)(value);
                }
            }
            _ => {
                println!("[!] forbidden write on {}", position);
            }
        }
    }

    /// Create new RAM. The `ram` argument is written directly to the newly created object.
    pub fn new(ram: [u16; RAM_LEN], memory_regions: RegionMap<u16, AccessLevels>) -> Self {
        Self {
            ram,
            write_callbacks: HashMap::new(),
            memory_regions,
        }
    }

    /// Registers a callback on a certain address. Only one callback will be registered,
    /// new callbacks on the same address will lead to the old one being overwritten.
    pub fn register_callback(&mut self, address: u16, cb: Box<dyn FnMut(u16)>) {
        let _ = self.write_callbacks.insert(address, cb);
    }

    /// Set the memory at an address in RAM without the usual checks
    /// ## Panics
    /// If address is not the ragne (0, RAM_SIZE)
    pub fn set_ram(&mut self, address: u16, value: u16) {
        self.ram[address as usize] = value;
    }

    /// Set a region in RAM with the values of an array. Does not perform any checks
    /// ## Errors
    /// When the region to be written goes out of the space allocated for the RAM [`BError::OutOfBoundsError`] is emitted.
    pub fn set_ram_region(&mut self, address: u16, values: &[u16]) -> Result<(), BError> {
        // Check wether the new piece will fit into the RAM
        if (address as usize) + values.len() > RAM_LEN {
            return Err(BError::OutOfBoundsError(address, values.len(), RAM_LEN));
        }

        for i in 0..values.len() {
            self.ram[i + address as usize] = values[i]
        }

        Ok(())
    }

    /// Gets a RAM region. If the region goes beyond the RAM, the resulting vec will only contain as many entries as the
    /// overlap between the specified region of RAM and RAM
    pub fn get_ram_region(&self, address: u16, length: usize) -> &[u16] {
        let start = address as usize;
        let end = start + length;

        let end = if end > RAM_LEN { RAM_LEN } else { end };
        let start = if start > end { end } else { start };

        &self.ram[start..end]
    }
}

/// A unit containing the RAM and the registers (except for the program counter register)
#[derive(Debug)]
pub struct MemoryUnit {
    a: u16,
    d: u16,
    e: u16,
    f: u16,
    g: u16,
    h: u16,
    ram: Ram,
}

impl MemoryUnit {
    /// Get the value currently in the register specified by `reg`
    pub fn get_reg(&self, reg: Register) -> u16 {
        match reg {
            Register::None => 0,
            Register::A => self.a,
            Register::MA => self.ram.read_ram(self.a),
            Register::D => self.d,
            Register::E => self.e,
            Register::F => self.f,
            Register::G => self.g,
            Register::H => self.h,
        }
    }

    /// Set the value of the register specified by `reg`
    pub fn set_reg(&mut self, reg: Register, value: u16) {
        match reg {
            Register::None => {}
            Register::A => self.a = value,
            Register::MA => self.ram.write_ram(self.a, value),
            Register::D => self.d = value,
            Register::E => self.e = value,
            Register::F => self.f = value,
            Register::G => self.g = value,
            Register::H => self.h = value,
        }
    }

    /// Creates a new memory unit
    /// ## Errors
    /// See [`Ram`]
    pub fn new(
        a: u16,
        d: u16,
        e: u16,
        f: u16,
        g: u16,
        h: u16,
        ram: [u16; 65536],
        regions: Vec<Region<u16, AccessLevels>>,
    ) -> Result<Self, BError> {
        Ok(Self {
            a,
            d,
            e,
            f,
            g,
            h,
            ram: Ram::new(ram, RegionMap::try_from(regions)?),
        })
    }

    /// Register a callback on the write of a specific memory address in RAM
    pub fn register_callback(&mut self, address: u16, cb: Box<dyn FnMut(u16)>) {
        self.ram.register_callback(address, cb);
    }

    /// Wraps [`Ram::set_ram()`]
    pub fn set_ram(&mut self, address: u16, value: u16) {
        self.ram.set_ram(address, value);
    }

    /// Get all the registers at once
    pub fn get_regs(&self) -> (u16, u16, u16, u16, u16, u16) {
        (self.a, self.d, self.e, self.f, self.g, self.h)
    }

    /// Wraps [`Ram::get_ram_region()`]
    pub fn get_ram_region(&self, address: u16, length: usize) -> &[u16] {
        self.ram.get_ram_region(address, length)
    }
}

impl Default for MemoryUnit {
    fn default() -> Self {
        Self {
            a: 0,
            d: 0,
            e: 0,
            f: 0,
            g: 0,
            h: 0,
            ram: Ram::new(
                [0; 65536],
                RegionMap::try_from(vec![Region::new(0, 65535, AccessLevels::ReadWrite)]).unwrap(),
            ),
        }
    }
}

/// Represents Read Only Memory. This will usually only store program text,
/// but can be configured to be mapped into memory also in order to also store constants (See [`VmDescription`]).
#[derive(Debug, Clone)]
struct Rom {
    program_text: Vec<u16>,
}

impl FromIterator<u16> for Rom {
    fn from_iter<T: IntoIterator<Item = u16>>(iter: T) -> Self {
        let o: Vec<u16> = iter.into_iter().collect();
        Self { program_text: o }
    }
}

impl Rom {
    fn new(program_text: Vec<u16>) -> Self {
        Self { program_text }
    }

    /// Get the value of ROM at the `address`
    fn get_address(&self, address: u16) -> Option<u16> {
        self.program_text.get(address as usize).map(|v| *v)
    }

    pub(crate) fn get_rom_region(&self, address: u16, length: usize) -> &[u16] {
        let start = address as usize;
        let end = start + length;
        let rom_len = self.program_text.len();
        let end = if end > rom_len { rom_len } else { end };
        let start = if start > end { end } else { start };

        &self.program_text[start..end]
    }
}

/// Represents the program counter. (A wrapper for the `Wrapping` type LOL)
#[derive(Debug, Clone)]
struct Pc {
    val: Wrapping<u16>,
}

impl Pc {
    fn new(val: u16) -> Self {
        Self { val: Wrapping(val) }
    }

    fn inc(&mut self) {
        self.val += 1;
    }

    fn set(&mut self, new_val: u16) {
        self.val = Wrapping(new_val);
    }

    fn get_val(&self) -> u16 {
        self.val.0
    }
}

bitfield::bitfield! {
    /// An ALU instruction represented by a `u16`
    pub struct AluInstruction(u16);
    impl Debug;
    u8;
    pub get_gt, set_gt:         0;
    pub get_eq, set_eq:         1;
    pub get_lt, set_lt:         2;
    pub get_target, set_target: 5, 3;
    pub get_zx, set_zx:         6;
    pub get_sw, set_sw:         7;
    pub get_op, set_op:         10, 8;
    pub get_u, set_u:           11;
    pub get_source, set_source: 14, 12;
    pub get_ci, _:              15
}

/// Represents both types of instruction
pub enum Instruction {
    Alu(AluInstruction),
    Data(u16),
}

impl Instruction {
    /// Turn a u16 into an instruction
    pub fn from_u16(inst: u16) -> Self {
        if (inst & BIT_15) == 0 {
            Self::Alu(AluInstruction(inst))
        } else {
            Self::Data(inst & (!BIT_15))
        }
    }

    /// Turn a instruction into a u16
    pub fn to_u16(self) -> u16 {
        match self {
            Self::Alu(inst) => inst.0,
            Self::Data(val) => val | BIT_15,
        }
    }
}

/// Describes the VM. This is mainly used for initialization of the VM but can also be used for serialization in order to capture the state of the VM.
/// ## Fields
/// - `pc`: The value of the program counter
/// - `rom`: The program text
/// - `mem`: The initial state of ram
/// - `callbacks`: Callbacks for reads on memory addresses. Ordered address: callback
/// - `rom_mappings`: Mappings from ROM to RAM. Ordered: rom_address, length, ram_address. Results in length bytes of ROM starting from rom_address being
///     copied into RAM at ram_address
/// - `regs`: A-H registers in alphabetical order
/// - `rom_blocks`: Rom regions to make read only for the processor Ordered: ram_address, length
/// ## Examples
/// This example instantiates a new VmDescription that maps the region from 0x0500 to 0x0600 into RAM at 0xf000 and has a callback at memory address 0x0123.
/// ```rust
/// use bric_vm::vm::VmDescription;
///
/// let callback = |new_value: u16| {
///     println!("got new value at 0x123: {}", new_value);
/// };
///
/// let vm_desc = VmDescription{
///     callbacks: vec![(0x123, Box::new(callback))],
///     rom_mappings: vec![(0x0500, 0x100, 0xf000)],
///     ..Default::default()
/// };
/// ```
pub struct VmDescription {
    pub pc: u16,
    pub rom: Vec<u16>,
    pub mem: Box<[u16; RAM_LEN]>,
    pub callbacks: Vec<(u16, Box<dyn FnMut(u16)>)>,
    pub rom_mappings: Vec<(u16, u16, u16)>,
    pub regs: [u16; 6],
    pub rom_blocks: Vec<(u16, u16)>,
}

impl Default for VmDescription {
    fn default() -> Self {
        Self {
            pc: 0,
            rom: vec![],
            mem: Box::new([0; RAM_LEN]),
            callbacks: Vec::new(),
            rom_mappings: Vec::new(),
            regs: [0; 6],
            rom_blocks: vec![],
        }
    }
}

// this implements serialization and deserialization
// could also be done using serde https://serde.rs/data-format.html
impl VmDescription {
    /// Serialize a VMDescription according to spec
    /// Currently the serialization does not support rom_blocks
    /// ## Errors
    /// - When the number of mappings is too large
    /// - When ROM is too large
    pub fn serialize(&self) -> Result<Vec<u8>, BError> {
        let mut output = Vec::new();

        // Magic
        output.append(&mut b"BVM\x00".to_vec());

        // PC
        output.append(&mut self.pc.to_be_bytes().to_vec());
        output.push(0x00);

        // Regs
        // Regs are already in the correct order
        for reg in self.regs.iter() {
            output.append(&mut reg.to_be_bytes().to_vec())
        }
        output.push(0x00);

        // Mappings
        // Magic
        output.append(&mut b"RMP\x00".to_vec());

        // Amount of Mappings
        let map_no = self.rom_mappings.len();
        if map_no > 0xffff {
            return Err(BError::SerializationError(
                "The number of ROM mappings to be written is to large".to_string(),
            ));
        }
        output.append(&mut (map_no as u16).to_be_bytes().to_vec());
        output.push(0x00);

        // write Mappings
        for (rom_addr, length, ram_addr) in self.rom_mappings.iter() {
            output.append(&mut rom_addr.to_be_bytes().to_vec());
            output.append(&mut length.to_be_bytes().to_vec());
            output.append(&mut ram_addr.to_be_bytes().to_vec());
            output.push(0x00);
        }

        // ROM
        // Magic
        output.append(&mut b"\x00ROM\x00".to_vec());

        // ROM length
        let rom_len = self.rom.len();
        if rom_len > 0xffff {
            return Err(BError::SerializationError(
                "The ROM to be written is to large".to_string(),
            ));
        }
        output.append(&mut (rom_len as u16).to_be_bytes().to_vec());
        output.push(0x00);

        // write ROM
        for val in self.rom.iter() {
            output.append(&mut val.to_be_bytes().to_vec());
        }

        // RAM
        output.append(&mut b"\x00RAM\x00".to_vec());
        for val in self.mem.iter() {
            output.append(&mut val.to_be_bytes().to_vec());
        }

        Ok(output)
    }

    /// Deserialize a VMDescription according to spec
    /// Currently the serialization does not support rom_blocks
    /// ## Errors
    /// When the spec is not correctly respected
    pub fn deserialize(input: &[u8]) -> Result<Self, BError> {
        use crate::util::{check_slice, extract_number};
        let current = input;
        // Check file magic
        if check_slice(current, 4)? != b"BVM\x00" {
            return Err(BError::DeserializationError(
                "Invalid file format".to_string(),
            ));
        }
        let current = &current[4..];

        // Get pc
        let pc_nums = check_slice(current, 3)?;
        let pc = extract_number(pc_nums)?;
        let current = &current[3..];

        // Get regs
        const REG_FIELD_SIZE: usize = 2 * 6;
        let reg_num = check_slice(current, REG_FIELD_SIZE + 1)?;
        if reg_num[REG_FIELD_SIZE] != 0x00 {
            return Err(BError::DeserializationError(
                "Invalid region separators".to_string(),
            ));
        }
        let mut regs = [0; 6];
        for i in 0..6 {
            regs[i] = u16::from_be_bytes([reg_num[2 * i], reg_num[2 * i + 1]]);
        }
        let current = &current[REG_FIELD_SIZE + 1..];

        // ROM mappings
        // Magic check
        if check_slice(current, 4)? != b"RMP\x00" {
            return Err(BError::DeserializationError("No ROM mappings".to_string()));
        }
        let current = &current[4..];

        // Amount of ROM mappings
        let rma_nums = check_slice(current, 3)?;
        let rom_map_amnt = extract_number(rma_nums)? as usize;
        let current = &current[3..];

        // Get ROM mappings
        let mut mappings = Vec::with_capacity(rom_map_amnt);
        let rrlen = rom_map_amnt * 7 + 1;
        let rom_map_region = check_slice(current, rrlen)?;
        if rom_map_region[rrlen - 1] != 0x00 {
            return Err(BError::DeserializationError(
                "Invalid region separators".to_string(),
            ));
        }

        for i in 0..rom_map_amnt {
            let j = 7 * i;
            let rom_addr = u16::from_be_bytes([rom_map_region[j], rom_map_region[j + 1]]);
            let length = u16::from_be_bytes([rom_map_region[j + 2], rom_map_region[j + 3]]);
            let ram_addr = u16::from_be_bytes([rom_map_region[j + 4], rom_map_region[j + 5]]);
            if rom_map_region[j + 6] != 0x00 {
                return Err(BError::DeserializationError(
                    "Invalid region separators".to_string(),
                ));
            }
            mappings.push((rom_addr, length, ram_addr));
        }
        let current = &current[rrlen..];

        // ROM
        // Magic check
        if check_slice(current, 4)? != b"ROM\x00" {
            return Err(BError::DeserializationError("No ROM".to_string()));
        }
        let current = &current[4..];

        // Amount of ROM
        let ra_nums = check_slice(current, 3)?;
        let rom_amnt = extract_number(ra_nums)? as usize;
        let current = &current[3..];

        // Get ROM
        let romlen = 2 * rom_amnt + 1;
        let rom_region = check_slice(current, romlen)?;
        if rom_region[romlen - 1] != 0x00 {
            return Err(BError::DeserializationError(
                "Invalid region separators".to_string(),
            ));
        }
        let mut rom = Vec::with_capacity(rom_amnt);
        for i in 0..(rom_amnt as usize) {
            let j = 2 * i;
            rom.push(u16::from_be_bytes([rom_region[j], rom_region[j + 1]]));
        }

        let current = &current[romlen..];

        // RAM
        // Magic check
        if check_slice(current, 4)? != b"RAM\x00" {
            return Err(BError::DeserializationError("No RAM".to_string()));
        }
        let current = &current[4..];

        if current.len() != RAM_LEN * 2 {
            return Err(BError::DeserializationError(
                "Invalid RAM length".to_string(),
            ));
        }
        let mut ram = Vec::with_capacity(RAM_LEN * 2);
        for i in 0..RAM_LEN {
            let j = 2 * i;
            ram.push(u16::from_be_bytes([current[j], current[j + 1]]));
        }

        // shouldn't fail. we set the size before
        let mem: Box<[u16; RAM_LEN]> = ram.into_boxed_slice().try_into().unwrap();

        Ok(Self {
            pc,
            rom,
            mem,
            callbacks: vec![],
            rom_mappings: mappings,
            regs,
            rom_blocks: vec![],
        })
    }
}

/// Represents the VM.
/// ### Examples
/// Setting up a VM with the default VmDescription. This will result in an error when calling [`Vm::cycle()`] because there is no code to run.
/// ```rust
/// use bric_vm::vm::{Vm, VmDescription};
///
/// let vm_desc = VmDescription::default();
/// let mut vm = Vm::new(vm_desc).unwrap();
///
/// // This is because `assert_matches!()` is in nightly.
/// let v = match vm.cycle() {
///     Err(_) => 1,
///     Ok(_) => 2
/// };
///
/// assert!(v == 1);
/// ```
#[derive(Debug)]
pub struct Vm {
    pc: Pc,
    rom: Rom,
    mem: MemoryUnit,
}

impl Vm {
    /// Create a new VM from a [`VmDescription`]
    /// Copies the values in the description into the correct places in the computer and sets up mappings and callbacks
    /// ## Errors
    /// Results in a [`BError::OutOfBoundsError`] if a memory mapped region of ROM is not in RAM
    pub fn new(description: VmDescription) -> Result<Self, BError> {
        let pc = Pc::new(description.pc);

        let mut ram = *description.mem;

        // Set up memory mapped regions
        let mut regions = Vec::new();
        for (source_low, length, addr) in description.rom_mappings {
            let sl = source_low as usize;
            let ad = addr as usize;
            if ad + length as usize > RAM_LEN {
                return Err(BError::OutOfBoundsError(addr, length as usize, RAM_LEN));
            }

            for i in 0..length as usize {
                if let Some(v) = description.rom.get(i + sl) {
                    ram[ad + i] = *v;
                } else {
                    return Err(BError::MapError(format!(
                        "invalid span in rom: {:?} with length {:?}",
                        sl, length
                    )));
                }
            }
            regions.push(Region::new(addr, addr + length, AccessLevels::Read));
        }

        for (addr, length) in description.rom_blocks {
            let start = addr as usize;
            let end = start + (length as usize);

            if start > RAM_LEN || end > RAM_LEN {
                return Err(BError::OutOfBoundsError(addr, length as usize, RAM_LEN));
            }

            regions.push(Region::new(start as u16, end as u16, AccessLevels::Read));
        }

        let rom = Rom::new(description.rom);
        let regs = description.regs;
        // Set up memory
        let mut mem = MemoryUnit::new(
            regs[0], regs[1], regs[2], regs[3], regs[4], regs[5], ram, regions,
        )?;

        for (idx, callback) in description.callbacks {
            mem.register_callback(idx, callback);
        }

        Ok(Self { pc, rom, mem })
    }

    /// Cycles the CPU. Interprets the instruction and increments the PC.
    /// ## Errors
    /// - A [`BError::ExecutionHaltedError`] if there are no more instructions to run
    /// - A [`BError::AsmParseError`] if there has been an error parseing the instruction
    pub fn cycle(&mut self) -> Result<(), BError> {
        let pcval = self.pc.get_val();
        let inst = self
            .rom
            .get_address(pcval)
            .ok_or(BError::ExecutionHaltedError { value: pcval })?;
        self.interpret_instruction(inst)?;
        self.pc.inc();
        Ok(())
    }

    fn interpret_instruction(&mut self, instruction: u16) -> Result<(), BError> {
        match Instruction::from_u16(instruction) {
            Instruction::Alu(inst) => {
                let source = Register::try_from(inst.get_source())?;

                // Switch order of operands in ALU
                let (x, y) = if inst.get_sw() {
                    (Register::A, source)
                } else {
                    (source, Register::A)
                };

                let target = Register::try_from(inst.get_target())?;

                // Apply the Zero X flag
                let x = if inst.get_zx() {
                    0
                } else {
                    self.mem.get_reg(x)
                };
                let y = self.mem.get_reg(y);

                // Do the computation
                let output = if inst.get_u() {
                    // Arithmetic instructions
                    match inst.get_op() {
                        0b000 => x.wrapping_add(y),
                        0b001 => x.wrapping_sub(y),
                        0b010 => x.wrapping_add(1),
                        0b011 => x.wrapping_sub(1),
                        0b100 => {
                            let t = x & BIT_15;
                            t | (x << 1)
                        }
                        _ => return Err(BError::InvalidInstructionError { instruction }),
                    }
                } else {
                    // Logic instructions
                    match inst.get_op() {
                        0b000 => x & y,
                        0b001 => x | y,
                        0b010 => x ^ y,
                        0b011 => !x,
                        0b100 => x.wrapping_shl(1),
                        0b101 => x.wrapping_shr(1),
                        0b110 => x.rotate_left(1),
                        0b111 => x.rotate_right(1),
                        _ => return Err(BError::InvalidInstructionError { instruction }),
                    }
                };

                // Set the condition flags
                let ioutput = output as i16;
                let lt = ioutput < 0;
                let gt = ioutput > 0;
                let eq = output == 0;

                if (lt & inst.get_lt()) | (gt & inst.get_gt()) | (eq & inst.get_eq()) {
                    // apply jump. We set the PC to A - 1, because we will increment after.
                    self.pc.set(self.mem.a - 1);
                }
                self.mem.set_reg(target, output);
            }
            Instruction::Data(val) => {
                self.mem.a = val;
            }
        };
        Ok(())
    }

    /// Serializes the current state of the VM. Currently serialization of access levels and ROM maps is not supported
    /// The ROM Memory will already be mapped into Memory at the point of serialization so no difference in runtime behavior is expected
    /// The only difference is that access levels of memory are now all R/W.
    pub fn to_vm_desc(&self) -> VmDescription {
        let pc = self.pc.get_val();
        let rom = self.rom.program_text.clone();
        let mem = Box::new(self.mem.ram.ram);
        /*let callbacks = self
        .mem
        .ram
        .write_callbacks
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();*/
        let rom_mappings = vec![];
        let regs = self.mem.get_regs().into();

        VmDescription {
            pc,
            rom,
            mem,
            callbacks: vec![],
            rom_mappings,
            regs,
            rom_blocks: vec![],
        }
    }

    /// Wraps [`Ram::set_ram()`]
    pub fn set_ram(&mut self, address: u16, value: u16) {
        self.mem.set_ram(address, value);
    }

    /// Wraps [`MemoryUnit::get_regs()`]
    pub fn get_regs(&self) -> (u16, u16, u16, u16, u16, u16) {
        self.mem.get_regs()
    }

    /// Wraps [`MemoryUnit::set_reg()`]
    pub fn set_reg(&mut self, reg: Register, val: u16) {
        self.mem.set_reg(reg, val);
    }

    /// Wraps [`MemoryUnit::get_reg()`]
    pub fn get_reg(&self, reg: Register) -> u16 {
        self.mem.get_reg(reg)
    }

    /// Wraps [`Ram::get_ram_region()`]
    pub fn get_ram_region(&self, address: u16, length: usize) -> &[u16] {
        self.mem.get_ram_region(address, length)
    }

    /// Returns a segment of ROM between `address` and `address + length`. If part of the specified segment is outside
    /// the ROM, it gets cut off
    pub fn get_rom_region(&self, address: u16, length: usize) -> &[u16] {
        self.rom.get_rom_region(address, length)
    }

    /// Set the PC
    pub fn set_pc(&mut self, new: u16) {
        self.pc.set(new);
    }

    /// Get PC
    pub fn get_pc(&self) -> u16 {
        self.pc.get_val()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[test]
    fn test_ram() {
        let mut ram = Ram::new(
            [0; 65536],
            RegionMap::try_from(vec![
                Region::new(0x100, 0x1ff, AccessLevels::Read),
                Region::new(0x200, 0x2ff, AccessLevels::None),
            ])
            .unwrap(),
        );

        // Test access levels
        ram.write_ram(0, 1);
        assert_eq!(ram.read_ram(0), 1);
        ram.set_ram(0x101, 1234);
        assert_eq!(ram.read_ram(0x101), 1234);
        ram.set_ram(0x100, 5432);
        ram.write_ram(0x100, 1);
        assert_eq!(ram.read_ram(0x100), 5432);
        ram.set_ram(0x200, 0xabc);
        assert_eq!(ram.read_ram(0x200), 0);
        ram.write_ram(0x200, 0xdef);
        assert_eq!(ram.read_ram(0x200), 0);
        assert_eq!(ram.get_ram_region(0x200, 1), &[0xabc]);

        // Test set_ram
        ram.set_ram(0, 1);
        assert_eq!(ram.read_ram(0), 1);
        ram.set_ram(0xff, 0xbeef);
        assert_eq!(ram.read_ram(0xff), 0xbeef);

        // Test set_ram_region
        let arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

        ram.set_ram_region(0, &arr).unwrap();
        assert_eq!(ram.get_ram_region(0, arr.len()), &arr);

        // Test callbacks
        let out_var = Arc::new(Mutex::new(0));
        let out = out_var.clone();

        let callback = Box::new(move |new_value: u16| {
            let mut out_p = out.lock().unwrap();
            *out_p = new_value;
        });

        ram.register_callback(0x99, callback);
        ram.write_ram(0x99, 0x1234);
        assert_eq!(*out_var.lock().unwrap(), 0x1234);
    }

    #[test]
    fn test_mem() {
        let mut mem = MemoryUnit::new(0, 0, 0, 0, 0, 0, [0; 65536], vec![]).unwrap();

        // test set_reg
        mem.set_reg(Register::A, 0x1234);
        assert_eq!(mem.get_reg(Register::A), 0x1234);

        // test get_reg on ram
        assert_eq!(mem.get_reg(Register::None), 0);
        mem.set_ram(0x1234, 0xabc);
        assert_eq!(mem.get_reg(Register::MA), 0xabc);

        // test get_reg on ram
        mem.set_reg(Register::MA, 0xdef);
        assert_eq!(mem.get_reg(Register::MA), 0xdef);
        assert_eq!(mem.get_ram_region(0x1234, 1), &[0xdef]);

        // test get_regs
        mem.set_reg(Register::D, 0x1234);
        assert_eq!(mem.get_regs(), (0x1234, 0x1234, 0, 0, 0, 0));

        // don't test other methods, they are just wrappers for RAM
    }

    #[test]
    fn test_rom() {
        let rvs = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let rom = Rom::from_iter(rvs.clone());
        assert_eq!(rom.get_address(0), Some(1));

        assert_eq!(rom.get_rom_region(2, 0xff), &rvs[2..]);
    }

    #[test]
    fn test_pc() {
        let mut pc = Pc::new(0);
        pc.inc();
        pc.inc();
        assert_eq!(pc.get_val(), 2);

        pc.set(0xffff);
        pc.inc();
        assert_eq!(pc.get_val(), 0);
    }

    #[test]
    fn test_instruction() {
        match Instruction::from_u16(0b1000000000000010) {
            Instruction::Alu(_) => {
                panic!("incorrect data instruction decoding")
            }
            Instruction::Data(val) => {
                assert_eq!(val, 2)
            }
        }

        match Instruction::from_u16(0b0000000000000010) {
            Instruction::Alu(inst) => {
                // no need to test all ALU instruction fields. we trust `bitfield` is correctly implemented
                assert_eq!(inst.get_eq(), true);
            }
            Instruction::Data(_) => {
                panic!("incorrect alu instruction decoding")
            }
        }
    }

    #[test]
    fn test_vm_description() {
        // this test is very limited and does not correctly explore all cases. It will do for now.

        // set up some values
        let vm_desc = VmDescription {
            pc: 0x123,
            rom: vec![0x1234, 0x5678],
            mem: Box::new(core::array::from_fn(|i| i as u16)),
            rom_mappings: vec![(0x123, 0x456, 0x789)],
            regs: [1, 2, 3, 4, 5, 6],
            ..Default::default()
        };

        // serialize and deserialize
        let serialized = vm_desc.serialize().unwrap();
        let deserialized = VmDescription::deserialize(&serialized).unwrap();

        // check values
        assert_eq!(deserialized.pc, 0x123);
        assert_eq!(deserialized.rom, vec![0x1234, 0x5678]);
        assert_eq!(
            deserialized.mem.as_slice(),
            &core::array::from_fn::<u16, RAM_LEN, _>(|i| i as u16)
        );
        assert_eq!(deserialized.rom_mappings, &[(0x123, 0x456, 0x789)]);
        assert_eq!(&deserialized.regs, &[1u16, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_vm() {
        // this test relies on the assembler being correctly implemented. We do not test all
        // functions of the VM but test wether some of them work correctly together.
        let code = r"[macros]
[text]
A = 0x1234
D = add, 0, A
A = 0
*A = add, 0, D
A = 0x512
JMP
[consts 0x100]";

        let vm_desc = crate::assembler::run(code).unwrap();

        let mut vm = Vm::new(vm_desc).unwrap();
        for _ in 0..6 {
            vm.cycle().unwrap();
        }
        assert_eq!(vm.get_reg(Register::A), 0x512);
        assert_eq!(vm.get_reg(Register::D), 0x1234);
        assert_eq!(vm.get_pc(), 0x512);
        assert_eq!(vm.get_ram_region(0x0, 2), &[0x1234, 0x0]);
        assert!(matches!(
            vm.cycle(),
            Err(BError::ExecutionHaltedError { .. })
        ));
    }
}
