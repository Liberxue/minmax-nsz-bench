#!/bin/bash
# Instruction counts for one scalar min, with and without nsz.
# Hand-written IR, because rustc's "target-cpu" attribute overrides llc -mcpu.
set -e
llc=${LLC:-llc}
mkdir -p build

# awk rather than grep: \t in a grep -E pattern is a BSD extension that GNU
# grep reads as a literal t, which silently counts nothing.
count() {
    awk -v f="$2" '
        $0 ~ "^"f":" { inside = 1 }
        inside && /^\t[a-z]/ { n++ }
        inside && /cfi_endproc/ { inside = 0 }
        END { print n + 0 }
    ' "$1"
}

for sel in -mcpu=x86-64 -mcpu=x86-64-v2 -mcpu=x86-64-v3 -mcpu=skylake-avx512 -mattr=+avx10.2-512; do
    $llc -mtriple=x86_64-unknown-linux-gnu $sel -O3 src/minmax.ll -o build/isa.s
    n=$(count build/isa.s f32_nsz)
    d=$(count build/isa.s f32_det)
    if [ "$n" -eq 0 ] || [ "$d" -eq 0 ]; then
        echo "counted nothing for ${sel#*=}; the assembly is not what this expects" >&2
        exit 1
    fi
    printf '%s\t%s\t%s\n' "$n" "$d" "${sel#*=}"
done
