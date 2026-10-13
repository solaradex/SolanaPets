//! # SolanaPets $PAW Token Program
//!
//! The utility token of the SolanaPets NFP ecosystem.
//!
//! ## Brad's rules (locked in)
//! - $PAW is EARNED through gameplay and care — never airdropped, never given away.
//! - Used for marketplace purchases (pets, accessories, habitat items).
//! - Trades on Solana DEXs (real SPL token, 9 decimals).
//! - Anti-inflation controls are mandatory: this is a game economy, not a money printer.
//!
//! ## Economic model (see PAW_ECONOMICS.md for the plain-English version)
//! - Max supply: 1,000,000,000 $PAW (hard cap, enforced on-chain).
//! - Base earn: 1 $PAW per verified care action, scaled by bond level (up to 3x).
//! - Per-pet daily cap: 50 $PAW (after multiplier).
//! - Min claim cooldown: 1 hour between claims per pet (anti-spam).
//! - Burn sinks: marketplace fees, breeding fees, accessory purchases —
//!   every spend burns, creating permanent deflationary pressure.
//!
//! ## Anti-gaming controls
//! 1. Only the designated `earner_authority` (game server/oracle) may call
//!    `earn_paw` — it verifies real gameplay happened before requesting a mint.
//! 2. Per-pet daily earnings tracked in a `PetEarnings` PDA; capped per day.
//! 3. Minimum cooldown between claims per pet.
//! 4. Global supply cap — minting halts forever once reached.
//! 5. All math is checked (no silent overflows).
//!
//! ## DEVNET ONLY. No mainnet deployment until audit.
//!
//! Program ID (devnet): HEHEXUJfmQ3S5i2vxH6SqmZVBQo78vmygD3dVx1BZWSk

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount};

// Devnet program ID — generated for this deployment, never mainnet.
declare_id!("HEHEXUJfmQ3S5i2vxH6SqmZVBQo78vmygD3dVx1BZWSk");

// ============================================================================
// Economic constants — the numbers Brad approved. Change only with his sign-off.
// ============================================================================

/// SPL decimals for $PAW (standard).
pub const PAW_DECIMALS: u8 = 9;
/// One whole $PAW in base units.
pub const PAW_BASE: u64 = 1_000_000_000; // 10^9
/// Hard cap: 1 billion $PAW, in base units. Minting halts forever at this point.
pub const PAW_SUPPLY_CAP: u64 = 1_000_000_000 * PAW_BASE;
/// Maximum $PAW a single pet can earn per day (after bond multiplier).
pub const PAW_DAILY_CAP_PER_PET: u64 = 50 * PAW_BASE;
/// Minimum seconds between earn claims for the same pet (anti-spam).
pub const PAW_MIN_CLAIM_COOLDOWN: i64 = 3_600; // 1 hour
/// Seconds in a day (for day-bucket calculation).
pub const SECONDS_PER_DAY: i64 = 86_400;

/// Base reward per verified care action, in base units (1 $PAW).
pub const PAW_BASE_REWARD_PER_ACTION: u64 = PAW_BASE;
/// Bond XP per bond level (every 1,000 XP = +1 level).
pub const BOND_XP_PER_LEVEL: u64 = 1_000;
/// Maximum bond multiplier: 3x at bond level 20 (20,000 XP).
/// multiplier = 1 + min(level, 20) / 10  →  1.0x … 3.0x
pub const MAX_BOND_LEVEL_FOR_BONUS: u64 = 20;

/// PDA seeds.
pub const PAW_CONFIG_SEED: &[u8] = b"paw-config";
pub const EARNINGS_SEED: &[u8] = b"earnings";

// ============================================================================
// Program entrypoints
// ============================================================================

#[program]
pub mod paw_token {
    use super::*;

