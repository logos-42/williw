import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { expect } from "chai";
import { DecentralizedTrainingContract } from "../target/types/decentralized_training_contract";

describe("decentralized-training-contract", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.DecentralizedTrainingContract as Program<DecentralizedTrainingContract>;
  const provider = anchor.AnchorProvider.env();
  const admin = provider.wallet;

  let globalStatePda: PublicKey;
  let globalStateBump: number;

  before(async () => {
    [globalStatePda, globalStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("global-state")],
      program.programId
    );
  });

  it("Initialize contract", async () => {
    const treasury = anchor.web3.Keypair.generate();

    // Airdrop to admin
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(admin.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL)
    );

    // Airdrop to treasury
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(treasury.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL)
    );

    const tx = await program.methods
      .initialize()
      .accounts({
        globalState: globalStatePda,
        admin: admin.publicKey,
        treasury: treasury.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize transaction signature", tx);

    // Verify global state
    const globalState = await program.account.globalState.fetch(globalStatePda);
    expect(globalState.admin.toString()).to.equal(admin.publicKey.toString());
    expect(globalState.treasury.toString()).to.equal(treasury.publicKey.toString());
    expect(globalState.totalNodes).to.equal(0);
    expect(globalState.totalContributions).to.equal(0);
    expect(globalState.baseRewardPerCompute.toString()).to.equal("1000000");
  });

  it("Register node", async () => {
    const nodeId = anchor.web3.Keypair.generate().publicKey;
    const owner = anchor.web3.Keypair.generate();

    // Airdrop to owner
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(owner.publicKey, anchor.web3.LAMPORTS_PER_SOL)
    );

    const [nodeAccountPda, nodeAccountBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("node"), nodeId.toBytes()],
      program.programId
    );

    const tx = await program.methods
      .registerNode(nodeId, "Test Node", "Desktop")
      .accounts({
        nodeAccount: nodeAccountPda,
        globalState: globalStatePda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    console.log("Register node transaction signature", tx);

    // Verify node account
    const nodeAccount = await program.account.nodeAccount.fetch(nodeAccountPda);
    expect(nodeAccount.nodeId.toString()).to.equal(nodeId.toString());
    expect(nodeAccount.owner.toString()).to.equal(owner.publicKey.toString());
    expect(nodeAccount.name).to.equal("Test Node");
    expect(nodeAccount.deviceType).to.equal("Desktop");
    expect(nodeAccount.status).to.deep.equal({ active: {} });
  });

  it("Record contribution", async () => {
    const nodeId = anchor.web3.Keypair.generate().publicKey;
    const owner = anchor.web3.Keypair.generate();
    const contributionId = "test-contribution-123";

    // Airdrop to owner
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(owner.publicKey, anchor.web3.LAMPORTS_PER_SOL)
    );

    // First register the node
    const [nodeAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("node"), nodeId.toBytes()],
      program.programId
    );

    await program.methods
      .registerNode(nodeId, "Test Node", "Desktop")
      .accounts({
        nodeAccount: nodeAccountPda,
        globalState: globalStatePda,
        owner: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    // Record contribution
    const [contributionAccountPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("contribution"), Buffer.from(contributionId)],
      program.programId
    );

    const startTime = Math.floor(Date.now() / 1000) - 3600; // 1 hour ago
    const endTime = Math.floor(Date.now() / 1000);

    const tx = await program.methods
      .recordContribution(
        contributionId,
        "task-123",
        new anchor.BN(startTime),
        new anchor.BN(endTime),
        new anchor.BN(3600), // 1 hour
        75.5, // GPU usage
        1024, // GPU memory
        45.2, // CPU usage
        2048, // Memory
        100, // Upload
        200, // Download
        10000, // Samples
        50, // Batches
        2.5 // Compute score
      )
      .accounts({
        contributionAccount: contributionAccountPda,
        nodeAccount: nodeAccountPda,
        globalState: globalStatePda,
        authority: owner.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    console.log("Record contribution transaction signature", tx);

    // Verify contribution account
    const contributionAccount = await program.account.contributionAccount.fetch(contributionAccountPda);
    expect(contributionAccount.id).to.equal(contributionId);
    expect(contributionAccount.nodeId.toString()).to.equal(nodeId.toString());
    expect(contributionAccount.computeScore).to.equal(2.5);
  });

  it("Create multisig", async () => {
    const creator = anchor.web3.Keypair.generate();
    const owner1 = anchor.web3.Keypair.generate();
    const owner2 = anchor.web3.Keypair.generate();

    // Airdrop to creator
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(creator.publicKey, anchor.web3.LAMPORTS_PER_SOL)
    );

    const [multisigPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("multisig"), creator.publicKey.toBytes()],
      program.programId
    );

    const owners = [creator.publicKey, owner1.publicKey, owner2.publicKey];
    const threshold = 2;

    const tx = await program.methods
      .createMultisig(owners, new anchor.BN(threshold))
      .accounts({
        multisigAccount: multisigPda,
        creator: creator.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    console.log("Create multisig transaction signature", tx);

    // Verify multisig account
    const multisigAccount = await program.account.multisigAccount.fetch(multisigPda);
    expect(multisigAccount.owners.length).to.equal(3);
    expect(multisigAccount.threshold.toString()).to.equal(threshold.toString());
  });
});