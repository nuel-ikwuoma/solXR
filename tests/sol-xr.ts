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
import {expect,} from "chai";
import {getAssociatedTokenAddressSync, getAccount, getMint} from '@solana/spl-token'

const IDL = require('../target/idl/sol_xr.json');
const PROGRAM_ID = new PublicKey(IDL.address);
const METADATA_PROGRAM_ID = new PublicKey('metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s');

describe("sol-xr", async () => {
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
    const individualAddressCap = 100 * LAMPORTS_PER_SOL;

    // Generate a new keypair for the payer
    const dev = Keypair.generate();
    await fundAccount(dev, 5000)


    // Find the mint authority PDA
    const [solStrategyPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("sol_strategy")],
        program.programId
    );

    // Find the mint PDA
    const [mintPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("mint")],
        program.programId
    );


    async function initialize(payer: Keypair, initialPoolCap: number, individualAddressCap: number) {
        await program
            .methods.initialize(new anchor.BN(initialPoolCap), new anchor.BN(individualAddressCap))
            .accounts({
                payer: payer.publicKey,
            })
            .signers([payer])
            .rpc();
    }

    await it("should initialize program", async () => {
        await initialize(dev, initialPoolCap, individualAddressCap);
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        expect(solStrategy.initialPoolCap.toNumber()).equal(initialPoolCap, "initial pool cap is wrong")
        expect(solStrategy.individualAddressCap.toNumber()).equal(individualAddressCap, "initial pool cap is wrong")
    })

    await it('should not initialize program again', async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await initialize(badActor, initialPoolCap, individualAddressCap)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
        }
    });

    await it("should test invest individual address cap", async () => {
        const testCases = [
            {
                desc: "amount is above cap",
                params: {amount: new anchor.BN(individualAddressCap + 1)},
                expectedValue: 0,
                shouldSucceed: false
            },
            {
                desc: "amount is valid 1",
                params: {amount: new anchor.BN(0.8 * individualAddressCap)},
                expectedValue: 0.8 * individualAddressCap,
                shouldSucceed: true
            },
            {
                desc: "amount is valid 2",
                params: {amount: new anchor.BN(0.2 * individualAddressCap)},
                expectedValue: individualAddressCap,
                shouldSucceed: true
            },
            {
                desc: "address balance is at cap",
                params: {amount: new anchor.BN(LAMPORTS_PER_SOL)},
                expectedValue: 0,
                shouldSucceed: false
            },
        ]

        const investor = Keypair.generate();
        await fundAccount(investor, 5000)
        for (const {desc, params, expectedValue, shouldSucceed} of testCases) {
            console.log(`When ${desc}`)
            if (shouldSucceed) {
                await program.methods.invest(params.amount)
                    .accounts({
                        investor: investor.publicKey,
                    })
                    .signers([investor])
                    .rpc();

                const solStrategy = await program.provider.connection.getAccountInfo(solStrategyPDA);
                const dataSize = solStrategy.data.length;
                const rentExemptionAmount = await program.provider.connection.getMinimumBalanceForRentExemption(dataSize);

                const mintInfo = await getMint(program.provider.connection, mintPDA);

                expect(solStrategy.lamports).equal(expectedValue + rentExemptionAmount, "current sol balance is wrong")
                expect(Number(mintInfo.supply)).equal(expectedValue, "current solxr balance is wrong")
            } else {
                try {
                    await program.methods.invest(params.amount)
                        .accounts({
                            investor: investor.publicKey,
                        })
                        .signers([investor])
                        .rpc();

                    expect.fail("Expected an error but the instruction succeeded");
                } catch (error: any) {
                    let msg = error.message as string
                    expect(msg.includes('AnchorError')).true
                    expect(msg.includes('Error Code: ATACapError')).true
                    expect(msg.includes('Error Number: 6000')).true
                    expect(msg.includes('Error Message: The amount would cause the ATA balance to exceed the individual address cap.')).true
                }
            }
        }
    });

    await it("should max out the initial pool", async () => {
        const list = new Array((initialPoolCap / individualAddressCap) - 1)

        const solStrategy = await program.provider.connection.getAccountInfo(solStrategyPDA);
        const mintInfo = await getMint(program.provider.connection, mintPDA);

        let prev_strategy_lamport = solStrategy.lamports;
        let prev_mint_supply = mintInfo.supply;
        for (const _ of list) {
            let investor = Keypair.generate()
            await fundAccount(investor, 500)
            await program.methods.invest(new anchor.BN(individualAddressCap))
                .accounts({
                    investor: investor.publicKey,
                })
                .signers([investor])
                .rpc();

            const solStrategy = await program.provider.connection.getAccountInfo(solStrategyPDA);
            const mintInfo = await getMint(program.provider.connection, mintPDA);

            expect(solStrategy.lamports).equal(prev_strategy_lamport + individualAddressCap, "current sol balance is wrong")
            expect(Number(mintInfo.supply)).equal(Number(prev_mint_supply) + individualAddressCap, "current solxr balance is wrong")

            prev_mint_supply += BigInt(individualAddressCap)
            prev_strategy_lamport += individualAddressCap
        }
    });

    await it("should fail address trying to inves", async () => {
        try {
            let lateInvestor = Keypair.generate()
            await fundAccount(lateInvestor, 500)
            await program.methods.invest(new anchor.BN(individualAddressCap))
                .accounts({
                    investor: lateInvestor.publicKey,
                })
                .signers([lateInvestor])
                .rpc();
        } catch (error) {
            let msg = error.message
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: InitialSolCapError')).true
            expect(msg.includes('Error Number: 6001')).true
            expect(msg.includes('Error Message: The amount would cause the program PDA to exceed the initial pool cap.')).true
        }
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
})
;