/*
 * Minimal ESP32 Test Firmware
 *
 * This firmware tests basic CPU functionality and ROM stub integration.
 * It calls ROM functions to verify the emulator can handle:
 * - Basic Xtensa instructions
 * - ROM function dispatching
 * - Memory-mapped flash execution
 */

.section .text
.global _start
.align 4

_start:
    # Initialize stack pointer to top of SRAM
    movi    a1, 0x3FFFFFFF

    # Test 1: Call ets_get_cpu_frequency (returns 160 MHz for ESP32)
    movi    a0, test_printf       # Set return address
    movi    a2, 0                 # Clear a2 for result
    movi    a3, 0x40008550        # ets_get_cpu_frequency ROM address
    callx0  a3                    # Indirect call

test_printf:
    # a2 should now contain CPU frequency (160)
    # Save it for later
    mov     a10, a2

    # Test 2: Call ets_printf to print result
    movi    a0, test_delay        # Set return address
    movi    a2, format_str        # Format string pointer (in SRAM)
    mov     a3, a10               # Frequency value
    movi    a4, 0x40007D54        # ets_printf ROM address
    callx0  a4                    # Call ets_printf

test_delay:
    # Test 3: Call ets_delay_us (delay 100 microseconds)
    movi    a0, test_memcpy       # Set return address
    movi    a2, 100               # Delay 100 us
    movi    a3, 0x40008534        # ets_delay_us ROM address
    callx0  a3

test_memcpy:
    # Test 4: Call memcpy to copy data
    movi    a0, halt_loop         # Set return address
    movi    a2, dest_buf          # Destination buffer
    movi    a3, src_buf           # Source buffer
    movi    a4, 16                # Copy 16 bytes
    movi    a5, 0x40008148        # memcpy ROM address
    callx0  a5

halt_loop:
    # Success - infinite loop
    j       halt_loop

.section .data
.align 4

format_str:
    .asciz "CPU Frequency: %d MHz\n"

src_buf:
    .byte 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08
    .byte 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10

dest_buf:
    .space 16, 0
