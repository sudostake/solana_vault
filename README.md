# Solana Vault

From the project root, run the project in this order.

1. Start the validator node.

```bash
yarn validator:start
```

Keep this terminal running.

2. Build the program.

```bash
anchor build
```

3. Deploy the program.

```bash
anchor deploy
```

4. Test the program.

```bash
anchor test --skip-build --skip-deploy --skip-local-validator
```
