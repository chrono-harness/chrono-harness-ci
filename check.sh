#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
mkdir -p target
scratch=$(mktemp -d "$(pwd -P)/target/check.XXXXXXXX")
trap 'rm -rf -- "$scratch"' EXIT
export TMPDIR="$scratch"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$scratch/cargo}"
cargo fmt --check --manifest-path crates/ci/Cargo.toml
product=crates/ci/Cargo.toml
cargo build --locked --message-format=json-render-diagnostics --manifest-path "$product" > "$scratch/build.jsonl"
cargo fmt --check --manifest-path crates/ci-tests/Cargo.toml
cargo test --locked --manifest-path crates/ci-tests/Cargo.toml
coverage_failure() {
  echo "E_VERUS_COVERAGE: $*" >&2
  exit 1
}

verifier=$(command -v "${VERUS:-verus}") || coverage_failure "registered verifier was not found"

prove() {
  chrono-host-bootstrap run-compiler-proof "$scratch" "$1" "$verifier" "$2" ||
    coverage_failure "$1: verifier failed"
}

admit_proof() {
  local operation=$1
  shift
  chrono-host-bootstrap collect-compiler-proof "$scratch" "$operation" "$verifier" \
    0.2026.04.24.f8e1704 f8e17045c4ac17a02e566b8997070f2eecc255be "$@" ||
    coverage_failure "$operation: required proof result was not admitted"
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
chrono-host-bootstrap check-compiler-sources native "$stem" src/verified_literals.rs src/cache.rs src/downloads.rs src/gating.rs \
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

prove "$literals_operation" "$src/verified_literals.rs"
prove "$downloads_operation" "$src/verified_download.rs"
prove "$inventory_operation" "$src/verified_inventory.rs"
prove "$downloads_tests_operation" "$tests/verified_download.rs"
prove "$literals_tests_operation" "$tests/verified_literals.rs"
prove "$inventory_tests_operation" "$tests/verified_inventory.rs"

admit_proof "$literals_operation" "$src/verified_literals.rs" \
  "$src/cache.rs" "$src/downloads.rs" "$src/gating.rs" "$src/release.rs" "$src/lib.rs" \
  "$src/startup.rs"
admit_proof "$downloads_operation" "$src/verified_download.rs"
admit_proof "$inventory_operation" "$src/verified_inventory.rs" \
  "$src/inventory.rs"
admit_proof "$downloads_tests_operation" "$tests/verified_download.rs" \
  "$include/verified_download.rs"
admit_proof "$literals_tests_operation" "$tests/verified_literals.rs" \
  "$support/cache.rs" "$support/gating.rs" "$support/initial_short.rs" \
  "$include/verified_literals.rs" "$include/cache.rs" "$include/downloads.rs" "$include/gating.rs" \
  "$include/release.rs" "$include/lib.rs" "$include/startup.rs" "$support/release.rs" \
  "$support/retained_receipts.rs" "$support/short.rs" \
  "$support/../worktree/support/diagnostic_copy.rs" "$support/startup.rs"
admit_proof "$inventory_tests_operation" "$tests/verified_inventory.rs" \
  "$support/inventory.rs" "$include/verified_inventory.rs" "$include/inventory.rs"
