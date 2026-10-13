//! # SolanaPets Genesis Raffle Program
//!
//! Distributes the 15,000 Genesis NFPs via a provably fair on-chain raffle.
//!
//! ## How it works
//! 1. **Sale** (`buy_tickets`): buyers purchase tickets in three tiers —
//!    Single (0.25 SOL, 1 ticket), 5-Pack (1 SOL, 5 tickets), Whale Pack
//!    (1.25 SOL, 10 tickets, capped at 500 packs). SOL goes to the raffle
//!    treasury PDA. Each buyer gets a `TicketPurchase` PDA recording their
//!    global ticket index range.
//! 2. **Draw** (`draw_winners`, admin only, once, after the sale ends):
//!    a Fisher-Yates shuffle (seeded on-chain) randomizes both the prize
//!    pool (15,000 species/gender slots honoring the 3,750-per-species and
//!    50/50 gender quotas) and the ticket order. The first 15,000 shuffled
//!    tickets win. Results are stored in a `DrawResult` PDA — fully public
//!    and verifiable.
//! 3. **Claim** (`claim_pet`): each winner claims by rank. The program
//!    verifies `winner_order[rank] == ticket_index` (O(1) — the client does
//!    the lookup off-chain from the public winner list), checks the ticket
//!    belongs to the signer, then CPI-mints the Genesis pet via
//!    pet-registry with the drawn species/gender and rolled traits.
//!    Genesis pets can roll Epic/Legendary/Mythic rarity — the headline
//!    Mythic pulls happen here.
//! 4. **Payout** (`distribute_funds`, admin only, after the draw): 20% of
//!    the treasury goes to Brad's dev wallet (founder compensation —
//!    non-negotiable, per the marketing plan). The remaining 80% stays in
//!    the treasury for the audit, development, and liquidity.
//!
//! ## Randomness — READ THIS
//! The draw uses a `RaffleRng` (splitmix64 seeded from slot hash material,
//! timestamp, and ticket count). This is PREDICTABLE — a validator can
//! reproduce it. It is devnet scaffolding ONLY. Production MUST replace
//! this with Switchboard VRF before mainnet and before the audit. The
//! marketing plan promises "provably fair" — VRF is how we keep that
//! promise.
//!
//! ## The 8 creator pets
//! IDs 1–8 are Brad's and are EXCLUDED from the raffle. The raffle mints
//! via `mint_nfp`, which starts at the registry's `next_id` (9+). The
//! creator pets are minted separately via the future `mint_creator_pet`.
//!
//! ## DEVNET ONLY. No mainnet deployment until audit.
//!
//! Program ID (devnet): 7gVjk6k9WPUanx2GyWaHwt229NXiQwtHMChBbMerfmyQ

use anchor_lang::prelude::*;
use anchor_lang::system_program;
use pet_registry::{
    self, Gender, Pet, Rarity, Registry, Species, PET_SEED, REGISTRY_SEED,
};

// Devnet program ID — generated for this deployment, never mainnet.
declare_id!("7gVjk6k9WPUanx2GyWaHwt229NXiQwtHMChBbMerfmyQ");

// ============================================================================
// Constants — the raffle parameters (Brad-approved, see marketing plan)
// ============================================================================

/// Total Genesis pets in the raffle pool.
pub const PRIZE_COUNT: u32 = 15_000;
/// Per-species quota: 3,750 each of otters, cats, dogs, monkeys.
pub const SPECIES_QUOTA: u32 = 3_750;
/// Per-species-per-gender quota: 1,875 each (50/50 M/F split).
pub const GENDER_QUOTA: u32 = 1_875;

/// Ticket tier prices in lamports.
pub const SINGLE_PRICE_LAMPORTS: u64 = 250_000_000; // 0.25 SOL
pub const FIVE_PACK_PRICE_LAMPORTS: u64 = 1_000_000_000; // 1.0 SOL
pub const WHALE_PACK_PRICE_LAMPORTS: u64 = 1_250_000_000; // 1.25 SOL

/// Tickets per tier.
pub const SINGLE_TICKET_COUNT: u32 = 1;
pub const FIVE_PACK_TICKET_COUNT: u32 = 5;
pub const WHALE_PACK_TICKET_COUNT: u32 = 10;

/// Whale packs are capped at 500 for scarcity.
pub const MAX_WHALE_PACKS: u32 = 500;

/// Brad's founder/dev share: 20% off the top, non-negotiable.
pub const DEV_SHARE_BPS: u64 = 2_000; // 20% in basis points
pub const BPS_DENOMINATOR: u64 = 10_000;

/// PDA seeds.
pub const RAFFLE_SEED: &[u8] = b"genesis-raffle";
pub const RAFFLE_TREASURY_SEED: &[u8] = b"raffle-treasury";
pub const TICKETS_SEED: &[u8] = b"tickets";
pub const DRAW_SEED: &[u8] = b"genesis-draw";

// ============================================================================
// Program
// ============================================================================

#[program]
pub mod raffle {
    use super::*;

