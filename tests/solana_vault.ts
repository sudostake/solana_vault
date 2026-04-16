import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";

import { SolanaVault } from "../target/types/solana_vault";

describe("solana_vault", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace.solanaVault as Program<SolanaVault>;
  const connection = provider.connection;
  const payer = (
    provider.wallet as anchor.Wallet & {
      payer: anchor.web3.Keypair;
    }
  ).payer;

  const REQUESTED = 1;
  const FUNDED = 2;
  const REPAID = 3;
  const LIQUIDATED = 4;
  const COUNTERED = 5;

  const sleep = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));

  const findVaultPda = (owner: anchor.web3.PublicKey) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault"), owner.toBuffer()],
      program.programId
    )[0];

  const findTreasuryPda = (vault: anchor.web3.PublicKey) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("treasury"), vault.toBuffer()],
      program.programId
    )[0];

  const findCounterOfferPda = (
    vault: anchor.web3.PublicKey,
    lender: anchor.web3.PublicKey
  ) =>
    anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("counter_offer"), vault.toBuffer(), lender.toBuffer()],
      program.programId
    )[0];

  const airdrop = async (
    recipient: anchor.web3.PublicKey,
    lamports: number
  ) => {
    const signature = await connection.requestAirdrop(recipient, lamports);
    await connection.confirmTransaction(signature, "confirmed");
  };

  const setupVault = async () => {
    const owner = anchor.web3.Keypair.generate();
    const lender = anchor.web3.Keypair.generate();

    await airdrop(owner.publicKey, 3 * anchor.web3.LAMPORTS_PER_SOL);
    await airdrop(lender.publicKey, 3 * anchor.web3.LAMPORTS_PER_SOL);

    const vaultPda = findVaultPda(owner.publicKey);
    const treasuryPda = findTreasuryPda(vaultPda);

    await program.methods
      .initializeVault()
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    const usdcMint = await createMint(
      connection,
      payer,
      payer.publicKey,
      null,
      6
    );

    const ownerUsdc = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      usdcMint,
      owner.publicKey
    );
    const lenderUsdc = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      usdcMint,
      lender.publicKey
    );

    return {
      owner,
      lender,
      vaultPda,
      treasuryPda,
      usdcMint,
      ownerUsdc,
      lenderUsdc,
      treasuryRentBalance: await connection.getBalance(treasuryPda),
    };
  };

  it("funds and repays a USDC request against SOL collateral", async () => {
    const amount = 250_000;
    const interest = 25_000;
    const collateral = 1 * anchor.web3.LAMPORTS_PER_SOL;
    const ownerStartingUsdc = 50_000;

    const {
      owner,
      lender,
      vaultPda,
      treasuryPda,
      usdcMint,
      ownerUsdc,
      lenderUsdc,
      treasuryRentBalance,
    } = await setupVault();

    await mintTo(
      connection,
      payer,
      usdcMint,
      lenderUsdc.address,
      payer,
      1_000_000
    );
    await mintTo(
      connection,
      payer,
      usdcMint,
      ownerUsdc.address,
      payer,
      ownerStartingUsdc
    );

    await program.methods
      .openLoanRequest(
        new anchor.BN(amount),
        new anchor.BN(interest),
        new anchor.BN(collateral),
        new anchor.BN(3_600)
      )
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    let vault = await program.account.vaultState.fetch(vaultPda);
    expect(vault.status).to.equal(REQUESTED);
    expect(vault.amount.toNumber()).to.equal(amount);
    expect(vault.interest.toNumber()).to.equal(interest);
    expect(vault.collateral.toNumber()).to.equal(collateral);
    expect(await connection.getBalance(treasuryPda)).to.equal(
      treasuryRentBalance + collateral
    );

    await program.methods
      .fundLoan()
      .accounts({
        vault: vaultPda,
        lender: lender.publicKey,
        lenderUsdc: lenderUsdc.address,
        borrowerUsdc: ownerUsdc.address,
        usdcMint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([lender])
      .rpc();

    const borrowerBalanceAfterFunding = await getAccount(
      connection,
      ownerUsdc.address
    );
    expect(Number(borrowerBalanceAfterFunding.amount)).to.equal(
      ownerStartingUsdc + amount
    );

    await program.methods
      .repayLoan()
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        owner: owner.publicKey,
        ownerUsdc: ownerUsdc.address,
        lenderUsdc: lenderUsdc.address,
        usdcMint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([owner])
      .rpc();

    vault = await program.account.vaultState.fetch(vaultPda);
    expect(vault.status).to.equal(REPAID);
    expect(vault.amount.toNumber()).to.equal(0);
    expect(vault.lender.toBase58()).to.equal(
      anchor.web3.PublicKey.default.toBase58()
    );
    expect(await connection.getBalance(treasuryPda)).to.equal(
      treasuryRentBalance
    );

    const borrowerBalanceAfterRepayment = await getAccount(
      connection,
      ownerUsdc.address
    );
    const lenderBalanceAfterRepayment = await getAccount(
      connection,
      lenderUsdc.address
    );

    expect(Number(borrowerBalanceAfterRepayment.amount)).to.equal(25_000);
    expect(Number(lenderBalanceAfterRepayment.amount)).to.equal(1_025_000);
  });

  it("lets the lender liquidate collateral after expiry", async () => {
    const amount = 120_000;
    const collateral = 500_000_000;

    const {
      owner,
      lender,
      vaultPda,
      treasuryPda,
      usdcMint,
      ownerUsdc,
      lenderUsdc,
      treasuryRentBalance,
    } = await setupVault();

    await mintTo(
      connection,
      payer,
      usdcMint,
      lenderUsdc.address,
      payer,
      amount
    );

    await program.methods
      .openLoanRequest(
        new anchor.BN(amount),
        new anchor.BN(12_000),
        new anchor.BN(collateral),
        new anchor.BN(1)
      )
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    await program.methods
      .fundLoan()
      .accounts({
        vault: vaultPda,
        lender: lender.publicKey,
        lenderUsdc: lenderUsdc.address,
        borrowerUsdc: ownerUsdc.address,
        usdcMint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([lender])
      .rpc();

    await sleep(3_000);

    const lenderLamportsBefore = await connection.getBalance(lender.publicKey);

    await program.methods
      .liquidateLoan()
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        lender: lender.publicKey,
      })
      .signers([lender])
      .rpc();

    const vault = await program.account.vaultState.fetch(vaultPda);
    expect(vault.status).to.equal(LIQUIDATED);
    expect(vault.collateral.toNumber()).to.equal(0);
    expect(await connection.getBalance(treasuryPda)).to.equal(
      treasuryRentBalance
    );

    const lenderLamportsAfter = await connection.getBalance(lender.publicKey);
    expect(lenderLamportsAfter - lenderLamportsBefore).to.equal(collateral);
  });

  it("lets lenders post counter offers that borrowers can accept", async () => {
    const requestedAmount = 250_000;
    const requestedInterest = 20_000;
    const requestedCollateral = 1 * anchor.web3.LAMPORTS_PER_SOL;
    const counterAmount = 225_000;
    const counterInterest = 35_000;
    const counterCollateral = 750_000_000;
    const counterDurationSeconds = 1_800;

    const {
      owner,
      lender,
      vaultPda,
      treasuryPda,
      usdcMint,
      ownerUsdc,
      lenderUsdc,
    } = await setupVault();
    const otherLender = anchor.web3.Keypair.generate();
    await airdrop(otherLender.publicKey, 3 * anchor.web3.LAMPORTS_PER_SOL);
    const otherLenderUsdc = await getOrCreateAssociatedTokenAccount(
      connection,
      payer,
      usdcMint,
      otherLender.publicKey
    );

    await mintTo(
      connection,
      payer,
      usdcMint,
      lenderUsdc.address,
      payer,
      1_000_000
    );
    await mintTo(
      connection,
      payer,
      usdcMint,
      otherLenderUsdc.address,
      payer,
      1_000_000
    );

    await program.methods
      .openLoanRequest(
        new anchor.BN(requestedAmount),
        new anchor.BN(requestedInterest),
        new anchor.BN(requestedCollateral),
        new anchor.BN(3_600)
      )
      .accounts({
        vault: vaultPda,
        treasury: treasuryPda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    const counterOfferPda = findCounterOfferPda(vaultPda, lender.publicKey);

    await program.methods
      .postCounterOffer(
        new anchor.BN(counterAmount),
        new anchor.BN(counterInterest),
        new anchor.BN(counterCollateral),
        new anchor.BN(counterDurationSeconds)
      )
      .accounts({
        vault: vaultPda,
        counterOffer: counterOfferPda,
        lender: lender.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([lender])
      .rpc();

    const counterOffer = await program.account.counterOffer.fetch(
      counterOfferPda
    );
    expect(counterOffer.lender.toBase58()).to.equal(
      lender.publicKey.toBase58()
    );
    expect(counterOffer.amount.toNumber()).to.equal(counterAmount);
    expect(counterOffer.interest.toNumber()).to.equal(counterInterest);
    expect(counterOffer.collateral.toNumber()).to.equal(counterCollateral);
    expect(counterOffer.durationSeconds.toNumber()).to.equal(
      counterDurationSeconds
    );

    await program.methods
      .acceptCounterOffer()
      .accounts({
        vault: vaultPda,
        counterOffer: counterOfferPda,
        owner: owner.publicKey,
      })
      .signers([owner])
      .rpc();

    const counteredVault = await program.account.vaultState.fetch(vaultPda);
    expect(counteredVault.status).to.equal(COUNTERED);
    expect(counteredVault.lender.toBase58()).to.equal(
      lender.publicKey.toBase58()
    );
    expect(counteredVault.amount.toNumber()).to.equal(counterAmount);
    expect(counteredVault.interest.toNumber()).to.equal(counterInterest);
    expect(counteredVault.collateral.toNumber()).to.equal(counterCollateral);
    expect(counteredVault.durationSeconds.toNumber()).to.equal(
      counterDurationSeconds
    );

    let unauthorizedFundingError: unknown;
    try {
      await program.methods
        .fundLoan()
        .accounts({
          vault: vaultPda,
          lender: otherLender.publicKey,
          lenderUsdc: otherLenderUsdc.address,
          borrowerUsdc: ownerUsdc.address,
          usdcMint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([otherLender])
        .rpc();
    } catch (error) {
      unauthorizedFundingError = error;
    }

    expect(String(unauthorizedFundingError)).to.include(
      "Only the accepted counter-offer lender can fund this loan"
    );

    await program.methods
      .fundLoan()
      .accounts({
        vault: vaultPda,
        lender: lender.publicKey,
        lenderUsdc: lenderUsdc.address,
        borrowerUsdc: ownerUsdc.address,
        usdcMint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([lender])
      .rpc();

    const fundedVault = await program.account.vaultState.fetch(vaultPda);
    expect(fundedVault.status).to.equal(FUNDED);
    expect(fundedVault.lender.toBase58()).to.equal(lender.publicKey.toBase58());

    const borrowerBalanceAfterFunding = await getAccount(
      connection,
      ownerUsdc.address
    );
    expect(Number(borrowerBalanceAfterFunding.amount)).to.equal(counterAmount);
  });
});
