import * as web3 from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import type { RoninRewards } from "../target/types/ronin_rewards";

describe("ronin_rewards - Mainnet Admin Shift", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.RoninRewards as anchor.Program<RoninRewards>;
  
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.RoninRewards;
  const admin = provider.wallet;

  // ============================================================
  // MAINNET CONSTANTS
  // ============================================================

  const EXPECTED_PROGRAM_ID = new PublicKey(
    "6Uyjo8oDGQJeb8zS1yFqwLCguc4gfUB1V4xWheAD7RYC"
  );

  // CURRENT on-chain admin
  const CURRENT_ADMIN = new PublicKey(
    "jcJnPd1i1VzaTy4gR4LrKcMyZSgKmC8vy5n5fLo7EHv"
  );

  // ============================================================
  // NEW ADMIN
  // ============================================================
  //
  // IMPORTANT:
  // Replace the value below with the PUBLIC ADDRESS
  // you got from get-pubkey.js.
  //
  // DO NOT put a recovery phrase or secret-key array here.
  //
  // ============================================================

  const NEW_ADMIN = new PublicKey(
    "Askr5PWLAm1ukcoHE8nHQh9MEdxD4RtJ1f1qGQD7U9Wn"
  );

  // ============================================================
  // REWARD CONFIG PDA
  // ============================================================

  const [rewardConfig] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("reward_config")],
    program.programId
  );

  // ============================================================
  // DISPLAY SETUP
  // ============================================================

  before(async () => {
    console.log("");
    console.log("=================================");
    console.log("RONIN REWARDS - MAINNET");
    console.log("ADMIN SHIFT");
    console.log("=================================");

    console.log("Program:", program.programId.toBase58());

    console.log("Connected wallet:", admin.publicKey.toBase58());

    console.log("Expected current admin:", CURRENT_ADMIN.toBase58());

    console.log("New admin:", NEW_ADMIN.toBase58());

    console.log("Reward Config:", rewardConfig.toBase58());

    console.log("=================================");
    console.log("");
  });

  // ============================================================
  // ADMIN SHIFT
  // ============================================================

  it("shifts Mainnet admin to the new wallet", async () => {
    // ----------------------------------------------------------
    // 1. VERIFY PROGRAM ID
    // ----------------------------------------------------------

    if (!program.programId.equals(EXPECTED_PROGRAM_ID)) {
      throw new Error(
        [
          "WRONG PROGRAM ID!",
          `Expected: ${EXPECTED_PROGRAM_ID.toBase58()}`,
          `Actual:   ${program.programId.toBase58()}`,
        ].join("\n")
      );
    }

    console.log("✅ Correct Mainnet program ID");

    // ----------------------------------------------------------
    // 2. VERIFY CONNECTED WALLET
    // ----------------------------------------------------------
    //
    // The CURRENT admin must sign this transaction.
    //
    // The NEW admin does NOT need to sign.
    //
    // ----------------------------------------------------------

    if (!admin.publicKey.equals(CURRENT_ADMIN)) {
      throw new Error(
        [
          "CONNECTED WALLET IS NOT THE CURRENT ADMIN!",
          `Expected: ${CURRENT_ADMIN.toBase58()}`,
          `Connected: ${admin.publicKey.toBase58()}`,
        ].join("\n")
      );
    }

    console.log("✅ Connected wallet is current admin");

    // ----------------------------------------------------------
    // 3. FETCH ON-CHAIN REWARD CONFIG
    // ----------------------------------------------------------

    const configBefore = await program.account.rewardConfig.fetch(rewardConfig);

    console.log("");
    console.log("ON-CHAIN CONFIG BEFORE SHIFT");
    console.log("---------------------------------");

    console.log("On-chain admin:", configBefore.admin.toBase58());

    console.log("Paused:", configBefore.paused);

    console.log("Total claims:", configBefore.totalClaims.toString());

    console.log("Total claimed:", configBefore.totalClaimed.toString());

    // ----------------------------------------------------------
    // 4. VERIFY ON-CHAIN ADMIN
    // ----------------------------------------------------------

    if (!configBefore.admin.equals(CURRENT_ADMIN)) {
      throw new Error(
        [
          "ON-CHAIN ADMIN DOES NOT MATCH EXPECTED CURRENT ADMIN!",
          `Expected: ${CURRENT_ADMIN.toBase58()}`,
          `On-chain: ${configBefore.admin.toBase58()}`,
        ].join("\n")
      );
    }

    console.log("✅ On-chain admin verified");

    // ----------------------------------------------------------
    // 5. VERIFY NEW ADMIN IS DIFFERENT
    // ----------------------------------------------------------

    if (NEW_ADMIN.equals(CURRENT_ADMIN)) {
      throw new Error("NEW_ADMIN is the same as CURRENT_ADMIN.");
    }

    console.log("✅ New admin address is different");

    // ----------------------------------------------------------
    // 6. SHOW WHAT WILL HAPPEN
    // ----------------------------------------------------------

    console.log("");
    console.log("=================================");
    console.log("ADMIN ROTATION");
    console.log("=================================");

    console.log("OLD ADMIN:", CURRENT_ADMIN.toBase58());

    console.log("NEW ADMIN:", NEW_ADMIN.toBase58());

    console.log("");
    console.log("Sending updateAdmin transaction...");

    // ----------------------------------------------------------
    // 7. EXECUTE ADMIN SHIFT
    // ----------------------------------------------------------

    const tx = await program.methods
      .updateAdmin(NEW_ADMIN)
      .accounts({
        rewardConfig,
        admin: admin.publicKey,
      })
      .rpc();

    // ----------------------------------------------------------
    // 8. TRANSACTION RESULT
    // ----------------------------------------------------------

    console.log("");
    console.log("=================================");
    console.log("TRANSACTION SENT");
    console.log("=================================");

    console.log("Transaction signature:");
    console.log(tx);

    console.log("");
    console.log("Solscan:");
    console.log(`https://solscan.io/tx/${tx}`);

    // ----------------------------------------------------------
    // 9. FETCH CONFIG AGAIN
    // ----------------------------------------------------------

    const configAfter = await program.account.rewardConfig.fetch(rewardConfig);

    console.log("");
    console.log("ON-CHAIN CONFIG AFTER SHIFT");
    console.log("---------------------------------");

    console.log("New on-chain admin:", configAfter.admin.toBase58());

    // ----------------------------------------------------------
    // 10. VERIFY ADMIN WAS ACTUALLY CHANGED
    // ----------------------------------------------------------

    if (!configAfter.admin.equals(NEW_ADMIN)) {
      throw new Error(
        [
          "ADMIN SHIFT FAILED!",
          `Expected new admin: ${NEW_ADMIN.toBase58()}`,
          `Actual on-chain admin: ${configAfter.admin.toBase58()}`,
        ].join("\n")
      );
    }

    // ----------------------------------------------------------
    // 11. SUCCESS
    // ----------------------------------------------------------

    console.log("");
    console.log("=================================");
    console.log("✅ ADMIN SHIFT SUCCESSFUL");
    console.log("=================================");

    console.log("Previous admin:", CURRENT_ADMIN.toBase58());

    console.log("New admin:", configAfter.admin.toBase58());

    console.log("Transaction:", tx);

    console.log("=================================");
    console.log("");
  });
});
