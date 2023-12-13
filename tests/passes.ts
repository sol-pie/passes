import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Passes, IDL } from "../target/types/passes";
import { assert } from "chai";
import * as spl from "@solana/spl-token";
// import NodeWallet from "@coral-xyz/anchor/dist/cjs/nodewallet";
import { createMint } from "@solana/spl-token";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";

describe("Passes", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const conn = anchor.getProvider().connection;
  const program = anchor.workspace.Passes as Program<Passes>;
  const admin = anchor.workspace.Passes.provider.wallet
    .payer as anchor.web3.Keypair;

  const owner = anchor.web3.Keypair.generate();
  const buyer = anchor.web3.Keypair.generate();

  let paymentMintKey: anchor.web3.PublicKey;
  let configKey: anchor.web3.PublicKey;
  let escrowTokenWalletKey: anchor.web3.PublicKey;
  let escrowSolWalletKey: anchor.web3.PublicKey;
  let protocolFeeWalletKey: anchor.web3.PublicKey;
  let passesSupplyKey: anchor.web3.PublicKey;
  let ownerFeeWalletKey: anchor.web3.PublicKey;

  const protocolFeePct = new anchor.BN(20000000);
  const ownerFeePct = new anchor.BN(20000000);

  before(async () => {
    // get test SOL
    let airdropSig = await conn.requestAirdrop(
      owner.publicKey,
      LAMPORTS_PER_SOL * 1
    );
    await conn.confirmTransaction(airdropSig);
    airdropSig = await conn.requestAirdrop(
      buyer.publicKey,
      LAMPORTS_PER_SOL * 1
    );
    await conn.confirmTransaction(airdropSig);

    // create mint account
    paymentMintKey = await createMint(
      anchor.getProvider().connection,
      admin,
      admin.publicKey,
      admin.publicKey,
      0
    );
  });

  beforeEach(async () => {
    // get pdas
    [configKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      program.programId
    );
    [escrowTokenWalletKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), paymentMintKey.toBuffer()],
      program.programId
    );
    [escrowSolWalletKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow")],
      program.programId
    );
    [passesSupplyKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("supply"), owner.publicKey.toBuffer()],
      program.programId
    );
    ownerFeeWalletKey = await spl.getAssociatedTokenAddress(
      paymentMintKey,
      owner.publicKey
    );
    protocolFeeWalletKey = await spl.getAssociatedTokenAddress(
      paymentMintKey,
      admin.publicKey
    );
  });

  it("Init", async () => {
    const txHash = await program.methods
      .init(protocolFeePct, ownerFeePct)
      .accounts({
        admin: admin.publicKey,
        config: configKey,
        escrowTokenWallet: escrowTokenWalletKey,
        escrowSolWallet: escrowSolWalletKey,
        protocolFeeWallet: protocolFeeWalletKey,
        paymentMint: paymentMintKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([admin])
      .rpc();
    await program.provider.connection.confirmTransaction(txHash);

    const configAccount = await program.account.config.fetch(configKey);
    assert(configAccount.protocolFeePct.eqn(20000000));
    assert(configAccount.ownerFeePct.eqn(20000000));
    assert(configAccount.escrowTokenWallet.equals(escrowTokenWalletKey));
    assert(configAccount.escrowSolWallet.equals(escrowSolWalletKey));
    assert(configAccount.protocolFeeTokenWallet.equals(protocolFeeWalletKey));
  });

  it("Issue Passes", async () => {
    const [passesBalanceKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("balance"),
        owner.publicKey.toBuffer(),
        owner.publicKey.toBuffer(),
      ],
      program.programId
    );
    const amount = new anchor.BN(1);
    const txHash = await program.methods
      .issuePasses(amount)
      .accounts({
        owner: owner.publicKey,
        passesSupply: passesSupplyKey,
        passesBalance: passesBalanceKey,
        config: configKey,
        ownerFeeWallet: ownerFeeWalletKey,
        paymentMint: paymentMintKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: spl.TOKEN_PROGRAM_ID,
        associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .signers([owner])
      .rpc();
    await program.provider.connection.confirmTransaction(txHash);

    const passesBalanceAccount = await program.account.passesBalance.fetch(
      passesBalanceKey
    );
    assert(passesBalanceAccount.amount.eqn(1));
  });

  it("Buy Passes With SOL", async () => {
    const [passesBalanceKey] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("balance"),
        owner.publicKey.toBuffer(),
        buyer.publicKey.toBuffer(),
      ],
      program.programId
    );
    const amount = new anchor.BN(10);
    const txHash = await program.methods
      .buyPassesSol(amount)
      .accounts({
        config: configKey,
        protocolFeeWallet: admin.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        passesSupply: passesSupplyKey,
        passesBalance: passesBalanceKey,
        buyer: buyer.publicKey,
        escrowWallet: escrowSolWalletKey,
        passesOwner: owner.publicKey,
      })
      .signers([buyer])
      .rpc();
    await program.provider.connection.confirmTransaction(txHash);

    const passesBalanceAccount = await program.account.passesBalance.fetch(
      passesBalanceKey
    );
    assert(passesBalanceAccount.amount.eqn(10));
  });
});