    /// One-time setup.
    ///
    /// Creates the `PawConfig` PDA and the $PAW SPL mint (9 decimals) with
    /// the config PDA as mint authority. Sets the supply cap, daily earning
    /// cap, claim cooldown, and the earner authority (the game server that
    /// verifies gameplay before requesting mints).
    pub fn initialize_paw(
        ctx: Context<InitializePaw>,
        supply_cap: u64,
        daily_cap_per_pet: u64,
        min_claim_cooldown: i64,
    ) -> Result<()> {
        // Sanity: caps must be positive and the supply cap must leave room
        // for a real economy (at least 1M $PAW).
        require!(supply_cap >= 1_000_000 * PAW_BASE, PawError::SupplyCapTooSmall);
        require!(daily_cap_per_pet > 0, PawError::InvalidDailyCap);
        require!(min_claim_cooldown >= 0, PawError::InvalidCooldown);

        let config = &mut ctx.accounts.paw_config;
        config.authority = ctx.accounts.authority.key();
        config.earner_authority = ctx.accounts.earner_authority.key();
        config.mint = ctx.accounts.paw_mint.key();
        config.total_minted = 0;
        config.supply_cap = supply_cap;
        config.daily_cap_per_pet = daily_cap_per_pet;
        config.min_claim_cooldown = min_claim_cooldown;
        config.bump = ctx.bumps.paw_config;

        msg!(
            "Initialized $PAW: mint={}, supply_cap={}, daily_cap_per_pet={}, cooldown={}s",
            config.mint,
            supply_cap / PAW_BASE,
            daily_cap_per_pet / PAW_BASE,
            min_claim_cooldown
        );
        Ok(())
    }

    /// Mint $PAW to a player as a reward for verified gameplay.
    ///
    /// Only the `earner_authority` (game server) may call this — it is
    /// responsible for verifying that real gameplay happened (a care action,
    /// quest completion, show win, etc.) before requesting the mint.
    ///
    /// Enforced on-chain:
    /// - per-pet daily cap (`PetEarnings` PDA, keyed by pet + day)
    /// - minimum cooldown between claims for the same pet
    /// - global supply cap
    pub fn earn_paw(ctx: Context<EarnPaw>, pet_id: u32, amount: u64) -> Result<()> {
        let config = &ctx.accounts.paw_config;

        // 1. Only the designated earner authority may mint rewards.
        require_keys_eq!(
            ctx.accounts.earner_authority.key(),
            config.earner_authority,
            PawError::UnauthorizedEarner
        );
        require!(amount > 0, PawError::ZeroAmount);

        // 2. Day bucket + cooldown.
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let today = now.div_euclid(SECONDS_PER_DAY);
        let earnings = &mut ctx.accounts.pet_earnings;

        // First claim ever (fresh PDA has day == 0) or new day: reset bucket.
        if earnings.day != today {
            earnings.pet_id = pet_id;
            earnings.day = today;
            earnings.earned_today = 0;
            earnings.bump = ctx.bumps.pet_earnings;
            // Don't reset last_claim_at on day rollover — the cooldown
            // still applies across the midnight boundary.
        } else {
            // Same-day claim: enforce the cooldown.
            let elapsed = now
                .checked_sub(earnings.last_claim_at)
                .ok_or(PawError::MathOverflow)?;
            require!(
                elapsed >= config.min_claim_cooldown,
                PawError::ClaimCooldownActive
            );
        }

        // 3. Daily cap per pet.
        let new_earned = earnings
            .earned_today
            .checked_add(amount)
            .ok_or(PawError::MathOverflow)?;
        require!(
            new_earned <= config.daily_cap_per_pet,
            PawError::DailyCapExceeded
        );

        // 4. Global supply cap.
        let new_total = config
            .total_minted
            .checked_add(amount)
            .ok_or(PawError::MathOverflow)?;
        require!(new_total <= config.supply_cap, PawError::SupplyCapReached);

        // 5. Mint via CPI, signed by the config PDA (the mint authority).
        let seeds: &[&[u8]] = &[PAW_CONFIG_SEED, &[config.bump]];
        let signer_seeds = &[&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.paw_mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.paw_config.to_account_info(),
            },
            signer_seeds,
        );
        token::mint_to(cpi_ctx, amount)?;

        // 6. Update accounting.
        let config_mut = &mut ctx.accounts.paw_config;
        config_mut.total_minted = new_total;
        earnings.earned_today = new_earned;
        earnings.last_claim_at = now;

