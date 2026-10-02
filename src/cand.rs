// Candidate AVX2 lowerings for minimumNumber with deterministic signed zeros,
// against the fast contract (nsz) as the baseline. LLVM's own sequence is not
// here; it comes out of bench.sh as build/det.o.
//
//   ./cand verify
//   ./cand time [n]
//   ./cand <nsz|twoblend|bitops|orminmin|orminmin2|signor|signor_z|intkey|keyacc> <n> <iters>
//
// orminmin, signor and signor_z are kept because they fail: each breaks on a
// negatively signed NaN, which verify reports.

// Built with -C target-cpu=x86-64-v3, so the whole crate carries avx2 and the
// helpers below can still be inlined into the loops. Without that flag the
// attributes keep the intrinsics legal but the inlining, and with it the
// measurement, is no longer guaranteed.
#[cfg(not(target_arch = "x86_64"))]
fn main() {
    eprintln!("x86_64 only: these are avx2 sequences");
    std::process::exit(1);
}

#[cfg(target_arch = "x86_64")]
mod avx2 {
    use std::arch::x86_64::*;
    use std::hint::black_box;

    // Reference, the formula already checked in verify_fixup.rs.
    fn min_det(a: f32, b: f32) -> f32 {
        let m = a.min(b);
        if a == b { f32::from_bits(a.to_bits() | b.to_bits()) } else { m }
    }