    /// One-time raffle setup. Sets the sale window, tier prices, wallets,
    /// and quotas. Only callable once (the config PDA init enforces this).
    pub fn initialize_raffle(
        ctx: Context<InitializeRaffle>,
        sale_start: i64,
        sale_end: i64,
        dev_wallet: Pubkey,
    ) -> Result<()> {
        require!(sale_end > sale_start, RaffleError::InvalidSaleWindow);
        require!(
            sale_start > 0 && sale_end > 0,
            RaffleError::InvalidSaleWindow
        );

        let config = &mut ctx.accounts.raffle_config;
        config.authority = ctx.accounts.authority.key();
        config.dev_wallet = dev_wallet;
        config.sale_start = sale_start;
        config.sale_end = sale_end;
        config.total_tickets_sold = 0;
        config.whale_packs_sold = 0;
        config.draw_complete = false;
        config.funds_distributed = false;
        config.bump = ctx.bumps.raffle_config;
        config.treasury_bump = ctx.bumps.treasury;

        msg!(
            "Genesis raffle initialized: sale {} -> {}, dev wallet {}",
            sale_start,
            sale_end,
            dev_wallet
        );
        Ok(())
    }

    /// Buy tickets at a given tier. Payment goes to the treasury PDA.
    /// The buyer's `TicketPurchase` PDA records their global ticket range.
    pub fn buy_tickets(ctx: Context<BuyTickets>, tier: TicketTier) -> Result<()> {
        let config = &mut ctx.accounts.raffle_config;
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;

        // Sale must be open.
        require!(
            now >= config.sale_start && now <= config.sale_end,
            RaffleError::SaleNotOpen
        );
        require!(!config.draw_complete, RaffleError::DrawAlreadyComplete);

        let (price, ticket_count) = tier.price_and_count();

        // Whale pack cap enforcement.
        if tier == TicketTier::WhalePack {
            require!(
                config.whale_packs_sold < MAX_WHALE_PACKS,
                RaffleError::WhaleCapExceeded
            );
        }

        // Buyer -> treasury.
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, price)?;

        // Record the purchase. init_if_needed: first purchase creates the
        // PDA, later purchases extend it.
        let purchase = &mut ctx.accounts.ticket_purchase;
        if purchase.ticket_count == 0 {
            // Fresh PDA — initialize the buyer and the starting index.
            purchase.buyer = ctx.accounts.buyer.key();
            purchase.first_ticket_index = config.total_tickets_sold;
            purchase.bump = ctx.bumps.ticket_purchase;
        } else {
            // Existing buyer — the new tickets extend their range.
            // (first_ticket_index stays; ticket_count grows.)
            require_keys_eq!(
                purchase.buyer,
                ctx.accounts.buyer.key(),
                RaffleError::TicketOwnerMismatch
            );
        }
        purchase.ticket_count = purchase
            .ticket_count
            .checked_add(ticket_count)
            .ok_or(RaffleError::MathOverflow)?;
        purchase.tier_counts[tier as usize] = purchase.tier_counts[tier as usize]
            .checked_add(1)
            .ok_or(RaffleError::MathOverflow)?;

        config.total_tickets_sold = config
            .total_tickets_sold
            .checked_add(ticket_count)
            .ok_or(RaffleError::MathOverflow)?;
        if tier == TicketTier::WhalePack {
            config.whale_packs_sold = config
                .whale_packs_sold
                .checked_add(1)
                .ok_or(RaffleError::MathOverflow)?;
        }