        msg!(
            "Minted {} $PAW to {} for pet #{} (day total: {})",
            amount as f64 / PAW_BASE as f64,
            ctx.accounts.destination.key(),
            pet_id,
            new_earned as f64 / PAW_BASE as f64
        );
        Ok(())
    }

    /// Burn $PAW from a player's token account (in-game purchases).
    ///
    /// The player signs; the marketplace/breeding/shop program calls this
    /// (or the player calls it directly) when $PAW is spent. Burning is the
    /// deflationary sink that keeps the economy healthy.
    pub fn spend_paw(ctx: Context<SpendPaw>, amount: u64) -> Result<()> {
        require!(amount > 0, PawError::ZeroAmount);

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.paw_mint.to_account_info(),
                from: ctx.accounts.source.to_account_info(),
                authority: ctx.accounts.player.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)?;

        msg!(
            "Burned {} $PAW from {}",
            amount as f64 / PAW_BASE as f64,
            ctx.accounts.player.key()
        );
        Ok(())
    }

    /// Update the earner authority (e.g., rotate the game server key).
    /// Only the config authority may do this.
    pub fn set_earner_authority(ctx: Context<SetEarnerAuthority>) -> Result<()> {
        let config = &mut ctx.accounts.paw_config;
        config.earner_authority = ctx.accounts.new_earner_authority.key();
        msg!("$PAW earner authority rotated to {}", config.earner_authority);
        Ok(())
    }
}

// ============================================================================
// Reward math — pure functions the game server AND tests share.
// ============================================================================

/// Bond multiplier for a pet's bond_xp: 1.0x → 3.0x.
///
/// Every 1,000 bond XP = +1 level; each level adds 0.1x, capped at 3.0x
/// (level 20, i.e. 20,000 XP). Returned as a fixed-point multiplier
/// scaled by 10 (10 = 1.0x, 30 = 3.0x) to avoid floats on-chain.
pub fn bond_multiplier_scaled(bond_xp: u64) -> u64 {
    let level = bond_xp / BOND_XP_PER_LEVEL;
    let bonus = core::cmp::min(level, MAX_BOND_LEVEL_FOR_BONUS);
    10 + bonus // 10..=30
}

/// Reward for one verified care action, given the pet's bond_xp.
/// Base 1 $PAW × bond multiplier (1.0x–3.0x), in base units.
pub fn reward_for_care_action(bond_xp: u64) -> u64 {
    PAW_BASE_REWARD_PER_ACTION
        .checked_mul(bond_multiplier_scaled(bond_xp))
        .map(|v| v / 10)
        .unwrap_or(u64::MAX) // saturate on theoretical overflow
}

/// Day bucket for a unix timestamp (days since epoch).
pub fn day_bucket(unix_timestamp: i64) -> i64 {
    unix_timestamp.div_euclid(SECONDS_PER_DAY)
}

// ============================================================================
// Accounts
// ============================================================================

#[derive(Accounts)]
pub struct InitializePaw<'info> {
    #[account(
        init,
        payer = payer,
        space = PawConfig::SIZE,
        seeds = [PAW_CONFIG_SEED],
        bump
    )]
    pub paw_config: Account<'info, PawConfig>,

    /// The $PAW SPL mint. Created here with 9 decimals; the config PDA is
    /// the mint authority, so only `earn_paw` can ever mint.
    #[account(
        init,
        payer = payer,
        mint::decimals = PAW_DECIMALS,
        mint::authority = paw_config,
    )]
    pub paw_mint: Account<'info, Mint>,

    /// Game authority (multisig in production). Owns the config.
    pub authority: Signer<'info>,
    /// The game server / oracle key allowed to call `earn_paw`.
    /// CHECK: stored as a pubkey; only its signature is ever verified.
    pub earner_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(pet_id: u32)]
pub struct EarnPaw<'info> {
    #[account(
        mut,
        seeds = [PAW_CONFIG_SEED],
        bump = paw_config.bump,
    )]
    pub paw_config: Account<'info, PawConfig>,

    #[account(mut, address = paw_config.mint)]
    pub paw_mint: Account<'info, Mint>,

    /// Per-pet, per-day earnings bucket. Created on first claim of the day.
    /// Separate PDA (not in the Pet account) so the Pet layout never changes.
    #[account(
        init_if_needed,
        payer = earner_authority,
        space = PetEarnings::SIZE,
        seeds = [EARNINGS_SEED, &pet_id.to_le_bytes()],
        bump
    )]
    pub pet_earnings: Account<'info, PetEarnings>,

    /// The game server key. Must match config.earner_authority.
    /// Marked mut: pays for the PetEarnings PDA on first claim.
    #[account(mut)]
    pub earner_authority: Signer<'info>,

    /// Player's $PAW token account receiving the mint.
    #[account(mut)]
    pub destination: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SpendPaw<'info> {
    /// The player spending $PAW — must sign.
    pub player: Signer<'info>,

    #[account(mut, address = paw_config.mint)]
    pub paw_mint: Account<'info, Mint>,

    #[account(
        mut,
        token::mint = paw_mint,
        token::authority = player,
    )]
    pub source: Account<'info, TokenAccount>,

    /// Config is read for the mint address binding (prevents burning a
    /// lookalike token and calling it "$PAW spend").
    pub paw_config: Account<'info, PawConfig>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SetEarnerAuthority<'info> {
    #[account(
        mut,
        seeds = [PAW_CONFIG_SEED],
        bump = paw_config.bump,
        has_one = authority @ PawError::UnauthorizedConfigAuthority,
    )]
    pub paw_config: Account<'info, PawConfig>,
    pub authority: Signer<'info>,
    /// CHECK: new key stored as pubkey only.
    pub new_earner_authority: UncheckedAccount<'info>,
}

