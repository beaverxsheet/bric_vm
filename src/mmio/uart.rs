use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{
    BError,
    vm::{Vm, VmDescription},
};

// input flags
/// Input FIFO overflowed
pub const IO: u16 = 1;
/// Data available
pub const DA: u16 = 1 << 1;
/// Output FIFO ready
pub const OR: u16 = 1 << 2;

// output flags
/// Output written
pub const OW: u16 = 1;
/// Input read
pub const IR: u16 = 1 << 1;
/// Reset
pub const RU: u16 = 1 << 2;

pub const INPUT_BUF_LEN: usize = 0xff;
pub const OUTPUT_BUF_LEN: usize = 0xff;

/// Baud rate register. Baud = 40_000_000 / U_BAUD
pub const U_BAUD: u16 = 0x6000;
/// UART output register
pub const U_OUT: u16 = 0x6001;
/// UART input register
pub const U_IN: u16 = 0x6002;
/// UART i flags register dbg -> vm
pub const U_IFL: u16 = 0x6003;
/// UART o flags register vm -> dbg
pub const U_OFL: u16 = 0x6004;

/// Represents the UART.
pub struct Uart {
    input: VecDeque<u8>,
    output: VecDeque<u8>,
    write_reg: u8,
    read_reg: u8,
    in_flags: u16,
}

impl Uart {
    /// called when the U_OUT register is written to
    pub fn write_reg_changed(&mut self, reg_content: u16) {
        self.write_reg = (reg_content & !(0xff << 8)) as u8;
    }

    /// called when the U_OFL register is written to
    pub fn output_flags_changed(&mut self, reg_content: u16) {
        // output written
        if reg_content & OW != 0 {
            self.output.push_front(self.write_reg);
            let out_len = self.output.len();
            if out_len > OUTPUT_BUF_LEN {
                self.output.pop_back();
            } else if out_len == OUTPUT_BUF_LEN {
                self.in_flags &= !OR;
            } else {
                self.in_flags |= OR;
            }
        }
        // input read
        if reg_content & IR != 0 {
            self.read_reg = self.input.pop_back().unwrap_or(0);
            if self.input.len() > 0 {
                self.in_flags |= DA;
            } else {
                self.in_flags &= !DA;
            }
            self.in_flags &= !IO;
        }
        // reset
        if reg_content & RU != 0 {
            self.input = VecDeque::new();
            self.output = VecDeque::new();
            self.write_reg = 0;
            self.read_reg = 0;
            self.in_flags = 0b100;
        }
    }

    /// write an input to the U_IN FIFO
    pub fn put_input(&mut self, input_byte: u8) {
        self.input.push_front(input_byte);
        self.in_flags |= DA;
        let inp_len = self.input.len();
        if inp_len > INPUT_BUF_LEN {
            self.input.pop_back();
        } else if inp_len == INPUT_BUF_LEN {
            self.in_flags |= IO;
        }
    }

    /// get the U_OFL flags
    pub fn get_in_flags(&self) -> u16 {
        self.in_flags
    }

    /// get the U_IN register
    pub fn get_input(&self) -> u16 {
        self.read_reg as u16
    }

    /// get the U_OUT register
    pub fn get_output(&mut self) -> Option<u8> {
        self.output.pop_back()
    }
}

impl Default for Uart {
    fn default() -> Self {
        Self {
            input: VecDeque::new(),
            output: VecDeque::new(),
            write_reg: 0,
            read_reg: 0,
            in_flags: 0b100,
        }
    }
}

/// Modifies a VmDescription to mount a UART, creates a UART
/// The UARTs registers must manually be updated when running the VM
/// The UART object itself can also be used on another thread.
pub fn connect_uart(mut vm_desc: VmDescription) -> Result<(Vm, Arc<Mutex<Uart>>), BError> {
    // Build Uart
    let uart = Arc::new(Mutex::new(Uart::default()));
    // Modify VmDescription
    let wc_uart = uart.clone();
    let of_uart = uart.clone();

    let write_change = Box::new(move |input: u16| {
        wc_uart.lock().unwrap().write_reg_changed(input);
    });

    let of_change = Box::new(move |input: u16| {
        of_uart.lock().unwrap().output_flags_changed(input);
    });

    vm_desc.rom_blocks.push((U_IN, 1));

    vm_desc.callbacks.push((U_OUT, write_change));
    vm_desc.callbacks.push((U_OFL, of_change));

    vm_desc.mem[U_IFL as usize] = 0b100;
    // Build Vm
    let vm = Vm::new(vm_desc)?;
    Ok((vm, uart))
}
