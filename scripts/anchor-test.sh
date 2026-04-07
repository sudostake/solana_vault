#!/bin/zsh

set -euo pipefail

solana_config_value() {
  local key="$1"
  solana config get 2>/dev/null | sed -n "s/^${key}: //p" | head -n 1 | sed 's/[[:space:]]*$//'
}

global_wallet_path="$(solana_config_value 'Keypair Path')"
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

case "${cluster_name}" in
  localhost)
    cluster_url="http://localhost:8899"
    wallet_path="${SOLANA_VAULT_LOCAL_WALLET:-${SOLANA_VAULT_LOCALNET_WALLET:-${ANCHOR_WALLET:-$HOME/.config/solana/id.json}}}"
    ;;
  devnet)
    cluster_url="https://api.devnet.solana.com"
    wallet_path="${SOLANA_VAULT_DEVNET_WALLET:-${SOLANA_VAULT_WALLET:-${ANCHOR_WALLET:-${global_wallet_path:-$HOME/.config/solana/id.json}}}}"
    ;;
  testnet)
    cluster_url="https://api.testnet.solana.com"
    wallet_path="${SOLANA_VAULT_TESTNET_WALLET:-${SOLANA_VAULT_WALLET:-${ANCHOR_WALLET:-${global_wallet_path:-$HOME/.config/solana/id.json}}}}"
    ;;
  mainnet-beta)
    cluster_url="https://api.mainnet-beta.solana.com"
    wallet_path="${SOLANA_VAULT_MAINNET_BETA_WALLET:-${SOLANA_VAULT_MAINNET_WALLET:-${SOLANA_VAULT_WALLET:-${ANCHOR_WALLET:-${global_wallet_path:-$HOME/.config/solana/id.json}}}}}"
    ;;
  *)
    print -u2 "Usage: ./scripts/anchor-test.sh [--localhost|--devnet|--testnet|--mainnet-beta] [anchor test args...]"
    exit 1
    ;;
esac

print "Test environment: ${cluster_name}"
print "Cluster: ${cluster_url}"
print "Wallet: ${wallet_path}"

NO_DNA=1 anchor test \
  --skip-build \
  --skip-deploy \
  --skip-local-validator \
  --provider.cluster "${cluster_url}" \
  --provider.wallet "${wallet_path}" \
  "${remaining_args[@]}"