        msg!(
            "{:?} purchased by {}: {} tickets ({}-{}), {} lamports",
            tier,
            ctx.accounts.buyer.key(),
            ticket_count,
            purchase.first_ticket_index,
            purchase.first_ticket_index + purchase.ticket_count - 1,
            price
        );
        Ok(())
    }

    /// Execute the draw. ADMIN ONLY, callable exactly once, after the sale
    /// ends. Shuffles the prize pool and ticket order with a seeded RNG,
    /// stores the results publicly in the `DrawResult` PDA.
    ///
    /// RANDOMNESS WARNING: uses `RaffleRng` (predictable). Production MUST
    /// use Switchboard VRF. See module docs.
    pub fn draw_winners(ctx: Context<DrawWinners>) -> Result<()> {
        let config = &mut ctx.accounts.raffle_config;
        let clock = Clock::get()?;

        require!(
            ctx.accounts.authority.key() == config.authority,
            RaffleError::UnauthorizedAuthority
        );
        require!(
            clock.unix_timestamp > config.sale_end,
            RaffleError::SaleNotEnded
        );
        require!(!config.draw_complete, RaffleError::DrawAlreadyComplete);
        require!(
            config.total_tickets_sold > 0,
            RaffleError::NoTicketsSold
        );

        let total_tickets = config.total_tickets_sold;
        let winner_count = total_tickets.min(PRIZE_COUNT);

        // Seed from on-chain entropy. PREDICTABLE — devnet only.
        let mut rng = RaffleRng::new(&clock, total_tickets);

        // 1. Build and shuffle the prize pool (species/gender quotas).
        let mut prizes = build_prize_pool();
        fisher_yates_bytes(&mut prizes, &mut rng);

        // 2. Shuffle ticket indices; the first `winner_count` win.
        let mut ticket_order: Vec<u32> = (0..total_tickets).collect();
        fisher_yates_u32(&mut ticket_order, &mut rng);
        ticket_order.truncate(winner_count as usize);

        // 3. Store the results.
        let draw = &mut ctx.accounts.draw_result;
        draw.winner_count = winner_count;
        draw.winner_order = ticket_order;
        // Pack prizes: byte = (species << 1) | gender. Only the first
        // winner_count are used, but store the full shuffled pool.
        draw.prizes = prizes;
        draw.claimed_bitmap = vec![0u8; (winner_count as usize + 7) / 8];
        draw.drawn_at = clock.unix_timestamp;
        draw.bump = ctx.bumps.draw_result;

        config.draw_complete = true;

        msg!(
            "Draw complete: {} winners from {} tickets. Seed material: slot {}, ts {}",
            winner_count,
            total_tickets,
            clock.slot,
            clock.unix_timestamp
        );
        Ok(())
    }

    /// Claim a Genesis pet. The winner provides their rank and ticket
    /// index; the program verifies `winner_order[rank] == ticket_index`
    /// (O(1) — the client looks up the rank from the public winner list
    /// off-chain). Then it CPI-mints the pet with the drawn species/gender
    /// and rolled traits.
    pub fn claim_pet(
        ctx: Context<ClaimPet>,
        rank: u32,
        ticket_index: u32,
    ) -> Result<()> {
        let config = &ctx.accounts.raffle_config;
        let draw = &mut ctx.accounts.draw_result;

        require!(config.draw_complete, RaffleError::DrawNotComplete);
        require!(rank < draw.winner_count, RaffleError::InvalidRank);

        // O(1) winner verification.
        require!(
            draw.winner_order[rank as usize] == ticket_index,
            RaffleError::NotAWinner
        );

        // Double-claim prevention.
        let byte_idx = (rank as usize) / 8;
        let bit_idx = (rank as usize) % 8;
        require!(
            draw.claimed_bitmap[byte_idx] & (1 << bit_idx) == 0,
            RaffleError::AlreadyClaimed
        );

        // The ticket must belong to the signer.
        let purchase = &ctx.accounts.ticket_purchase;
        require_keys_eq!(
            purchase.buyer,
            ctx.accounts.winner.key(),
            RaffleError::TicketOwnerMismatch
        );
        require!(
            ticket_index >= purchase.first_ticket_index
                && ticket_index < purchase.first_ticket_index + purchase.ticket_count,
            RaffleError::TicketNotOwned
        );

        // Decode the prize: byte = (species << 1) | gender.
        let packed = draw.prizes[rank as usize];
        let species = match (packed >> 1) & 0x03 {
            0 => Species::Otter,
            1 => Species::Cat,
            2 => Species::Dog,
            _ => Species::Monkey,
        };
        let gender = if packed & 0x01 == 0 {
            Gender::Male
        } else {
            Gender::Female
        };

        // Roll Genesis traits deterministically from the draw seed + rank.
        // Same input → same pet, verifiable by anyone.
        let mut trait_rng = RaffleRng::new_seeded(
            (draw.drawn_at as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15)
                .wrapping_add(rank as u64),
        );
        let traits = roll_genesis_traits(&mut trait_rng, species);

        // Mark claimed BEFORE the CPI (reentrancy hygiene).
        draw.claimed_bitmap[byte_idx] |= 1 << bit_idx;

        // CPI-mint the Genesis pet. The raffle authority co-signs (it is
        // the registry authority in this deployment).
        let params = pet_registry::MintNfpParams {
            species,
            gender,
            color_primary: traits.color_primary,
            color_secondary: traits.color_secondary,
            marking: traits.marking,
            eye_color: traits.eye_color,
            curiosity: traits.curiosity,
            playfulness: traits.playfulness,
            affection: traits.affection,
            loyalty: traits.loyalty,
            intelligence: traits.intelligence,
            calmness: traits.calmness,
            rarity: traits.rarity,
            owner: ctx.accounts.winner.key(),
            legendary_expressed: traits.legendary_expressed,
            mythic_expressed: traits.mythic_expressed,
        };
        let pet_id = ctx.accounts.registry.next_id;
        let cpi_accounts = pet_registry::cpi::accounts::MintNfp {
            authority: ctx.accounts.authority.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
        );
        pet_registry::cpi::mint_nfp(cpi_ctx, params)?;

        msg!(
            "Genesis NFP #{} claimed by {} (rank {}, {:?} {:?}, {:?})",
            pet_id,
            ctx.accounts.winner.key(),
            rank,
            species,
            gender,
            traits.rarity
        );
        Ok(())
    }

    /// Distribute raffle proceeds. ADMIN ONLY, callable once, after the
    /// draw. 20% goes to Brad's dev wallet (founder compensation);
    /// the remaining 80% stays in the treasury for the audit, development,
    /// and liquidity. Emits the amounts for public transparency.
    pub fn distribute_funds(ctx: Context<DistributeFunds>) -> Result<()> {
        let config = &mut ctx.accounts.raffle_config;

        require!(
            ctx.accounts.authority.key() == config.authority,
            RaffleError::UnauthorizedAuthority
        );
        require!(config.draw_complete, RaffleError::DrawNotComplete);
        require!(
            !config.funds_distributed,
            RaffleError::FundsAlreadyDistributed
        );

        let treasury_balance = ctx.accounts.treasury.lamports();
        // Leave a rent-exempt minimum in the treasury PDA so the account
        // survives. (Devnet: 8,904 lamports for a 0-byte account is the
        // classic minimum; we keep 1M lamports as a safety margin.)
        const TREASURY_RESERVE: u64 = 1_000_000;
        let distributable = treasury_balance.saturating_sub(TREASURY_RESERVE);
        require!(distributable > 0, RaffleError::NothingToDistribute);

        let (dev_share, project_share) = split_funds(distributable)?;

        // Treasury PDA signs the transfer to the dev wallet.
        let treasury_bump = config.treasury_bump;
        let treasury_seeds: &[&[u8]] = &[RAFFLE_TREASURY_SEED, &[treasury_bump]];
        let signer_seeds = &[treasury_seeds];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.treasury.to_account_info(),
                to: ctx.accounts.dev_wallet.to_account_info(),
            },
            signer_seeds,
        );
        system_program::transfer(cpi_ctx, dev_share)?;

        config.funds_distributed = true;

        msg!(
            "Funds distributed: {} lamports total → {} to dev wallet (20%), {} retained in treasury (80%)",
            distributable,
            dev_share,
            project_share
        );
        Ok(())
    }
}