    // --- the fast contract: one blend for NaN, nothing for zeros ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_nsz(a: __m256, b: __m256) -> __m256 {
        let m = _mm256_min_ps(a, b);                       // b on NaN or tie
        let ord_b = _mm256_cmp_ps(b, b, _CMP_ORD_Q);
        _mm256_blendv_ps(a, m, ord_b)                      // 1 blend
    }

    // --- fix the tie directly: or the bits when a == b ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_twoblend(a: __m256, b: __m256) -> __m256 {
        unsafe {
            let m = v_nsz(a, b);
            let eq = _mm256_cmp_ps(a, b, _CMP_EQ_OQ);          // false for NaN
            let orv = _mm256_or_ps(a, b);
            _mm256_blendv_ps(m, orv, eq)                       // 2 blends
        }
    }

    // --- same, but the select is and/andnot/or, which are not pinned to port 5 ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_bitops(a: __m256, b: __m256) -> __m256 {
        unsafe {
            let m = v_nsz(a, b);
            let eq = _mm256_cmp_ps(a, b, _CMP_EQ_OQ);
            let orv = _mm256_or_ps(a, b);
            _mm256_or_ps(_mm256_andnot_ps(eq, m), _mm256_and_ps(eq, orv))   // 1 blend
        }
    }


    // --- or of the two operand orders: minps returns src2 on a tie, so one of the
    //     two picks the negative zero and the or keeps it ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_orminmin(a: __m256, b: __m256) -> __m256 {
        let m = _mm256_or_ps(_mm256_min_ps(a, b), _mm256_min_ps(b, a));
        let ord_b = _mm256_cmp_ps(b, b, _CMP_ORD_Q);
        _mm256_blendv_ps(a, m, ord_b)
    }

    // --- same, but mask on both operands being ordered ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_orminmin2(a: __m256, b: __m256) -> __m256 {
        unsafe {
            let m = _mm256_or_ps(_mm256_min_ps(a, b), _mm256_min_ps(b, a));
            let nsz = v_nsz(a, b);
            let ord = _mm256_and_ps(_mm256_cmp_ps(a, a, _CMP_ORD_Q), _mm256_cmp_ps(b, b, _CMP_ORD_Q));
            _mm256_blendv_ps(nsz, m, ord)
        }
    }

    // --- unconditional sign or; expected to break on a negatively signed NaN ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_signor(a: __m256, b: __m256) -> __m256 {
        let sign = _mm256_castsi256_ps(_mm256_set1_epi32(i32::MIN));
        let s = _mm256_and_ps(_mm256_or_ps(a, b), sign);
        let m = _mm256_or_ps(_mm256_min_ps(a, b), s);
        let ord_b = _mm256_cmp_ps(b, b, _CMP_ORD_Q);
        _mm256_blendv_ps(a, m, ord_b)
    }

    // --- gate the sign or on the result being zero, which also stops NaN leaking in ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_signor_z(a: __m256, b: __m256) -> __m256 {
        unsafe {
            let sign = _mm256_castsi256_ps(_mm256_set1_epi32(i32::MIN));
            let m = v_nsz(a, b);
            let z = _mm256_cmp_ps(m, _mm256_setzero_ps(), _CMP_EQ_OQ);
            let s = _mm256_and_ps(_mm256_or_ps(a, b), _mm256_and_ps(sign, z));
            _mm256_or_ps(m, s)
        }
    }

    // --- integer key: flip so signed int order matches float order, then vpminsd.
    //     -0.0 maps below +0.0 for free; NaN has to be lifted to INT_MAX by hand. ---
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn key(x: __m256) -> __m256i {
        let b = _mm256_castps_si256(x);
        _mm256_xor_si256(b, _mm256_srli_epi32(_mm256_srai_epi32(b, 31), 1))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn unkey(k: __m256i) -> __m256 {
        _mm256_castsi256_ps(_mm256_xor_si256(k, _mm256_srli_epi32(_mm256_srai_epi32(k, 31), 1)))
    }
    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn v_intkey(a: __m256, b: __m256) -> __m256 {
        unsafe {
            let max = _mm256_set1_epi32(i32::MAX);
            let ka = _mm256_castps_si256(_mm256_blendv_ps(
                _mm256_castsi256_ps(key(a)), _mm256_castsi256_ps(max), _mm256_cmp_ps(a, a, _CMP_UNORD_Q)));
            let kb = _mm256_castps_si256(_mm256_blendv_ps(
                _mm256_castsi256_ps(key(b)), _mm256_castsi256_ps(max), _mm256_cmp_ps(b, b, _CMP_UNORD_Q)));
            unkey(_mm256_min_epi32(ka, kb))
        }
    }

    macro_rules! reduce {
        ($name:ident, $op:ident) => {
            #[inline(never)]
            #[target_feature(enable = "avx2")]
            unsafe fn $name(xs: &[f32]) -> f32 {
                unsafe {
                    let mut a0 = _mm256_set1_ps(f32::INFINITY);
                    let mut a1 = a0; let mut a2 = a0; let mut a3 = a0;
                    let p = xs.as_ptr();
                    let chunks = xs.len() / 32;
                    for i in 0..chunks {
                        let q = p.add(i * 32);
                        a0 = $op(a0, _mm256_loadu_ps(q));
                        a1 = $op(a1, _mm256_loadu_ps(q.add(8)));
                        a2 = $op(a2, _mm256_loadu_ps(q.add(16)));
                        a3 = $op(a3, _mm256_loadu_ps(q.add(24)));
                    }
                    let acc = $op($op(a0, a1), $op(a2, a3));
                    let mut buf = [0f32; 8];
                    _mm256_storeu_ps(buf.as_mut_ptr(), acc);
                    let mut m = f32::INFINITY;
                    for &x in &buf { m = min_det(m, x); }
                    for &x in &xs[chunks * 32..] { m = min_det(m, x); }
                    m
                }
            }
        };
    }
    reduce!(r_nsz, v_nsz);
    reduce!(r_twoblend, v_twoblend);
    reduce!(r_bitops, v_bitops);
    reduce!(r_orminmin, v_orminmin);
    reduce!(r_orminmin2, v_orminmin2);
    reduce!(r_signor, v_signor);
    reduce!(r_signor_z, v_signor_z);
    reduce!(r_intkey, v_intkey);

    #[target_feature(enable = "avx2")]
    unsafe fn check(name: &str, f: unsafe fn(__m256, __m256) -> __m256) {
        unsafe {
            let specials = [0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, -f32::MIN_POSITIVE,
                f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -f32::NAN, f32::MAX, f32::MIN,
                1e-45, -1e-45, 5.0, -5.0, f32::from_bits(0x7F800001), f32::from_bits(0xFF800001)];
            let same = |x: f32, y: f32| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan());
            let mut bad = 0u64; let mut n = 0u64; let mut first = String::new();
            let mut lane = |av: &[f32; 8], bv: &[f32; 8]| {
                let mut out = [0f32; 8];
                _mm256_storeu_ps(out.as_mut_ptr(), f(_mm256_loadu_ps(av.as_ptr()), _mm256_loadu_ps(bv.as_ptr())));
                for i in 0..8 {
                    n += 1;
                    if !same(out[i], min_det(av[i], bv[i])) {
                        bad += 1;
                        if first.is_empty() {
                            first = format!("a={:#010x} b={:#010x} got={:#010x} want={:#010x}",
                                av[i].to_bits(), bv[i].to_bits(), out[i].to_bits(), min_det(av[i], bv[i]).to_bits());
                        }
                    }
                }
            };
            // all special pairs, broadcast into lanes
            for &a in &specials { for &b in &specials { lane(&[a; 8], &[b; 8]); } }
            // specials mixed across lanes
            for c in 0..specials.len() {
                let av: [f32; 8] = core::array::from_fn(|i| specials[(c + i) % specials.len()]);
                let bv: [f32; 8] = core::array::from_fn(|i| specials[(c + 2 * i + 1) % specials.len()]);
                lane(&av, &bv);
            }
            // random bit patterns
            let mut s: u64 = 0x853c49e6748fea9b;
            let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 32) as u32 };
            for _ in 0..200_000 {
                let av: [f32; 8] = core::array::from_fn(|_| f32::from_bits(rnd()));
                let bv: [f32; 8] = core::array::from_fn(|_| f32::from_bits(rnd()));
                lane(&av, &bv);
            }
            // zero-heavy: every sign/zero combination against probes
            let zs = [0.0f32, -0.0, 1.0, -1.0, f32::NAN];
            for &a in &zs { for &b in &zs {
                let av: [f32; 8] = core::array::from_fn(|i| if i % 2 == 0 { a } else { b });
                let bv: [f32; 8] = core::array::from_fn(|i| if i % 3 == 0 { b } else { a });
                lane(&av, &bv);
            } }
            println!("{name:<10} {n} lane comparisons, {bad} mismatches{}",
                if bad > 0 { format!("  first: {first}") } else { String::new() });
        }
    }


    #[inline]
    #[target_feature(enable = "avx2")]
    unsafe fn keystep(acc: __m256i, x: __m256, max: __m256i) -> __m256i {
        unsafe {
            let nan = _mm256_cmp_ps(x, x, _CMP_UNORD_Q);
            let k = _mm256_castps_si256(_mm256_blendv_ps(
                _mm256_castsi256_ps(key(x)), _mm256_castsi256_ps(max), nan));
            _mm256_min_epi32(acc, k)
        }
    }

    // Reduction-specific: keep the accumulator in key space, so each loaded vector
    // costs only the key transform (two shifts on p0, one xor) plus vpminsd.
    #[inline(never)]
    #[target_feature(enable = "avx2")]
    unsafe fn r_keyacc(xs: &[f32]) -> f32 {
        unsafe {
            let max = _mm256_set1_epi32(i32::MAX);
            let mut a0 = max; let mut a1 = max; let mut a2 = max; let mut a3 = max;
            let p = xs.as_ptr();
            let chunks = xs.len() / 32;
            // Four accumulators written out. Looping over `[(&mut a0, 0), ...]`
            // left the blend count varying between builds, 2.32 to 2.56 port-5
            // uops per vector, which makes the measurement unreproducible.
            for i in 0..chunks {
                let q = p.add(i * 32);
                a0 = keystep(a0, _mm256_loadu_ps(q), max);
                a1 = keystep(a1, _mm256_loadu_ps(q.add(8)), max);
                a2 = keystep(a2, _mm256_loadu_ps(q.add(16)), max);
                a3 = keystep(a3, _mm256_loadu_ps(q.add(24)), max);
            }
            let acc = _mm256_min_epi32(_mm256_min_epi32(a0, a1), _mm256_min_epi32(a2, a3));
            let mut buf = [0f32; 8];
            _mm256_storeu_ps(buf.as_mut_ptr(), unkey(acc));
            let mut m = f32::INFINITY;
            for &x in &buf { m = min_det(m, x); }
            for &x in &xs[chunks * 32..] { m = min_det(m, x); }
            m
        }
    }

    // Array-level check: the whole reduction against a scalar fold of min_det.
    #[target_feature(enable = "avx2")]
    unsafe fn check_red(name: &str, f: unsafe fn(&[f32]) -> f32) {
        unsafe {
            let probes = [0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NEG_INFINITY,
                f32::NAN, -f32::NAN, f32::MAX, f32::MIN, 1e-45, -1e-45,
                f32::from_bits(0x7F800001), f32::from_bits(0xFF800001)];
            let mut s: u64 = 0xdeadbeefcafef00d;
            let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (s >> 32) as u32 };
            let mut bad = 0u64; let mut n = 0u64; let mut first = String::new();
            for len in [1usize, 7, 8, 31, 32, 33, 64, 96, 97, 160, 1024] {
                for case in 0..400 {
                    let xs: Vec<f32> = (0..len).map(|i| {
                        if case % 4 == 0 { probes[(i + case) % probes.len()] }
                        else if case % 4 == 1 { if i % 5 == 0 { probes[(i + case) % probes.len()] } else { f32::from_bits(rnd()) } }
                        else if case % 4 == 2 { if i % 3 == 0 { -0.0 } else { 0.0 } }
                        else { f32::from_bits(rnd()) }
                    }).collect();
                    let want = xs.iter().copied().fold(f32::INFINITY, min_det);
                    let got = f(&xs);
                    n += 1;
                    if !(got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan())) {
                        bad += 1;
                        if first.is_empty() { first = format!("len={len} case={case} got={:#010x} want={:#010x}", got.to_bits(), want.to_bits()); }
                    }
                }
            }
            println!("{name:<11} {n} reductions, {bad} mismatches{}", if bad > 0 { format!("  first: {first}") } else { String::new() });
        }
    }

    const REP: usize = 21;

    fn repeat_min<F: FnMut()>(mut f: F, iters: u32) -> f64 {
        for _ in 0..(iters / 2).max(1) { f(); }
        let mut v: Vec<f64> = (0..REP).map(|_| {
            let t = std::time::Instant::now();
            for _ in 0..iters { f(); }
            t.elapsed().as_secs_f64() / iters as f64
        }).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[0]
    }

    // Wall-clock on a machine with no usable PMU. Same convention as bench.rs:
    // minimum of 21 repeats, and the baseline's own median/min as a noise floor.
    fn timed(n: usize) {
        let xs: Vec<f32> = (0..n).map(|i| 1.0 + (i % 10007) as f32).collect();
        let iters = ((1u64 << 26) / n as u64).max(3) as u32;
        let names = ["nsz", "keyacc", "twoblend", "bitops", "orminmin2", "intkey"];
        let base = repeat_min(|| { black_box(unsafe { r_nsz(&xs) }); }, iters);
        println!("n={n} rep={REP} iters={iters}");
        for name in names {
            let t = match name {
                "nsz" => base,
                "keyacc" => repeat_min(|| { black_box(unsafe { r_keyacc(&xs) }); }, iters),
                "twoblend" => repeat_min(|| { black_box(unsafe { r_twoblend(&xs) }); }, iters),
                "bitops" => repeat_min(|| { black_box(unsafe { r_bitops(&xs) }); }, iters),
                "orminmin2" => repeat_min(|| { black_box(unsafe { r_orminmin2(&xs) }); }, iters),
                "intkey" => repeat_min(|| { black_box(unsafe { r_intkey(&xs) }); }, iters),
                _ => unreachable!(),
            };
            println!("{name:<11}{:.3}", t / base);
        }
    }

    pub fn run() {
        if !is_x86_feature_detected!("avx2") {
            eprintln!("this cpu has no avx2; every sequence here is an avx2 one");
            std::process::exit(1);
        }
        let args: Vec<String> = std::env::args().collect();
        if args[1] == "time" {
            timed(args.get(2).and_then(|a| a.parse().ok()).unwrap_or(16384));
            return;
        }
        if args[1] == "verify" {
            unsafe {
                check("nsz", v_nsz);
                check("twoblend", v_twoblend);
                check("bitops", v_bitops);
                check("orminmin", v_orminmin);
                check("orminmin2", v_orminmin2);
                check("signor", v_signor);
                check("signor_z", v_signor_z);
                check("intkey", v_intkey);
                println!("--- whole-reduction checks ---");
                check_red("twoblend", r_twoblend);
                check_red("intkey", r_intkey);
                check_red("keyacc", r_keyacc);
            }
            println!("note: nsz is expected to mismatch on signed-zero ties, that is the point");
            return;
        }
        let n: usize = args[2].parse().unwrap();
        let iters: usize = args[3].parse().unwrap();
        let xs: Vec<f32> = (0..n).map(|i| 1.0 + (i % 10007) as f32).collect();
        unsafe {
            for _ in 0..iters {
                black_box(match args[1].as_str() {
                    "nsz" => r_nsz(&xs),
                    "twoblend" => r_twoblend(&xs),
                    "bitops" => r_bitops(&xs),
                    "orminmin" => r_orminmin(&xs),
                    "orminmin2" => r_orminmin2(&xs),
                    "signor" => r_signor(&xs),
                    "signor_z" => r_signor_z(&xs),
                    "intkey" => r_intkey(&xs),
                    "keyacc" => r_keyacc(&xs),
                    o => panic!("unknown candidate {}", o),
                });
            }
        }
    }

}

#[cfg(target_arch = "x86_64")]
fn main() {
    avx2::run();
}
