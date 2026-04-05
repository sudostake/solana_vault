import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";

import { SolanaVault } from "../target/types/solana_vault";

describe("solana_vault", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace.solanaVault as Program<SolanaVault>;

  it("initializes the single-vault state", async () => {
    const owner = provider.wallet.publicKey;
    const preferredVoteAccount = anchor.web3.Keypair.generate().publicKey;

    const [vaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault")],
      program.programId
    );
    await program.methods
      .initializeVault(owner, preferredVoteAccount, new anchor.BN(1_000_000))
      .rpc();

    const vault = await program.account.vaultState.fetch(vaultPda);
    expect(vault.owner.toBase58()).to.equal(owner.toBase58());
    expect(vault.preferredVoteAccount.toBase58()).to.equal(
      preferredVoteAccount.toBase58()
    );
  });
});