// ============================================================================
// Account validation
// ============================================================================

#[derive(Accounts)]
pub struct InitializeRaffle<'info> {
    /// Raffle admin — runs the draw and distributes funds.
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + RaffleConfig::SIZE,
        seeds = [RAFFLE_SEED],
        bump,
    )]
    pub raffle_config: Account<'info, RaffleConfig>,

    /// CHECK: treasury PDA — receives all ticket payments.
    #[account(
        seeds = [RAFFLE_TREASURY_SEED],
        bump,
    )]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyTickets<'info> {
    /// The ticket buyer — pays SOL, receives the ticket record.
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        seeds = [RAFFLE_SEED],
        bump = raffle_config.bump,
    )]
    pub raffle_config: Account<'info, RaffleConfig>,

    /// CHECK: treasury PDA — receives the payment.
    #[account(
        mut,
        seeds = [RAFFLE_TREASURY_SEED],
        bump = raffle_config.treasury_bump,
    )]
    pub treasury: UncheckedAccount<'info>,

    /// The buyer's ticket record. Created on first purchase, extended after.
    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + TicketPurchase::SIZE,
        seeds = [TICKETS_SEED, buyer.key().as_ref()],
        bump,
    )]
    pub ticket_purchase: Account<'info, TicketPurchase>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DrawWinners<'info> {
    /// Must be the raffle authority (checked in the handler).
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [RAFFLE_SEED],
        bump = raffle_config.bump,
    )]
    pub raffle_config: Account<'info, RaffleConfig>,

    /// The draw result account — ~85KB, paid for by the authority.
    #[account(
        init,
        payer = authority,
        space = 8 + DrawResult::SPACE,
        seeds = [DRAW_SEED],
        bump,
    )]
    pub draw_result: Account<'info, DrawResult>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimPet<'info> {
    /// The winner — must own the ticket, receives the pet.
    #[account(mut)]
    pub winner: Signer<'info>,

    #[account(
        seeds = [RAFFLE_SEED],
        bump = raffle_config.bump,
    )]
    pub raffle_config: Account<'info, RaffleConfig>,

    #[account(
        mut,
        seeds = [DRAW_SEED],
        bump = draw_result.bump,
    )]
    pub draw_result: Account<'info, DrawResult>,

    #[account(
        seeds = [TICKETS_SEED, winner.key().as_ref()],
        bump = ticket_purchase.bump,
    )]
    pub ticket_purchase: Account<'info, TicketPurchase>,

    /// Registry authority — co-signs the CPI mint (is the registry
    /// authority in this deployment).
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [REGISTRY_SEED],
        bump = registry.bump,
        has_one = authority @ RaffleError::UnauthorizedAuthority,
    )]
    pub registry: Account<'info, Registry>,

    /// CHECK: created by pet-registry via CPI into `mint_nfp` at
    /// seeds ["pet", registry.next_id]. The inner program enforces this.
    #[account(mut)]
    pub pet: UncheckedAccount<'info>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DistributeFunds<'info> {
    /// Must be the raffle authority (checked in the handler).
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [RAFFLE_SEED],
        bump = raffle_config.bump,
    )]
    pub raffle_config: Account<'info, RaffleConfig>,

    /// CHECK: treasury PDA — sends the dev share. Seeds verified.
    #[account(
        mut,
        seeds = [RAFFLE_TREASURY_SEED],
        bump = raffle_config.treasury_bump,
    )]
    pub treasury: UncheckedAccount<'info>,

    /// CHECK: must match the configured dev wallet (Brad's).
    #[account(
        mut,
        address = raffle_config.dev_wallet @ RaffleError::WrongDevWallet,
    )]
    pub dev_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Account data
