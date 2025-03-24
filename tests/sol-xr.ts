import {describe, it} from 'node:test';
import * as anchor from '@coral-xyz/anchor';
import {
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import {BankrunProvider} from 'anchor-bankrun';
import {startAnchor} from 'solana-bankrun';
import {SolXr} from "../target/types/sol_xr";
import {expect} from "chai";

const IDL = require('../target/idl/sol_xr.json');
const PROGRAM_ID = new PublicKey(IDL.address);
const METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

describe("sol-xr", async () => {
    // Configure the client to use the local cluster.
    const context = await startAnchor(
        "",
        [
            {name: 'sol_xr', programId: PROGRAM_ID},
            {name: 'token_metadata', programId: METADATA_PROGRAM_ID},
        ],
        [],
    );
    const provider = new BankrunProvider(context);
    const payer = provider.wallet as anchor.Wallet;
    const program = new anchor.Program<SolXr>(IDL, provider);

    const initialPoolCap = 10_000 * LAMPORTS_PER_SOL;

    it("Is initialized!", async () => {
        // Generate a new keypair for the payer
        const dev = Keypair.generate();

        await fundAccount(dev, 500)

        await program
            .methods.initialize(new anchor.BN(initialPoolCap))
            .accounts({
                payer: dev.publicKey,
            })
            .signers([dev])
            .rpc();

        // Find the mint PDA
        const [mintPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("mint")],
            program.programId
        );


        console.log("Mint Address: ", mintPDA)

        // Find the mint authority PDA
        const [solStrategyPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("sol_strategy")],
            program.programId
        );

        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

        expect(solStrategy.initialPoolCap.toNumber()).equal(10_000 * LAMPORTS_PER_SOL, "initial pool cap is wrong")
        expect(solStrategy.currentSolBalance.toNumber()).equal(0, "current sol balance should be zero")
        expect(solStrategy.currentSolxrBalance.toNumber()).equal(0, "current solxr balance should be zero")

    });

    async function fundAccount(keyPair: Keypair, amount: number) {
        const instruction = SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: keyPair.publicKey,
            lamports: amount * LAMPORTS_PER_SOL,
        });
        const transaction = new Transaction().add(instruction);
        await provider.sendAndConfirm(transaction, [payer.payer]);
    }
});
