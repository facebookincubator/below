#!/bin/bash
#
# Script to test all crates recursively. Must be run from root of
# repository.

set -e

for cargo_dir in $(find . -name Cargo.toml -printf '%h\n'); do
  echo "Running tests in: $cargo_dir"
  pushd "$cargo_dir"
  cargo clean
  cargo test \
    --release \
    -- \
    --skip test_dump \
    --skip advance_forward_and_reverse \
    --skip disable_disk_stat \
    --skip disable_io_stat \
    --skip record_replay_integration \
    --skip test_belowrc_to_event \
    --skip test_event_controller_override \
    --skip test_event_controller_override_failed \
    --skip test_viewrc_collapse_cgroups \
    --skip test_viewrc_default_view
  popd
done