// ============================================================================

/// Raffle configuration. PDA seeds: ["genesis-raffle"]
#[account]
pub struct RaffleConfig {
    /// Admin — runs the draw, distributes funds.
    pub authority: Pubkey, // 32
    /// Brad's dev wallet — receives 20% of proceeds.
    pub dev_wallet: Pubkey, // 32
    /// Sale window (unix timestamps).
    pub sale_start: i64, // 8
    pub sale_end: i64,   // 8
    /// Counters.
    pub total_tickets_sold: u32, // 4
    pub whale_packs_sold: u32,   // 4
    /// Lifecycle flags.
    pub draw_complete: bool,     // 1
    pub funds_distributed: bool, // 1
    /// PDA bumps.
    pub bump: u8,          // 1
    pub treasury_bump: u8, // 1
}

impl RaffleConfig {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 4 + 4 + 1 + 1 + 1 + 1; // 92
}

/// A buyer's ticket record. PDA seeds: ["tickets", buyer]
#[account]
pub struct TicketPurchase {
    /// The buyer.
    pub buyer: Pubkey, // 32
    /// Global index of this buyer's first ticket.
    pub first_ticket_index: u32, // 4
    /// Total tickets owned.
    pub ticket_count: u32, // 4
    /// Packs bought per tier [single, five_pack, whale_pack].
    pub tier_counts: [u32; 3], // 12
    /// PDA bump.
    pub bump: u8, // 1
}

impl TicketPurchase {
    pub const SIZE: usize = 32 + 4 + 4 + 12 + 1; // 53
}

/// The draw results. PDA seeds: ["genesis-draw"]
///
/// `winner_order[rank]` = the global ticket index of the rank-th winner.
/// `prizes[rank]` = packed species/gender byte for that rank:
/// byte = (species << 1) | gender, species 0=Otter 1=Cat 2=Dog 3=Monkey.
#[account]
pub struct DrawResult {
    /// Number of winners (min(tickets_sold, 15,000)).
    pub winner_count: u32, // 4
    /// Winner ticket indices in rank order.
    pub winner_order: Vec<u32>, // 4 + 60,000
    /// Packed prize bytes in rank order.
    pub prizes: Vec<u8>, // 4 + 15,000
    /// Claim bitmap (1 bit per rank).
    pub claimed_bitmap: Vec<u8>, // 4 + 1,875
    /// Draw timestamp (also trait-roll seed material).
    pub drawn_at: i64, // 8
    /// PDA bump.
    pub bump: u8, // 1
}

impl DrawResult {
    /// 4 + (4+60000) + (4+15000) + (4+1875) + 8 + 1
    pub const SPACE: usize = 4 + 60_004 + 15_004 + 1_879 + 8 + 1; // 76,900
}

/// Ticket tiers (Brad-approved pricing).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TicketTier {
    Single,
    FivePack,
    WhalePack,
}

impl TicketTier {
    /// (price in lamports, tickets granted).
    pub fn price_and_count(&self) -> (u64, u32) {
        match self {
            TicketTier::Single => (SINGLE_PRICE_LAMPORTS, SINGLE_TICKET_COUNT),
            TicketTier::FivePack => (FIVE_PACK_PRICE_LAMPORTS, FIVE_PACK_TICKET_COUNT),
            TicketTier::WhalePack => (WHALE_PACK_PRICE_LAMPORTS, WHALE_PACK_TICKET_COUNT),
        }
    }
}

/// Rolled traits for a Genesis pet claim.
pub struct GenesisTraits {
    pub color_primary: u8,
    pub color_secondary: u8,
    pub marking: u8,
    pub eye_color: u8,
    pub curiosity: u8,
    pub playfulness: u8,
    pub affection: u8,
    pub loyalty: u8,
    pub intelligence: u8,
    pub calmness: u8,
    pub rarity: Rarity,
    pub legendary_expressed: u16,
    pub mythic_expressed: u8,
}

// ============================================================================
// Pure functions — the draw algorithm (unit-testable)
// ============================================================================

/// Devnet-scaffold RNG: splitmix64 seeded from on-chain entropy.
///
/// PREDICTABLE — a validator or observer can reproduce every roll.
/// Production MUST replace this with Switchboard VRF before mainnet.
pub struct RaffleRng {
    seed: u64,
    counter: u64,
}

