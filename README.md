What dropping `nsz` from `f32::min` lowering costs on x86 (rust-lang/rust#154061).

Both variants come from the same rustc-emitted IR, one as emitted, one with `nsz`
stripped off the `minimumnum` / `vector.reduce.fmin` calls, compiled by the same
`llc` and linked into the same harness.

    LLC=/path/to/llc ./bench.sh x86-64-v3
    taskset -c 2 ./build/bench 16384      # columns: min ratio, median ratio, nsz spread
    ./build/bench 16384 --zeros
    LLC=/path/to/llc ./isa.sh             # instruction counts, one scalar min

Cross-building for linux: `./bench.sh x86-64-v3 x86_64-unknown-linux-gnu`, then
copy `build/*.o` and `src/bench.rs` and link on the other machine.

`src/verify_fixup.rs` checks a source-level zero fixup against a reference
minimumNumber. `src/fastpath_reduce.rs` goes the other way: today's lowering
against a single `_mm256_min_ps` reduction.

Two traps. rustc writes `target-cpu` into the IR as a function attribute and it
beats `llc -mcpu`, so `isa.sh` uses attribute-free hand-written IR. And rewriting
`min` as `if a < b { a } else { b }` in source is not a stand-in for a faster
lowering: LLVM stops recognising the reduction, the loop goes scalar, and
wall-clock rises ~9x while the instruction count falls.
