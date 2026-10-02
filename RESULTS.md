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

With one element in eight set to `±0.0`, the wall-clock ratios move by about
10% in both directions. That is noise: under hardware counters the same input
leaves port-5 uops identical to five figures (247224917 against 247225864) and
cycles within 0.04%. Nothing in either lowering is data-dependent.

## Where the cost goes

Same Xeon, `perf stat` under sudo, pinned to an idle core, n = 16384, 20000
calls per run, 40.96M `ymm` vectors per run. Port counts are per vector and
repeat exactly; cycle counts do not, so they are repeat-minimums over four runs,
the same convention as the wall-clock tables above.

| kernel | variant | cyc/ymm | p0 | p1 | p5 | p5 busy |
|---|---|---:|---:|---:|---:|---:|
| reduce | `nsz` | 2.48 | 0.26 | 2.03 | 2.03 | 78% |
| reduce | deterministic | 6.23 | ~0 | 3.07 | 6.04 | 97% |
| chain | `nsz` | 2.57 | 0.25 | 2.03 | 2.03 | 79% |
| chain | deterministic | 6.23 | ~0 | 3.07 | 6.04 | 97% |
| elementwise | `nsz` | 8.40 | | | 2.01 | 24% |
| elementwise | deterministic | 9.07 | | | 5.99 | 66% |
| clamp | either | 8.30 | | | 0.04 | 0.4% |

On this part `vminps` and `vcmpps` issue only to port 1, `vblendvps` and the
vector logicals only to port 5, and port 0 sits idle. `vblendvps ymm` is 2 uops.
The deterministic sequence needs three blends per vector against one, so port 5
goes from 2.03 to 6.04 uops per vector and saturates at 97%. Cycles then track
port-5 uops, which is why the reduction runs 2.51x.

The elementwise loop issues the same three blends but is memory-bound, which is
why it lands near 1.0. `clamp` has no blends at all, the bound being the
constant `+0.0`, and the two builds are instruction-identical there. Branch
mispredictions are flat throughout. The elementwise and `clamp` cycle counts
drift by up to 20% on this shared machine; reduce and chain hold to 0.5%.

## A cheaper lowering

Nine candidate sequences, in `src/cand.rs`, measured in the reduction shape
against `nsz` in the same binary. Correctness is lane-wise against the fixup
reference over 1602936 comparisons, and whole-reduction against a scalar fold
over 4400 arrays covering both NaN signs, mixed signed zeros, and lengths with
partial tails.

| sequence | correct | p5/ymm | cyc/ymm | vs `nsz` |
|---|---|---:|---:|---:|
| `nsz`, the fast contract | no, by design | 2.04 | 2.36 | 1.00 |
| `keyacc` | yes | 2.38 | 3.47 | 1.47 |
| `twoblend` | yes | 5.04 | 5.17 | 2.19 |
| `bitops` | yes | 6.05 | 6.19 | 2.62 |
| `orminmin2` | yes | 6.04 | 6.17 | 2.61 |
| `intkey` | yes | 4.80 | 6.62 | 2.81 |

What rustc emits today is 6.04 port-5 uops per vector and 2.51x, measured the
same way in its own binary. That binary is not exactly comparable: its `nsz`
reduction folds the load into `vminps` where the candidates issue a separate
`vmovups`, which is worth about 5% on the baseline, so read the cross-binary
ratios as approximate and the port counts as exact.

`twoblend` keeps the `nsz` tail and fixes the tie with `vorps` plus one blend,
one blend fewer than the operand sort rustc emits. `bitops` replaces that blend
with and/andnot/or and gains nothing, because the logicals land on port 5 too.

`keyacc` is reduction-specific: the accumulator stays in the integer key
`bits ^ ((bits >> 31) >>> 1)`, whose signed order is float order, so `-0.0`
falls below `+0.0` and the tie needs no fixup at all. Each loaded vector costs
two shifts on port 0, one xor, one blend to lift NaN to `INT_MAX`, and a
`vpminsd`. Port 5 drops to 2.38 and the work spreads across all three ports,
none above 73%, so the loop stops being port-bound. It is the cheapest correct
sequence found here, not a proven floor.

`orminmin`, `signor` and `signor_z` are in the file because they fail. Each
breaks on a negatively signed NaN, which `./build/cand verify` reports.

## The same sequences on a newer part

i7-8559U (Coffee Lake), wall-clock since macOS exposes no usable PMU, not pinned,
`./cand time 16384`, minimum of 21 repeats, three runs:

| sequence | Broadwell | Coffee Lake |
|---|---:|---:|
| `keyacc` | 1.47 | 1.50-1.55 |
| `twoblend` | 2.19 | 1.64-1.77 |
| `bitops` | 2.62 | 1.87-1.91 |
| `orminmin2` | 2.61 | 1.94-2.04 |
| `intkey` | 2.81 | 3.05-3.14 |
| what rustc emits today | 2.51 | 1.60-1.65 |

Everything that spends blends got cheaper by 0.55 to 0.75, `keyacc` did not move,
and `intkey` got worse. That is what the port reading predicts: `vblendvps ymm`
is one uop on a Skylake-derived part and issues to more than one port, so the
sequences bounded by blend throughput gain and the one that avoids blends does
not. `intkey` is bounded by its shifts, and loses ground as the baseline speeds
up.

The practical consequence is that the gap between what rustc emits and the best
sequence here is 41% on Broadwell and about 7% on Coffee Lake. The room on
Broadwell is real but it does not generalise: on the newer part the current
lowering is already close to what these sequences can do, and the cost of
determinism itself falls from 2.51 to about 1.6.

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
- The wall-clock tables predate the counter runs and report the repeat-minimum,
  with the `nsz` variant's own median/min as the noise floor (at most 1.055).
  `perf_event_paranoid` is 4 on the Xeon, so `perf.sh` runs perf under sudo; the
  counter rows come from there. A GitHub Actions runner exposes no PMU, so the
  workflow cannot reproduce them.
- `keyacc` is a reduction strategy, not a lowering for a single `minimumnum`
  call. It pays off by holding the accumulator in key space across the loop.
- The port assignments behind the counter rows are specific to Broadwell. On
  parts where `vblendvps ymm` is one uop and issues to more than one port, the
  ratios will be lower; the runner numbers above are consistent with that.
- Only `keyacc` and `intkey` vary at all between runs of one binary, and by
  0.6% and 1.6% of their port-5 count. The rest repeat exactly. An earlier
  `keyacc` written as a loop over `[(&mut a0, 0), ...]` varied between builds
  instead, 2.32 to 2.56 uops per vector; the accumulators are written out now.
- No AVX-512 or AVX10.2 hardware; those two rows are codegen only.
- Rewriting `min` as `if a < b { a } else { b }` in Rust source is not a valid
  stand-in for a faster lowering in reduction shapes: LLVM stops recognising the
  reduction, the loop goes scalar, and wall-clock rises about 9x while the
  instruction count falls. The reduce row above therefore uses `_mm256_min_ps`.
