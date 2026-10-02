use std::hint::black_box;
use std::time::Instant;

unsafe extern "C" {
    fn nsz_reduce(p: *const f32, n: usize) -> f32;
    fn det_reduce(p: *const f32, n: usize) -> f32;
    fn nsz_chain(x: f32, p: *const f32, n: usize) -> f32;
    fn det_chain(x: f32, p: *const f32, n: usize) -> f32;
    fn nsz_ew(o: *mut f32, on: usize, a: *const f32, an: usize, b: *const f32, bn: usize);
    fn det_ew(o: *mut f32, on: usize, a: *const f32, an: usize, b: *const f32, bn: usize);
    fn nsz_clamp(o: *mut f32, on: usize, a: *const f32, an: usize);
    fn det_clamp(o: *mut f32, on: usize, a: *const f32, an: usize);
    fn nsz_reduce64(p: *const f64, n: usize) -> f64;
    fn det_reduce64(p: *const f64, n: usize) -> f64;
    fn nsz_chain64(x: f64, p: *const f64, n: usize) -> f64;
    fn det_chain64(x: f64, p: *const f64, n: usize) -> f64;
    fn nsz_ew64(o: *mut f64, on: usize, a: *const f64, an: usize, b: *const f64, bn: usize);
    fn det_ew64(o: *mut f64, on: usize, a: *const f64, an: usize, b: *const f64, bn: usize);
}

const REP: usize = 21;

fn time<F: FnMut()>(mut f: F, iters: u32) -> (f64, f64) {
    for _ in 0..(iters / 2).max(1) {
        f();
    }
    let mut v: Vec<f64> = (0..REP)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..iters {
                f();
            }
            t.elapsed().as_secs_f64() / iters as f64
        })
        .collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (v[0], v[REP / 2])
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(16384);
    let zeros = args.iter().any(|a| a == "--zeros");
    let iters = ((1u64 << 26) / n as u64).max(3) as u32;

    let a: Vec<f32> = (0..n)
        .map(|i| match (zeros, i % 16) {
            (true, 0) => -0.0,
            (true, 8) => 0.0,
            _ => 1.0 + (i % 10007) as f32,
        })
        .collect();
    let b: Vec<f32> = (0..n)
        .map(|i| if zeros && i % 8 == 4 { 0.0 } else { 2.0 + ((i * 7) % 9973) as f32 })
        .collect();
    let a6: Vec<f64> = a.iter().map(|&x| x as f64).collect();
    let b6: Vec<f64> = b.iter().map(|&x| x as f64).collect();
    let mut o = vec![0f32; n];
    let mut o6 = vec![0f64; n];
    let (p, q, r) = (a.as_ptr(), b.as_ptr(), o.as_mut_ptr());
    let (p6, q6, r6) = (a6.as_ptr(), b6.as_ptr(), o6.as_mut_ptr());

    println!("n={n} rep={REP} zeros={zeros}");
    let mut out = |name: &str, x: (f64, f64), y: (f64, f64)| {
        println!("{name:<16}{:.3}\t{:.3}\t{:.3}", y.0 / x.0, y.1 / x.1, x.1 / x.0);
    };
    out("reduce",
        time(|| { black_box(unsafe { nsz_reduce(p, n) }); }, iters),
        time(|| { black_box(unsafe { det_reduce(p, n) }); }, iters));
    out("chain",
        time(|| { black_box(unsafe { nsz_chain(1e30, p, n) }); }, iters),
        time(|| { black_box(unsafe { det_chain(1e30, p, n) }); }, iters));
    out("elementwise",
        time(|| unsafe { nsz_ew(r, n, p, n, q, n) }, iters),
        time(|| unsafe { det_ew(r, n, p, n, q, n) }, iters));
    out("clamp",
        time(|| unsafe { nsz_clamp(r, n, p, n) }, iters),
        time(|| unsafe { det_clamp(r, n, p, n) }, iters));
    out("reduce f64",
        time(|| { black_box(unsafe { nsz_reduce64(p6, n) }); }, iters),
        time(|| { black_box(unsafe { det_reduce64(p6, n) }); }, iters));
    out("chain f64",
        time(|| { black_box(unsafe { nsz_chain64(1e30, p6, n) }); }, iters),
        time(|| { black_box(unsafe { det_chain64(1e30, p6, n) }); }, iters));
    out("elementwise f64",
        time(|| unsafe { nsz_ew64(r6, n, p6, n, q6, n) }, iters),
        time(|| unsafe { det_ew64(r6, n, p6, n, q6, n) }, iters));
}
