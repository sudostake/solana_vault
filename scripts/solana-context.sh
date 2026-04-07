#!/bin/zsh

solana_cli() {
  local -a cmd
  cmd=(solana)
  if [[ -n "${SOLANA_CONFIG_PATH:-}" ]]; then
    cmd+=(--config "${SOLANA_CONFIG_PATH}")
  fi
  cmd+=("$@")
  "${cmd[@]}"
}

solana_config_value() {
  local key="$1"
  solana_cli config get 2>/dev/null | sed -n "s/^${key}: //p" | head -n 1 | sed 's/[[:space:]]*$//'
}

solana_cluster_name_from_url() {
  local rpc_url="${1%/}"

  if solana_is_local_rpc_url "${rpc_url}"; then
    print "localhost"
    return 0
  fi

  case "${rpc_url}" in
    https://api.devnet.solana.com|https://api.devnet.solana.com/*)
      print "devnet"
      ;;
    https://api.testnet.solana.com|https://api.testnet.solana.com/*)
      print "testnet"
      ;;
    https://api.mainnet-beta.solana.com|https://api.mainnet-beta.solana.com/*)
      print "mainnet-beta"
      ;;
    *)
      print "custom"
      ;;
  esac
}

solana_is_local_rpc_url() {
  local rpc_url="${1%/}"
  case "${rpc_url}" in
    http://localhost|http://localhost:*|http://127.0.0.1|http://127.0.0.1:*|http://0.0.0.0|http://0.0.0.0:*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

solana_local_binding_from_url() {
  local rpc_url="${1%/}"
  local host_and_port host port

  if ! solana_is_local_rpc_url "${rpc_url}"; then
    return 1
  fi

  host_and_port="${rpc_url#http://}"
  host="${host_and_port%%:*}"
  if [[ "${host_and_port}" == "${host}" ]]; then
    port="8899"
  else
    port="${host_and_port##*:}"
  fi

  if [[ -z "${host}" || -z "${port}" ]]; then
    return 1
  fi

  case "${host}" in
    localhost)
      host="127.0.0.1"
      ;;
  esac

  SOLANA_LOCAL_HOST="${host}"
  SOLANA_LOCAL_PORT="${port}"
}

solana_resolve_context() {
  SOLANA_RPC_URL="$(solana_config_value 'RPC URL')"
  SOLANA_KEYPAIR_PATH="$(solana_config_value 'Keypair Path')"
  SOLANA_CLUSTER_NAME="$(solana_cluster_name_from_url "${SOLANA_RPC_URL}")"
}

solana_require_rpc_url() {
  solana_resolve_context

  if [[ -z "${SOLANA_RPC_URL}" ]]; then
    print -u2 "Unable to read the Solana CLI RPC URL. Run 'solana config get' or set SOLANA_CONFIG_PATH."
    return 1
  fi
}

solana_require_provider_context() {
  solana_require_rpc_url

  if [[ -z "${SOLANA_KEYPAIR_PATH}" ]]; then
    print -u2 "Unable to read the Solana CLI keypair path. Run 'solana config get' or set SOLANA_CONFIG_PATH."
    return 1
  fi
}

solana_wallet_address() {
  local keypair_path="$1"
  solana_cli address -k "${keypair_path}"
}

solana_provider_wallet_path() {
  local explicit_wallet_path
  explicit_wallet_path="${SOLANA_VAULT_WALLET:-${ANCHOR_WALLET:-}}"

  if [[ -n "${explicit_wallet_path}" ]]; then
    print -r -- "${explicit_wallet_path}"
    return 0
  fi

  print -r -- "${SOLANA_KEYPAIR_PATH}"
}

anchor_with_provider() {
  local action="$1"
  shift

  solana_require_provider_context

  local wallet_path
  wallet_path="$(solana_provider_wallet_path)"

  print "${(C)action} environment: ${SOLANA_CLUSTER_NAME}"
  print "Cluster: ${SOLANA_RPC_URL}"
  print "Wallet: ${wallet_path}"

  exec anchor "${action}" \
    --provider.cluster "${SOLANA_RPC_URL}" \
    --provider.wallet "${wallet_path}" \
    "$@"
}
