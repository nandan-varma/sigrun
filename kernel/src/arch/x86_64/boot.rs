//! Kernel boot entry: multiboot2 header, 32→64 bit long-mode transition,
//! and the first 64-bit Rust function that builds BootParams and calls kmain.
//!
//! All assembly uses Intel syntax (Rust's global_asm! default on x86_64).

use core::arch::global_asm;

/// Multiboot2 magic the bootloader places in EAX before calling _start.
pub const MB2_BOOTLOADER_MAGIC: u32 = 0x36d76289;

// This defines `_start`, which collides with the host C runtime's own
// `_start` (from Scrt1.o) when linking a host-target test binary — bare
// metal only.
#[cfg(target_os = "none")]
global_asm!(
    // ── Boot stack (64 KB, zero-initialised by multiboot2 loader) ─────────
    ".section .boot.stack, \"aw\", @nobits",
    ".align 4096",
    ".global _boot_stack_bot",
    "_boot_stack_bot: .skip 65536",
    ".global _boot_stack_top",
    "_boot_stack_top:",
    // ── Boot page tables (zero-initialised by multiboot2 loader) ──────────
    ".section .boot.pgtbl, \"aw\", @nobits",
    ".align 4096",
    ".global _boot_pml4",
    "_boot_pml4: .skip 4096",
    ".global _boot_pdpt",
    "_boot_pdpt: .skip 4096",
    // ── Boot GDT ──────────────────────────────────────────────────────────
    ".section .boot.data, \"aw\"",
    ".align 8",
    "_boot_gdt_start:",
    ".quad 0x0000000000000000", // 0x00 null
    ".quad 0x00af9a000000ffff", // 0x08 64-bit code, DPL=0
    ".quad 0x00cf92000000ffff", // 0x10 64-bit data, DPL=0
    "_boot_gdt_end:",
    // GDTR pseudo-descriptor: 2-byte limit then 4-byte base
    "_boot_gdtr:",
    ".short _boot_gdt_end - _boot_gdt_start - 1",
    ".long  _boot_gdt_start",
    // ── Multiboot2 header (must lie in the first 32 KB of the image) ──────
    ".section .multiboot2_header, \"a\"",
    ".align 8",
    "_mb2_start:",
    ".long 0xe85250d6",
    ".long 0",
    ".long _mb2_end - _mb2_start",
    ".long -(0xe85250d6 + 0 + (_mb2_end - _mb2_start))",
    ".short 0",
    ".short 0",
    ".long 8", // end tag
    "_mb2_end:",
    // ── 32-bit protected-mode entry ────────────────────────────────────────
    // QEMU's multiboot2 loader starts us here with:
    //   EAX = 0x36d76289 (multiboot2 magic)
    //   EBX = physical address of multiboot2 info structure
    //   CS  = 32-bit flat code segment, CPL=0
    //   PE  = 1 (protected mode)
    //   PG  = 0 (no paging yet)
    ".section .boot.text, \"ax\"",
    ".code32",
    ".global _start",
    ".type _start, @function",
    "_start:",
    "cli",
    "cld",
    // Establish the boot stack.  The BSS is zero-filled by the loader.
    // Intel syntax: lea loads the EFFECTIVE ADDRESS (not the contents).
    "lea esp, [_boot_stack_top]",
    // Save EAX (magic) and EBX (info ptr) on the new stack.
    "push ebx", // [esp+0] = info ptr
    "push eax", // [esp+4] = magic  (pushed last, popped first)
    // ── CPUID: verify long-mode is supported ──────────────────────────────
    "mov eax, 0x80000000",
    "cpuid",
    "cmp eax, 0x80000001",
    "jb  _boot_halt32",
    "mov eax, 0x80000001",
    "cpuid",
    "test edx, 0x20000000", // bit 29 = LM (long mode)
    "jz  _boot_halt32",
    // ── Page tables: identity-map first 4 GB with 1-GB huge pages ─────────
    // PML4[0] = &_boot_pdpt | PRESENT(1) | WRITE(2)
    "lea eax, [_boot_pdpt]",
    "or  eax, 3",
    "mov dword ptr [_boot_pml4],     eax",
    "mov dword ptr [_boot_pml4 + 4], 0",
    // PDPT[0] maps 0–1 GB  (PRESENT | WRITE | HUGE = 0x83)
    "mov dword ptr [_boot_pdpt +  0], 0x00000083",
    "mov dword ptr [_boot_pdpt +  4], 0",
    // PDPT[1] maps 1–2 GB
    "mov dword ptr [_boot_pdpt +  8], 0x40000083",
    "mov dword ptr [_boot_pdpt + 12], 0",
    // PDPT[2] maps 2–3 GB
    "mov dword ptr [_boot_pdpt + 16], 0x80000083",
    "mov dword ptr [_boot_pdpt + 20], 0",
    // PDPT[3] maps 3–4 GB  (LAPIC at 0xFEE0_0000 lives here)
    "mov dword ptr [_boot_pdpt + 24], 0xC0000083",
    "mov dword ptr [_boot_pdpt + 28], 0",
    // Load our 64-bit GDT.
    "lgdt [_boot_gdtr]",
    // Enable PAE (CR4 bit 5).
    "mov eax, cr4",
    "or  eax, 0x20",
    "mov cr4, eax",
    // Point CR3 at PML4.
    "lea eax, [_boot_pml4]",
    "mov cr3, eax",
    // Set EFER.LME (EFER MSR = 0xC0000080, bit 8).
    "mov ecx, 0xC0000080",
    "rdmsr",
    "or  eax, 0x100",
    "wrmsr",
    // Enable paging + protected-mode (CR0 bit 31 + bit 0).
    // Setting PG with PAE+LME active enters 64-bit long mode.
    "mov eax, cr0",
    "or  eax, 0x80000001",
    "mov cr0, eax",
    // Restore EAX (magic → will be EDI = 1st arg) and EBX (info → ESI = 2nd arg).
    "pop edi", // magic  (System-V 1st arg register)
    "pop esi", // info   (System-V 2nd arg register)
    // Far jump to the 64-bit code segment (selector 0x08 = _boot_gdt[1]).
    // Encoded manually because the Intel-syntax far-jump mnemonic varies
    // between assembler implementations.
    ".byte 0xEA",
    ".long _boot_64_entry",
    ".short 0x08",
    "_boot_halt32:",
    "hlt",
    "jmp _boot_halt32",
    // ── 64-bit long-mode entry ────────────────────────────────────────────
    ".code64",
    ".global _boot_64_entry",
    "_boot_64_entry:",
    // Flush stale segment-register caches with the data selector (0x10).
    "mov ax, 0x10",
    "mov ds, ax",
    "mov es, ax",
    "mov ss, ax",
    "xor ax, ax",
    "mov fs, ax",
    "mov gs, ax",
    // RSP: the 32-bit ESP set in protected-mode is still valid now that
    // paging identity-maps the low 4 GB.

    // EDI/ESI were set in 32-bit mode and are now zero-extended to
    // RDI/RSI – the correct System-V argument registers for:
    //   kmain_from_multiboot2(magic: u32, info_ptr: u64)
    "call kmain_from_multiboot2",
    ".global _boot_halt64",
    "_boot_halt64:",
    "cli",
    "hlt",
    "jmp _boot_halt64",
);

// `kmain_from_multiboot2` is defined in main.rs (crate root) so it can call
// `kmain` without a cross-module path.  The assembly stub above uses the
// exported symbol name regardless of which Rust module owns the definition.
