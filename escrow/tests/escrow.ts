import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Escrow } from "../target/types/escrow";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAccount,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";

describe("escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Escrow as Program<Escrow>;

  // Участники
  const maker = anchor.web3.Keypair.generate();
  const taker = anchor.web3.Keypair.generate();

  // Минты и аккаунты — заполним в before()
  let mintA: anchor.web3.PublicKey;
  let mintB: anchor.web3.PublicKey;
  let makerTokenA: anchor.web3.PublicKey;
  let makerTokenB: anchor.web3.PublicKey;
  let takerTokenA: anchor.web3.PublicKey;
  let takerTokenB: anchor.web3.PublicKey;

  const seed = new anchor.BN(42);

  // PDA адреса — вычислим офчейн
  let escrowPda: anchor.web3.PublicKey;
  let vaultPda: anchor.web3.PublicKey;

  // ─── Подготовка ───
  before(async () => {
    // Пополнить SOL для оплаты транзакций
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(maker.publicKey, 2e9)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(taker.publicKey, 2e9)
    );

    // Создать два тестовых токена
    mintA = await createMint(provider.connection, maker, maker.publicKey, null, 6);
    mintB = await createMint(provider.connection, taker, taker.publicKey, null, 6);

    // Создать token accounts
    makerTokenA = await createAssociatedTokenAccount(provider.connection, maker, mintA, maker.publicKey);
    makerTokenB = await createAssociatedTokenAccount(provider.connection, maker, mintB, maker.publicKey);
    takerTokenA = await createAssociatedTokenAccount(provider.connection, taker, mintA, taker.publicKey);
    takerTokenB = await createAssociatedTokenAccount(provider.connection, taker, mintB, taker.publicKey);

    // Выдать токены
    await mintTo(provider.connection, maker, mintA, makerTokenA, maker, 1_000_000); // 1 токен с 6 decimals
    await mintTo(provider.connection, taker, mintB, takerTokenB, taker, 2_000_000);

    // Вычислить PDA адреса
    [escrowPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), maker.publicKey.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // vault — это ATA где authority = escrowPda
    vaultPda = getAssociatedTokenAddressSync(mintA, escrowPda, true);
  });

  // ─── Тест 1: make_offer ───
  it("make_offer — создаёт escrow и переводит токены в vault", async () => {
    const makerBalanceBefore = (await getAccount(provider.connection, makerTokenA)).amount;

    await program.methods
      .makeOffer(seed, taker.publicKey, new anchor.BN(1_000_000), new anchor.BN(2_000_000))
      .accounts({
        maker: maker.publicKey,
        mintA,
        mintB,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    // Проверяем escrow аккаунт
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.equal(escrow.maker.toBase58(), maker.publicKey.toBase58());
    assert.equal(escrow.taker.toBase58(), taker.publicKey.toBase58());
    assert.equal(escrow.amountB.toNumber(), 2_000_000);

    // Проверяем что токены ушли в vault
    const vault = await getAccount(provider.connection, vaultPda);
    assert.equal(vault.amount.toString(), "1000000");

    // Проверяем что у maker списалось
    const makerBalanceAfter = (await getAccount(provider.connection, makerTokenA)).amount;
    assert.equal(makerBalanceBefore - makerBalanceAfter, BigInt(1_000_000));
  });

  // ─── Тест 2: take_offer ───
  it("take_offer — атомарный обмен токенами", async () => {
    await program.methods
      .takeOffer()
      .accountsPartial({
        taker: taker.publicKey,
        maker: maker.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        mintA,
        mintB,
        takerTokenA,
        takerTokenB,
        makerTokenB,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([taker])
      .rpc();

    // Taker получил Token A
    const takerA = await getAccount(provider.connection, takerTokenA);
    assert.equal(takerA.amount.toString(), "1000000");

    // Maker получил Token B
    const makerB = await getAccount(provider.connection, makerTokenB);
    assert.equal(makerB.amount.toString(), "2000000");

    // Escrow закрыт
    try {
      await program.account.escrow.fetch(escrowPda);
      assert.fail("Escrow должен быть закрыт");
    } catch (e) {
      assert.include(e.message, "Account does not exist");
    }
  });

  // ─── Тест 3: cancel_offer ───
  it("cancel_offer — возвращает токены maker-у", async () => {
    // Создаём новый escrow для теста отмены
    const cancelSeed = new anchor.BN(99);
    const [cancelEscrow] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("escrow"), maker.publicKey.toBuffer(), cancelSeed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    const cancelVault = getAssociatedTokenAddressSync(mintA, cancelEscrow, true);

    // Пополнить maker снова
    await mintTo(provider.connection, maker, mintA, makerTokenA, maker, 1_000_000);

    await program.methods
      .makeOffer(cancelSeed, null, new anchor.BN(1_000_000), new anchor.BN(2_000_000))
      .accounts({
        maker: maker.publicKey,
        mintA,
        mintB,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    const balanceBefore = (await getAccount(provider.connection, makerTokenA)).amount;

    await program.methods
      .cancelOffer()
      .accountsPartial({
        maker: maker.publicKey,
        escrow: cancelEscrow,
        vault: cancelVault,
        mintA,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([maker])
      .rpc();

    // Токены вернулись
    const balanceAfter = (await getAccount(provider.connection, makerTokenA)).amount;
    assert.equal(balanceAfter - balanceBefore, BigInt(1_000_000));

    // Escrow закрыт
    try {
      await program.account.escrow.fetch(cancelEscrow);
      assert.fail("Escrow должен быть закрыт");
    } catch (e) {
      assert.include(e.message, "Account does not exist");
    }
  });
});