#!/bin/zsh

set -euo pipefail

script_dir="${0:A:h}"
source "${script_dir}/solana-context.sh"

anchor_with_provider test \
  --skip-build \
  --skip-deploy \
  --skip-local-validator \
  "$@"
