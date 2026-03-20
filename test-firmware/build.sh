#!/bin/bash
#
# Build script for minimal ESP32 test firmware
#
# Requirements:
# - xtensa-esp32-elf-gcc (ESP32 toolchain)
# - esptool.py (for creating ESP32 binary format)
#
# Install via:
#   esp-idf setup or standalone xtensa toolchain

set -e

TOOLCHAIN_PREFIX=xtensa-esp32-elf-
ASM_FILE=minimal.S
ELF_FILE=minimal.elf
BIN_FILE=minimal.bin

echo "Building minimal ESP32 test firmware..."

# Assemble and link
${TOOLCHAIN_PREFIX}gcc \
    -nostdlib \
    -Wl,-Ttext=0x40080000 \
    -Wl,-Tdata=0x3FFA0000 \
    -o ${ELF_FILE} \
    ${ASM_FILE}

echo "ELF file created: ${ELF_FILE}"

# Convert to ESP32 binary format
if command -v esptool.py &> /dev/null; then
    esptool.py --chip esp32 elf2image --output ${BIN_FILE} ${ELF_FILE}
    echo "Binary file created: ${BIN_FILE}"

    # Show file info
    ls -lh ${BIN_FILE}
    xxd -l 64 ${BIN_FILE}
else
    echo "Warning: esptool.py not found - skipping binary conversion"
    echo "Install with: pip install esptool"

    # Fallback: extract .text section as raw binary
    ${TOOLCHAIN_PREFIX}objcopy -O binary ${ELF_FILE} ${BIN_FILE}.raw
    echo "Raw binary created: ${BIN_FILE}.raw (not in ESP32 format)"
fi

echo "Build complete!"
