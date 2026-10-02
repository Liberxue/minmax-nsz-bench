// One kernel, one variant, in a loop, so perf stat attributes counters to it.
// argv: <nsz|det> <reduce|chain|ew|clamp> <n> <iters>
use std::hint::black_box;

unsafe extern "C" {
    fn nsz_reduce(p: *const f32, n: usize) -> f32;
    fn det_reduce(p: *const f32, n: usize) -> f32;
    fn nsz_chain(x: f32, p: *const f32, n: usize) -> f32;
    fn det_chain(x: f32, p: *const f32, n: usize) -> f32;
    fn nsz_ew(o: *mut f32, on: usize, a: *const f32, an: usize, b: *const f32, bn: usize);
    fn det_ew(o: *mut f32, on: usize, a: *const f32, an: usize, b: *const f32, bn: usize);
    fn nsz_clamp(o: *mut f32, on: usize, a: *const f32, an: usize);
    fn det_clamp(o: *mut f32, on: usize, a: *const f32, an: usize);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let var = a[1].as_str();
    let kern = a[2].as_str();
    let n: usize = a[3].parse().unwrap();
    let iters: usize = a[4].parse().unwrap();
    let zeros = a.iter().any(|s| s == "--zeros");

    let xs: Vec<f32> = (0..n)
        .map(|i| match (zeros, i % 16) {
            (true, 0) => -0.0,
            (true, 8) => 0.0,
            _ => 1.0 + (i % 10007) as f32,
        })
        .collect();
    let ys: Vec<f32> = (0..n)
        .map(|i| if zeros && i % 8 == 4 { 0.0 } else { 2.0 + ((i * 7) % 9973) as f32 })
        .collect();
    let mut o = vec![0f32; n];
    let (p, q, r) = (xs.as_ptr(), ys.as_ptr(), o.as_mut_ptr());

    // Resolved once: a match on argv inside the loop would be counted too.
    let mut run: Box<dyn FnMut()> = match (var, kern) {
        ("nsz", "reduce") => Box::new(move || { black_box(unsafe { nsz_reduce(p, n) }); }),
        ("det", "reduce") => Box::new(move || { black_box(unsafe { det_reduce(p, n) }); }),
        ("nsz", "chain") => Box::new(move || { black_box(unsafe { nsz_chain(1e30, p, n) }); }),
        ("det", "chain") => Box::new(move || { black_box(unsafe { det_chain(1e30, p, n) }); }),
        ("nsz", "ew") => Box::new(move || unsafe { nsz_ew(r, n, p, n, q, n) }),
        ("det", "ew") => Box::new(move || unsafe { det_ew(r, n, p, n, q, n) }),
        ("nsz", "clamp") => Box::new(move || unsafe { nsz_clamp(r, n, p, n) }),
        ("det", "clamp") => Box::new(move || unsafe { det_clamp(r, n, p, n) }),
        _ => panic!("unknown {}/{}", var, kern),
    };
    for _ in 0..iters {
        run();
    }

    black_box(&o);
}
