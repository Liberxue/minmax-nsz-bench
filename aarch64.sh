#!/bin/bash
# On aarch64, fminnm already orders signed zeros, so nsz should change nothing.
# Compares the two variants instruction for instruction.
set -e
llc=${LLC:-llc}
mkdir -p build
status=0
for cpu in generic neoverse-n1 apple-m1; do
    $llc -mtriple=aarch64-unknown-linux-gnu -mcpu=$cpu -O3 src/minmax.ll -o build/a64.s
    for f in f32 f64 v8f32 max_f32; do
        a=$(awk "/^${f}_nsz:/,/cfi_endproc/" build/a64.s | grep -E '^\t[a-z]' | sed -E 's/[sdqvz][0-9]+/R/g')
        b=$(awk "/^${f}_det:/,/cfi_endproc/" build/a64.s | grep -E '^\t[a-z]' | sed -E 's/[sdqvz][0-9]+/R/g')
        n=$(printf '%s\n' "$a" | grep -c . || true)
        if [ "$a" = "$b" ]; then
            printf '%-13s %-8s same, %s instructions\n' "$cpu" "$f" "$n"
        else
            printf '%-13s %-8s DIFFERENT\n' "$cpu" "$f"
            status=1
        fi
    done
done
exit $status
