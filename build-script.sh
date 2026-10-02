#!/bin/sh
set -e
export PATH="/root/.local/share/solana/install/active_release/bin:$PATH"
git clone -q https://github.com/kkainatAMIR/ronin-rewards /build
cd /build
cargo update -p hashbrown:0.17.1 --precise 0.14.5 >/dev/null 2>&1 || true
cargo update -p indexmap:2.14.2 --precise 2.2.6 >/dev/null 2>&1 || true
echo PINS_DONE
cargo build-sbf --manifest-path programs/ronin-rewards/Cargo.toml -- --locked > /tmp/buildlog.txt 2>&1 &
BPID=$!
for i in $(seq 1 60); do
  if ! kill -0 $BPID 2>/dev/null; then break; fi
  sleep 10
done
if kill -0 $BPID 2>/dev/null; then echo "STILL_RUNNING_AFTER_10MIN"; kill $BPID; fi
echo "=== BUILD LOG TAIL ==="
tail -10 /tmp/buildlog.txt
echo "=== HASH ==="
find /build/target -name 'ronin_rewards.so' -exec sha256sum {} \;
echo "=== LOCK_B64 ==="
base64 -w0 /build/Cargo.lock
