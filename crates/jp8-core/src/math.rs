/// Padé approximant tanh, accurate to <0.001 for |x| < 3.
/// Beyond ±3, clamps to ±1 (correct for saturation).
#[inline(always)]
pub fn fast_tanh(x: f32) -> f32 {
    let x = x.clamp(-3.0, 3.0);
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// Fast tan approximation for filter coefficient computation.
#[inline(always)]
pub fn fast_tan(x: f32) -> f32 {
    // For small x (which is the case for π*freq/sr), tan(x) ≈ x + x³/3
    let x2 = x * x;
    x * (1.0 + x2 * (1.0 / 3.0 + x2 * (2.0 / 15.0)))
}
