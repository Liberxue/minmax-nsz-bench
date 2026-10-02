use std::hint::black_box;
use std::time::Instant;
#[cfg(target_arch = "x86_64")] use std::arch::x86_64::*;

// Today: the autovectorised f32::min reduction (llvm.minimumnum + nsz).
#[inline(never)]
fn red_now(xs: &[f32]) -> f32 { xs.iter().copied().fold(f32::INFINITY, f32::min) }

// The "fast" contract in its real form: one vminps per 8 lanes.
#[inline(never)]
#[target_feature(enable = "avx")]
unsafe fn red_fast(xs: &[f32]) -> f32 {
    unsafe {
        let mut acc = _mm256_set1_ps(f32::INFINITY);
        let chunks = xs.len() / 8;
        for i in 0..chunks {
            let v = _mm256_loadu_ps(xs.as_ptr().add(i * 8));
            acc = _mm256_min_ps(acc, v);
        }
        let mut buf = [0f32; 8];
        _mm256_storeu_ps(buf.as_mut_ptr(), acc);
        let mut m = buf.iter().copied().fold(f32::INFINITY, |a, b| if a < b { a } else { b });
        for &x in &xs[chunks * 8..] { if x < m { m = x; } }
        m
    }
}
const REP: usize = 15;
fn stats<F: FnMut()>(mut f: F, it: u32) -> f64 {
    for _ in 0..(it/2).max(1) { f(); }
    let mut v: Vec<f64> = (0..REP).map(|_| { let t=Instant::now(); for _ in 0..it { f(); } t.elapsed().as_secs_f64()/it as f64 }).collect();
    v.sort_by(|a,b| a.partial_cmp(b).unwrap()); v[0]
}
fn main(){
    println!("{:<8} {:>9} {:>11} {:>11} {:>9}", "kernel", "n", "now(us)", "fast(us)", "now/fast");
    for &lg in &[10u32, 14, 18, 22] {
        let n = 1usize<<lg;
        let a: Vec<f32> = (0..n).map(|i| 1.0+(i%10007) as f32).collect();
        let it = ((1u64<<26)/n as u64).max(3) as u32;
        let s = &a[..];
        let x = stats(||{black_box(red_now(s));}, it);
        let y = stats(||{black_box(unsafe{red_fast(s)});}, it);
        // same result either way for this data
        assert_eq!(red_now(s).to_bits(), unsafe{red_fast(s)}.to_bits());
        println!("{:<8} {:>9} {:>11.3} {:>11.3} {:>9.3}", "reduce", n, x*1e6, y*1e6, x/y);
    }
}
