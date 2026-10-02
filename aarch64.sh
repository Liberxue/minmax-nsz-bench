#!/bin/bash
# On aarch64, fminnm already orders signed zeros, so nsz should change nothing.
# Compares the two variants instruction for instruction.
set -e
llc=${LLC:-llc}
mkdir -p build

body() {
    awk -v f="$2" '
        $0 ~ "^"f":" { inside = 1 }
        inside && /^\t[a-z]/ { line = $0; gsub(/[sdqvz][0-9]+/, "R", line); print line }
        inside && /cfi_endproc/ { inside = 0 }
    ' "$1"
}

status=0
for cpu in generic neoverse-n1 apple-m1; do
    $llc -mtriple=aarch64-unknown-linux-gnu -mcpu=$cpu -O3 src/minmax.ll -o build/a64.s
    for f in f32 f64 v8f32 max_f32; do
        a=$(body build/a64.s "${f}_nsz")
        b=$(body build/a64.s "${f}_det")
        n=$(printf '%s' "$a" | awk 'NF { n++ } END { print n + 0 }')
        if [ "$n" -eq 0 ]; then
            printf '%-13s %-8s counted nothing, check the assembly\n' "$cpu" "$f"
            status=1
        elif [ "$a" = "$b" ]; then
            printf '%-13s %-8s same, %s instructions\n' "$cpu" "$f" "$n"
        else
            printf '%-13s %-8s DIFFERENT\n' "$cpu" "$f"
            status=1
        fi
    done
done
exit $status
