What dropping `nsz` from `f32::min` lowering costs on x86 (rust-lang/rust#154061).

Both variants come from the same rustc-emitted IR, one as emitted, one with `nsz`
stripped off the `minimumnum` / `vector.reduce.fmin` calls, compiled by the same
`llc` and linked into the same harness. The A/B is the semantic change itself,
not a source-level imitation of it.

## Run

Needs rustc and an `llc`. Pass the `llc` path, since the one rustc uses is not
on PATH.

    LLC=/usr/local/opt/llvm/bin/llc ./bench.sh x86-64-v3

That emits the IR, writes the two variants, compiles both and links
`build/bench`. Then:

    ./build/bench 16384            # 1024 = L1, 16384 = L2, 262144 = L3, 4194304 = RAM
    ./build/bench 16384 --zeros    # one element in eight set to +-0.0

Columns are the ratio of the repeat-minimum, the ratio of the median, and the
`nsz` variant's own median/min. The third column is the noise floor: above about
1.1 the first two are not worth reading. Pin the run where the OS allows it:

    taskset -c 2 ./build/bench 16384

Instruction counts for one scalar `min`, per x86 level:

    LLC=/usr/local/opt/llvm/bin/llc ./isa.sh

Columns are `nsz`, deterministic, and the level. Two standalone checks:

    rustc -O -o build/verify src/verify_fixup.rs && ./build/verify
    rustc -O -C target-cpu=x86-64-v3 -o build/fastpath src/fastpath_reduce.rs && ./build/fastpath

`verify_fixup` checks a source-level zero fixup against a reference
minimumNumber over special values, random bit patterns and a semi-exhaustive
sweep. `fastpath_reduce` asks the opposite question: today's lowering against a
single `_mm256_min_ps` reduction.

To measure on another machine, build the objects for its triple and link there:

    LLC=/usr/local/opt/llvm/bin/llc ./bench.sh x86-64-v3 x86_64-unknown-linux-gnu
    scp build/nsz.o build/det.o src/bench.rs host:
    ssh host 'rustc -O -C target-cpu=x86-64-v3 -C link-arg=nsz.o -C link-arg=det.o -o bench bench.rs && ./bench 16384'

## Two traps

rustc writes `target-cpu` into the IR as a function attribute and it beats
`llc -mcpu`, so `isa.sh` works from attribute-free hand-written IR instead.

Rewriting `min` as `if a < b { a } else { b }` in source is not a stand-in for a
faster lowering. LLVM stops recognising the reduction, the loop goes scalar, and
wall-clock rises about 9x while the instruction count falls.
