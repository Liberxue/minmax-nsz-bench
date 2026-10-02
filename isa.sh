#!/bin/bash
# Instruction counts for one scalar min, with and without nsz.
# Hand-written IR, because rustc's "target-cpu" attribute overrides llc -mcpu.
set -e
llc=${LLC:-llc}
mkdir -p build
for sel in -mcpu=x86-64 -mcpu=x86-64-v2 -mcpu=x86-64-v3 -mcpu=skylake-avx512 -mattr=+avx10.2-512; do
    $llc -mtriple=x86_64-unknown-linux-gnu $sel -O3 src/minmax.ll -o build/isa.s
    for f in f32_nsz f32_det; do
        printf '%s\t' "$(awk "/^$f:/,/cfi_endproc/" build/isa.s | grep -cE '^\t[a-z]')"
    done
    echo "${sel#*=}"
done
