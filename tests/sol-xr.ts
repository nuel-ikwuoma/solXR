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
import {min} from "bn.js";

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


    it("Is initialized!", async () => {
        // Find the mint authority PDA
        const [mintAuthorityPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("solxr")],
            program.programId
        );

        const mintPDA = new Keypair();

        // Find the metadata PDA
        const [metadataPDA] = PublicKey.findProgramAddressSync(
            [
                Buffer.from("metadata"),
                METADATA_PROGRAM_ID.toBuffer(),
                mintPDA.publicKey.toBuffer()
            ],
            METADATA_PROGRAM_ID
        );

        console.log({metadataPDA, mintPDA: mintPDA.publicKey, mintAuthorityPDA})


        const sig = await program
            .methods.initialize()
            .accounts({
                payer: payer.publicKey,
                mint: mintPDA.publicKey,
                // @ts-ignore
                mintAuthority: mintAuthorityPDA,
                metadata: metadataPDA,
                tokenMetadataProgram: METADATA_PROGRAM_ID,
                tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .signers([payer.payer, mintPDA])
            .rpc();

        console.log('Success!');
        console.log(`   Mint Address: ${mintPDA.publicKey}`);
        console.log(`   Transaction Signature: ${sig}`);
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
