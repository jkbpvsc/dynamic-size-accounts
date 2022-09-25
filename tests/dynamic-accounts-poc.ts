import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import {
  SystemProgram,
  Keypair,
} from "@solana/web3.js";
import { DynamicAccountsPoc } from "../target/types/dynamic_accounts_poc";

describe("dynamic-accounts-poc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace
    .DynamicAccountsPoc as Program<DynamicAccountsPoc>;

  const stateKey = Keypair.generate();
  const provider = anchor.getProvider();
  const initialSize = 4;

  let keys = [
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
    Keypair.generate().publicKey,
  ];

  it("Is initialized!", async () => {
    // Add your test here.
    let createAccountIx = SystemProgram.createAccount({
      programId: program.programId,
      fromPubkey: provider.publicKey,
      space: initialSize,
      lamports: await anchor
        .getProvider()
        .connection.getMinimumBalanceForRentExemption(initialSize),
      newAccountPubkey: stateKey.publicKey,
    });

    const tx = await program.methods
      .initialize()
      .accounts({ state: stateKey.publicKey })
      .signers([stateKey])
      .preInstructions([createAccountIx])
      .rpc({ skipPreflight: true });

    console.log("Your transaction signature", tx);
  });

  it("Should add keys", async () => {
    await Promise.all(
      keys.map(async (key) => {
        await program.methods
          .update(true, key)
          .accounts({
            signer: provider.publicKey,
            systemProgram: SystemProgram.programId,
            state: stateKey.publicKey,
          })
          .rpc();
      })
    );
  });

  it("Should remove keys", async () => {
    await Promise.all(
      keys.map(async (key) => {
        await program.methods
          .update(false, key)
          .accounts({
            signer: provider.publicKey,
            systemProgram: SystemProgram.programId,
            state: stateKey.publicKey,
          })
          .rpc();
      })
    );
  });
});
