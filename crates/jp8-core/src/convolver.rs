/// FFT-based convolution using rustfft.
/// Used at init time to pre-convolve excitation × body IR.
/// Not called in the audio render hot path.

extern crate alloc;
use alloc::vec;
use alloc::vec::Vec;
use rustfft::{FftPlanner, num_complex::Complex};

/// Convolve two signals using FFT. Returns result of length a.len() + b.len() - 1.
/// Truncates to max_len if specified.
pub fn fft_convolve(a: &[f32], b: &[f32], max_len: usize) -> Vec<f32> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }

    let result_len = a.len() + b.len() - 1;
    let fft_len = result_len.next_power_of_two();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);

    // Zero-pad inputs into complex buffers
    let mut buf_a: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); fft_len];
    let mut buf_b: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); fft_len];

    for (i, &v) in a.iter().enumerate() {
        buf_a[i] = Complex::new(v, 0.0);
    }
    for (i, &v) in b.iter().enumerate() {
        buf_b[i] = Complex::new(v, 0.0);
    }

    // Forward FFT both
    fft.process(&mut buf_a);
    fft.process(&mut buf_b);

    // Element-wise complex multiply
    for i in 0..fft_len {
        buf_a[i] = buf_a[i] * buf_b[i];
    }

    // Inverse FFT
    ifft.process(&mut buf_a);

    // Normalize and extract real part
    let scale = 1.0 / fft_len as f32;
    let out_len = result_len.min(max_len);
    let mut result = Vec::with_capacity(out_len);
    for i in 0..out_len {
        result.push(buf_a[i].re * scale);
    }

    result
}

/// Convolve, then truncate, fade out, and normalize.
/// Produces a ready-to-use wavetable for the waveguide.
pub fn convolve_and_prepare(
    excitation: &[f32],
    body: &[f32],
    max_len: usize,
    fade_len: usize,
    normalize_peak: f32,
) -> Vec<f32> {
    let mut result = fft_convolve(excitation, body, max_len);

    if result.is_empty() {
        return result;
    }

    // Cosine fade-out at truncation boundary
    let len = result.len();
    let fade = fade_len.min(len);
    for i in 0..fade {
        let t = i as f32 / fade as f32;
        result[len - fade + i] *= 0.5 * (1.0 + (core::f32::consts::PI * t).cos());
    }

    // Normalize peak
    let mut peak = 0.0f32;
    for &s in &result {
        peak = peak.max(s.abs());
    }
    if peak > 0.001 {
        let scale = normalize_peak / peak;
        for s in &mut result {
            *s *= scale;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_identity() {
        // Convolving with [1, 0, 0...] should return the original signal
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let impulse = vec![1.0];
        let result = fft_convolve(&signal, &impulse, 100);
        assert_eq!(result.len(), 5);
        for (i, &v) in result.iter().enumerate() {
            assert!((v - signal[i]).abs() < 0.001, "sample {i}: expected {}, got {v}", signal[i]);
        }
    }

    #[test]
    fn known_convolution() {
        // [1, 2, 3] * [1, 1] = [1, 3, 5, 3]
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 1.0];
        let result = fft_convolve(&a, &b, 100);
        assert_eq!(result.len(), 4);
        let expected = [1.0, 3.0, 5.0, 3.0];
        for (i, &v) in result.iter().enumerate() {
            assert!((v - expected[i]).abs() < 0.01, "sample {i}: expected {}, got {v}", expected[i]);
        }
    }

    #[test]
    fn empty_input() {
        let result = fft_convolve(&[], &[1.0, 2.0], 100);
        assert!(result.is_empty());
    }

    #[test]
    fn max_len_truncation() {
        let a = vec![1.0; 1000];
        let b = vec![1.0; 1000];
        let result = fft_convolve(&a, &b, 500);
        assert_eq!(result.len(), 500);
    }

    #[test]
    fn convolve_and_prepare_normalizes() {
        let a = vec![0.1; 100];
        let b = vec![0.1; 100];
        let result = convolve_and_prepare(&a, &b, 200, 32, 0.5);
        let peak = result.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!((peak - 0.5).abs() < 0.01, "peak should be ~0.5, got {peak}");
    }

    #[test]
    fn no_nan() {
        let a: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
        let b: Vec<f32> = (0..2048).map(|i| (i as f32 * 0.02).cos()).collect();
        let result = fft_convolve(&a, &b, 16384);
        for (i, &v) in result.iter().enumerate() {
            assert!(!v.is_nan(), "NaN at index {i}");
            assert!(!v.is_infinite(), "Inf at index {i}");
        }
    }
}