impl RaffleRng {
    pub fn new(clock: &Clock, total_tickets: u32) -> Self {
        let mut bytes = [0u8; 20];
        bytes[0..8].copy_from_slice(&clock.unix_timestamp.to_le_bytes());
        bytes[8..16].copy_from_slice(&clock.slot.to_le_bytes());
        bytes[16..20].copy_from_slice(&total_tickets.to_le_bytes());
        Self::from_bytes(&bytes)
    }

    pub fn new_seeded(seed: u64) -> Self {
        Self { seed, counter: 0 }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let mut seed: u64 = 0xCBF2_9CE4_4842_2252; // FNV offset basis
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            seed ^= u64::from_le_bytes(word);
            seed = seed.wrapping_mul(0x100_0000_01B3); // FNV prime
        }
        // Avalanche once through splitmix64.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Self {
            seed: z ^ (z >> 31),
            counter: 0,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.counter = self.counter.wrapping_add(1);
        let mut z = self
            .seed
            .wrapping_add(self.counter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform random in [0, bound). Uses rejection-free modular reduction
    /// (slight modulo bias is irrelevant for devnet scaffolding).
    pub fn next_bounded(&mut self, bound: u64) -> u64 {
        debug_assert!(bound > 0);
        self.next_u64() % bound
    }
}

/// Build the 15,000-entry prize pool: 3,750 per species (Otter, Cat, Dog,
/// Monkey), each split 1,875 male / 1,875 female. Entries are packed bytes:
/// (species << 1) | gender.
pub fn build_prize_pool() -> Vec<u8> {
    let mut pool = Vec::with_capacity(PRIZE_COUNT as usize);
    for species in 0..4u8 {
        for gender in 0..2u8 {
            for _ in 0..GENDER_QUOTA {
                pool.push((species << 1) | gender);
            }
        }
    }
    debug_assert_eq!(pool.len(), PRIZE_COUNT as usize);
    pool
}

/// Fisher-Yates shuffle for byte slices (the prize pool).
pub fn fisher_yates_bytes(data: &mut [u8], rng: &mut RaffleRng) {
    for i in (1..data.len()).rev() {
        let j = rng.next_bounded((i + 1) as u64) as usize;
        data.swap(i, j);
    }
}

/// Fisher-Yates shuffle for u32 slices (the ticket order).
pub fn fisher_yates_u32(data: &mut [u32], rng: &mut RaffleRng) {
    for i in (1..data.len()).rev() {
        let j = rng.next_bounded((i + 1) as u64) as usize;
        data.swap(i, j);
    }
}

/// Split distributable lamports into (dev_share, project_share).
/// Dev gets exactly 20% (rounded down); the project keeps the rest.
pub fn split_funds(distributable: u64) -> Result<(u64, u64)> {
    let dev_share = (distributable as u128)
        .checked_mul(DEV_SHARE_BPS as u128)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR as u128))
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(RaffleError::MathOverflow)?;
    let project_share = distributable
        .checked_sub(dev_share)
        .ok_or(RaffleError::MathOverflow)?;
    Ok((dev_share, project_share))
}

