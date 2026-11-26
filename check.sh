#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
if [[ $# -eq 0 ]]; then
  /bin/sh .chrono-harness/ci/install.sh
  export PATH="$PWD/.chrono-harness/bin:$PATH"
  export CHRONO_CHECK_SOURCE=${CHRONO_CHECK_SOURCE:-local}
  exec .chrono-harness/bin/chrono-harness check
fi
if [[ $# -ne 1 || $1 != --native ]]; then
  echo 'usage: ./check.sh [--native]' >&2
  exit 2
fi
scratch_root=$(pwd -P)/.chrono-harness/state/native-check
mkdir -p "$scratch_root"
scratch=$(mktemp -d "$scratch_root/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/ci/Cargo.toml
product=crates/ci/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/ci-tests/Cargo.toml
CHRONO_TEST_CI="$CARGO_TARGET_DIR/debug/chrono-ci" cargo test --locked --manifest-path crates/ci-tests/Cargo.toml
coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

# Read the exact library selected by Cargo in the build of the tested executable.
metadata=$(jq -ers --arg manifest "$(pwd -P)/$product" '
  [.[] | select(.reason == "compiler-artifact" and .manifest_path == $manifest
                 and .target.kind == ["lib"]) | .filenames[] | select(endswith(".rmeta"))]
  | if length == 1 then .[0] else error("expected the product library") end
' "$scratch/build.jsonl") || coverage_failure "native library was not identified"
filename=${metadata##*/}
stem=${filename#lib}
stem=${metadata%/*}/${stem%.rmeta}
chrono-host-bootstrap check-compiler-sources native "$stem" src/actions_lock.rs src/units.rs src/migrate.rs src/check_inputs.rs src/verified_literals.rs src/cache.rs src/downloads.rs src/gating.rs \
  src/release.rs src/lib.rs src/startup.rs src/verified_download.rs \
  src/verified_inventory.rs src/inventory.rs src/full.rs src/cache_action.rs src/gather.rs ||
  coverage_failure "native library differs from required sources"

src=crates/ci/src
tests=crates/ci-tests/tests
include=$tests/../../ci/src
support=$tests/proof_support/ci
literals_operation=verify.ci-literals
downloads_operation=verify.ci-downloads
inventory_operation=verify.ci-inventory
downloads_tests_operation=verify.ci-downloads-tests
literals_tests_operation=verify.ci-literals-tests
inventory_tests_operation=verify.ci-inventory-tests

chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$literals_operation.rlib" "$src/verified_literals.rs" \
  "$src/actions_lock.rs" "$src/cache_action.rs" "$src/cache.rs" "$src/check_inputs.rs" "$src/downloads.rs" "$src/gating.rs" "$src/release.rs" "$src/lib.rs" \
  "$src/startup.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$downloads_operation.rlib" "$src/verified_download.rs" "$src/gather.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$inventory_operation.rlib" "$src/verified_inventory.rs" \
  "$src/inventory.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$downloads_tests_operation.rlib" "$tests/verified_download.rs" "$include/verified_download.rs" "$include/gather.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$literals_tests_operation.rlib" "$tests/verified_literals.rs" \
  "$support/cache.rs" "$support/gating.rs" "$support/initial_short.rs" \
  "$include/verified_literals.rs" "$include/actions_lock.rs" "$include/cache_action.rs" "$include/cache.rs" "$include/check_inputs.rs" "$include/downloads.rs" "$include/gating.rs" \
  "$include/release.rs" "$include/lib.rs" "$include/startup.rs" "$support/release.rs" \
  "$support/retained_receipts.rs" "$support/short.rs" \
  "$support/../worktree/support/diagnostic_copy.rs" "$support/startup.rs"
chrono-host-bootstrap check-compiler-sources proof ".chrono-harness/state/ci-proofs/$inventory_tests_operation.rlib" "$tests/verified_inventory.rs" \
  "$support/inventory.rs" "$include/verified_inventory.rs" "$include/inventory.rs"
