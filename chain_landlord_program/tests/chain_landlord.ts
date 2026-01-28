import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ChainLandlord } from "../target/types/chain_landlord";
import { expect } from "chai";

describe("chain_landlord", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.ChainLandlord as Program<ChainLandlord>;

  let gameState: anchor.web3.Keypair;
  let table: anchor.web3.Keypair;

  before(async () => {
    gameState = anchor.web3.Keypair.generate();
    table = anchor.web3.Keypair.generate();
  });

  it("初始化游戏", async () => {
    await program.methods
      .initialize()
      .accounts({
        gameState: gameState.publicKey,
        owner: provider.wallet.publicKey,
      })
      .signers([gameState])
      .rpc();

    const state = await program.account.gameState.fetch(gameState.publicKey);
    expect(state.nextTableId.toNumber()).to.equal(1);
  });

  it("玩家加入游戏", async () => {
    const player1 = anchor.web3.Keypair.generate();
    
    // 空投 SOL 给玩家
    await provider.connection.requestAirdrop(
      player1.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );

    await program.methods
      .joinGame(player1.publicKey)
      .accounts({
        gameState: gameState.publicKey,
        table: table.publicKey,
        player: player1.publicKey,
      })
      .signers([player1, table])
      .rpc();

    const tableData = await program.account.table.fetch(table.publicKey);
    expect(tableData.players[0].toString()).to.equal(player1.publicKey.toString());
  });
});