//! LAPIC timer driver.
//!
//! Uses the PIT (channel 2) to calibrate the LAPIC bus frequency, then
//! programs the LAPIC to fire a periodic interrupt every ~10 ms.

use crate::arch::x86_64::apic::{Register, LOCAL_APIC};

// ── PIT I/O ports ────────────────────────────────────────────────────────────
const PIT_CHANNEL2: u16 = 0x42;
const PIT_COMMAND: u16 = 0x43;
const PORT_SYSCTRL: u16 = 0x61; // PC system control port A

// PIT input frequency (Hz).
const PIT_FREQ: u64 = 1_193_182;

// We ask the PIT to count for ~10 ms.
const PIT_TICKS_10MS: u16 = (PIT_FREQ / 100) as u16; // 11931

// LAPIC timer vector (must match the IDT entry installed in interrupt::mod).
pub const TIMER_VECTOR: u8 = 32;

// LAPIC divide-by value encoded in the TimerDivide register.
// 0x03 → divide by 16.
const DIVIDE_BY_16: u32 = 0x03;

/// Ticks-per-10-ms measured during `calibrate`.  Stored so `on_irq` can
/// accumulate elapsed nanoseconds without re-reading hardware.
static TICKS_PER_10MS: core::sync::atomic::AtomicU32 =
    core::sync::atomic::AtomicU32::new(0);

// ── Port I/O helpers ──────────────────────────────────────────────────────────

#[inline]
unsafe fn outb(port: u16, val: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") val,
        options(nomem, nostack, preserves_flags)
    );
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let v: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") v,
        options(nomem, nostack, preserves_flags)
    );
    v
}

// ── Calibration via PIT channel 2 ────────────────────────────────────────────

/// Use PIT channel 2 in one-shot mode to measure how many LAPIC bus-clock
/// ticks (with ÷16 divider) occur in ~10 ms.
///
/// Returns the measured count, or a safe fallback if something looks wrong.
unsafe fn calibrate() -> u32 {
    // Mask the LAPIC timer while calibrating.
    LOCAL_APIC.write_reg(Register::TimerDivide, DIVIDE_BY_16);
    LOCAL_APIC.write_reg(Register::TimerVector, 0x0001_0000 | TIMER_VECTOR as u32); // masked

    // ── PIT channel 2 setup ──────────────────────────────────────────────
    // Save gate state.
    let gate_orig = inb(PORT_SYSCTRL);
    // Disable gate (clear bit 0) to stop any prior count.
    outb(PORT_SYSCTRL, gate_orig & !0x01);

    // Program channel 2: mode 0 (interrupt on terminal count), RW both bytes.
    outb(PIT_COMMAND, 0xB0);
    outb(PIT_CHANNEL2, (PIT_TICKS_10MS & 0xFF) as u8);
    outb(PIT_CHANNEL2, (PIT_TICKS_10MS >> 8) as u8);

    // Arm the LAPIC counter at maximum value.
    LOCAL_APIC.write_reg(Register::TimerInit, 0xFFFF_FFFF);

    // Enable PIT gate (bit 0 → 1): starts the countdown.
    outb(PORT_SYSCTRL, (gate_orig & !0x01) | 0x01);

    // Wait for PIT channel 2 output to go high (bit 5 of port 0x61).
    loop {
        if inb(PORT_SYSCTRL) & 0x20 != 0 {
            break;
        }
    }

    // Read LAPIC remaining count.
    let remaining = LOCAL_APIC.read_reg(Register::TimerCurrent);
    let elapsed = 0xFFFF_FFFF_u32.wrapping_sub(remaining);

    // Restore PIT gate.
    outb(PORT_SYSCTRL, gate_orig);

    if elapsed == 0 || elapsed == 0xFFFF_FFFF {
        // Something went wrong – fall back to a value that gives ~10 ms at
        // an assumed 1 GHz bus / ÷16 = 62.5 MHz → 625 000 ticks / 10 ms.
        625_000
    } else {
        elapsed
    }
}

// ── Public interface ──────────────────────────────────────────────────────────

/// Initialise the LAPIC timer with a ~10 ms periodic interrupt.
pub fn init() {
    crate::log::info("  - LAPIC timer: calibrating");
    unsafe {
        let ticks = calibrate();
        TICKS_PER_10MS.store(ticks, core::sync::atomic::Ordering::Relaxed);

        // Set up periodic timer:
        //   bit 17 = periodic mode, bits 7:0 = vector.
        LOCAL_APIC.write_reg(Register::TimerDivide, DIVIDE_BY_16);
        LOCAL_APIC.write_reg(Register::TimerInit, ticks);
        LOCAL_APIC.write_reg(
            Register::TimerVector,
            (1 << 17) | TIMER_VECTOR as u32, // periodic, unmasked
        );
    }
    crate::log::info("  - LAPIC timer: running");
}

/// Stop the LAPIC timer (mask the LVT entry and zero the count).
pub fn stop() {
    unsafe {
        LOCAL_APIC.write_reg(Register::TimerVector, 0x0001_0000); // mask
        LOCAL_APIC.write_reg(Register::TimerInit, 0);
    }
}

/// Return the calibrated ticks-per-10-ms value.
pub fn ticks_per_10ms() -> u32 {
    TICKS_PER_10MS.load(core::sync::atomic::Ordering::Relaxed)
}

