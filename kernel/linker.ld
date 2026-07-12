/* SIGRUN Kernel Linker Script */
ENTRY(_start)

SECTIONS {
    . = 0x100000;   /* Load at 1MB physical */

    /* Multiboot2 header must appear in the first 32KB */
    .multiboot2_header ALIGN(8) : {
        KEEP(*(.multiboot2_header))
    }

    /* 32-bit boot code + data */
    .boot.text ALIGN(16) : {
        KEEP(*(.boot.text))
    }
    .boot.data ALIGN(8) : {
        KEEP(*(.boot.data))
    }

    /* 64-bit kernel code */
    .text ALIGN(4096) : {
        *(.text.unlikely*)
        *(.text*)
    }

    .rodata ALIGN(4096) : {
        *(.rodata*)
    }

    .data ALIGN(4096) : {
        *(.data*)
    }

    /* Boot page tables – must be 4096-aligned, zero-initialised */
    .boot.pgtbl ALIGN(4096) (NOLOAD) : {
        KEEP(*(.boot.pgtbl))
    }

    /* Boot stack */
    .boot.stack ALIGN(4096) (NOLOAD) : {
        KEEP(*(.boot.stack))
    }

    /* Regular BSS */
    .bss ALIGN(4096) : {
        __bss_start = .;
        *(.bss*)
        *(COMMON)
        __bss_end = .;
    }

    __kernel_end = ALIGN(4096);

    /DISCARD/ : {
        *(.eh_frame*)
        *(.note.GNU-stack)
    }
}
