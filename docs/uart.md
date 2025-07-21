# UART
This UART is my own design and has never been tested on electronics, so if implemented in reality, there might be a ton of syncing issues. Therefor this is a preliminary version and subject to change.

UART similar to the Mini-UART available on the Raspberry Pi 3:
https://datasheets.raspberrypi.com/bcm2711/bcm2711-peripherals.pdf
This UART works as follows:
- UART clock is controlled by system clock
- One FIFO for input from the UART
- One FIFO for output into the UART
- One Baud rate register which sets the Baud relative to the system clock
- One flags register which gives status about the FIFOs
The raspberrypi UART also has support for interrupts, which this architecture does not support

We map the UART in the following way
| Address       | Name    | Function                          |
| ------------- | ------- | --------------------------------- |
| 0x6000        | U_BAUD  | Set Baud                          |
| 0x6001        | U_OUT   | Write one byte into output FIFO   |
| 0x6002        | U_IN    | Read one byte from input FIFO     |
| 0x6003        | U_IFL   | Input flags                       |
| 0x6004        | U_OFL   | Output flags                      |

The Output (Writable) Flags are as follows
- OW: Output written
- IR: Input read
- RU: Reset uart

The Input (Readable) Flags are as follows
- IO: Input FIFO overflowed
- DA: Input FIFO has data
- OR: Output FIFO is ready

The general way of interacting with the UART is as follows
1. Choose a baud rate. Calculate the number of clock cycles it would take for a 40MHz clock to complete one cycle of that baud rate
2. Write that baud rate to the U_BAUD register
3. To write a byte: set the OW flag to low, wait for the OR flag to be high, then write a byte to the U_OUT register, set the OW written flag to high
4. To read a byte: set IR to low, wait for the DA flag to be high, read a byte from U_IN, set IR to high

## Notes on the VM implementation
The implementation for the VM is instant. That means there is not time needed to transmit data. Otherwise it would take the amount of cycles as are written into U_BAUD to transmit one byte.