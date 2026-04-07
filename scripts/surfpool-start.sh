#!/bin/zsh

set -euo pipefail

script_dir="${0:A:h}"
source "${script_dir}/solana-context.sh"

latest_surfpool_log() {
  ls -1t .surfpool/logs 2>/dev/null | head -n 1
}

print_surfpool_failure_logs() {
  local latest_log
  latest_log="$(latest_surfpool_log)"

  if [[ -z "${latest_log}" ]]; then
    return
  fi

  print -u2 "Latest Surfpool log: .surfpool/logs/${latest_log}"
  sed -n '1,120p' ".surfpool/logs/${latest_log}" >&2
}

uses_native_network_flags() {
  local arg
  for arg in "$@"; do
    case "${arg}" in
      -n|--network|--network=*|-u|--rpc-url|--rpc-url=*|-o|--host|--host=*|-p|--port|--port=*)
        return 0
        ;;
    esac
  done
  return 1
}

uses_custom_airdrop_flags() {
  local arg
  for arg in "$@"; do
    case "${arg}" in
      -a|--airdrop|--airdrop=*|-k|--airdrop-keypair-path|--airdrop-keypair-path=*)
        return 0
        ;;
    esac
  done
  return 1
}

for arg in "$@"; do
  case "${arg}" in
    -h|--help)
      exec surfpool start --no-deploy --help
      ;;
  esac
done

if uses_native_network_flags "$@"; then
  exec surfpool start --no-deploy "$@"
fi

solana_require_rpc_url

if ! solana_is_local_rpc_url "${SOLANA_RPC_URL}"; then
  print -u2 "The current Solana CLI RPC URL is not local: ${SOLANA_RPC_URL}"
  print -u2 "Run 'solana config set --url localhost' to target localhost before starting Surfpool."
  print -u2 "For remote clusters, skip 'yarn validator:start' and run build, deploy, and test against the configured RPC."
  exit 1
fi

if ! solana_local_binding_from_url "${SOLANA_RPC_URL}"; then
  print -u2 "Unable to derive a local host and port from ${SOLANA_RPC_URL}"
  exit 1
fi

print "Surfpool environment: ${SOLANA_CLUSTER_NAME}"
print "Local RPC: ${SOLANA_RPC_URL}"

command=(surfpool start --no-deploy)

if [[ "${SOLANA_LOCAL_HOST}" != "127.0.0.1" || "${SOLANA_LOCAL_PORT}" != "8899" ]]; then
  command+=(--host "${SOLANA_LOCAL_HOST}" --port "${SOLANA_LOCAL_PORT}")
fi

if ! uses_custom_airdrop_flags "$@" && [[ -n "${SOLANA_KEYPAIR_PATH:-}" ]]; then
  set +e
  current_wallet_address="$(solana_wallet_address "${SOLANA_KEYPAIR_PATH}" 2>/dev/null)"
  wallet_address_status=$?
  set -e

  if (( wallet_address_status == 0 )) && [[ -n "${current_wallet_address}" ]]; then
    print "Airdrop wallet: ${current_wallet_address}"
    command+=(--airdrop "${current_wallet_address}")
  else
    print -u2 "Unable to resolve the configured wallet address from ${SOLANA_KEYPAIR_PATH}."
    print -u2 "Surfpool will start without auto-airdropping the current signer."
  fi
fi

command+=("$@")

set +e
"${command[@]}"
exit_code=$?
set -e

if (( exit_code != 0 )); then
  print_surfpool_failure_logs
fi

exit "${exit_code}"
