#!/usr/bin/env python3
"""
Generate a minimal ESP32 test binary in the correct format.

This creates a valid ESP32 firmware binary that can be loaded by the emulator.
The binary contains a simple program that executes a few instructions and halts.
"""

import struct

def create_esp32_binary(output_file):
    """
    Create a minimal ESP32 binary with the proper format.

    ESP32 Binary Format:
    - Magic byte: 0xE9
    - Segment count: 1-16
    - SPI mode: 0 (QIO)
    - SPI speed/size: 0x20 (40MHz, 4MB)
    - Entry point: 4 bytes (little-endian)
    - Segments: variable
    """

    # Valid Xtensa code with proper opcodes
    # This creates a minimal program that executes valid instructions

    # We'll create a simple program:
    # 1. Execute a few NOPs
    # 2. Use BEQZ a0, -7 to loop back (since a0 starts at 0, this always branches)

    # BEQZ instruction format (op0=6, op1=0):
    # Bits: [offset:8][s:4][t:4][op1:4][op0:4]
    # For BEQZ a0, offset:
    #   - op0 = 6
    #   - op1 = 0  (bits [12:15])
    #   - s = 0 (a0) (bits [8:11])
    #   - t = ? (bits [4:7])
    #   - offset = signed 8-bit in bits [16:23]

    # To create BEQZ a0, -7 (jump back to first NOP):
    # - Offset -7 in 8-bit signed = 0xF9 (two's complement)
    # - Word = 0xF90016 (offset=F9, s=0, t=0, op1=0, op0=6)
    # - Bytes (little-endian): 16 00 F9

    code = bytes([
        # NOP #1 (address 0x40080000)
        0xF0, 0x20, 0x00,       # NOP

        # NOP #2 (address 0x40080003)
        0xF0, 0x20, 0x00,       # NOP

        # BEQZ a0, -7 (address 0x40080006, jumps back to 0x40080000)
        # This creates an infinite loop since a0 is 0
        # Offset calculation: target (0x40080000) - (PC+4) where PC=0x40080006
        # = 0x40080000 - 0x4008000A = -10 (-0xA)
        # Actually offset is relative to PC+4, so: -10 decimal = 0xF6 in 8-bit signed
        0x16, 0x00, 0xF6,       # BEQZ a0, -10

        # Extra NOPs as padding (won't execute due to loop)
        0xF0, 0x20, 0x00,
        0xF0, 0x20, 0x00,
    ])

    # Build the binary
    binary = bytearray()

    # Header
    binary.append(0xE9)                    # Magic byte
    binary.append(1)                       # Segment count (1 segment)
    binary.append(0)                       # SPI mode (QIO)
    binary.append(0x20)                    # SPI speed/size (40MHz, 4MB)

    # Entry point (0x40080000 - flash instruction region)
    entry_point = 0x40080000
    binary.extend(struct.pack('<I', entry_point))

    # Segment 1: Code in flash instruction region
    segment_addr = 0x40080000
    segment_data = code

    binary.extend(struct.pack('<I', segment_addr))      # Load address
    binary.extend(struct.pack('<I', len(segment_data))) # Data length
    binary.extend(segment_data)                         # Data

    # Write to file
    with open(output_file, 'wb') as f:
        f.write(binary)

    print(f"Created {output_file} ({len(binary)} bytes)")
    print(f"  Magic: 0x{binary[0]:02X}")
    print(f"  Segments: {binary[1]}")
    print(f"  Entry point: 0x{entry_point:08X}")
    print(f"  Segment 0: addr=0x{segment_addr:08X}, size={len(segment_data)}")

    # Dump first 64 bytes for verification
    print("\nFirst 64 bytes:")
    for i in range(0, min(64, len(binary)), 16):
        hex_str = ' '.join(f'{b:02X}' for b in binary[i:i+16])
        print(f"  {i:04X}: {hex_str}")

if __name__ == '__main__':
    create_esp32_binary('minimal_test.bin')
