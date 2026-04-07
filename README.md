# Solana Vault

This workspace uses:

- Anchor for build, deploy, and integration test commands
- Surfpool for the local validator on `localhost`

The local workflow is intentionally split into separate phases:

1. Start validator
2. Build program
3. Deploy program
4. Test program

`yarn test` does not start the validator for you.

## Default Localhost Workflow

The default cluster for local development is `localhost`.

That means:

- Surfpool should run on `http://localhost:8899`
- local deploy should target `http://localhost:8899`
- local Anchor integration tests should target `http://localhost:8899`

The clean local sequence is:

```bash
yarn solana:config:set
yarn validator:start
yarn build
yarn deploy
yarn test
```

Run `yarn validator:start` in one terminal and the remaining commands in a second terminal.

## Scripts

The main scripts are defined in [package.json](package.json):

- `yarn solana:config:get`
  - Show the active Solana CLI config
- `yarn solana:config:set`
  - Point Solana CLI to `localhost`
- `yarn validator:start`
  - Start Surfpool using the native `surfpool start` command with `--no-deploy`
- `yarn localnet:start`
  - Alias for `yarn validator:start`
- `yarn build`
  - Build the Anchor program
- `yarn deploy`
  - Deploy the program
- `yarn test`
  - Run Anchor integration tests against an already-running validator and an already-deployed program
- `yarn test:program`
  - Run Rust-only program tests

## Step-by-Step Setup

### 1. Point Solana CLI at localhost

```bash
yarn solana:config:set
yarn solana:config:get
```

Expected local RPC:

```text
RPC URL: http://localhost:8899
```

This only sets the default Solana CLI RPC URL. It does not start the validator.

### 2. Start the validator

In a dedicated terminal:

```bash
yarn validator:start
```

This runs the native Surfpool startup flow with `--no-deploy`, so Surfpool itself prints the validator settings and startup details.

If this step fails, nothing else in the local workflow will work. You need the validator running first.

### 3. Build the program

In a second terminal:

```bash
yarn build
```

This produces:

- `target/deploy/solana_vault.so`
- `target/deploy/solana_vault-keypair.json`

### 4. Deploy the program

Still in the second terminal:

```bash
yarn deploy
```

This deploys the built program to the validator already running on `localhost`.

Default local wallet:

- `~/.config/solana/id.json`

Override if needed:

```bash
SOLANA_VAULT_LOCAL_WALLET=/path/to/local-wallet.json yarn deploy
```

### 5. Run integration tests

After the validator is running and the program is deployed:

```bash
yarn test
```

This command only runs the Anchor integration test phase. It does not:

- start Surfpool
- build the program
- deploy the program

That separation is intentional.

## Why `yarn test` Fails Without a Validator

Anchor integration tests need a reachable RPC endpoint and a deployed program.

So this will fail:

```bash
yarn test
```

if you have not already done:

1. `yarn validator:start`
2. `yarn build`
3. `yarn deploy`

That behavior is now expected and by design.

## Program Tests

For fast Rust-only tests that do not require a validator:

```bash
yarn test:program
```

## Remote Cluster Flags

The deploy and test wrappers accept explicit remote flags:

- `--devnet`
- `--testnet`
- `--mainnet-beta`

Examples:

```bash
yarn deploy -- --devnet
yarn test -- --devnet
```

Use remote test flags only when the program is already deployed on that cluster and you intentionally want to test against remote state.

For Surfpool-specific startup behavior, pass native Surfpool flags directly after `--`. Examples:

```bash
yarn validator:start -- --network devnet
yarn validator:start -- --offline
yarn validator:start -- --host 127.0.0.1
```

## Troubleshooting

### Surfpool must start first

If `yarn deploy` or `yarn test` fails on localhost, verify Surfpool is running:

```bash
yarn validator:start
```

### `Failed to start subgraph and explorer server: Operation not permitted (os error 1)`

That means the current terminal environment is blocking Surfpool from opening local services.

This is not an Anchor build or deploy bug in the project.

Run the local workflow in a normal system terminal outside the restricted session.

Surfpool logs are written under:

```text
.surfpool/logs
```

## Files That Define The Workflow

- [Anchor.toml](Anchor.toml)
- [package.json](package.json)
- [scripts/surfpool-start.sh](scripts/surfpool-start.sh)
- [scripts/anchor-deploy.sh](scripts/anchor-deploy.sh)
- [scripts/anchor-test.sh](scripts/anchor-test.sh)
- [scripts/solana-config.sh](scripts/solana-config.sh)
- [txtx.yml](txtx.yml)
