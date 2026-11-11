#!/usr/bin/env bash
# Snapshot antlion's output for every pair in out/, and compare a later run with it.
set -euo pipefail
cd "$(dirname "$0")/.."

BIN=target/release/antlion
GOLD=target/golden
NOW=target/current

pairs() { ls out; }

# Fill directory $1 with the output of every mode, per pair and for all pairs at once.
snapshot() {
    rm -rf "$1"
    mkdir -p "$1"
    for p in $(pairs); do
        $BIN --compare "$p" --report all > "$1/$p.txt"
        $BIN --compare "$p" --report all --json > "$1/$p.json"
        $BIN --compare "$p" --report opcodes --detail > "$1/$p.detail.txt"
        $BIN --compare "$p" --report opcodes --dump > "$1/$p.dump.txt"
    done
    $BIN --compare $(pairs) --report all > "$1/all.txt"
    $BIN --compare $(pairs) --report all --json > "$1/all.json"
}

case "${1:-}" in
build)
    cargo build --release
    for d in contracts/*/; do $BIN --build "$d"; done
    ;;
save)
    cargo build --release
    snapshot "$GOLD"
    (cd out && sha256sum */*.wasm) > "$GOLD/wasm.sha256"
    echo "OK: saved to $GOLD"
    ;;
diff)
    cargo build --release
    snapshot "$NOW"
    diff -r -x wasm.sha256 "$GOLD" "$NOW"
    echo "OK: output identical"
    ;;
wasm)
    (cd out && sha256sum --quiet -c "../$GOLD/wasm.sha256")
    echo "OK: wasm identical"
    ;;
time)
    cargo build --release
    /usr/bin/time -f "%e s, %M KB" $BIN --compare $(pairs) --report all > /dev/null
    ;;
*)
    echo "usage: scripts/check.sh build|save|diff|wasm|time"
    exit 2
    ;;
esac
