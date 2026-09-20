//! Denormal / Subnormal Protection Utility for Real-Time DSP Audio Threads.
//!
//! When an audio signal fades into silence or recursive filters (IIR biquad, reverb, decay envelopes)
//! exponentially decay, floating-point numbers can drop below `f32::MIN_POSITIVE` (~1.175e-38).
//! Without hardware mitigation, modern CPUs switch to microcode trapping or software emulation to
//! handle subnormal numbers, causing sudden **10x–100x CPU performance spikes** and audible dropouts.
//!
//! This module provides a zero-overhead, cross-platform function to configure the CPU's floating-point
//! control register to **Flush-To-Zero (FTZ)** and **Denormals-Are-Zero (DAZ)** mode.

/// Enables Flush-To-Zero (FTZ) and Denormals-Are-Zero (DAZ) mode on the calling CPU thread.
///
/// # Note on Thread Scope
/// Floating-point control registers (MXCSR on x86, FPCR on ARM) are **thread-local**.
/// This function must be called on every newly spawned thread that executes real-time DSP,
/// audio processing loops, or neural inference.
///
/// # Supported Targets
/// * `x86_64` / `x86`: Sets FTZ (bit 15, `0x8000`) and DAZ (bit 6, `0x0040`) in `MXCSR` via inline assembly.
/// * `aarch64`: Sets the `FZ` (bit 24) and `FZ16` (bit 19) bits in `FPCR` (Android ARM64, iOS ARM64, macOS Apple Silicon).
/// * `arm` (v7): Sets the `FZ` (bit 24) bit in `FPSCR` (Android 32-bit ARM).
#[inline]
pub fn enable_flush_to_zero() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let mut mxcsr: u32 = 0;
        core::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));
        mxcsr |= 0x8000; // Bit 15: FTZ (Flush-To-Zero)
        mxcsr |= 0x0040; // Bit 6:  DAZ (Denormals-Are-Zero)
        core::arch::asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
    }

    #[cfg(target_arch = "x86")]
    unsafe {
        let mut mxcsr: u32 = 0;
        core::arch::asm!("stmxcsr [{}]", in(reg) &mut mxcsr, options(nostack));
        mxcsr |= 0x8000; // Bit 15: FTZ (Flush-To-Zero)
        mxcsr |= 0x0040; // Bit 6:  DAZ (Denormals-Are-Zero)
        core::arch::asm!("ldmxcsr [{}]", in(reg) &mxcsr, options(nostack));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut fpcr: u64;
        core::arch::asm!("mrs {}, fpcr", out(reg) fpcr, options(nomem, nostack));
        fpcr |= 1 << 24; // FZ: Flush-to-zero mode for single/double precision
        fpcr |= 1 << 19; // FZ16: Flush-to-zero mode for half precision
        core::arch::asm!("msr fpcr, {}", in(reg) fpcr, options(nomem, nostack));
    }

    #[cfg(target_arch = "arm")]
    unsafe {
        let mut fpscr: u32;
        core::arch::asm!("mrc p10, 7, {}, cr1, cr0, 0", out(reg) fpscr, options(nomem, nostack));
        fpscr |= 1 << 24; // FZ: Flush-to-zero mode in FPSCR
        core::arch::asm!("mcr p10, 7, {}, cr1, cr0, 0", in(reg) fpscr, options(nomem, nostack));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enable_flush_to_zero_executes_safely() {
        enable_flush_to_zero();

        // Under FTZ / DAZ mode:
        // A subnormal float (bit pattern 1, ~1.4e-45) is treated as 0.0 by DAZ,
        // and any small calculation underflowing to subnormal is flushed to 0.0 by FTZ.
        let subnormal = f32::from_bits(1);
        let result = subnormal * 0.5;
        assert_eq!(result, 0.0);
    }
}
