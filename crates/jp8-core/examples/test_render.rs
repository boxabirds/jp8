use jp8_core::engine::Engine;
use jp8_core::params::PARAM_COUNT;

fn main() {
    let mut engine = Engine::new(44100.0);

    let mut raw = [0.0f32; PARAM_COUNT];
    raw[0] = 1.0;    // VCO1 saw
    raw[2] = 0.5;    // PW
    raw[3] = 0.8;    // VCO1 level
    raw[4] = 0.0;    // VCO2 off
    raw[7] = 0.0;    // VCO2 level 0
    raw[12] = 8000.0; // cutoff
    raw[14] = 0.0;   // no filter env mod
    raw[16] = 20.0;  // HPF
    raw[17] = 0.001; raw[18] = 10.0; raw[19] = 1.0; raw[20] = 10.0;
    raw[22] = 0.001; raw[23] = 10.0; raw[24] = 1.0; raw[25] = 10.0;
    raw[32] = 0.0;   // chorus off
    raw[33] = 0.7;   // volume
    engine.apply_params(&raw);

    engine.note_on(69, 100); // A4

    let mut buf = [0.0f32; 256];
    engine.render(&mut buf);

    println!("Block 1 — first 20 samples (L, R):");
    for i in 0..20 {
        println!("  [{:3}] L={:+.6} R={:+.6}", i, buf[i * 2], buf[i * 2 + 1]);
    }

    let peak = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let any_bad = buf.iter().any(|s| s.is_nan() || s.is_infinite());
    let nonzero = buf.iter().filter(|s| **s != 0.0).count();
    println!("\nPeak={:.6} BadValues={} NonZero={}/256", peak, any_bad, nonzero);

    engine.render(&mut buf);
    println!("\nBlock 2 — first 20 samples:");
    for i in 0..20 {
        println!("  [{:3}] L={:+.6} R={:+.6}", i, buf[i * 2], buf[i * 2 + 1]);
    }

    let peak2 = buf.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    println!("Peak={:.6}", peak2);
}
