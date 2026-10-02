fn min_fixed(a: f32, b: f32) -> f32 {
    let m = a.min(b);
    if a == b { f32::from_bits(a.to_bits() | b.to_bits()) } else { m }
}
fn min_ref(a: f32, b: f32) -> f32 {
    if a.is_nan() { return b; }
    if b.is_nan() { return a; }
    if a < b { a } else if b < a { b } else if a.is_sign_negative() { a } else { b }
}
fn same(x: f32, y: f32) -> bool { x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()) }
fn main() {
    let specials = [0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -f32::NAN, f32::MAX, f32::MIN,
        1e-45, -1e-45, 5.0, -5.0, f32::from_bits(0x7F800001), f32::from_bits(0xFF800001)];
    let mut bad = 0u64; let mut n = 0u64;
    for &a in &specials { for &b in &specials {
        n += 1; if !same(min_fixed(a, b), min_ref(a, b)) { bad += 1;
            if bad <= 5 { println!("special-value mismatch: a={:e}({:#x}) b={:e}({:#x})", a, a.to_bits(), b, b.to_bits()); } } } }
    println!("special values, all pairs: {} cases, {} mismatches", n, bad);
    // random, then all high-16-bit patterns against a set of probes
    let mut s: u64 = 0x853c49e6748fea9b;
    let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 32) as u32 };
    let mut bad2 = 0u64; let total = 20_000_000u64;
    for _ in 0..total {
        let (a, b) = (f32::from_bits(rnd()), f32::from_bits(rnd()));
        if !same(min_fixed(a, b), min_ref(a, b)) { bad2 += 1; if bad2 <= 5 { println!("random mismatch: {:#x} {:#x}", a.to_bits(), b.to_bits()); } }
    }
    println!("random bit patterns: {} cases, {} mismatches", total, bad2);
    // sweeps every exponent and sign combination
    let mut bad3 = 0u64; let mut n3 = 0u64;
    for ah in 0..=0xFFFFu32 { let a = f32::from_bits(ah << 16);
        for bh in [0x0000u32, 0x8000, 0x3F80, 0xBF80, 0x7F80, 0xFF80, 0x7FC0, 0x0001] {
            let b = f32::from_bits(bh << 16); n3 += 1;
            if !same(min_fixed(a, b), min_ref(a, b)) { bad3 += 1; } } }
    println!("semi-exhaustive (all high 16 bits x 8 probes): {} cases, {} mismatches", n3, bad3);
}
