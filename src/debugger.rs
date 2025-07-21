use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use crate::{
    BError,
    mmio::uart::{Uart, connect_uart},
    vm::{self, Register, Vm, VmDescription},
};

/// Represents a debugger with breakpoints and uart
pub struct Debugger {
    vm: Vm,
    breakpoints: HashSet<u16>,
    halted: bool,
    uart: Option<Arc<Mutex<Uart>>>,
    current_uart_out: String,
}

impl Debugger {
    /// Create a new debugger. `use_uart` specifies whether UART is initialized.
    pub fn new(
        description: VmDescription,
        breakpoints: Vec<u16>,
        use_uart: bool,
    ) -> Result<Self, BError> {
        let (vm, uart) = if use_uart {
            let (v, u) = connect_uart(description)?;
            (v, Some(u))
        } else {
            (Vm::new(description)?, None)
        };

        Ok(Self {
            vm,
            breakpoints: HashSet::from_iter(breakpoints),
            halted: false,
            uart,
            current_uart_out: String::new(),
        })
    }

    fn cycle(&mut self) -> Result<(), BError> {
        self.vm.cycle()?;
        if let Some(uart) = &mut self.uart {
            let mut curt = uart.lock().unwrap();
            // read uart vm -> dbg
            if let Some(v) = curt.get_output() {
                self.current_uart_out.push(v as char);
            }
            // synchronize the uart object with the vm
            self.vm.set_ram(crate::mmio::uart::U_IN, curt.get_input());
            self.vm
                .set_ram(crate::mmio::uart::U_IFL, curt.get_in_flags());
        }
        Ok(())
    }

    /// Write a byte to the UART
    pub fn write_uart_byte(&mut self, byte: u8) {
        if let Some(uart) = &mut self.uart {
            uart.lock().unwrap().put_input(byte);
        }
    }

    /// Get the UART output as a string
    pub fn get_uart_out(&mut self) -> String {
        let out = self.current_uart_out.clone();
        self.current_uart_out = String::new();
        out
    }

    /// Get ROM
    pub fn get_rom(&self) -> &[u16] {
        self.vm.get_rom_region(0x00, 0xffff)
    }

    /// Step the CPU
    pub fn step(&mut self) {
        if self.halted {
            return;
        }
        match self.cycle() {
            Ok(_) => {}
            Err(BError::ExecutionHaltedError { value: _ }) => self.halted = true,
            Err(e) => panic!("{}", e),
        }
    }

    /// Inspect memory in range `from`:`from + length`
    pub fn inspect_memory(&self, from: u16, length: u16) -> &[u16] {
        self.vm.get_ram_region(from, length as usize)
    }

    /// Set memory in range `from`:`from + values.len()` to `values`
    pub fn set_memory(&mut self, from: u16, values: Vec<u16>) -> Result<(), BError> {
        if (from as usize) + values.len() > vm::RAM_LEN {
            return Err(BError::OutOfBoundsError(from, values.len(), vm::RAM_LEN));
        }
        for (idx, val) in values.iter().enumerate() {
            self.vm.set_ram(from + idx as u16, *val);
        }
        Ok(())
    }

    /// Inspect a register
    pub fn inspect_reg(&self, register: Register) -> u16 {
        self.vm.get_reg(register)
    }

    /// Inspect ROM in range `from`:`from + length`
    pub fn inspect_rom(&self, from: u16, length: u16) -> &[u16] {
        self.vm.get_rom_region(from, length as usize)
    }

    /// Set a register
    pub fn set_reg(&mut self, register: Register, value: u16) {
        self.vm.set_reg(register, value);
    }

    /// Set the program counter
    pub fn set_pc(&mut self, new_value: u16) {
        self.vm.set_pc(new_value);
    }

    /// Get the program counter
    pub fn get_pc(&self) -> u16 {
        self.vm.get_pc()
    }

    /// Run the VM until we hit a breakpoint, halt or reach max_iter cycles
    pub fn run(&mut self, max_iter: usize) {
        if self.halted {
            return;
        }
        for _ in 0..max_iter {
            match self.cycle() {
                Ok(_) => {}
                Err(BError::ExecutionHaltedError { value: _ }) => {
                    self.halted = true;
                    return;
                }
                Err(e) => panic!("{}", e),
            }
            if self.breakpoints.contains(&self.get_pc()) {
                return;
            }
        }
    }

    /// Register a breakpoint at ROM address `breakpoint`
    pub fn register_breakpoint(&mut self, breakpoint: u16) {
        self.breakpoints.insert(breakpoint);
    }

    /// Remove a breakpoint at ROM address `breakpoint`
    pub fn remove_breakpoint(&mut self, breakpoint: u16) -> bool {
        self.breakpoints.remove(&breakpoint)
    }

    /// Serialize the current state of the debugger. Does not save Memory access levels, ROM mappings, or callbacks.
    /// Also does not serialize the halted state
    pub fn serialize(&self) -> Result<Vec<u8>, BError> {
        let mut output = Vec::new();

        // Magic
        output.append(&mut b"BDB\x00BPS\x00".to_vec());

        // Number of Breakpoints
        let no_bps = self.breakpoints.len();
        if no_bps > 0xffff {
            return Err(BError::SerializationError(
                "Number of breakpoints to large".to_string(),
            ));
        }
        output.append(&mut (no_bps as u16).to_be_bytes().to_vec());
        output.push(0x00);

        // Breakpoints
        for bp in self.breakpoints.iter() {
            output.append(&mut bp.to_be_bytes().to_vec());
        }
        output.push(0x00);

        let mut vm_ser = self.vm.to_vm_desc().serialize()?;
        output.append(&mut vm_ser);

        Ok(output)
    }

    /// Create a debugger from a .bdb file
    /// The UART is not yet implemented in the .bdb spec, so we do not deserialize it here
    pub fn deserialize(input: &[u8]) -> Result<Self, BError> {
        use crate::util::{check_slice, extract_number};
        let current = input;

        // Check Magic
        if check_slice(current, 4)? != b"BDB\x00" {
            return Err(BError::DeserializationError(
                "Invalid file format".to_string(),
            ));
        }
        let current = &current[4..];

        // Breakpoints
        let bp_nums = check_slice(current, 3)?;
        let bp_amount = extract_number(bp_nums)? as usize;
        let current = &current[3..];

        let bp_len = 2 * bp_amount + 1;
        let bp_region = check_slice(current, bp_len)?;
        if bp_region[bp_len - 1] != 0x00 {
            return Err(BError::DeserializationError(
                "Invalid region separators".to_string(),
            ));
        }
        let mut breakpoints = HashSet::new();
        for i in 0..bp_amount {
            let j = 2 * i;
            breakpoints.insert(u16::from_be_bytes([bp_region[j], bp_region[j + 1]]));
        }
        let current = &current[bp_len..];

        let vm = Vm::new(VmDescription::deserialize(current)?)?;

        Ok(Self {
            vm,
            breakpoints,
            halted: false,
            uart: None,
            current_uart_out: String::new(),
        })
    }
}
