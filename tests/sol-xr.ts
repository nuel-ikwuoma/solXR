import {describe, it} from 'node:test';
import * as anchor from '@coral-xyz/anchor';
import {Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram, Transaction} from '@solana/web3.js';
import {BankrunProvider} from 'anchor-bankrun';
import {Clock, startAnchor} from 'solana-bankrun';
import {SolXr} from "../target/types/sol_xr";
import {expect,} from "chai";
import devKey from '../dev.json'
import {getAccount, getAssociatedTokenAddress, getMint} from '@solana/spl-token'
import {
    deserializeEdition,
    deserializeMasterEdition,
    Edition,
    MasterEdition
} from '@metaplex-foundation/mpl-token-metadata';
import {lamports, PublicKey as MTPublicKey} from '@metaplex-foundation/umi';

const {ComputeBudgetProgram} = require('@solana/web3.js');

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
    const providerKeypair = provider.wallet as anchor.Wallet;
    const program = new anchor.Program<SolXr>(IDL, provider);

    const initialPoolCap = 10_000 * LAMPORTS_PER_SOL;
    const individualAddressCap = 100 * LAMPORTS_PER_SOL;
    const maxMintPerWallet = 10 * LAMPORTS_PER_SOL;

    // Generate a new keypair for the governance_authority
    const dev = Keypair.fromSecretKey(new Uint8Array(devKey));
    const platformDesignatedAccount = Keypair.fromSecretKey(new Uint8Array(devKey));
    await fundAccount(dev, 500000)

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

    await it("should fail to initialize token", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await initializeToken(badActor, initialPoolCap, individualAddressCap)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UnauthorizedGovernanceAuthority')).true
            expect(msg.includes('Error Number: 6000')).true
            expect(msg.includes('Error Message: The account that calls this function must match the token initializer.')).true
        }
    })

    await it("should initialize token", async () => {
        await initializeToken(dev, initialPoolCap, individualAddressCap);
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        expect(solStrategy.initialPoolCap.toNumber()).equal(initialPoolCap, "initial pool cap is wrong")
        expect(solStrategy.individualAddressCap.toNumber()).equal(individualAddressCap, "initial pool cap is wrong")
        expect(solStrategy.solInTreasury.toNumber()).equal(0, "bond price should be zero")
    })

    await it('should not initialize token again', async () => {
        try {
            await initializeToken(dev, initialPoolCap, individualAddressCap)
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
            expect.fail("Expected an error but the instruction succeeded");
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
            expect(msg.includes('Error Code: UnauthorizedGovernanceAuthority')).true
        }
    })

    await it("should open round for minting", async () => {
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
                    errorCode: "IncorrectRoundId",
                }
            },
            {
                desc: "market value is lower than threshold",
                params: {
                    roundID: 1,
                    marketValue: 1.49 * LAMPORTS_PER_SOL
                },
                shouldSucceed: false,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 0,
                    errorCode: "MarketValueBelowMinPremium",
                }
            },
            {
                desc: "round open successfully",
                params: {
                    roundID: 1,
                    marketValue: 1.75 * LAMPORTS_PER_SOL
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
                    marketValue: 1.85 * LAMPORTS_PER_SOL
                },
                shouldSucceed: false,
                expectedResults: {
                    solxrMinted: 0,
                    solxrAvailable: 1538461538461,
                    errorCode: "MintingAlreadyAllowed",
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
                    console.log(msg)
                    expect(msg.includes('AnchorError')).true
                    expect(msg.includes(`Error Code: ${expectedResults.errorCode}`)).true
                }
            }

        }
    })

    await it("should mint solxr for investor", async () => {
        const investor = Keypair.generate();
        await fundAccount(investor, 5000)

        const testCases = [
            {
                desc: "invalid round",
                params: {
                    roundID: 2,
                    amount: LAMPORTS_PER_SOL,
                    errorCode: "AccountNotInitialized"
                },
                expectedValue: 0,
                shouldSucceed: false
            },
            {
                desc: "amount is above available mint",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(2_692_307_692_309), // 2_692_307_692_309 sol in lamport is 1538461538461 solxr in lamport
                    errorCode: "ExceedsAvailableSolxr"
                },
                expectedValue: 0,
                shouldSucceed: false
            },
            {
                desc: "amount is above wallet cap",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(maxMintPerWallet + 1),
                    errorCode: "ExceedsMaxMintPerWallet"
                },
                expectedValue: 0,
                shouldSucceed: false
            },
            {
                desc: "amount is valid 1",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(0.8 * maxMintPerWallet),
                    errorCode: null
                },
                expectedValue: 4434285714,
                shouldSucceed: true
            },
            {
                desc: "amount is valid 2",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(0.2 * maxMintPerWallet),
                    errorCode: null
                },
                expectedValue: 5542857142, // 4434285714 + 1108571428
                shouldSucceed: true
            },
            {
                desc: "amount is above wallet cap",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(1),
                    errorCode: "ExceedsMaxMintPerWallet"
                },
                expectedValue: 5542857142,
                shouldSucceed: false
            },
            {
                desc: "duration should be over",
                params: {
                    roundID: 1,
                    amount: new anchor.BN(0),
                    errorCode: "MintingDurationEnded"
                },
                expectedValue: 5542857142,
                shouldSucceed: false,
                forwardTime: true,
            },
        ]
        for (const {desc, params, shouldSucceed, expectedValue, forwardTime} of testCases) {
            console.log(`When ${desc}`)

            if (shouldSucceed) {
                const idBuffer = Buffer.alloc(8);
                idBuffer.writeBigUInt64LE(BigInt(params.roundID));
                const [mintRoundPDA] = PublicKey.findProgramAddressSync(
                    [Buffer.from("mint_round"), idBuffer],
                    program.programId
                );
                await program.methods.buySolxr(new anchor.BN(params.roundID), new anchor.BN(params.amount))
                    .accounts({investor: investor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                    .signers([investor])
                    .rpc();
                const mintRound = await program.account.mintRound.fetch(mintRoundPDA)
                expect(mintRound.solxrMinted.toNumber()).equal(expectedValue);
            } else {
                try {
                    if (forwardTime) {
                        const idBuffer = Buffer.alloc(8);
                        idBuffer.writeBigUInt64LE(BigInt(params.roundID));
                        const [mintRoundPDA] = PublicKey.findProgramAddressSync(
                            [Buffer.from("mint_round"), idBuffer],
                            program.programId
                        );
                        await program.methods.buySolxr(new anchor.BN(params.roundID), new anchor.BN(params.amount))
                            .accounts({
                                investor: investor.publicKey,
                                platformAddress: platformDesignatedAccount.publicKey
                            })
                            .signers([investor])
                            .rpc();
                        const mintRound = await program.account.mintRound.fetch(mintRoundPDA)
                        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

                        let start = mintRound.start.toNumber();
                        let duration = solStrategy.mintDuration.toNumber();
                        const currentClock = await provider.context.banksClient.getClock();
                        provider.context.setClock(
                            new Clock(
                                currentClock.slot,
                                currentClock.epochStartTimestamp,
                                currentClock.epoch,
                                currentClock.leaderScheduleEpoch,
                                BigInt(start + duration + 1),
                            ),
                        );
                    }
                    await program.methods.buySolxr(new anchor.BN(params.roundID), new anchor.BN(params.amount))
                        .accounts({investor: investor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                        .signers([investor])
                        .rpc();

                    expect.fail("Expected an error but the instruction succeeded");
                } catch (error: any) {
                    let msg = error.message as string
                    console.log(msg, "->", params.errorCode)
                    expect(msg.includes('AnchorError')).true
                    expect(msg.includes(`Error Code: ${params.errorCode}`)).true
                }
            }

        }
    })

    await it("should max out available solxr to mint", async () => {
        const list = new Array(276);
        const roundID = 1
        const idBuffer = Buffer.alloc(8);
        idBuffer.writeBigUInt64LE(BigInt(roundID));
        const [mintRoundPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("mint_round"), idBuffer],
            program.programId
        );
        const mintRound = await program.account.mintRound.fetch(mintRoundPDA)
        // reverse time
        let start = mintRound.start.toNumber();
        const currentClock = await provider.context.banksClient.getClock();
        provider.context.setClock(
            new Clock(
                currentClock.slot,
                currentClock.epochStartTimestamp,
                currentClock.epoch,
                currentClock.leaderScheduleEpoch,
                BigInt(start),
            ),
        );

        let expectedValue = 5542857142;

        for (const _ of list) {
            let investor = Keypair.generate()
            await fundAccount(investor, 500)
            await program.methods.buySolxr(new anchor.BN(roundID), new anchor.BN(maxMintPerWallet))
                .accounts({investor: investor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                .signers([investor])
                .rpc();

            const mintRound = await program.account.mintRound.fetch(mintRoundPDA)

            expect(mintRound.solxrMinted.toNumber()).equal(expectedValue + 5542857142);

            expectedValue += 5542857142;
        }
    });

    await it("should fail address trying to invest", async () => {
        try {
            const roundID = 1
            let lateInvestor = Keypair.generate()
            await fundAccount(lateInvestor, 500)
            await program.methods.buySolxr(new anchor.BN(roundID), new anchor.BN(maxMintPerWallet))
                .accounts({investor: lateInvestor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                .signers([lateInvestor])
                .rpc();
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: ExceedsAvailableSolxr')).true
        }
    });

    async function closeMintingRound(governance_authority: Keypair) {
        await program
            .methods.closeMintRound()
            .accounts({
                governanceAuthority: governance_authority.publicKey,
            })
            .signers([governance_authority])
            .rpc();
    }

    await it("should fail to close minting round", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            await closeMintingRound(badActor)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UnauthorizedGovernanceAuthority')).true
        }
    })

    await it("should close minting round", async () => {
        await closeMintingRound(dev);
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        expect(solStrategy.allowNewMint).equal(false, "allow_new_mint should be false")
        expect(solStrategy.nextMintingRounds.toNumber()).equal(2, "next minting round id should be 2")
    })

    await it('should not close minting round when already closed', async () => {
        try {
            await closeMintingRound(dev)
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: MintingAlreadyClosed')).true
        }
    });

    await it('should fail address trying to invest after close', async () => {
        try {
            let lateInvestor = Keypair.generate()
            await fundAccount(lateInvestor, 500)

            await program.methods.buySolxr(new anchor.BN(1), new anchor.BN(maxMintPerWallet))
                .accounts({investor: lateInvestor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                .signers([lateInvestor])
                .rpc();
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: MintingNotAllowed')).true
        }
    });

    await it('should fail address trying to invest in old round', async () => {
        try {
            await program.methods.openMintRound(new anchor.BN(2), new anchor.BN(2 * LAMPORTS_PER_SOL))
                .accounts({governanceAuthority: dev.publicKey})
                .signers([dev])
                .rpc();

            let lateInvestor = Keypair.generate()
            await fundAccount(lateInvestor, 500)

            await program.methods.buySolxr(new anchor.BN(1), new anchor.BN(maxMintPerWallet))
                .accounts({investor: lateInvestor.publicKey, platformAddress: platformDesignatedAccount.publicKey})
                .signers([lateInvestor])
                .rpc();
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: InvalidMintingRound')).true
            await closeMintingRound(dev)
        }
    });

    await it("should fail to sell bond", async () => {
        try {
            const badActor = Keypair.generate();
            await fundAccount(badActor, 5000)

            let now = Date.now();
            const name = "Bond #1";
            const symbol = "B#1";
            const uri = "https://bafybeiauoz3l4ssofopdg36a4teo5at6paavgjzvyhcyr5e4bvk5fwqlpy.ipfs.w3s.link/metadata.json";
            const maturity = new anchor.BN(now + 3600); // 1 hour from now
            const strike_price = new anchor.BN(1_500_000_000);
            const supply = new anchor.BN(1);
            const price = new anchor.BN(10_000_000_000);
            const maxMintPerWallet = new anchor.BN(1);
            const startTime = new anchor.BN(now);
            const endTime = new anchor.BN(now + 600);
            await program.methods.sellBond(
                name,
                symbol,
                uri,
                maturity,
                strike_price,
                supply,
                price,
                maxMintPerWallet,
                startTime,
                endTime
            )
                .accounts({governanceAuthority: badActor.publicKey})
                .signers([badActor])
                .rpc();
            expect.fail("Expected an error but the instruction succeeded");
        } catch (error) {
            let msg = error.message as string
            console.log(msg)
            expect(msg.includes('AnchorError')).true
            expect(msg.includes('Error Code: UnauthorizedGovernanceAuthority')).true
        }
    })

    await it("should sell bond", async () => {
        let now = Date.now();
        const name = "Bond #1";
        const symbol = "B#1";
        const uri = "https://bafybeiauoz3l4ssofopdg36a4teo5at6paavgjzvyhcyr5e4bvk5fwqlpy.ipfs.w3s.link/metadata.json";
        const maturity = new anchor.BN(now + 3600); // 1 hour from now
        const strike_price = new anchor.BN(1.5 * LAMPORTS_PER_SOL);
        const supply = new anchor.BN(2);
        const price = new anchor.BN(10 * LAMPORTS_PER_SOL);
        const maxMintPerWallet = new anchor.BN(1);
        const startTime = new anchor.BN(now);
        const endTime = new anchor.BN(now + 600);
        await program.methods.sellBond(
            name,
            symbol,
            uri,
            maturity,
            strike_price,
            supply,
            price,
            maxMintPerWallet,
            startTime,
            endTime
        )
            .accounts({governanceAuthority: dev.publicKey})
            .signers([dev])
            .rpc();

        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)
        const idBuffer = Buffer.alloc(8);
        idBuffer.writeBigUInt64LE(BigInt(solStrategy.nextBondId.toNumber() - 1));
        const [bondPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("bond"), idBuffer],
            program.programId
        );
        const bond = await program.account.bond.fetch(bondPDA)
        idBuffer.writeBigUInt64LE(BigInt(bond.nextEditionNumber.toNumber()));
        const [bondNFTPDA] = PublicKey.findProgramAddressSync(
            [bondPDA.toBuffer()],
            program.programId
        );
        const bondNFT = await getMint(program.provider.connection, bondNFTPDA);

        const edition = await getMasterEdition(bondNFTPDA);

        expect(bond.maturity.toNumber()).equal(maturity.toNumber());
        expect(bond.strikePrice.toNumber()).equal(strike_price.toNumber());
        expect(bond.supply.toNumber()).equal(supply.toNumber());
        expect(bond.price.toNumber()).equal(price.toNumber());
        expect(bond.maxMintPerWallet.toNumber()).equal(maxMintPerWallet.toNumber());
        expect(bond.startTime.toNumber()).equal(startTime.toNumber());
        expect(bond.endTime.toNumber()).equal(endTime.toNumber());
        expect(bond.nextEditionNumber.toNumber()).equal(1);
        expect(bond.nextEditionNumber.toNumber()).equal(1);
        expect(Number(edition.supply)).equal(0);
        if (edition.maxSupply.__option === 'Some') {
            const maxSupplyValue = edition.maxSupply.value;
            expect(Number(maxSupplyValue)).equal(2);
        } else {
            expect.fail("The supply should be 2");
        }
    });

    // Key pairs to be used for further test
    const firstBuyer = Keypair.generate();
    await fundAccount(firstBuyer, 5000)
    const secondBuyer = Keypair.generate();
    await fundAccount(secondBuyer, 5000)
    const lateBuyer = Keypair.generate();
    await fundAccount(lateBuyer, 5000)

    await it("should mint bond nft", async () => {
            const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

            const idBuffer = Buffer.alloc(8);
            idBuffer.writeBigUInt64LE(BigInt(solStrategy.nextBondId.toNumber() - 1));
            const [bondPDA] = PublicKey.findProgramAddressSync(
                [Buffer.from("bond"), idBuffer],
                program.programId
            );
            const bond = await program.account.bond.fetch(bondPDA)
            const [bondNFTPDA] = PublicKey.findProgramAddressSync(
                [bondPDA.toBuffer()],
                program.programId
            );
            const [bondEditionPDA] = PublicKey.findProgramAddressSync(
                [
                    Buffer.from('metadata'),
                    METADATA_PROGRAM_ID.toBuffer(),
                    bondNFTPDA.toBuffer(),
                    Buffer.from('edition')
                ],
                METADATA_PROGRAM_ID
            );


            const testCases = [
                {
                    desc: "buying has not started",
                    buyer: firstBuyer,
                    params: {
                        errorCode: "MintingNotStarted"
                    },
                    expectedValue: {},
                    shouldSucceed: false,
                    time: bond.startTime.toNumber() - 3600,
                },
                {
                    desc: "buying has ended",
                    buyer: firstBuyer,
                    params: {
                        errorCode: "MintingEnded"
                    },
                    expectedValue: {},
                    shouldSucceed: false,
                    time: bond.startTime.toNumber() + 601,
                },
                {
                    desc: "buy",
                    buyer: firstBuyer,
                    params: {
                        errorCode: null
                    },
                    expectedValue: {
                        nextEditionNumber: 2,
                        nextEditionMarker: "0",
                        solFromBond: bond.price.toNumber(),
                    },
                    shouldSucceed: true,
                    time: bond.startTime.toNumber(),
                },
                {
                    desc: "buy again",
                    buyer: firstBuyer,
                    params: {
                        errorCode: "MaxMintPerWalletReached"
                    },
                    expectedValue: {
                        nextEditionNumber: 2,
                        nextEditionMarker: "0",
                        solFromBond: bond.price.toNumber(),
                    },
                    shouldSucceed: false,
                    time: 0,
                },
                {
                    desc: "another buyer",
                    buyer: secondBuyer,
                    params: {
                        errorCode: null
                    },
                    expectedValue: {
                        nextEditionNumber: 3,
                        nextEditionMarker: "0",
                        solFromBond: bond.price.toNumber() * 2,
                    },
                    shouldSucceed: true,
                    time: 0,
                },
                {
                    desc: "supply is finished",
                    buyer: lateBuyer,
                    params: {
                        errorCode: "MaxSupplyReached"
                    },
                    expectedValue: {
                        nextEditionNumber: 4,
                        nextEditionMarker: "0",
                        solFromBond: bond.price.toNumber(),
                    },
                    shouldSucceed: false,
                    time: 0,
                },
            ]
            for (const {desc, params, shouldSucceed, expectedValue, time, buyer} of testCases) {
                console.log(`when ${desc}`)
                if (time > 0) {
                    const currentClock = await provider.context.banksClient.getClock();
                    provider.context.setClock(
                        new Clock(
                            currentClock.slot,
                            currentClock.epochStartTimestamp,
                            currentClock.epoch,
                            currentClock.leaderScheduleEpoch,
                            BigInt(time),
                        ),
                    );
                }

                if (shouldSucceed) {
                    const tx = new anchor.web3.Transaction();

                    tx.add(
                        ComputeBudgetProgram.setComputeUnitLimit({
                            units: 250_000,
                        })
                    );
                    tx.add(
                        await program.methods
                            .buyBond(new anchor.BN(solStrategy.nextBondId.toNumber() - 1))
                            .accounts({buyer: buyer.publicKey})
                            .instruction()
                    );
                    await provider.sendAndConfirm(tx, [buyer]);

                    const newSolStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

                    const bond = await program.account.bond.fetch(bondPDA)

                    const [bondRecordPDA] = PublicKey.findProgramAddressSync(
                        [Buffer.from("bond_record"), bondPDA.toBuffer(), buyer.publicKey.toBuffer()],
                        program.programId
                    );
                    const bondRecord = await program.account.bondRecord.fetch(bondRecordPDA)

                    const editionBuffer = Buffer.alloc(8);
                    editionBuffer.writeBigUInt64LE(BigInt(bond.nextEditionNumber.toNumber() - 1));
                    const [buyerBondNFTPDA] = PublicKey.findProgramAddressSync(
                        [bondPDA.toBuffer(), editionBuffer],
                        program.programId
                    );
                    const buyerBondTokenAccount = await getAssociatedTokenAddress(buyerBondNFTPDA, buyer.publicKey);
                    const buyerBondTokenAccountInfo = await getAccount(program.provider.connection, buyerBondTokenAccount);

                    let edition = await getEdition(buyerBondNFTPDA)

                    expect(bond.nextEditionNumber.toNumber()).equal(expectedValue.nextEditionNumber, `next edition number should be ${expectedValue.nextEditionNumber}`)
                    expect(bond.nextEditionMarker).equal(expectedValue.nextEditionMarker, `next edition marker should be ${expectedValue.nextEditionMarker}`)
                    expect(newSolStrategy.solFromBond.toNumber()).equal(expectedValue.solFromBond, `sol made from bond should be ${expectedValue.solFromBond}`)
                    expect(bondRecord.collection.toBase58()).equal(bondPDA.toBase58(), `collection should be the bond public key`)
                    expect(bondRecord.user.toBase58()).equal(buyer.publicKey.toBase58(), `user should be the buyer public key`)
                    expect(bondRecord.minted.toNumber()).equal(1, `minted should be 1`)
                    expect(Number(buyerBondTokenAccountInfo.amount)).equal(1, `buyer bond nft account amount should be 1`)
                    expect(Number(edition.edition)).equal(expectedValue.nextEditionNumber - 1, `edition of mint edition should be ${expectedValue.nextEditionNumber - 1}`)
                    expect(edition.parent).equal(bondEditionPDA.toBase58(), `parent of mint edition should be ${bondNFTPDA.toBase58()}`)
                } else {
                    try {
                        await program.methods.buyBond(new anchor.BN(solStrategy.nextBondId.toNumber() - 1))
                            .accounts({buyer: buyer.publicKey})
                            .signers([buyer])
                            .rpc();
                        expect.fail("Expected an error but the instruction succeeded");

                    } catch (error: any) {
                        let msg = error.message as string
                        console.log(msg, "->", params.errorCode)
                        expect(msg.includes('AnchorError')).true
                        expect(msg.includes(`Error Code: ${params.errorCode}`)).true
                    }
                }
            }
        }
    )

    await it("should convert bond nft", async () => {
        const solStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

        const idBuffer = Buffer.alloc(8);
        idBuffer.writeBigUInt64LE(BigInt(solStrategy.nextBondId.toNumber() - 1));
        const [bondPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from("bond"), idBuffer],
            program.programId
        );
        const bond = await program.account.bond.fetch(bondPDA)
        const testCases = [
            {
                desc: "when account doesn't own edition",
                buyer: lateBuyer,
                params: {
                    edition: "1",
                    convert: true,
                    errorCode: "AccountNotInitialized"
                },
                expectedValue: {},
                shouldSucceed: false,
                time: bond.endTime.toNumber(),
            },
            {
                desc: "when bond has not matured",
                buyer: firstBuyer,
                params: {
                    edition: "1",
                    convert: true,
                    errorCode: "BondNotMatured"
                },
                expectedValue: {},
                shouldSucceed: false,
                time: bond.endTime.toNumber(),
            },
            {
                desc: "buyer doesn't convert bond",
                buyer: firstBuyer,
                params: {
                    edition: "1",
                    convert: false,
                    errorCode: null
                },
                expectedValue: {
                    solInTreasury: solStrategy.solInTreasury.toNumber(),
                    solFromBond: solStrategy.solFromBond.toNumber() - bond.price.toNumber(),
                    newBuyerBalance: 4999975599760,
                },
                shouldSucceed: true,
                time: bond.maturity.toNumber(),
            },
            {
                desc: "buyer tries to convert bond again",
                buyer: firstBuyer,
                params: {
                    edition: "1",
                    convert: false,
                    errorCode: "InvalidTokenAmount"
                },
                expectedValue: {
                    solInTreasury: solStrategy.solInTreasury.toNumber(),
                    solFromBond: solStrategy.solFromBond.toNumber() - bond.price.toNumber(),
                    newBuyerBalance: 4999975599760,
                },
                shouldSucceed: false,
                time: bond.maturity.toNumber(),
            },
            {
                desc: "buyer converts bond",
                buyer: secondBuyer,
                params: {
                    edition: "2",
                    convert: true,
                    errorCode: null
                },
                expectedValue: {
                    solFromBond: solStrategy.solFromBond.toNumber() - (bond.price.toNumber() * 2),
                    solInTreasury: solStrategy.solInTreasury.toNumber() + bond.price.toNumber(),
                    newBuyerBalance: 4989976713360,
                    newSolxrBalance: 6666666666,
                },
                shouldSucceed: true,
                time: bond.maturity.toNumber(),
            },
            {
                desc: "buyer tries to converts bond again",
                buyer: secondBuyer,
                params: {
                    edition: "2",
                    convert: true,
                    errorCode: "InvalidTokenAmount"
                },
                expectedValue: {
                    solFromBond: solStrategy.solFromBond.toNumber() - (bond.price.toNumber() * 2),
                    solInTreasury: solStrategy.solInTreasury.toNumber() + bond.price.toNumber(),
                    newBuyerBalance: 4989976713360,
                    newSolxrBalance: 6666666666,
                },
                shouldSucceed: false,
                time: bond.maturity.toNumber(),
            },
        ]
        for (const {desc, params, shouldSucceed, expectedValue, time, buyer} of testCases) {
            console.log(`when ${desc}`)
            if (time > 0) {
                const currentClock = await provider.context.banksClient.getClock();
                provider.context.setClock(
                    new Clock(
                        currentClock.slot,
                        currentClock.epochStartTimestamp,
                        currentClock.epoch,
                        currentClock.leaderScheduleEpoch,
                        BigInt(time),
                    ),
                );
            }
            if (shouldSucceed) {
                const buyAccountBefore = await provider.connection.getAccountInfo(buyer.publicKey);

                await program.methods.convertBond(
                    new anchor.BN(solStrategy.nextBondId.toNumber() - 1),
                    new anchor.BN(params.edition),
                    params.convert
                )
                    .accounts({buyer: buyer.publicKey})
                    .signers([buyer])
                    .rpc();

                const newSolStrategy = await program.account.solStrategy.fetch(solStrategyPDA)

                const bond = await program.account.bond.fetch(bondPDA)

                const editionBuffer = Buffer.alloc(8);
                editionBuffer.writeBigUInt64LE(BigInt(params.edition));
                const [buyerBondNFTPDA] = PublicKey.findProgramAddressSync(
                    [bondPDA.toBuffer(), editionBuffer],
                    program.programId
                );
                const buyerBondTokenAccount = await getAssociatedTokenAddress(buyerBondNFTPDA, buyer.publicKey);
                const buyerBondTokenAccountInfo = await getAccount(program.provider.connection, buyerBondTokenAccount);
                const solxrTokenAccount = await getAssociatedTokenAddress(tokenPDA, buyer.publicKey);
                const solxrTokenAccountInfo = await getAccount(program.provider.connection, solxrTokenAccount);
                const buyerAccountAfter = await provider.connection.getAccountInfo(buyer.publicKey);

                expect(newSolStrategy.solFromBond.toNumber()).equal(expectedValue.solFromBond, `sol made from bond should be ${expectedValue.solFromBond}`)
                expect(newSolStrategy.solInTreasury.toNumber()).equal(expectedValue.solInTreasury, `sol in treasury should be ${expectedValue.solFromBond}`)
                expect(Number(buyerBondTokenAccountInfo.amount)).equal(0, `buyer bond nft account amount should be 0`)
                expect(buyerAccountAfter.lamports).equal(expectedValue.newBuyerBalance, `buyer new balance should be ${expectedValue.newBuyerBalance}`)
                params.convert && expect(Number(solxrTokenAccountInfo.amount)).equal(expectedValue.newSolxrBalance, `buyer solxr account amount should be ${expectedValue.newSolxrBalance}`)
            } else {
                try {
                    await program.methods.convertBond(
                        new anchor.BN(solStrategy.nextBondId.toNumber() - 1),
                        new anchor.BN(params.edition),
                        params.convert
                    )
                        .accounts({buyer: buyer.publicKey})
                        .signers([buyer])
                        .rpc();
                    expect.fail("Expected an error but the instruction succeeded");

                } catch (error: any) {
                    let msg = error.message as string
                    console.log(msg, "->", params.errorCode)
                    expect(msg.includes('AnchorError')).true
                    expect(msg.includes(`Error Code: ${params.errorCode}`)).true
                }
            }
        }

    })

    async function getMasterEdition(mintAddress: PublicKey): Promise<MasterEdition> {
        // Get master edition PDA
        const [editionPDA] = PublicKey.findProgramAddressSync(
            [
                Buffer.from('metadata'),
                METADATA_PROGRAM_ID.toBuffer(),
                mintAddress.toBuffer(),
                Buffer.from('edition')
            ],
            METADATA_PROGRAM_ID
        );
        // Fetch the accounts
        const editionAccountInfo = await provider.connection.getAccountInfo(editionPDA);


        return deserializeMasterEdition({
            executable: editionAccountInfo.executable,
            rentEpoch: BigInt(editionAccountInfo.rentEpoch),
            lamports: lamports(editionAccountInfo.lamports),
            owner: editionAccountInfo.owner.toBase58() as MTPublicKey,
            data: editionAccountInfo.data,
            publicKey: editionPDA.toBase58() as MTPublicKey
        })
    }

    async function getEdition(mintAddress: PublicKey): Promise<Edition> {
        // Get master edition PDA
        const [editionPDA] = PublicKey.findProgramAddressSync(
            [
                Buffer.from('metadata'),
                METADATA_PROGRAM_ID.toBuffer(),
                mintAddress.toBuffer(),
                Buffer.from('edition')
            ],
            METADATA_PROGRAM_ID
        );
        // Fetch the accounts
        const editionAccountInfo = await provider.connection.getAccountInfo(editionPDA);

        return deserializeEdition({
            executable: editionAccountInfo.executable,
            rentEpoch: BigInt(editionAccountInfo.rentEpoch),
            lamports: lamports(editionAccountInfo.lamports),
            owner: editionAccountInfo.owner.toBase58() as MTPublicKey,
            data: editionAccountInfo.data,
            publicKey: editionPDA.toBase58() as MTPublicKey
        })
    }

    // todo: test convert after transfer

    async function fundAccount(keyPair: Keypair, amount: number) {
        const instruction = SystemProgram.transfer({
            fromPubkey: providerKeypair.publicKey,
            toPubkey: keyPair.publicKey,
            lamports: amount * LAMPORTS_PER_SOL,
        });
        const transaction = new Transaction().add(instruction);
        await provider.sendAndConfirm(transaction, [providerKeypair.payer]);
    }
});