/// Roll traits for a Genesis pet claim. Deterministic from the RNG —
/// same seed + rank → same pet, verifiable by anyone.
///
/// Rarity weights: Common 40%, Uncommon 30%, Rare 20%, Epic 7%,
/// Legendary 2.5%, Mythic 0.5%. The Mythic Genesis pulls are the
/// headline moments — rare enough to matter, possible enough to dream.
pub fn roll_genesis_traits(rng: &mut RaffleRng, species: Species) -> GenesisTraits {
    // Personality: uniform 20–90 (Genesis pets skew lively).
    let stat = |rng: &mut RaffleRng| 20 + rng.next_bounded(71) as u8;

    // Colors from the canonical pools (see COLOR_POOLS.md).
    let color_primary = rng.next_bounded(8) as u8;
    let color_secondary = rng.next_bounded(8) as u8;
    let marking = rng.next_bounded(16) as u8;
    let eye_color = rng.next_bounded(6) as u8;

    // Rarity roll.
    let roll = rng.next_bounded(1000);
    let rarity = if roll < 400 {
        Rarity::Common
    } else if roll < 700 {
        Rarity::Uncommon
    } else if roll < 900 {
        Rarity::Rare
    } else if roll < 970 {
        Rarity::Epic
    } else if roll < 995 {
        Rarity::Legendary
    } else {
        Rarity::Mythic
    };

    // Expressed special traits seed the gene pool. Legendary pets express
    // 1–2 random legendary traits; Mythic pets express 1 mythic mutation.
    // (Which trait index maps to which founder's trait is documented in
    // CREATOR_PETS.md — indices 0-15 legendary, 0-7 mythic.)
    let (legendary_expressed, mythic_expressed) = match rarity {
        Rarity::Legendary => {
            let t1 = rng.next_bounded(16) as u16;
            let t2 = rng.next_bounded(16) as u16;
            ((1u16 << t1) | (1u16 << t2), 0u8)
        }
        Rarity::Mythic => {
            let m = rng.next_bounded(8) as u8;
            (0u16, 1u8 << m)
        }
        _ => (0u16, 0u8),
    };

    // Species is used for future species-weighted trait tables; today the
    // pools are shared. Keep the parameter for the evolution.
    let _ = species;

    GenesisTraits {
        color_primary,
        color_secondary,
        marking,
        eye_color,
        curiosity: stat(rng),
        playfulness: stat(rng),
        affection: stat(rng),
        loyalty: stat(rng),
        intelligence: stat(rng),
        calmness: stat(rng),
        rarity,
        legendary_expressed,
        mythic_expressed,
    }
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum RaffleError {
    #[msg("Sale end must be after sale start, both positive")]
    InvalidSaleWindow,

    #[msg("Ticket sale is not currently open")]
    SaleNotOpen,

    #[msg("Ticket sale has not ended yet")]
    SaleNotEnded,

    #[msg("The draw has already been executed")]
    DrawAlreadyComplete,

    #[msg("The draw has not been executed yet")]
    DrawNotComplete,

    #[msg("No tickets were sold")]
    NoTicketsSold,

    #[msg("Whale pack cap (500) exceeded")]
    WhaleCapExceeded,

    #[msg("Ticket purchase record does not belong to this buyer")]
    TicketOwnerMismatch,

    #[msg("Ticket index is not in the signer's owned range")]
    TicketNotOwned,

    #[msg("Rank is out of range for this draw")]
    InvalidRank,

    #[msg("This ticket did not win")]
    NotAWinner,

    #[msg("This prize has already been claimed")]
    AlreadyClaimed,

    #[msg("Signer is not the raffle authority")]
    UnauthorizedAuthority,

    #[msg("Funds have already been distributed")]
    FundsAlreadyDistributed,

    #[msg("Dev wallet does not match raffle config")]
    WrongDevWallet,

    #[msg("Nothing to distribute")]
    NothingToDistribute,

    #[msg("Arithmetic overflow")]
    MathOverflow,
}

// ============================================================================
// Tests — pure-function unit tests (no validator needed)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_rng() -> RaffleRng {
        RaffleRng::new_seeded(0xDEAD_BEEF_CAFE_1234)
    }

    // --- Prize pool quotas ---------------------------------------------------

    #[test]
    fn prize_pool_has_correct_quotas() {
        let pool = build_prize_pool();
        assert_eq!(pool.len(), PRIZE_COUNT as usize);

        // Count per species and per gender.
        let mut species_counts = [0u32; 4];
        let mut gender_counts = [0u32; 2];
        for &packed in &pool {
            let species = (packed >> 1) & 0x03;
            let gender = packed & 0x01;
            species_counts[species as usize] += 1;
            gender_counts[gender as usize] += 1;
        }
        for (i, &c) in species_counts.iter().enumerate() {
            assert_eq!(c, SPECIES_QUOTA, "species {}", i);
        }
        // 7,500 male + 7,500 female total (50/50).
        assert_eq!(gender_counts[0], 7_500);
        assert_eq!(gender_counts[1], 7_500);

        // Per species per gender: 1,875 each.
        for species in 0..4u8 {
            for gender in 0..2u8 {
                let packed = (species << 1) | gender;
                let count = pool.iter().filter(|&&p| p == packed).count();
                assert_eq!(count, GENDER_QUOTA as usize);
            }
        }
    }

    #[test]
    fn shuffle_preserves_quotas() {
        let mut pool = build_prize_pool();
        let mut rng = test_rng();
        fisher_yates_bytes(&mut pool, &mut rng);
        // Same multiset after shuffling.
        let mut sorted = pool.clone();
        sorted.sort_unstable();
        let mut expected = build_prize_pool();
        expected.sort_unstable();
        assert_eq!(sorted, expected);
        // And it's actually shuffled (overwhelmingly likely).
        assert_ne!(pool, build_prize_pool());
    }

    #[test]
    fn ticket_shuffle_selects_unique_winners() {
        let total = 30_000u32;
        let mut order: Vec<u32> = (0..total).collect();
        let mut rng = test_rng();
        fisher_yates_u32(&mut order, &mut rng);
        order.truncate(PRIZE_COUNT as usize);
        assert_eq!(order.len(), PRIZE_COUNT as usize);
        let mut sorted = order.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), PRIZE_COUNT as usize, "winners must be unique");
        assert!(order.iter().all(|&t| t < total));
    }

    // --- Fund split ----------------------------------------------------------

    #[test]
    fn fund_split_is_20_80() {
        let (dev, project) = split_funds(1_000_000_000).unwrap();
        assert_eq!(dev, 200_000_000);
        assert_eq!(project, 800_000_000);
    }

    #[test]
    fn fund_split_sums_to_distributable() {
        for amount in [1u64, 7, 999, 1_000_000_000, u64::MAX / 2] {
            let (dev, project) = split_funds(amount).unwrap();
            assert_eq!(dev + project, amount, "amount {}", amount);
        }
    }

    #[test]
    fn fund_split_rounds_dev_down() {
        // 7 lamports: 20% = 1.4 → dev gets 1, project gets 6.
        let (dev, project) = split_funds(7).unwrap();
        assert_eq!(dev, 1);
        assert_eq!(project, 6);
    }

    // --- Tier pricing ----------------------------------------------------------

    #[test]
    fn tier_prices_match_marketing_plan() {
        assert_eq!(
            TicketTier::Single.price_and_count(),
            (250_000_000, 1)
        );
        assert_eq!(
            TicketTier::FivePack.price_and_count(),
            (1_000_000_000, 5)
        );
        assert_eq!(
            TicketTier::WhalePack.price_and_count(),
            (1_250_000_000, 10)
        );
    }

    #[test]
    fn tier_per_ticket_discounts() {
        // Single: 0.25/ticket. 5-pack: 0.20/ticket (20% off). Whale: 0.125/ticket (50% off).
        let (single_price, _) = TicketTier::Single.price_and_count();
        let (five_price, five_n) = TicketTier::FivePack.price_and_count();
        let (whale_price, whale_n) = TicketTier::WhalePack.price_and_count();
        assert_eq!(five_price / five_n as u64, single_price * 4 / 5);
        assert_eq!(whale_price / whale_n as u64, single_price / 2);
    }

    // --- Genesis trait rolls ---------------------------------------------------

    #[test]
    fn genesis_traits_in_valid_ranges() {
        let mut rng = test_rng();
        for _ in 0..100 {
            let t = roll_genesis_traits(&mut rng, Species::Otter);
            assert!(t.color_primary < 8);
            assert!(t.color_secondary < 8);
            assert!(t.marking < 16);
            assert!(t.eye_color < 6);
            for stat in [
                t.curiosity,
                t.playfulness,
                t.affection,
                t.loyalty,
                t.intelligence,
                t.calmness,
            ] {
                assert!((20..=90).contains(&stat));
            }
        }
    }

    #[test]
    fn genesis_rarity_distribution_is_sane() {
        // 20,000 rolls: mythic should appear but stay rare (<2%).
        let mut rng = test_rng();
        let mut mythic = 0u32;
        let mut legendary = 0u32;
        for _ in 0..20_000 {
            match roll_genesis_traits(&mut rng, Species::Cat).rarity {
                Rarity::Mythic => mythic += 1,
                Rarity::Legendary => legendary += 1,
                _ => {}
            }
        }
        assert!(mythic > 0, "mythic should be possible");
        assert!(mythic < 400, "mythic must stay rare, got {}", mythic);
        assert!(legendary > 100 && legendary < 1_200, "legendary out of band: {}", legendary);
    }

    #[test]
    fn legendary_roll_expresses_traits() {
        // Force many rolls; every Legendary must express ≥1 trait bit.
        let mut rng = test_rng();
        let mut checked = 0;
        for _ in 0..20_000 {
            let t = roll_genesis_traits(&mut rng, Species::Dog);
            if t.rarity == Rarity::Legendary {
                assert_ne!(t.legendary_expressed, 0);
                assert_eq!(t.mythic_expressed, 0);
                checked += 1;
            }
            if t.rarity == Rarity::Mythic {
                assert_ne!(t.mythic_expressed, 0);
                assert_eq!(t.legendary_expressed, 0);
                checked += 1;
            }
            if checked > 50 {
                break;
            }
        }
        assert!(checked > 0);
    }

    #[test]
    fn genesis_rolls_are_deterministic() {
        fn rarity_rank(r: Rarity) -> u8 {
            match r {
                Rarity::Common => 0,
                Rarity::Uncommon => 1,
                Rarity::Rare => 2,
                Rarity::Epic => 3,
                Rarity::Legendary => 4,
                Rarity::Mythic => 5,
            }
        }
        let mut r1 = RaffleRng::new_seeded(42);
        let mut r2 = RaffleRng::new_seeded(42);
        for _ in 0..10 {
            let t1 = roll_genesis_traits(&mut r1, Species::Monkey);
            let t2 = roll_genesis_traits(&mut r2, Species::Monkey);
            assert_eq!(t1.color_primary, t2.color_primary);
            assert_eq!(rarity_rank(t1.rarity), rarity_rank(t2.rarity));
        }
    }

    // --- Account sizes -----------------------------------------------------------

    #[test]
    fn account_sizes_are_sane() {
        assert_eq!(RaffleConfig::SIZE, 92);
        assert_eq!(TicketPurchase::SIZE, 53);
        // DrawResult space must fit the full draw.
        let need = 4 + (4 + 15_000 * 4) + (4 + 15_000) + (4 + 1_875) + 8 + 1;
        assert_eq!(DrawResult::SPACE, need);
    }

    #[test]
    fn prize_packing_round_trips() {
        for species in 0..4u8 {
            for gender in 0..2u8 {
                let packed = (species << 1) | gender;
                assert_eq!((packed >> 1) & 0x03, species);
                assert_eq!(packed & 0x01, gender);
            }
        }
    }
}