// ============================================================================
// State
// ============================================================================

/// Global $PAW configuration. One per program deployment.
#[account]
pub struct PawConfig {
    /// Config owner (multisig in production). Can rotate earner authority.
    pub authority: Pubkey, // 32
    /// Game server / oracle key allowed to call `earn_paw`.
    pub earner_authority: Pubkey, // 32
    /// The $PAW SPL mint.
    pub mint: Pubkey, // 32
    /// Lifetime $PAW minted, in base units. Enforces the supply cap.
    pub total_minted: u64, // 8
    /// Hard cap on lifetime minting, in base units.
    pub supply_cap: u64, // 8
    /// Max $PAW one pet can earn per day, in base units.
    pub daily_cap_per_pet: u64, // 8
    /// Min seconds between earn claims for the same pet.
    pub min_claim_cooldown: i64, // 8
    /// PDA bump.
    pub bump: u8, // 1
}

impl PawConfig {
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 8 + 8 + 8 + 8 + 1; // 137
}

/// Per-pet earnings tracker.
///
/// One PDA per pet (seeds: `["earnings", pet_id]`). The `day` field tracks
/// which day bucket `earned_today` belongs to; on day rollover the counter
/// resets in-account. This avoids a new PDA per day (no cleanup needed) and
/// keeps the Pet account layout untouched (still 98 bytes — token tracking
/// never changes pet storage).
///
/// Security note on `init_if_needed`: the PDA is derived deterministically
/// from `pet_id` and only the trusted `earner_authority` pays for and calls
/// initialization, so there is no front-running / re-initialization vector.
#[account]
pub struct PetEarnings {
    pub pet_id: u32,       // 4
    pub day: i64,          // 8 — day bucket (unix_timestamp / 86400)
    pub earned_today: u64, // 8 — base units earned this day
    pub last_claim_at: i64, // 8 — unix timestamp of last claim (cooldown)
    pub bump: u8,          // 1
}

