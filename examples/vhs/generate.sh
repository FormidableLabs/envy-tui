#!/bin/bash

# Exit on error. Append "|| true" if you expect an error.
set -o errexit
# Exit on error inside any functions or subshells.
set -o errtrace
# Do not allow use of undefined vars. Use ${VAR:-} to use an undefined VAR
set -o nounset

# ensure that running each example doesn't have to wait for the build
cargo build

for tape_path in examples/vhs/*.tape; do
    tape_file=${tape_path/examples\/vhs\//} # strip the examples/vhs/ prefix
    gif_file=${tape_file/.tape/.gif}        # replace the .tape suffix with .gif
    vhs $tape_path --quiet
done