#!/bin/bash
# ./bench.sh [target-cpu] [llc-triple]
set -e
cpu=${1:-x86-64-v3}
llc=${LLC:-llc}
mkdir -p build

rustc -O --emit llvm-ir -C target-cpu=$cpu -o build/k.ll src/kernels.rs
sed -E 's/ nocreateundeforpoison//g' build/k.ll | sed -E 's/@k_/@nsz_/g' > build/nsz.ll
sed -E 's/ nocreateundeforpoison//g' build/k.ll | sed -E 's/call nsz /call /g; s/@k_/@det_/g' > build/det.ll

for v in nsz det; do
    $llc ${2:+-mtriple=$2} -mcpu=$cpu -O3 -filetype=obj build/$v.ll -o build/$v.o
done
rustc -O -C target-cpu=$cpu -C link-arg=build/nsz.o -C link-arg=build/det.o -o build/bench src/bench.rs
