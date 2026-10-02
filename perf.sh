#!/bin/bash
# Hardware counters for the two variants, and for the candidate sequences.
# Needs a PMU: a GitHub Actions runner has none, so this is local-only.
# perf_event_paranoid is 4 on the Xeon used here, so perf runs under sudo.
#
#   LLC=/usr/local/opt/llvm/bin/llc CORE=11 ./perf.sh
#
# With objects built elsewhere (the Xeon has no llc), skip the llc step:
#
#   PREBUILT=1 CORE=11 ./perf.sh
set -e
cpu=${CPU:-x86-64-v3}
llc=${LLC:-llc}
n=${N:-16384}
iters=${ITERS:-20000}
core=${CORE:-2}
reps=${REPS:-5}
mkdir -p build

if [ -z "$PREBUILT" ]; then
    rustc --edition 2024 -O --emit llvm-ir -C target-cpu=$cpu -o build/k.ll src/kernels.rs
    sed -E 's/ nocreateundeforpoison//g' build/k.ll | sed -E 's/@k_/@nsz_/g' > build/nsz.ll
    sed -E 's/ nocreateundeforpoison//g' build/k.ll | sed -E 's/call nsz /call /g; s/@k_/@det_/g' > build/det.ll
    for v in nsz det; do $llc -mcpu=$cpu -O3 -filetype=obj build/$v.ll -o build/$v.o; done
fi
rustc --edition 2024 -O -C target-cpu=$cpu -C link-arg=build/nsz.o -C link-arg=build/det.o -o build/perfone src/perfone.rs
rustc --edition 2024 -O -C target-cpu=$cpu -o build/cand src/cand.rs

EV=uops_dispatched_port.port_0,uops_dispatched_port.port_1,uops_dispatched_port.port_5,cycles,instructions,br_misp_retired.all_branches
vectors=$(( n / 8 * iters ))

one() { # $1=label  $2.. = command
    local label=$1; shift
    sudo -n perf stat -x, -e $EV -- taskset -c $core "$@" 2>&1 >/dev/null \
    | awk -F, -v l="$label" -v v="$vectors" '
        $3 ~ /port_0/ { p0 = $1 } $3 ~ /port_1/ { p1 = $1 } $3 ~ /port_5/ { p5 = $1 }
        $3 == "cycles" { c = $1 } $3 == "instructions" { i = $1 } $3 ~ /br_misp/ { b = $1 }
        END { printf "%-20s %7.2f %7.2f %7.2f %7.2f %6.1f%% %12d\n", l, c/v, p0/v, p1/v, p5/v, 100*p5/c, b }'
}

echo "n=$n iters=$iters core=$core cpu=$cpu vectors=$vectors"
printf "%-20s %7s %7s %7s %7s %7s %12s\n" "" cyc/ymm p0/ymm p1/ymm p5/ymm p5/cyc mispred

# What rustc emits today, both variants, per kernel shape.
for k in reduce chain ew clamp; do
    for v in nsz det; do
        one "$k/$v" ./build/perfone $v $k $n $iters
    done
done

# Candidate lowerings, in the reduction shape. Interleaved, since the Xeon is
# shared and absolute cycle counts drift with whatever else is running.
echo
for r in $(seq 1 $reps); do
    for c in nsz twoblend bitops orminmin2 intkey keyacc; do
        one "cand $c" ./build/cand $c $n $iters
    done
done
