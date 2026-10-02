## Deterministic signed zeros: cost

Both variants built from the same rustc 1.96 IR for `f32::min`/`f64::min`
kernels; one as emitted, one with `nsz` removed from the `minimumnum`,
`maximumnum` and `vector.reduce.fmin` calls. Compiled by the same `llc`
(21.1.0, `-O3`), linked into the same harness.

Instruction counts, one scalar `min`, hand-written IR:

| cpu / attr | `nsz` | deterministic | delta |
|---|---:|---:|---:|
| `x86-64` (SSE2) | 9 | 16 | +7 |
| `x86-64-v2` | 7 | 15 | +8 |
| `x86-64-v3` (AVX2) | 4 | 14, branches | +10 |
| `skylake-avx512` | 5 | 12 | +7 |
| `+avx10.2-512` | 2 | 2 | 0 |

`f64` and `max` match `f32` and `min`. Vector form `<8 x float>`: 4 → 6 on AVX2,
5 → 8 on `skylake-avx512`, 2 → 2 on AVX10.2.

On aarch64 the two variants are the same assembly, instruction for instruction,
on `generic`, `neoverse-n1` and `apple-m1`: `f32` and `f64` 4 each, `<8 x float>`
7, `max` 4. `fminnm` already orders signed zeros, so `nsz` changes nothing
there and the cost above is specific to x86.

Wall-clock, Xeon E5-2686 v4 (Broadwell-EP), pinned to one core, n = 16384
(L2-resident), 21 repeats, ratio = deterministic / `nsz`:

| kernel | min | median |
|---|---:|---:|
| `fold(INFINITY, f32::min)` | 2.473 | 2.447 |
| `x = x.min(y)` | 2.537 | 2.479 |
| `o[i] = a[i].min(b[i])` | 1.146 | 1.149 |
| `clamp(0.0, 6.0)` | 0.984 | 1.009 |
| `fold` f64 | 2.479 | 2.374 |
| `x = x.min(y)` f64 | 2.511 | 2.441 |
| `o[i] = a[i].min(b[i])` f64 | 0.999 | 0.996 |

Same machine, f32, by working-set size:

| kernel | 4 KiB | 64 KiB | 1 MiB | 16 MiB |
|---|---:|---:|---:|---:|
| reduce | 2.14 | 2.48 | 1.91 | 1.93 |
| scalar chain | 1.99 | 2.37 | 1.93 | 1.89 |
| elementwise | 2.09 | 1.13 | 1.00 | 1.02 |
| clamp | 1.01 | 1.01 | 0.99 | 1.02 |

i7-8559U (Coffee Lake), same sweep: reduce 1.38–2.05, scalar chain 1.35–1.83,
elementwise 1.04–1.42, clamp 0.79–1.02.

A GitHub Actions runner (Ubuntu 26.04, LLVM 21.1.8, not pinned to a core) gives
reduce 1.663, scalar chain 1.683, elementwise 1.003, clamp 1.008, and for `f64`
1.557, 1.543, 1.054, at the same n. Same shape, lower on a newer part than on
the 2016 Broadwell above. The workflow in this repo recomputes it on each push,
and reproduces the instruction counts exactly.

With one element in eight set to `±0.0`, the ratios move by about 10%, in both
directions.

## Fast lowering: gain

`a.min(b)` against `if a < b { a } else { b }`, rustc 1.96, whole function:

| cpu | today | fast |
|---|---:|---:|
| `x86-64` (SSE2) | 11 | 5 |
| `x86-64-v3` (AVX2) | 7 | 5 |

AVX2 sequence: `vminss; vcmpunordss; vblendvps` against `vminss`.

Wall-clock, ratio = today / fast (above 1 means the fast form is faster), by
working-set size 4 KiB / 64 KiB / 1 MiB / 16 MiB:

| kernel | Xeon E5-2686 v4 | i7-8559U |
|---|---|---|
| elementwise | 0.86 / 1.03 / 1.01 / 1.00 | 1.49 / 1.26 / 1.09 / 1.03 |
| reduce, via `_mm256_min_ps` | 0.92 / 0.85 / 1.03 / 1.03 | 1.17 / 0.93 / 1.04 / 1.25 |
| clamp | 1.02 / 1.02 / 1.00 / 1.00 | 1.00 / 0.95 / 0.98 / 1.04 |

## Limits

- `llc` is 21.1.0; rustc 1.96 pins LLVM at a 2026-03-28 snapshot.
  `llvm/llvm-project@4a1c14678` ("[X86] Fix NaN handling in
  minimumnum/maximumnum zero fixup", 2026-08-25) adds another `setcc` + `select`
  to this fixup and is in neither, so the cost figures are a lower bound.
- No hardware counters: `perf_event_paranoid` is 4 on the Xeon, which is a
  shared machine, and valgrind is not installed there. The repeat-minimum is
  reported instead, with the `nsz` variant's own median/min as the noise floor
  (at most 1.055 in the table above).
- No AVX-512 or AVX10.2 hardware; those two rows are codegen only.
- Rewriting `min` as `if a < b { a } else { b }` in Rust source is not a valid
  stand-in for a faster lowering in reduction shapes: LLVM stops recognising the
  reduction, the loop goes scalar, and wall-clock rises about 9x while the
  instruction count falls. The reduce row above therefore uses `_mm256_min_ps`.
