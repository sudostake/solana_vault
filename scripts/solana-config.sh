#!/bin/zsh

set -euo pipefail

cluster_name="localhost"
remaining_args=()

while (( $# > 0 )); do
  case "$1" in
    --localhost|--localnet)
      cluster_name="localhost"
      ;;
    --devnet)
      cluster_name="devnet"
      ;;
    --testnet)
      cluster_name="testnet"
      ;;
    --mainnet-beta|--mainnet)
      cluster_name="mainnet-beta"
      ;;
    --)
      shift
      while (( $# > 0 )); do
        remaining_args+=("$1")
        shift
      done
      break
      ;;
    *)
      remaining_args+=("$1")
      ;;
  esac
  shift
done

print "Setting Solana CLI RPC URL to ${cluster_name}"

solana config set --url "${cluster_name}" "${remaining_args[@]}"