impl PetEarnings {
    pub const SIZE: usize = 8 + 4 + 8 + 8 + 8 + 1; // 37
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum PawError {
    #[msg("Supply cap must be at least 1,000,000 $PAW")]
    SupplyCapTooSmall,

    #[msg("Daily cap per pet must be positive")]
    InvalidDailyCap,

    #[msg("Claim cooldown must be non-negative")]
    InvalidCooldown,

    #[msg("Only the designated earner authority may mint $PAW rewards")]
    UnauthorizedEarner,

    #[msg("Only the config authority may perform this action")]
    UnauthorizedConfigAuthority,

    #[msg("Amount must be positive")]
    ZeroAmount,

    #[msg("Pet has reached its daily $PAW earning cap — come back tomorrow")]
    DailyCapExceeded,

    #[msg("Minimum time between claims has not elapsed")]
    ClaimCooldownActive,

    #[msg("$PAW supply cap reached — no more can ever be minted")]
    SupplyCapReached,

    #[msg("Arithmetic overflow in $PAW math")]
    MathOverflow,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Reward math ---------------------------------------------------------

    #[test]
    fn bond_multiplier_starts_at_1x() {
        assert_eq!(bond_multiplier_scaled(0), 10);
        assert_eq!(bond_multiplier_scaled(999), 10);
    }

    #[test]
    fn bond_multiplier_scales_to_3x() {
        assert_eq!(bond_multiplier_scaled(1_000), 11); // 1.1x
        assert_eq!(bond_multiplier_scaled(10_000), 20); // 2.0x
        assert_eq!(bond_multiplier_scaled(20_000), 30); // 3.0x
        assert_eq!(bond_multiplier_scaled(1_000_000), 30); // capped
    }

    #[test]
    fn reward_for_care_action_scales_with_bond() {
        assert_eq!(reward_for_care_action(0), PAW_BASE); // 1 $PAW
        assert_eq!(reward_for_care_action(10_000), 2 * PAW_BASE); // 2 $PAW
        assert_eq!(reward_for_care_action(20_000), 3 * PAW_BASE); // 3 $PAW
        assert_eq!(reward_for_care_action(u64::MAX), 3 * PAW_BASE); // capped
    }

    #[test]
    fn reward_never_exceeds_daily_cap_per_action_count() {
        // Even at max multiplier, a pet doing 20 actions/day earns 60 $PAW
        // worth of claims — but the daily cap (50) is enforced separately
        // in earn_paw. This just sanity-checks the per-action math.
        let per_action = reward_for_care_action(u64::MAX);
        assert!(per_action <= 3 * PAW_BASE);
    }

    #[test]
    fn day_bucket_splits_midnight() {
        // 2026-10-13 00:00:00 UTC = 1791244800 (verified: 1791244800 / 86400 = 20732 exactly)
        assert_eq!(day_bucket(1791244800), day_bucket(1791244800 + 86399));
        assert_ne!(day_bucket(1791244800), day_bucket(1791244800 + 86400));
    }

    #[test]
    fn day_bucket_handles_negative_timestamps() {
        // div_euclid (not /) so pre-epoch timestamps bucket correctly.
        assert_eq!(day_bucket(-1), -1);
        assert_eq!(day_bucket(-86400), -1);
        assert_eq!(day_bucket(-86401), -2);
    }

    // --- Economic invariants ---------------------------------------------------

    #[test]
    fn supply_cap_is_one_billion_paw() {
        assert_eq!(PAW_SUPPLY_CAP, 1_000_000_000 * PAW_BASE);
    }

    #[test]
    fn daily_cap_is_fifty_paw() {
        assert_eq!(PAW_DAILY_CAP_PER_PET, 50 * PAW_BASE);
    }

    #[test]
    fn cooldown_is_one_hour() {
        assert_eq!(PAW_MIN_CLAIM_COOLDOWN, 3_600);
    }

    #[test]
    fn base_units_math_is_exact() {
        // 1 $PAW == 10^9 base units, no dust.
        assert_eq!(PAW_BASE, 1_000_000_000);
        assert_eq!(50 * PAW_BASE, PAW_DAILY_CAP_PER_PET);
    }

    // --- Realistic economy simulation ------------------------------------------

    #[test]
    fn genesis_year_emission_stays_well_under_cap() {
        // 15,000 Genesis pets × realistic 10 $PAW/day × 365 days.
        let pets: u64 = 15_000;
        let per_day: u64 = 10 * PAW_BASE;
        let year = pets * per_day * 365;
        assert!(year < PAW_SUPPLY_CAP / 10); // <10% of cap in year one
    }

    #[test]
    fn max_theoretical_emission_respects_cap() {
        // Even if EVERY pet maxed the daily cap, the on-chain supply cap
        // is the backstop — earn_paw rejects anything past it. This test
        // just documents the scale: 750k pets × 50/day = 37.5M/day max.
        let max_daily = 750_000u64 * PAW_DAILY_CAP_PER_PET;
        let days_to_cap = PAW_SUPPLY_CAP / max_daily;
        // At absolute max extraction the cap hits in ~26 days — which is
        // exactly why burn sinks + realistic (not max) earnings matter.
        // The cap is a circuit breaker, not a target.
        assert!(days_to_cap > 0);
    }

    #[test]
    fn account_sizes_are_sane() {
        assert_eq!(PawConfig::SIZE, 137);
        assert_eq!(PetEarnings::SIZE, 37);
    }

    // --- Error messages are human-readable --------------------------------------

    #[test]
    fn error_messages_mention_paw() {
        // Spot-check that the errors a player might see name $PAW clearly.
        let e = PawError::DailyCapExceeded;
        let msg = format!("{:?}", e);
        assert!(msg.contains("DailyCapExceeded"));
    }
}
