use anchor_lang::prelude::*;

declare_id!("YOUR_PROGRAM_ID_WILL_BE_HERE");

#[program]
pub mod chain_landlord {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        game_state.owner = ctx.accounts.owner.key();
        game_state.next_table_id = 1;
        Ok(())
    }

    pub fn join_game(ctx: Context<JoinGame>, beneficiary: Pubkey) -> Result<()> {
        let game_state = &mut ctx.accounts.game_state;
        let table = &mut ctx.accounts.table;
        let player = &ctx.accounts.player;

        // 找到空位
        let mut seat_index = 0;
        for i in 0..3 {
            if table.players[i] == Pubkey::default() {
                seat_index = i;
                break;
            }
        }

        // 记录玩家信息
        table.players[seat_index] = player.key();
        table.beneficiaries[seat_index] = beneficiary;
        table.pot += ctx.accounts.entry_fee.lamports();

        emit!(PlayerJoined {
            table_id: table.id,
            burner: player.key(),
            beneficiary,
            seat_index: seat_index as u8,
        });

        // 检查是否3人齐全
        if table.players[0] != Pubkey::default()
            && table.players[1] != Pubkey::default()
            && table.players[2] != Pubkey::default()
        {
            start_game(table)?;
        }

        Ok(())
    }

    pub fn bid(ctx: Context<Bid>, score: u8) -> Result<()> {
        let table = &mut ctx.accounts.table;
        let player = &ctx.accounts.player;

        require!(table.state == GameState::Bidding as u8, ErrorCode::NotBiddingPhase);

        let player_index = get_player_index(table, player.key())?;
        require!(player_index == table.current_turn, ErrorCode::NotYourTurn);

        if score > 0 {
            table.highest_bid = score;
            table.landlord_index = player_index;
        }

        table.current_turn = (table.current_turn + 1) % 3;

        // 一轮后确定地主
        if table.current_turn == 0 {
            if table.highest_bid == 0 {
                table.landlord_index = 0;
            }
            finalize_landlord(table)?;
        }

        Ok(())
    }

    pub fn play_hand(ctx: Context<PlayHand>, cards: Vec<u8>) -> Result<()> {
        let table = &mut ctx.accounts.table;
        let player = &ctx.accounts.player;

        require!(table.state == GameState::Playing as u8, ErrorCode::NotPlaying);

        let player_index = get_player_index(table, player.key())?;
        require!(player_index == table.current_turn, ErrorCode::NotYourTurn);

        // 记录出牌
        table.last_hand_cards = cards.clone();
        table.last_hand_player_index = player_index;

        emit!(PlayerPlayed {
            table_id: table.id,
            player_index,
            cards,
        });

        table.current_turn = (table.current_turn + 1) % 3;

        Ok(())
    }

    pub fn end_game(ctx: Context<EndGame>, winner_index: u8) -> Result<()> {
        let table = &mut ctx.accounts.table;
        
        require!(table.state == GameState::Playing as u8, ErrorCode::NotPlaying);
        require!(winner_index < 3, ErrorCode::InvalidWinner);

        settle_game(table, winner_index as usize, ctx.accounts.owner.to_account_info())?;

        Ok(())
    }
}

// ========== 辅助函数 ==========

fn start_game(table: &mut Account<Table>) -> Result<()> {
    table.state = GameState::Bidding as u8;

    // 生成随机底牌（简化版）
    let clock = Clock::get()?;
    let seed = clock.unix_timestamp as u64;
    table.hole_cards = [
        ((seed >> 0) % 54) as u8,
        ((seed >> 8) % 54) as u8,
        ((seed >> 16) % 54) as u8,
    ];

    table.current_turn = 0;

    emit!(GameStarted {
        table_id: table.id,
    });

    Ok(())
}

fn finalize_landlord(table: &mut Account<Table>) -> Result<()> {
    table.state = GameState::Playing as u8;
    table.current_turn = table.landlord_index;

    emit!(LandlordElected {
        table_id: table.id,
        landlord_index: table.landlord_index,
        hole_cards: table.hole_cards.to_vec(),
    });

    Ok(())
}

