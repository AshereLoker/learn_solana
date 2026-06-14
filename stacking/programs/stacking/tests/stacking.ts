import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Stacking } from "../../../target/types/stacking";
import {
    createMint,
    createAssociatedTokenAccount,
    mintTo,
    getAccount,
    getAssociatedTokenAddressSync,
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountInstruction,
    ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token"
import { assert } from "chai";
import { get } from "node:http";

describe("stacking", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const program = anchor.workspace.Stacking as Program<Stacking>

    // Users.
    const admin = anchor.web3.Keypair.generate();
    const staker = anchor.web3.Keypair.generate();

    // Mints and Accounts. Fills in before().
    let rewardMint: anchor.web3.PublicKey;
    let lockMint: anchor.web3.PublicKey;
    let adminRewardToken: anchor.web3.PublicKey;
    let stakerLockToken: anchor.web3.PublicKey;
    let stakerRewardToken: anchor.web3.PublicKey;

    const seed = new anchor.BN(42);
    const rate = 1_000;
    const fundAmout = 2_000_000;
    const lockAmout = 500_000;
    const lockPeriod = 1;

    // PDAs adress. Off-chain.
    let rewardConfigPda: anchor.web3.PublicKey;
    let rewardVaultPda: anchor.web3.PublicKey;
    let lockEntryPda: anchor.web3.PublicKey;
    let lockVaultPda: anchor.web3.PublicKey;

    before(async () => {
        const connection = provider.connection;
        // Request SOLs for test accounts.
        const { blockhash, lastValidBlockHeight} = await connection.getLatestBlockhash();
        const adminSignature = await connection.requestAirdrop(admin.publicKey, 2e9);
        const stakerSignature = await connection.requestAirdrop(staker.publicKey, 2e9);
        
        await connection.confirmTransaction(
            {
                signature: adminSignature,
                blockhash,
                lastValidBlockHeight,
            },
        );

        await connection.confirmTransaction(
            {
                signature: stakerSignature,
                blockhash,
                lastValidBlockHeight,
            },
        );

        // Create test Mints.
        rewardMint = await createMint(connection, admin, admin.publicKey, null, 6);
        lockMint = await createMint(connection, admin, admin.publicKey, null, 6);

        // Create token wallets.
        adminRewardToken = await createAssociatedTokenAccount(connection, admin, rewardMint, admin.publicKey);
        stakerLockToken = await createAssociatedTokenAccount(connection, admin, lockMint, staker.publicKey);
        stakerRewardToken = await createAssociatedTokenAccount(connection, admin, rewardMint, staker.publicKey);

        // Add tokens to accounts.
        await mintTo(connection, admin, rewardMint, adminRewardToken, admin, 2_000_000);
        await mintTo(connection, admin, lockMint, stakerLockToken, admin, 1_000_000);

        // Calculate off-chain PDA adresses. 
        [rewardConfigPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("config"), admin.publicKey.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
            program.programId,
        );
        [lockEntryPda] = anchor.web3.PublicKey.findProgramAddressSync(
            [Buffer.from("entry"), staker.publicKey.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
            program.programId,
        );

        // Calculate Vault addresses.
        rewardVaultPda = getAssociatedTokenAddressSync(
            rewardMint,
            rewardConfigPda,
            true,
        );
        lockVaultPda = getAssociatedTokenAddressSync(
            lockMint,
            lockEntryPda,
            true,
        )
    });

    it("create_reward | created reward for funding", async () => {
        await program.methods       
            .createReward(seed, new anchor.BN(rate))
            .accounts({
                admin: admin.publicKey,
                lockMint,
                rewardMint,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .signers([admin])
            .rpc();
        
        // Test fresh created config 
        const config = await program.account.stakeConfig.fetch(rewardConfigPda);
        assert.equal(config.admin.toBase58(), admin.publicKey.toBase58());
        assert.equal(config.rewardRate, rate);
    });
    
    it("fund_reward | admin create and fund reward", async () => {
        const balaceBefore = (await getAccount(provider.connection, adminRewardToken)).amount;

        await program.methods
            .fundRewards(new anchor.BN(fundAmout))
            .accountsPartial({
                admin: admin.publicKey,
                config: rewardConfigPda,
                rewardMint,
                adminRewardToken,
                rewardVault: rewardVaultPda,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .signers([admin])
            .rpc();
        
        // Tokens is added to vault
        const vault = await getAccount(provider.connection, rewardVaultPda);
        assert.equal(vault.amount, BigInt(2_000_000));

        // Admin tokens correctly changed
        const balaceAfter = (await getAccount(provider.connection, adminRewardToken)).amount;
        assert.equal(balaceBefore - balaceAfter, BigInt(2_000_000)); // 2_000_000 - 0 = 2_000_000
    }); 
    
    it("stake | user create stake and fund lock vault", async () => {
        const stackerLockBalanceBefore = (await getAccount(provider.connection, stakerLockToken)).amount;

        await program.methods
            .stake(seed, new anchor.BN(lockAmout), new anchor.BN(lockPeriod))
            .accountsPartial({
                staker: staker.publicKey,
                entry: lockEntryPda,
                lockMint,
                stakerLockToken,
                lockVault: lockVaultPda,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .signers([staker])
            .rpc();

        // Test fresh created entry
        const config = await program.account.stakeEntry.fetch(lockEntryPda);
        assert.equal(config.staker.toBase58(), staker.publicKey.toBase58());
        assert.equal(config.amount, lockAmout);
        assert.equal(config.lockPeriod, lockPeriod);

        // Tokens is added to vault
        const vault = await getAccount(provider.connection, lockVaultPda);
        assert.equal(vault.amount, BigInt(500_000));

        // Stacker tokens correctly changed
        const stackerLockBalanceAfter = (await getAccount(provider.connection, stakerLockToken)).amount;
        assert.equal(stackerLockBalanceBefore - stackerLockBalanceAfter, BigInt(500_000)); // 1_000_000 - 500_000 = 500_000
    });

    describe("unstake", () => {
        // Other user.
        const alice = anchor.web3.Keypair.generate();

        // Others tokens.
        let aliceLockToken: anchor.web3.PublicKey;
        let aliceRewardToken: anchor.web3.PublicKey;

        before(async () => {
            aliceRewardToken = await createAssociatedTokenAccount(provider.connection, admin, rewardMint, alice.publicKey);
            aliceLockToken = await createAssociatedTokenAccount(provider.connection, admin, lockMint, alice.publicKey);
        })

        it("unstake from other that owner | alice cant unstake staker stake", async () => {
            let failed = false;
            try {
                await program.methods
                    .unstake()
                    .accountsPartial({
                        staker: alice.publicKey,
                        admin: admin.publicKey,
                        entry: lockEntryPda,
                        stakerLockToken: aliceLockToken,
                        stakerRewardToken: aliceRewardToken,
                        lockVault: lockVaultPda,
                        rewardVault: rewardVaultPda,
                        config: rewardConfigPda,
                        rewardMint,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    })
                    .signers([alice])
                    .rpc();
             } catch(e: any) {
                failed = true;
                if (e?.error?.errorCode?.code) {
                    assert.notEqual(e.error.errorCode.code, "CantTakeEarly");
                }
            }
            assert.equal(failed, true);
        });

        it("unstake before grace period | CantTakeEarly is catched", async () => {
            try {
                await program.methods
                    .unstake()
                    .accountsPartial({
                        staker: staker.publicKey,
                        admin: admin.publicKey,
                        entry: lockEntryPda,
                        config: rewardConfigPda,
                        stakerLockToken: stakerLockToken,
                        stakerRewardToken: stakerRewardToken,
                        rewardVault: rewardVaultPda,
                        rewardMint,
                        tokenProgram: TOKEN_PROGRAM_ID,
                    })
                    .signers([alice])
                    .rpc();
            } catch(e: any) {
                if (e?.error?.errorCode?.code) {
                    assert.equal(e.error.errorCode.code, "CantTakeEarly");

                    return;
                }
            }
        });

        it("unstake after grace period | staker get reward and locked tokens", async () => {
            const rewardBalanceBefore = (await getAccount(provider.connection, stakerRewardToken)).amount;
            const lockBalaceBefore = (await getAccount(provider.connection, stakerLockToken)).amount;
            const config = await program.account.stakeConfig.fetch(rewardConfigPda);
            const entry = await program.account.stakeEntry.fetch(lockEntryPda);
            await sleep(2000);

            await program.methods
                .unstake()
                .accountsPartial({
                    staker: staker.publicKey,
                    admin: admin.publicKey,
                    entry: lockEntryPda,
                    config: rewardConfigPda,
                    stakerLockToken: stakerLockToken,
                    stakerRewardToken: stakerRewardToken,
                    lockVault: lockVaultPda,
                    rewardVault: rewardVaultPda,
                    rewardMint,
                    tokenProgram: TOKEN_PROGRAM_ID,
                })
                .signers([staker])
                .rpc();

            // Reward is correct. Reward formula - amount * reward_rate * (now - lock_at) / 1_000_000.
            const rewardBalanceAfter = (await getAccount(provider.connection, stakerRewardToken)).amount;
            const rewardActual = rewardBalanceAfter - rewardBalanceBefore;
            const rewardMin = BigInt(entry.amount.toNumber() * config.rewardRate.toNumber() * 1 / 1_000_000);
            const rewardMax = BigInt(entry.amount.toNumber() * config.rewardRate.toNumber() * 5 / 1_000_000);

            assert.isAtLeast(Number(rewardActual), Number(rewardMin), "reward too low");
            assert.isAtMost(Number(rewardActual), Number(rewardMax), "reward too high");

            // Tokens are correctly return from lock vault.
            const lockBalaceAfter = (await getAccount(provider.connection, stakerLockToken)).amount;
            assert.equal(lockBalaceAfter - lockBalaceBefore, BigInt(500_000)); // 1_000_000 - 500_000 = 500_000

            // Entry must be closed.
            try {
                await program.account.stakeEntry.fetch(lockEntryPda);
                assert.fail("Entry must be closed")
            } catch (e: any) {
                assert.include(e.message, "Account does not exist");
            }
        });
    });
})

async function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}