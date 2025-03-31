import {describe, it} from 'node:test';
import * as anchor from '@coral-xyz/anchor';
import {
    Keypair,
    LAMPORTS_PER_SOL,
    PublicKey,
    SystemProgram,
    Transaction,
    Connection
} from '@solana/web3.js';
import {BankrunProvider} from 'anchor-bankrun';
import {startAnchor} from 'solana-bankrun';
import {SolXr} from "../target/types/sol_xr";
import {expect,} from "chai";
import devKey from '../dev.json'

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
    const connection = new Connection("http://localhost:8899");

    const provider = new BankrunProvider(context);
    const providerKeypair = provider.wallet as anchor.Wallet;
    const program = new anchor.Program<SolXr>(IDL, provider);

    const initialPoolCap = 10_000 * LAMPORTS_PER_SOL;
    const individualAddressCap = 100 * LAMPORTS_PER_SOL;
    const bondPrice = LAMPORTS_PER_SOL;

    // Generate a new keypair for the governance_authority
    const dev = Keypair.fromSecretKey(new Uint8Array(devKey));
    await fundAccount(dev, 50000)


    // Find the mint authority PDA
    const [solStrategyPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("sol_strategy")],
        program.programId
    );

    // Find the mint PDA
    const [tokenPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("token")],
        program.programId
    );


    async function initializeToken(governance_authority: Keypair, initialPoolCap: number, individualAddressCap: number) {
        await program
            .methods.initializeToken(new anchor.BN(initialPoolCap), new anchor.BN(individualAddressCap))
            .accounts({
                governanceAuthority: governance_authority.publicKey,
            })
            .signers([governance_authority])
            .rpc();
    }

    async function initializeNFT(governance_authority: Keypair, bondPrice: number) {
        await program
            .methods.initializeNft(new anchor.BN(bondPrice))
            .accounts({
                governanceAuthority: governance_authority.publicKey,
            })
            .signers([governance_authority])
            .rpc();
    }


    await it("should fail to initialize token", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await initializeToken(badActor, initialPoolCap, individualAddressCap)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UNAUTHORIZED')).true
            expect(msg.includes('Error Number: 6000')).true
            expect(msg.includes('Error Message: The account that calls this function must match the token initializer.')).true
        }
    })

    await it("should initialize token", async () => {
        console.log(dev.publicKey)
        await initializeToken(dev, initialPoolCap, individualAddressCap);
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        expect(solStrategy.initialPoolCap.toNumber()).equal(initialPoolCap, "initial pool cap is wrong")
        expect(solStrategy.individualAddressCap.toNumber()).equal(individualAddressCap, "initial pool cap is wrong")
        expect(solStrategy.bondPrice.toNumber()).equal(LAMPORTS_PER_SOL, "bond price should be 1 sol")
        expect(solStrategy.solInTreasury.toNumber()).equal(0, "bond price should be zero")
    })

    await it('should not initialize token again', async () => {
        try {
            await initializeToken(dev, initialPoolCap, individualAddressCap)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
        }
    });

    await it("should fail to initialize nft", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await initializeNFT(badActor, bondPrice)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UNAUTHORIZED')).true
            expect(msg.includes('Error Number: 6000')).true
            expect(msg.includes('Error Message: The account that calls this function must match the nft initializer.')).true
        }
    })

    await it("should initialize nft", async () => {
        await initializeNFT(dev, bondPrice);
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        expect(solStrategy.initialPoolCap.toNumber()).equal(initialPoolCap, "initial pool cap is wrong")
        expect(solStrategy.individualAddressCap.toNumber()).equal(individualAddressCap, "initial pool cap is wrong")
        expect(solStrategy.bondPrice.toNumber()).equal(bondPrice, "bond price is wrong")
        expect(solStrategy.solInTreasury.toNumber()).equal(0, "bond price is wrong")
    })

    await it('should not initialize nft again', async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await initializeNFT(badActor, bondPrice)
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

                const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
                const tokenInfo = await getMint(program.provider.connection, tokenPDA);

                expect(solStrategy.solInTreasury.toNumber()).equal(expectedValue, "current sol balance is wrong")
                expect(Number(tokenInfo.supply)).equal(expectedValue, "current solxr balance is wrong")
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

        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        const tokenInfo = await getMint(program.provider.connection, tokenPDA);

        let prev_strategy_lamport = solStrategy.solInTreasury.toNumber();
        let prev_mint_supply = tokenInfo.supply;
        for (const _ of list) {
            let investor = Keypair.generate()
            await fundAccount(investor, 500)
            await program.methods.invest(new anchor.BN(individualAddressCap))
                .accounts({
                    investor: investor.publicKey,
                })
                .signers([investor])
                .rpc();

            const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
            const tokenInfo = await getMint(program.provider.connection, tokenPDA);

            expect(solStrategy.solInTreasury.toNumber()).equal(prev_strategy_lamport + individualAddressCap, "current sol balance is wrong")
            expect(Number(tokenInfo.supply)).equal(Number(prev_mint_supply) + individualAddressCap, "current solxr balance is wrong")

            prev_mint_supply += BigInt(individualAddressCap)
            prev_strategy_lamport += individualAddressCap
        }
    });

    await it("should fail address trying to invest", async () => {
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

    await it("should fail to open round for minting", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await program.methods.openMintRound(new anchor.BN(1), new anchor.BN(LAMPORTS_PER_SOL))
                .accounts({governanceAuthority: badActor.publicKey})
                .signers([badActor])
                .rpc();
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UNAUTHORIZED')).true
        }
    })

    await it("should opening round for minting", async () => {
        const testCases = [
            {
                desc: "round id is invalid",
                params: {
                    roundID: 2,
                    marketValue: 0
                },
                shouldSucceed: false,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 0,
                    errorCode: "InvalidRoundID",
                }
            },
            {
                desc: "market value is lower than threshold",
                params: {
                    roundID: 1,
                    marketValue: 1.49 *LAMPORTS_PER_SOL
                },
                shouldSucceed: false,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 0,
                    errorCode: "PremiumBelowThreshold",
                }
            },
            {
                desc: "market value lower than threshold",
                params: {
                    roundID: 1,
                    marketValue: 1.75 *LAMPORTS_PER_SOL
                },
                shouldSucceed: true,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 1538461538461,
                    errorCode: null,
                }
            },
            {
                desc: "round has already been opened",
                params: {
                    roundID: 1,
                    marketValue: 1.75 *LAMPORTS_PER_SOL
                },
                shouldSucceed: false,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 1538461538461,
                    errorCode: "ActiveRound",
                }
            }
        ]
        for (const {desc, params, shouldSucceed, expectedResults} of testCases) {
            console.log(`When ${desc}`)

            if (shouldSucceed) {
                await program.methods.openMintRound(new anchor.BN(params.roundID), new anchor.BN(params.marketValue))
                    .accounts({governanceAuthority: dev.publicKey})
                    .signers([dev])
                    .rpc();
                const idBuffer = Buffer.alloc(8);
                idBuffer.writeBigUInt64LE(BigInt(params.roundID));

                const [mintRoundPDA] = PublicKey.findProgramAddressSync(
                    [Buffer.from("mint_round"), idBuffer],
                    program.programId
                );


                const mintRound = await program.account.mintRound.fetch(mintRoundPDA)
                const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

                expect(solStrategy.allowNewMint).equal(true, "current sol balance is wrong")

                expect(solStrategy.allowNewMint).equal(true, "current sol balance is wrong")

                expect(mintRound.premium.toNumber()).equal(params.marketValue);
                expect(mintRound.maxMintPerWallet.toNumber()).equal(solStrategy.maxMintPerWallet.toNumber());
                expect(mintRound.solxrMinted.toNumber()).equal(expectedResults.solxrMinted);
                expect(mintRound.solxrAvailable.toNumber()).equal(expectedResults.solxrAvailable)
            } else {
                try {
                    await program.methods.openMintRound(new anchor.BN(params.roundID), new anchor.BN(params.marketValue))
                        .accounts({governanceAuthority: dev.publicKey})
                        .signers([dev])
                        .rpc();

                    expect.fail("Expected an error but the instruction succeeded");
                } catch (error: any) {
                    let msg = error.message as string
                    expect(msg.includes('AnchorError')).true
                    expect(msg.includes(`Error Code: ${expectedResults.errorCode}`)).true
                }
            }

        }
    })

    async function fundAccount(keyPair: Keypair, amount: number) {
        const instruction = SystemProgram.transfer({
            fromPubkey: providerKeypair.publicKey,
            toPubkey: keyPair.publicKey,
            lamports: amount * LAMPORTS_PER_SOL,
        });
        const transaction = new Transaction().add(instruction);
        await provider.sendAndConfirm(transaction, [providerKeypair.payer]);
    }
})
;