fn settle_game(
    table: &mut Account<Table>,
    winner_index: usize,
    owner: AccountInfo,
) -> Result<()> {
    table.state = GameState::Ended as u8;

    let is_landlord_win = winner_index == table.landlord_index as usize;
    let protocol_fee = table.pot * 5 / 100;
    let reward_pool = table.pot - protocol_fee;

    // 转账逻辑（这里需要实际的转账实现）
    // 在 Solana 中需要通过 CPI 调用来实现

    emit!(GameEnded {
        table_id: table.id,
        winner_burner: table.players[winner_index],
        winner_beneficiary: table.beneficiaries[winner_index],
        win_amount: reward_pool,
    });

    Ok(())
}

fn get_player_index(table: &Account<Table>, player: Pubkey) -> Result<u8> {
    for i in 0..3 {
        if table.players[i] == player {
            return Ok(i as u8);
        }
    }
    Err(ErrorCode::PlayerNotFound.into())
}

// ========== 账户结构 ==========

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = owner, space = 8 + 32 + 8)]
    pub game_state: Account<'info, GameState>,
    #[account(mut)]
    pub owner: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinGame<'info> {
    #[account(mut)]
    pub game_state: Account<'info, GameState>,
    #[account(init_if_needed, payer = player, space = 8 + Table::SIZE)]
    pub table: Account<'info, Table>,
    #[account(mut)]
    pub player: Signer<'info>,
    /// CHECK: Entry fee account
    #[account(mut)]
    pub entry_fee: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Bid<'info> {
    #[account(mut)]
    pub table: Account<'info, Table>,
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct PlayHand<'info> {
    #[account(mut)]
    pub table: Account<'info, Table>,
    pub player: Signer<'info>,
}

#[derive(Accounts)]
pub struct EndGame<'info> {
    #[account(mut)]
    pub table: Account<'info, Table>,
    /// CHECK: Owner account
    pub owner: AccountInfo<'info>,
}

// ========== 数据结构 ==========

#[account]
pub struct GameState {
    pub owner: Pubkey,
    pub next_table_id: u64,
}

#[account]
pub struct Table {
    pub id: u64,
    pub state: u8,
    pub players: [Pubkey; 3],
    pub beneficiaries: [Pubkey; 3],
    pub hole_cards: [u8; 3],
    pub current_turn: u8,
    pub landlord_index: u8,
    pub highest_bid: u8,
    pub last_hand_cards: Vec<u8>,
    pub last_hand_player_index: u8,
    pub pot: u64,
}

impl Table {
    pub const SIZE: usize = 8 + // discriminator
        8 + // id
        1 + // state
        32 * 3 + // players
        32 * 3 + // beneficiaries
        3 + // hole_cards
        1 + // current_turn
        1 + // landlord_index
        1 + // highest_bid
        (4 + 54) + // last_hand_cards (max 54 cards)
        1 + // last_hand_player_index
        8; // pot
}

#[repr(u8)]
pub enum GameState {
    Waiting = 0,
    Bidding = 1,
    Playing = 2,
    Ended = 3,
}

// ========== 事件 ==========

#[event]
pub struct PlayerJoined {
    pub table_id: u64,
    pub burner: Pubkey,
    pub beneficiary: Pubkey,
    pub seat_index: u8,
}

#[event]
pub struct GameStarted {
    pub table_id: u64,
}

#[event]
pub struct LandlordElected {
    pub table_id: u64,
    pub landlord_index: u8,
    pub hole_cards: Vec<u8>,
}

#[event]
pub struct PlayerPlayed {
    pub table_id: u64,
    pub player_index: u8,
    pub cards: Vec<u8>,
}

#[event]
pub struct GameEnded {
    pub table_id: u64,
    pub winner_burner: Pubkey,
    pub winner_beneficiary: Pubkey,
    pub win_amount: u64,
}

// ========== 错误码 ==========

#[error_code]
pub enum ErrorCode {
    #[msg("Not in bidding phase")]
    NotBiddingPhase,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Not playing")]
    NotPlaying,
    #[msg("Invalid winner")]
    InvalidWinner,
    #[msg("Player not found")]
    PlayerNotFound,
}