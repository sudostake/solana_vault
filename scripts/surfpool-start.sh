#!/bin/zsh

set -euo pipefail

for arg in "$@"; do
  case "${arg}" in
    -h|--help)
      exec env NO_DNA=1 surfpool start --no-deploy --help
      ;;
  esac
done

exec env NO_DNA=1 surfpool start --no-deploy "$@"
