#![crate_type = "lib"]

#[unsafe(no_mangle)]
pub fn k_reduce(xs: &[f32]) -> f32 {
    xs.iter().copied().fold(f32::INFINITY, f32::min)
}

#[unsafe(no_mangle)]
pub fn k_chain(mut x: f32, ys: &[f32]) -> f32 {
    for &y in ys {
        x = x.min(y);
    }
    x
}

#[unsafe(no_mangle)]
pub fn k_ew(o: &mut [f32], a: &[f32], b: &[f32]) {
    for i in 0..o.len().min(a.len()).min(b.len()) {
        o[i] = a[i].min(b[i]);
    }
}

#[unsafe(no_mangle)]
pub fn k_clamp(o: &mut [f32], a: &[f32]) {
    for i in 0..o.len().min(a.len()) {
        o[i] = a[i].clamp(0.0, 6.0);
    }
}

#[unsafe(no_mangle)]
pub fn k_reduce64(xs: &[f64]) -> f64 {
    xs.iter().copied().fold(f64::INFINITY, f64::min)
}

#[unsafe(no_mangle)]
pub fn k_chain64(mut x: f64, ys: &[f64]) -> f64 {
    for &y in ys {
        x = x.min(y);
    }
    x
}

#[unsafe(no_mangle)]
pub fn k_ew64(o: &mut [f64], a: &[f64], b: &[f64]) {
    for i in 0..o.len().min(a.len()).min(b.len()) {
        o[i] = a[i].min(b[i]);
    }
}
