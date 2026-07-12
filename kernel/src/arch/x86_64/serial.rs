//! Serial port driver for early console output

const COM1_PORT: u16 = 0x3F8;

pub fn init() {
    unsafe {
        x86_out8(COM1_PORT + 1, 0x00);
        x86_out8(COM1_PORT + 3, 0x80);
        x86_out8(COM1_PORT + 0, 0x03);
        x86_out8(COM1_PORT + 1, 0x00);
        x86_out8(COM1_PORT + 3, 0x03);
        x86_out8(COM1_PORT + 2, 0xC7);
        x86_out8(COM1_PORT + 4, 0x0B);
    }
}

pub fn write_byte(b: u8) {
    unsafe {
        while (x86_in8(COM1_PORT + 5) & 0x20) == 0 {}
        x86_out8(COM1_PORT, b);
    }
}

pub fn write(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

pub fn writeln(s: &str) {
    write(s);
    write_byte(b'\r');
    write_byte(b'\n');
}

#[inline(always)]
unsafe fn x86_out8(port: u16, value: u8) {
    // `out dx, al` — port in DX, byte value in AL.
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags)
    );
}

#[inline(always)]
unsafe fn x86_in8(port: u16) -> u8 {
    let value: u8;
    // `in al, dx` — read from port in DX into AL.
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags)
    );
    value
}
