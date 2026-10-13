//! # SolanaPets Pet Registry
//!
//! The foundation of the SolanaPets NFP (Non-Fungible Pet) ecosystem.
//!
//! Unlike NFTs — static collectibles — every NFP minted here is a LIVING
//! on-chain pet. All trait data, lineage, care stats, and health live in the
//! Pet account. Nothing about a pet's identity lives off-chain.
//!
//! ## Design notes
//! - Pet IDs are sequential (1–750,000). IDs 1–8 are reserved for Brad's
//!   8 creator pets (Luna, Lenny, Lucy, Loboe, Lilah, Leo, Lola, Loki) and
//!   can NEVER be minted through `mint_nfp` — they get a dedicated
//!   instruction with mythic traits later.
//! - Public minting starts at ID 9. The first 15,000 pets are Genesis.
//! - Only the registry `authority` (a multisig in production) can mint.
//!   This is what the raffle program and breeding program will call.
//! - DEVNET ONLY. No mainnet deployment until audit.
//!
//! Program ID (devnet): HZhS2qw6edrL7YKjUZopjJGdSfh9UJniVCAZ9ZMkQFkz

use anchor_lang::prelude::*;

// Devnet program ID — generated for this deployment, never mainnet.
declare_id!("HZhS2qw6edrL7YKjUZopjJGdSfh9UJniVCAZ9ZMkQFkz");

// ============================================================================
// Constants
// ============================================================================

/// Absolute cap on the number of NFPs that can ever exist.
pub const MAX_PETS: u32 = 750_000;
/// First 15,000 pets (IDs 9–15,008... see note) are Genesis.
/// Note: IDs 1–8 are creator pets, so Genesis = IDs 1–15,000 inclusive.
pub const GENESIS_SUPPLY: u32 = 15_000;
/// IDs 1–8 are reserved for the 8 creator pets. Never mintable via mint_nfp.
pub const CREATOR_IDS_END: u32 = 8;
/// Public minting begins here.
pub const FIRST_PUBLIC_ID: u32 = 9;

/// Trait value bounds (validated on mint).
pub const MAX_COLOR_GENE: u8 = 7; // 8 interchangeable coat colors (0–7)
pub const MAX_EYE_COLOR: u8 = 5; // 6 eye colors (0–5)
pub const MAX_MARKING: u8 = 15; // species-specific markings (0–15)
pub const MAX_STAT: u8 = 100; // care stats and personality traits are 0–100

/// Canonical pool sizes — see COLOR_POOLS.md (sourced from Brad's 8
/// Genesis Founder character sheets). The on-chain trait is an index
/// into the per-species pool; names live in `coat_color_name` /
/// `eye_color_name` below.
pub const COAT_POOL_SIZE: u8 = 8; // == MAX_COLOR_GENE + 1
pub const EYE_POOL_SIZE: u8 = 6; // == MAX_EYE_COLOR + 1

/// Creator-pet trait canon — see CREATOR_PETS.md (locked in by Brad,
/// October 13, 2026). Each of the 8 founders carries exactly this many
/// legendary traits and mythic mutations, all fully visual. These traits
/// seed the gene pool every future pet carries (expression is rare —
/// see the low-penetrance model in CREATOR_PETS.md).
pub const CREATOR_PET_COUNT: u8 = 8; // == CREATOR_IDS_END
pub const LEGENDARY_TRAITS_PER_CREATOR: u8 = 2;
pub const MYTHIC_MUTATIONS_PER_CREATOR: u8 = 1;

/// PDA seeds.
pub const REGISTRY_SEED: &[u8] = b"registry";
pub const PET_SEED: &[u8] = b"pet";

// ============================================================================
// Care tuning — all amounts are in stat points (0–100) or bond XP.
// ============================================================================

/// Bond XP granted per care action.
pub const BOND_XP_FEED: u64 = 5;
pub const BOND_XP_PLAY: u64 = 10;
pub const BOND_XP_CLEAN: u64 = 5;
pub const BOND_XP_REST: u64 = 3;
pub const BOND_XP_AFFECTION: u64 = 15;

/// Stat deltas per action.
pub const FEED_AMOUNT: u8 = 25;
pub const PLAY_HAPPINESS_GAIN: u8 = 20;
pub const PLAY_ENERGY_COST: u8 = 10;
pub const REST_ENERGY_GAIN: u8 = 30;
pub const AFFECTION_GAIN: u8 = 20;

/// When every care stat is at least this high, health_score ticks up by 1.
pub const HEALTH_BOOST_THRESHOLD: u8 = 80;

// ============================================================================
// Program entrypoints
// ============================================================================

#[program]
pub mod pet_registry {
    use super::*;

    /// One-time setup: creates the Registry PDA, sets the mint authority,
    /// and starts public minting at ID 9 (IDs 1–8 stay reserved forever).
    pub fn initialize_registry(ctx: Context<InitializeRegistry>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.authority = ctx.accounts.authority.key();
        registry.next_id = FIRST_PUBLIC_ID;
        registry.total_minted = 0;
        registry.bump = ctx.bumps.registry;

        msg!(
            "Pet registry initialized. Public minting starts at ID {}. IDs 1-{} reserved for creator pets.",
            FIRST_PUBLIC_ID,
            CREATOR_IDS_END
        );
        Ok(())
    }

    /// Mint a new living NFP. Assigns the next sequential ID from the
    /// registry, validates all traits, and writes the full Pet account.
    ///
    /// Only the registry authority may call this (enforced via has_one).
    /// Newborns start at full care stats — they're babies, freshly loved.
    pub fn mint_nfp(ctx: Context<MintNfp>, params: MintNfpParams) -> Result<()> {
        let registry = &mut ctx.accounts.registry;

        // --- ID guards -------------------------------------------------------
        // Belt-and-suspenders: registry starts at 9, but never trust state.
        require!(
            registry.next_id > CREATOR_IDS_END,
            PetRegistryError::CreatorIdsReserved
        );
        require!(
            registry.next_id <= MAX_PETS,
            PetRegistryError::MaxSupplyReached
        );

        // --- Trait validation (pure fn, unit-tested) -------------------------
        validate_mint_params(&params)?;

        // --- Write the pet ---------------------------------------------------
        let pet = &mut ctx.accounts.pet;
        let clock = Clock::get()?;

        pet.id = registry.next_id;
        pet.species = params.species;
        pet.gender = params.gender;
        pet.generation = 0; // founders/genesis are generation 0
        pet.color_primary = params.color_primary;
        pet.color_secondary = params.color_secondary;
        pet.marking = params.marking;
        pet.eye_color = params.eye_color;

        // Personality: rolled by the caller (raffle/breeding/game client).
        // Must already be 0–100 — validated above.
        pet.curiosity = params.curiosity;
        pet.playfulness = params.playfulness;
        pet.affection = params.affection;
        pet.loyalty = params.loyalty;
        pet.intelligence = params.intelligence;
        pet.calmness = params.calmness;

        pet.rarity = params.rarity;
        pet.parent_a = 0; // no parents — founder mint
        pet.parent_b = 0;
        pet.owner = params.owner;
        pet.born_at = clock.unix_timestamp;
        pet.life_stage = LifeStage::Baby;

        // Newborns start full and healthy.
        pet.hunger = MAX_STAT;
        pet.happiness = MAX_STAT;
        pet.cleanliness = MAX_STAT;
        pet.energy = MAX_STAT;
        pet.love = MAX_STAT;
        pet.health_score = MAX_STAT;
        pet.bond_xp = 0;
        pet.last_cared_at = clock.unix_timestamp;
        pet.legendary_expressed = params.legendary_expressed;
        pet.mythic_expressed = params.mythic_expressed;
        pet.breed_cooldown_until = 0; // ready to breed once Adult
        pet.bump = ctx.bumps.pet;

        // --- Advance the registry --------------------------------------------
        registry.next_id = registry
            .next_id
            .checked_add(1)
            .ok_or(PetRegistryError::MaxSupplyReached)?;
        registry.total_minted = registry.total_minted.checked_add(1).unwrap();

        msg!(
            "Minted NFP #{} ({:?} {:?}) for {}",
            pet.id,
            pet.species,
            pet.gender,
            pet.owner
        );
        Ok(())
    }

    // ========================================================================
    // Care actions — only the pet's owner or the registry authority may act.
    // Each action restores a care stat, grants bond XP, stamps last_cared_at,
    // and may nudge health_score when all stats are thriving.
    // ========================================================================

    /// Feed the pet: +hunger, small bond gain.
    pub fn feed_pet(ctx: Context<CareForPet>) -> Result<()> {
        authorize_caregiver(&ctx)?;
        let now = Clock::get()?.unix_timestamp;
        let pet = &mut ctx.accounts.pet;
        apply_feed(pet, now)?;
        msg!("Fed NFP #{} — hunger: {}", pet.id, pet.hunger);
        Ok(())
    }

    /// Play with the pet: +happiness, −energy, solid bond gain.
    pub fn play_with_pet(ctx: Context<CareForPet>) -> Result<()> {
        authorize_caregiver(&ctx)?;
        let now = Clock::get()?.unix_timestamp;
        let pet = &mut ctx.accounts.pet;
        apply_play(pet, now)?;
        msg!(
            "Played with NFP #{} — happiness: {}, energy: {}",
            pet.id,
            pet.happiness,
            pet.energy
        );
        Ok(())
    }

    /// Clean the pet: cleanliness restored to full.
    pub fn clean_pet(ctx: Context<CareForPet>) -> Result<()> {
        authorize_caregiver(&ctx)?;
        let now = Clock::get()?.unix_timestamp;
        let pet = &mut ctx.accounts.pet;
        apply_clean(pet, now)?;
        msg!("Cleaned NFP #{} — sparkling", pet.id);
        Ok(())
    }

    /// Rest the pet: restores energy. Rejected at full energy — a rested
    /// pet has nothing to recover.
    pub fn rest_pet(ctx: Context<CareForPet>) -> Result<()> {
        authorize_caregiver(&ctx)?;
        let now = Clock::get()?.unix_timestamp;
        let pet = &mut ctx.accounts.pet;
        apply_rest(pet, now)?;
        msg!("NFP #{} is resting — energy: {}", pet.id, pet.energy);
        Ok(())
    }

    /// Shower the pet with affection: +love, the biggest bond gain.
    pub fn give_affection(ctx: Context<CareForPet>) -> Result<()> {
        authorize_caregiver(&ctx)?;
        let now = Clock::get()?.unix_timestamp;
        let pet = &mut ctx.accounts.pet;
        apply_affection(pet, now)?;
        msg!("NFP #{} feels loved — love: {}", pet.id, pet.love);
        Ok(())
    }

    /// Transfer a pet to a new owner.
    ///
    /// The signer must be the pet's CURRENT owner, or the registry
    /// authority (the authority override exists for the marketplace's
    /// rescue flow — neglected pets can be reclaimed for adoption).
    ///
    /// `breed_cooldown_until`: when `Some`, stamps a breeding cooldown on
    /// the pet as part of the transfer. Used by the marketplace adoption
    /// flow (restoration cooldown). Pass `None` for a plain transfer.
    /// Anyone transferring a pet they own may set a cooldown — it only
    /// restricts the pet being given away.
    pub fn transfer_pet(
        ctx: Context<TransferPet>,
        new_owner: Pubkey,
        breed_cooldown_until: Option<i64>,
    ) -> Result<()> {
        let signer_key = ctx.accounts.owner.key();
        require!(
            is_authorized_transfer(&signer_key, &ctx.accounts.pet, &ctx.accounts.registry),
            PetRegistryError::UnauthorizedTransfer
        );
        require_keys_neq!(signer_key, new_owner, PetRegistryError::TransferToSelf);

        let pet = &mut ctx.accounts.pet;
        pet.owner = new_owner;
        if let Some(until) = breed_cooldown_until {
            require!(until >= 0, PetRegistryError::InvalidCooldown);
            pet.breed_cooldown_until = until;
        }

        msg!(
            "NFP #{} transferred to {}{}",
            pet.id,
            new_owner,
            match breed_cooldown_until {
                Some(u) => format!(" (breeding cooldown until {})", u),
                None => String::new(),
            }
        );
        Ok(())
    }
}

// ============================================================================
// Account structs
// ============================================================================

/// Global registry: the single source of truth for ID sequencing.
/// PDA seeds: ["registry"]
#[account]
pub struct Registry {
    /// Only this key may mint (multisig in production).
    pub authority: Pubkey, // 32
    /// Next sequential pet ID to assign.
    pub next_id: u32, // 4
    /// Lifetime mint count (excludes reserved creator IDs).
    pub total_minted: u32, // 4
    /// PDA bump.
    pub bump: u8, // 1
}

impl Registry {
    pub const SIZE: usize = 32 + 4 + 4 + 1; // 41
}

/// A living NFP. Every byte of a pet's identity is here, on-chain.
/// PDA seeds: ["pet", id.to_le_bytes()]
#[account]
pub struct Pet {
    // --- Identity ------------------------------------------------------------
    pub id: u32,          // 4 — sequential, 1–750,000
    pub species: Species, // 1 — Otter | Cat | Dog | Monkey
    pub gender: Gender,   // 1 — Male | Female
    pub generation: u8,   // 1 — 0 = founder/genesis, 1+ = bred

    // --- Genetics ------------------------------------------------------------
    pub color_primary: u8,   // 1 — coat gene A (0–7)
    pub color_secondary: u8, // 1 — coat gene B (0–7)
    pub marking: u8,         // 1 — species marking index (0–15)
    pub eye_color: u8,       // 1 — eye gene (0–5)

    // --- Personality (0–100 each, rolled at mint, fixed for life) -------------
    pub curiosity: u8,    // 1
    pub playfulness: u8,  // 1
    pub affection: u8,    // 1
    pub loyalty: u8,      // 1
    pub intelligence: u8, // 1
    pub calmness: u8,     // 1

    // --- Rarity & lineage ------------------------------------------------------
    pub rarity: Rarity, // 1 — Common..Mythic
    pub parent_a: u32,  // 4 — sire/dam pet ID (0 = none)
    pub parent_b: u32,  // 4

    // --- Ownership & lifecycle ---------------------------------------------------
    pub owner: Pubkey,        // 32
    pub born_at: i64,         // 8 — unix timestamp
    pub life_stage: LifeStage, // 1 — Baby | Adolescent | Adult

    // --- Living stats (0–100, decay over time, restored by care) -----------------
    pub hunger: u8,      // 1
    pub happiness: u8,   // 1
    pub cleanliness: u8, // 1
    pub energy: u8,      // 1
    pub love: u8,        // 1 — the affection care stat
    pub health_score: u8, // 1 — derived; gates breeding

    // --- Bond --------------------------------------------------------------------
    pub bond_xp: u64, // 8 — grows with care, never decreases

    // --- Care tracking ---------------------------------------------------------
    pub last_cared_at: i64, // 8 — unix timestamp of the last care action

    // --- Expressed special traits (bitmasks — see helpers below) ----------------
    // EVERY pet carries the full gene set; these masks record which traits
    // are VISUALLY EXPRESSED (low-penetrance model — most pets show none).
    // Set at mint (breeding computes them; founders get them via the
    // future mint_creator_pet instruction).
    pub legendary_expressed: u16, // 2 — bit i = LegendaryTrait discriminant i
    pub mythic_expressed: u8,     // 1 — bit i = MythicMutation discriminant i

    // --- Breeding ------------------------------------------------------------------
    pub breed_cooldown_until: i64, // 8 — unix timestamp; 0 = ready to breed

    /// PDA bump.
    pub bump: u8, // 1
}

impl Pet {
    /// 98 (Steps 1–2) + 2 (legendary mask) + 1 (mythic mask)
    /// + 8 (breeding cooldown) = 109
    pub const SIZE: usize = 109;
}

// ============================================================================
// Enums
// ============================================================================

/// The four SolanaPets species.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Species {
    Otter,
    Cat,
    Dog,
    Monkey,
}

/// Male / female — breeding requires one of each.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gender {
    Male,
    Female,
}

/// Rarity ladder, lowest to highest.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

/// Life stages. Baby ≥ 7 days → Adolescent 7 days → Adult for life.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LifeStage {
    Baby,
    Adolescent,
    Adult,
}

// ============================================================================
// Creator-pet traits — the canonical gene pool (see CREATOR_PETS.md)
// ============================================================================

/// The 16 legendary traits across the 8 founders (2 per pet).
/// Legendary = one striking feature: a glowing marking, luminous eyes,
/// a shimmer in the coat. Fully visual on the founder; rarely expressed
/// in bred pets.
///
/// `#[repr(u8)]` locks the discriminants: the breeding program uses them
/// as bit positions in `Pet::legendary_expressed`. NEVER reorder variants.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum LegendaryTrait {
    // Luna (#1)
    CrescentMoonChest,
    GlowingTail,
    // Lenny (#2)
    FlowMarkTail,
    TidecallerEyes,
    // Lucy (#3)
    HeartlightChest,
    SunsetPaws,
    // Loboe (#4)
    EmberEyes,
    StonehideMarkings,
    // Lilah (#5)
    MoonbeamWhiskers,
    RoseGlowEyes,
    // Leo (#6)
    GlowingArrowheadChest,
    GlowingGreenEyes,
    // Lola (#7)
    GlowingBananaMark,
    Bioluminescence,
    // Loki (#8)
    GlowingRedTailTip,
    SparkEyes,
}

/// The 8 mythic mutations, one per founder.
/// Mythic = transformative and unmistakable: a full-body aura, star
/// patterns in the fur, a coat that shifts color. The rarest expression
/// in the game.
///
/// `#[repr(u8)]` locks the discriminants: the breeding program uses them
/// as bit positions in `Pet::mythic_expressed`. NEVER reorder variants.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MythicMutation {
    LunarAura,          // Luna — moonlight glow around the body
    RiverSpiritTrail,   // Lenny — water ripples follow when swimming
    GoldenHeartAura,    // Lucy — warm golden glow of love
    MountainsStandAura, // Loboe — steady earth-toned glow
    DreamweaverAura,    // Lilah — soft pink mist, half in a dream
    ConstellationCoat,  // Leo — star patterns in black fur
    JunglePrism,        // Lola — coat shifts through jungle greens and golds
    EmberTrail,         // Loki — red sparks trail the tail
}

/// The canonical trait set for one creator pet: 2 legendary + 1 mythic.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CreatorTraits {
    pub legendary_a: LegendaryTrait,
    pub legendary_b: LegendaryTrait,
    pub mythic: MythicMutation,
}

// ============================================================================
// Instruction params
// ============================================================================

/// Everything the minter supplies. The program assigns id, timestamps,
/// lineage (none — founder mints have no parents), and starting stats.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct MintNfpParams {
    pub species: Species,
    pub gender: Gender,
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
    pub owner: Pubkey,
    /// Visually expressed special traits (bitmasks). The breeding program
    /// computes these; raffle mints pass 0 (no expression); the future
    /// mint_creator_pet sets them from the canonical founder assignments.
    /// The authority is trusted to set these correctly.
    pub legendary_expressed: u16,
    pub mythic_expressed: u8,
}

// ============================================================================
// Account validation structs
// ============================================================================

#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Registry::SIZE,
        seeds = [REGISTRY_SEED],
        bump,
    )]
    pub registry: Account<'info, Registry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintNfp<'info> {
    /// Must equal registry.authority — enforced by has_one below.
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [REGISTRY_SEED],
        bump = registry.bump,
        has_one = authority @ PetRegistryError::UnauthorizedMinter,
    )]
    pub registry: Account<'info, Registry>,

    /// Pet PDA derived from the NEXT id — so the address itself proves
    /// sequencing. Seeds: ["pet", next_id.to_le_bytes()]
    #[account(
        init,
        payer = authority,
        space = 8 + Pet::SIZE,
        seeds = [PET_SEED, registry.next_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub pet: Account<'info, Pet>,

    pub system_program: Program<'info, System>,
}

/// Care actions: the signer must be the pet's owner or the registry
/// authority (checked in `authorize_caregiver`). The registry is read-only
/// here — care never changes who can mint.
#[derive(Accounts)]
pub struct CareForPet<'info> {
    /// The caregiver — pet owner or registry authority.
    pub caregiver: Signer<'info>,

    #[account(
        mut,
        seeds = [PET_SEED, pet.id.to_le_bytes().as_ref()],
        bump = pet.bump,
    )]
    pub pet: Account<'info, Pet>,

    #[account(
        seeds = [REGISTRY_SEED],
        bump = registry.bump,
    )]
    pub registry: Account<'info, Registry>,
}

/// Ownership transfers. The signer must be the pet's current owner or the
/// registry authority (authority override = marketplace rescue flow).
/// The registry is read-only — transfers never change who can mint.
#[derive(Accounts)]
pub struct TransferPet<'info> {
    /// Current owner (or registry authority for forced rescue transfers).
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [PET_SEED, pet.id.to_le_bytes().as_ref()],
        bump = pet.bump,
    )]
    pub pet: Account<'info, Pet>,

    #[account(
        seeds = [REGISTRY_SEED],
        bump = registry.bump,
    )]
    pub registry: Account<'info, Registry>,
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum PetRegistryError {
    #[msg("IDs 1-8 are reserved for creator pets and cannot be minted here")]
    CreatorIdsReserved,

    #[msg("Maximum pet supply (750,000) reached")]
    MaxSupplyReached,

    #[msg("Invalid color gene — must be 0-7")]
    InvalidColorGene,

    #[msg("Invalid eye color — must be 0-5")]
    InvalidEyeColor,

    #[msg("Invalid marking index — must be 0-15")]
    InvalidMarking,

    #[msg("Invalid personality trait — must be 0-100")]
    InvalidPersonality,

    #[msg("Signer is not the registry mint authority")]
    UnauthorizedMinter,

    #[msg("Only the pet's owner or the registry authority may perform care actions")]
    UnauthorizedCaregiver,

    #[msg("Pet is already at full energy — nothing to rest")]
    AlreadyRested,

    #[msg("Arithmetic overflow in care stat math")]
    MathOverflow,

    #[msg("ID is not a creator pet ID (1-8)")]
    NotACreatorId,

    #[msg("Creator pet traits do not match the canonical assignment for this ID")]
    CreatorTraitsMismatch,

    #[msg("Only the pet's owner or the registry authority may transfer it")]
    UnauthorizedTransfer,

    #[msg("Cannot transfer a pet to yourself")]
    TransferToSelf,

    #[msg("Invalid cooldown timestamp")]
    InvalidCooldown,
}

// ============================================================================
// Pure logic (unit-testable, no accounts needed)
// ============================================================================

/// True for IDs 1–8 — Brad's creator pets. Never publicly mintable.
pub fn is_creator_id(id: u32) -> bool {
    (1..=CREATOR_IDS_END).contains(&id)
}

/// True for IDs 1–15,000 — the Genesis supply (includes creator IDs).
pub fn is_genesis_id(id: u32) -> bool {
    (1..=GENESIS_SUPPLY).contains(&id)
}

// ============================================================================
// Canonical color pools (pure fns — see COLOR_POOLS.md)
// ============================================================================

/// Coat color name for a species + pool index (0–7).
/// Gene letters (A–H) and names are canonical from Brad's character sheets;
/// indices are the on-chain `color_primary` / `color_secondary` values.
pub fn coat_color_name(species: Species, index: u8) -> Option<&'static str> {
    if index >= COAT_POOL_SIZE {
        return None;
    }
    let name = match species {
        Species::Otter => match index {
            0 => "A - River Brown",
            1 => "B - Sand",
            2 => "C - Clay",
            3 => "D - Chestnut",
            4 => "E - White",
            5 => "F - Driftwood",
            6 => "G - Pebble",
            _ => "H - Midnight",
        },
        Species::Dog => match index {
            0 => "A - Black",
            1 => "B - Brown", // Neutral Brown (Lucy) / Fawn Brown (Loboe)
            2 => "C - Tan",
            3 => "D - Dark Brown",
            4 => "E - White",
            5 => "F - Golden",
            6 => "G - Slate",
            _ => "H - Cream",
        },
        Species::Cat => match index {
            0 => "A - Black",
            1 => "B - Neutral Brown",
            2 => "C - Smoke Gray",
            3 => "D - Dark Brown",
            4 => "E - White",
            5 => "F - Ginger",
            6 => "G - Cream",
            _ => "H - Silver",
        },
        Species::Monkey => match index {
            0 => "A - Ebony",
            1 => "B - Light Brown",
            2 => "C - Copper",
            3 => "D - Dark Brown",
            4 => "E - White",
            5 => "F - Golden",
            6 => "G - Ash Gray",
            _ => "H - Tan",
        },
    };
    Some(name)
}

/// Eye color name for a pool index (0–5).
/// Gene codes (E-A…E-P) are canonical from the sheets. Note: Lucy's sheet
/// lists E-B Warm Brown, but E-B was triple-booked (Ocean Blue, Warm Brown,
/// Blue Bright); Warm Brown is normalized to its own slot E-D. See
/// COLOR_POOLS.md for the full mapping.
pub fn eye_color_name(index: u8) -> Option<&'static str> {
    match index {
        0 => Some("E-A Amber"),              // Luna, Loboe (Hazel Brown shade)
        1 => Some("E-B Ocean Blue"),         // Lenny, Loki (Blue Bright shade)
        2 => Some("E-C Rose Amber"),         // Lilah
        3 => Some("E-D Warm Brown"),          // Lucy (normalized from E-B)
        4 => Some("E-G Green (Glowing)"),    // Leo
        5 => Some("E-P Pink (Glowing)"),     // Lola
        _ => None,
    }
}

// ============================================================================
// Creator-pet trait canon (pure fns — see CREATOR_PETS.md)
// ============================================================================

/// Human-readable name for a legendary trait.
pub fn legendary_trait_name(trait_: LegendaryTrait) -> &'static str {
    match trait_ {
        LegendaryTrait::CrescentMoonChest => "Crescent Moon (chest)",
        LegendaryTrait::GlowingTail => "Glowing Tail (luminescent)",
        LegendaryTrait::FlowMarkTail => "Flow Mark (tail, luminescent)",
        LegendaryTrait::TidecallerEyes => "Tidecaller Eyes (glowing ocean blue)",
        LegendaryTrait::HeartlightChest => "Heartlight Chest (glowing pink heart)",
        LegendaryTrait::SunsetPaws => "Sunset Paws (warm glowing paw pads)",
        LegendaryTrait::EmberEyes => "Ember Eyes (glowing amber)",
        LegendaryTrait::StonehideMarkings => "Stonehide Markings (glowing canyon-stone cracks)",
        LegendaryTrait::MoonbeamWhiskers => "Moonbeam Whiskers (glowing silver)",
        LegendaryTrait::RoseGlowEyes => "Rose Glow Eyes (soft pink luminescence)",
        LegendaryTrait::GlowingArrowheadChest => "Glowing Arrowhead (chest)",
        LegendaryTrait::GlowingGreenEyes => "Glowing Green Eyes (luminescent)",
        LegendaryTrait::GlowingBananaMark => "Glowing Banana Mark (stomach)",
        LegendaryTrait::Bioluminescence => "Bioluminescence (living glow)",
        LegendaryTrait::GlowingRedTailTip => "Glowing Red Tail Tip (luminescent)",
        LegendaryTrait::SparkEyes => "Spark Eyes (glowing blue)",
    }
}

/// Human-readable name for a mythic mutation.
pub fn mythic_mutation_name(mutation: MythicMutation) -> &'static str {
    match mutation {
        MythicMutation::LunarAura => "Lunar Aura (moonlight glow around body)",
        MythicMutation::RiverSpiritTrail => "River Spirit Trail (water ripples follow when swimming)",
        MythicMutation::GoldenHeartAura => "Golden Heart Aura (warm golden glow)",
        MythicMutation::MountainsStandAura => "Mountain's Stand Aura (steady earth-toned glow)",
        MythicMutation::DreamweaverAura => "Dreamweaver Aura (soft pink mist)",
        MythicMutation::ConstellationCoat => "Constellation Coat (star patterns in fur)",
        MythicMutation::JunglePrism => "Jungle Prism (coat shifts through jungle greens and golds)",
        MythicMutation::EmberTrail => "Ember Trail (red sparks trail the tail)",
    }
}

/// The canonical (legendary_a, legendary_b, mythic) for a creator pet ID.
/// Returns `None` for any ID outside 1–8.
///
/// This is the single source of truth the future `mint_creator_pet`
/// instruction will enforce: a creator pet is only valid when its traits
/// match this mapping exactly. Founder names use Brad's originals.
pub fn creator_pet_traits(pet_id: u32) -> Option<CreatorTraits> {
    let traits = match pet_id {
        1 => CreatorTraits {
            // Luna — female otter
            legendary_a: LegendaryTrait::CrescentMoonChest,
            legendary_b: LegendaryTrait::GlowingTail,
            mythic: MythicMutation::LunarAura,
        },
        2 => CreatorTraits {
            // Lenny — male otter
            legendary_a: LegendaryTrait::FlowMarkTail,
            legendary_b: LegendaryTrait::TidecallerEyes,
            mythic: MythicMutation::RiverSpiritTrail,
        },
        3 => CreatorTraits {
            // Lucy — female dachshund
            legendary_a: LegendaryTrait::HeartlightChest,
            legendary_b: LegendaryTrait::SunsetPaws,
            mythic: MythicMutation::GoldenHeartAura,
        },
        4 => CreatorTraits {
            // Loboe — male English bulldog
            legendary_a: LegendaryTrait::EmberEyes,
            legendary_b: LegendaryTrait::StonehideMarkings,
            mythic: MythicMutation::MountainsStandAura,
        },
        5 => CreatorTraits {
            // Lilah — female cat
            legendary_a: LegendaryTrait::MoonbeamWhiskers,
            legendary_b: LegendaryTrait::RoseGlowEyes,
            mythic: MythicMutation::DreamweaverAura,
        },
        6 => CreatorTraits {
            // Leo — male tuxedo cat
            legendary_a: LegendaryTrait::GlowingArrowheadChest,
            legendary_b: LegendaryTrait::GlowingGreenEyes,
            mythic: MythicMutation::ConstellationCoat,
        },
        7 => CreatorTraits {
            // Lola — female capuchin
            legendary_a: LegendaryTrait::GlowingBananaMark,
            legendary_b: LegendaryTrait::Bioluminescence,
            mythic: MythicMutation::JunglePrism,
        },
        8 => CreatorTraits {
            // Loki — male capuchin
            legendary_a: LegendaryTrait::GlowingRedTailTip,
            legendary_b: LegendaryTrait::SparkEyes,
            mythic: MythicMutation::EmberTrail,
        },
        _ => return None,
    };
    Some(traits)
}

/// Validate that a proposed creator-pet trait set matches the canon for
/// the given ID. The future `mint_creator_pet` instruction will call this
/// so a founder can never be minted with the wrong traits.
pub fn validate_creator_traits(
    pet_id: u32,
    legendary_a: LegendaryTrait,
    legendary_b: LegendaryTrait,
    mythic: MythicMutation,
) -> Result<()> {
    let canon = creator_pet_traits(pet_id).ok_or(PetRegistryError::NotACreatorId)?;
    require!(
        legendary_a == canon.legendary_a
            && legendary_b == canon.legendary_b
            && mythic == canon.mythic,
        PetRegistryError::CreatorTraitsMismatch
    );
    Ok(())
}

// ============================================================================
// Expressed-trait bitmasks (used by the breeding program)
// ============================================================================

/// Bit position for a legendary trait = its `#[repr(u8)]` discriminant.
pub fn legendary_trait_bit(trait_: LegendaryTrait) -> u16 {
    1u16 << (trait_ as u16)
}

/// Bit position for a mythic mutation = its `#[repr(u8)]` discriminant.
pub fn mythic_mutation_bit(mutation: MythicMutation) -> u8 {
    1u8 << (mutation as u8)
}

/// True when the pet visually expresses the given legendary trait.
pub fn expresses_legendary(mask: u16, trait_: LegendaryTrait) -> bool {
    mask & legendary_trait_bit(trait_) != 0
}

/// True when the pet visually expresses the given mythic mutation.
pub fn expresses_mythic(mask: u8, mutation: MythicMutation) -> bool {
    mask & mythic_mutation_bit(mutation) != 0
}

/// How many legendary traits the mask expresses (0–16).
pub fn count_legendary_expressed(mask: u16) -> u32 {
    mask.count_ones()
}

/// How many mythic mutations the mask expresses (0–8).
pub fn count_mythic_expressed(mask: u8) -> u32 {
    mask.count_ones()
}

/// LegendaryTrait from its bit index (0–15). Panics on out-of-range —
/// callers must bound the index first.
pub fn legendary_trait_from_index(index: u8) -> LegendaryTrait {
    match index {
        0 => LegendaryTrait::CrescentMoonChest,
        1 => LegendaryTrait::GlowingTail,
        2 => LegendaryTrait::FlowMarkTail,
        3 => LegendaryTrait::TidecallerEyes,
        4 => LegendaryTrait::HeartlightChest,
        5 => LegendaryTrait::SunsetPaws,
        6 => LegendaryTrait::EmberEyes,
        7 => LegendaryTrait::StonehideMarkings,
        8 => LegendaryTrait::MoonbeamWhiskers,
        9 => LegendaryTrait::RoseGlowEyes,
        10 => LegendaryTrait::GlowingArrowheadChest,
        11 => LegendaryTrait::GlowingGreenEyes,
        12 => LegendaryTrait::GlowingBananaMark,
        13 => LegendaryTrait::Bioluminescence,
        14 => LegendaryTrait::GlowingRedTailTip,
        15 => LegendaryTrait::SparkEyes,
        _ => panic!("legendary trait index out of range: {}", index),
    }
}

/// MythicMutation from its bit index (0–7). Panics on out-of-range.
pub fn mythic_mutation_from_index(index: u8) -> MythicMutation {
    match index {
        0 => MythicMutation::LunarAura,
        1 => MythicMutation::RiverSpiritTrail,
        2 => MythicMutation::GoldenHeartAura,
        3 => MythicMutation::MountainsStandAura,
        4 => MythicMutation::DreamweaverAura,
        5 => MythicMutation::ConstellationCoat,
        6 => MythicMutation::JunglePrism,
        7 => MythicMutation::EmberTrail,
        _ => panic!("mythic mutation index out of range: {}", index),
    }
}

/// The full legendary mask for a creator pet ID (both traits set).
/// Used by the future `mint_creator_pet` instruction.
pub fn creator_legendary_mask(pet_id: u32) -> Option<u16> {
    let t = creator_pet_traits(pet_id)?;
    Some(legendary_trait_bit(t.legendary_a) | legendary_trait_bit(t.legendary_b))
}

/// The mythic mask for a creator pet ID (single bit set).
pub fn creator_mythic_mask(pet_id: u32) -> Option<u8> {
    let t = creator_pet_traits(pet_id)?;
    Some(mythic_mutation_bit(t.mythic))
}

/// Validate every caller-supplied trait value. Keeps the handler lean and
/// lets us unit-test the rules without spinning up accounts.
pub fn validate_mint_params(params: &MintNfpParams) -> Result<()> {
    require!(
        params.color_primary <= MAX_COLOR_GENE && params.color_secondary <= MAX_COLOR_GENE,
        PetRegistryError::InvalidColorGene
    );
    require!(
        params.eye_color <= MAX_EYE_COLOR,
        PetRegistryError::InvalidEyeColor
    );
    require!(
        params.marking <= MAX_MARKING,
        PetRegistryError::InvalidMarking
    );

    // Personality traits are fixed at mint — clamp them to 0–100 here.
    for trait_value in [
        params.curiosity,
        params.playfulness,
        params.affection,
        params.loyalty,
        params.intelligence,
        params.calmness,
    ] {
        require!(
            trait_value <= MAX_STAT,
            PetRegistryError::InvalidPersonality
        );
    }
    Ok(())
}

// ============================================================================
// Care logic (pure fns — unit-testable without accounts)
// ============================================================================

/// True when the signer is allowed to care for this pet: the pet's owner
/// or the registry authority (e.g. for rescue flows later).
pub fn is_authorized_caregiver(caregiver: &Pubkey, pet: &Pet, registry: &Registry) -> bool {
    *caregiver == pet.owner || *caregiver == registry.authority
}

/// Enforces the caregiver rule for the `CareForPet` context.
pub fn authorize_caregiver(ctx: &Context<CareForPet>) -> Result<()> {
    require!(
        is_authorized_caregiver(
            &ctx.accounts.caregiver.key(),
            &ctx.accounts.pet,
            &ctx.accounts.registry
        ),
        PetRegistryError::UnauthorizedCaregiver
    );
    Ok(())
}

/// True when the signer may transfer this pet: the pet's current owner,
/// or the registry authority (the authority override powers the
/// marketplace's rescue flow — neglected pets can be reclaimed).
pub fn is_authorized_transfer(signer: &Pubkey, pet: &Pet, registry: &Registry) -> bool {
    *signer == pet.owner || *signer == registry.authority
}

/// Shared tail of every care action: stamp the clock, add bond XP
/// (checked — bond never wraps), and nudge health when the pet is thriving.
pub fn finish_care(pet: &mut Pet, now: i64, bond_gain: u64) -> Result<()> {
    pet.bond_xp = pet
        .bond_xp
        .checked_add(bond_gain)
        .ok_or(PetRegistryError::MathOverflow)?;
    pet.last_cared_at = now;
    maybe_boost_health(pet);
    Ok(())
}

/// If every care stat is thriving (>= HEALTH_BOOST_THRESHOLD) and the pet
/// isn't already at full health, tick health_score up by 1. Small, steady,
/// never a shortcut past real neglect.
pub fn maybe_boost_health(pet: &mut Pet) {
    let thriving = [
        pet.hunger,
        pet.happiness,
        pet.cleanliness,
        pet.energy,
        pet.love,
    ]
    .iter()
    .all(|&s| s >= HEALTH_BOOST_THRESHOLD);
    if thriving && pet.health_score < MAX_STAT {
        pet.health_score = pet.health_score.saturating_add(1);
    }
}

/// Feed: restores hunger.
pub fn apply_feed(pet: &mut Pet, now: i64) -> Result<()> {
    pet.hunger = pet.hunger.saturating_add(FEED_AMOUNT).min(MAX_STAT);
    finish_care(pet, now, BOND_XP_FEED)
}

/// Play: happiness up, energy down (playing is tiring).
pub fn apply_play(pet: &mut Pet, now: i64) -> Result<()> {
    pet.happiness = pet.happiness.saturating_add(PLAY_HAPPINESS_GAIN).min(MAX_STAT);
    pet.energy = pet.energy.saturating_sub(PLAY_ENERGY_COST);
    finish_care(pet, now, BOND_XP_PLAY)
}

/// Clean: full sparkle restore.
pub fn apply_clean(pet: &mut Pet, now: i64) -> Result<()> {
    pet.cleanliness = MAX_STAT;
    finish_care(pet, now, BOND_XP_CLEAN)
}

/// Rest: restores energy. Errors at full energy — nothing to recover.
pub fn apply_rest(pet: &mut Pet, now: i64) -> Result<()> {
    require!(pet.energy < MAX_STAT, PetRegistryError::AlreadyRested);
    pet.energy = pet.energy.saturating_add(REST_ENERGY_GAIN).min(MAX_STAT);
    finish_care(pet, now, BOND_XP_REST)
}

/// Affection: restores love, grants the biggest bond gain.
pub fn apply_affection(pet: &mut Pet, now: i64) -> Result<()> {
    pet.love = pet.love.saturating_add(AFFECTION_GAIN).min(MAX_STAT);
    finish_care(pet, now, BOND_XP_AFFECTION)
}

// ============================================================================
// Unit tests — `cargo test` (no validator needed; pure logic only)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_params() -> MintNfpParams {
        MintNfpParams {
            species: Species::Otter,
            gender: Gender::Female,
            color_primary: 2,
            color_secondary: 3,
            marking: 0,
            eye_color: 4,
            curiosity: 80,
            playfulness: 75,
            affection: 90,
            loyalty: 85,
            intelligence: 70,
            calmness: 60,
            rarity: Rarity::Rare,
            owner: Pubkey::new_unique(),
            legendary_expressed: 0,
            mythic_expressed: 0,
        }
    }

    #[test]
    fn valid_traits_pass() {
        assert!(validate_mint_params(&valid_params()).is_ok());
    }

    #[test]
    fn boundary_traits_pass() {
        // All-maximum legal values must be accepted.
        let mut p = valid_params();
        p.color_primary = MAX_COLOR_GENE;
        p.color_secondary = MAX_COLOR_GENE;
        p.eye_color = MAX_EYE_COLOR;
        p.marking = MAX_MARKING;
        p.curiosity = MAX_STAT;
        assert!(validate_mint_params(&p).is_ok());
    }

    #[test]
    fn rejects_bad_color_gene() {
        let mut p = valid_params();
        p.color_primary = MAX_COLOR_GENE + 1; // 8 colors: 0-7
        let err = validate_mint_params(&p).unwrap_err();
        assert_eq!(err, PetRegistryError::InvalidColorGene.into());
    }

    #[test]
    fn rejects_bad_eye_color() {
        let mut p = valid_params();
        p.eye_color = MAX_EYE_COLOR + 1; // 6 eye colors: 0-5
        let err = validate_mint_params(&p).unwrap_err();
        assert_eq!(err, PetRegistryError::InvalidEyeColor.into());
    }

    #[test]
    fn rejects_bad_marking() {
        let mut p = valid_params();
        p.marking = MAX_MARKING + 1;
        let err = validate_mint_params(&p).unwrap_err();
        assert_eq!(err, PetRegistryError::InvalidMarking.into());
    }

    #[test]
    fn rejects_bad_personality() {
        let mut p = valid_params();
        p.loyalty = MAX_STAT + 1;
        let err = validate_mint_params(&p).unwrap_err();
        assert_eq!(err, PetRegistryError::InvalidPersonality.into());
    }

    #[test]
    fn creator_ids_are_reserved() {
        for id in 1..=8 {
            assert!(is_creator_id(id), "ID {} should be reserved", id);
        }
        assert!(!is_creator_id(0));
        assert!(!is_creator_id(9));
        assert!(!is_creator_id(15_000));
    }

    #[test]
    fn public_minting_starts_at_nine() {
        assert!(!is_creator_id(FIRST_PUBLIC_ID));
        assert_eq!(FIRST_PUBLIC_ID, CREATOR_IDS_END + 1);
    }

    #[test]
    fn genesis_range_is_correct() {
        assert!(is_genesis_id(1));
        assert!(is_genesis_id(9));
        assert!(is_genesis_id(GENESIS_SUPPLY));
        assert!(!is_genesis_id(GENESIS_SUPPLY + 1));
    }

    #[test]
    fn max_supply_guard() {
        assert!(MAX_PETS == 750_000);
        assert!(FIRST_PUBLIC_ID <= MAX_PETS);
    }

    #[test]
    fn pet_account_size_is_sane() {
        // Rent math sanity: 109 bytes + 8 discriminator.
        // (98 from Steps 1–2 + 2 legendary mask + 1 mythic mask
        //  + 8 breeding cooldown, added in Step 5.)
        assert_eq!(Pet::SIZE, 109);
        assert_eq!(Registry::SIZE, 41);
    }

    // --- Care action tests -----------------------------------------------------

    /// A mid-life pet with middling stats — the standard test fixture.
    fn test_pet() -> Pet {
        Pet {
            id: 9,
            species: Species::Cat,
            gender: Gender::Male,
            generation: 0,
            color_primary: 1,
            color_secondary: 2,
            marking: 3,
            eye_color: 4,
            curiosity: 50,
            playfulness: 50,
            affection: 50,
            loyalty: 50,
            intelligence: 50,
            calmness: 50,
            rarity: Rarity::Common,
            parent_a: 0,
            parent_b: 0,
            owner: Pubkey::new_unique(),
            born_at: 1_000_000,
            life_stage: LifeStage::Baby,
            hunger: 50,
            happiness: 50,
            cleanliness: 50,
            energy: 50,
            love: 50,
            health_score: 90,
            bond_xp: 0,
            last_cared_at: 1_000_000,
            legendary_expressed: 0,
            mythic_expressed: 0,
            breed_cooldown_until: 0,
            bump: 255,
        }
    }

    fn test_registry(authority: Pubkey) -> Registry {
        Registry {
            authority,
            next_id: 10,
            total_minted: 1,
            bump: 254,
        }
    }

    #[test]
    fn feed_restores_hunger_and_grants_bond() {
        let mut pet = test_pet();
        apply_feed(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.hunger, 50 + FEED_AMOUNT);
        assert_eq!(pet.bond_xp, BOND_XP_FEED);
        assert_eq!(pet.last_cared_at, 2_000_000);
    }

    #[test]
    fn feed_clamps_at_100() {
        let mut pet = test_pet();
        pet.hunger = 90;
        apply_feed(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.hunger, 100); // would be 115 without the clamp
    }

    #[test]
    fn play_boosts_happiness_costs_energy() {
        let mut pet = test_pet();
        apply_play(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.happiness, 50 + PLAY_HAPPINESS_GAIN);
        assert_eq!(pet.energy, 50 - PLAY_ENERGY_COST);
        assert_eq!(pet.bond_xp, BOND_XP_PLAY);
        assert_eq!(pet.last_cared_at, 2_000_000);
    }

    #[test]
    fn play_energy_floors_at_zero() {
        let mut pet = test_pet();
        pet.energy = 5;
        apply_play(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.energy, 0); // no underflow
    }

    #[test]
    fn clean_restores_fully() {
        let mut pet = test_pet();
        pet.cleanliness = 12;
        apply_clean(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.cleanliness, 100);
        assert_eq!(pet.bond_xp, BOND_XP_CLEAN);
    }

    #[test]
    fn rest_restores_energy() {
        let mut pet = test_pet();
        pet.energy = 40;
        apply_rest(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.energy, 40 + REST_ENERGY_GAIN);
        assert_eq!(pet.bond_xp, BOND_XP_REST);
    }

    #[test]
    fn rest_rejected_at_full_energy() {
        let mut pet = test_pet();
        pet.energy = 100;
        let err = apply_rest(&mut pet, 2_000_000).unwrap_err();
        assert_eq!(err, PetRegistryError::AlreadyRested.into());
        assert_eq!(pet.bond_xp, 0); // failed action grants nothing
    }

    #[test]
    fn rest_clamps_at_100() {
        let mut pet = test_pet();
        pet.energy = 85;
        apply_rest(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.energy, 100);
    }

    #[test]
    fn affection_grants_biggest_bond() {
        let mut pet = test_pet();
        pet.love = 30;
        apply_affection(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.love, 30 + AFFECTION_GAIN);
        assert_eq!(pet.bond_xp, BOND_XP_AFFECTION);
        assert!(BOND_XP_AFFECTION > BOND_XP_PLAY);
        assert!(BOND_XP_AFFECTION > BOND_XP_FEED);
    }

    #[test]
    fn bond_xp_accumulates_across_actions() {
        let mut pet = test_pet();
        apply_feed(&mut pet, 2_000_000).unwrap();
        apply_play(&mut pet, 2_000_001).unwrap();
        apply_affection(&mut pet, 2_000_002).unwrap();
        assert_eq!(
            pet.bond_xp,
            BOND_XP_FEED + BOND_XP_PLAY + BOND_XP_AFFECTION
        );
    }

    #[test]
    fn health_boosts_when_all_stats_thriving() {
        let mut pet = test_pet();
        pet.hunger = 85;
        pet.happiness = 90;
        pet.cleanliness = 100;
        pet.energy = 80;
        pet.love = 95;
        pet.health_score = 90;
        apply_feed(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.health_score, 91);
    }

    #[test]
    fn no_health_boost_when_any_stat_low() {
        let mut pet = test_pet();
        pet.hunger = 85;
        pet.happiness = 90;
        pet.cleanliness = 100;
        pet.energy = 40; // too low
        pet.love = 95;
        pet.health_score = 90;
        apply_feed(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.health_score, 90);
    }

    #[test]
    fn health_never_exceeds_100() {
        let mut pet = test_pet();
        pet.hunger = 100;
        pet.happiness = 100;
        pet.cleanliness = 100;
        pet.energy = 100;
        pet.love = 100;
        pet.health_score = 100;
        apply_feed(&mut pet, 2_000_000).unwrap();
        assert_eq!(pet.health_score, 100);
    }

    #[test]
    fn owner_is_authorized_caregiver() {
        let pet = test_pet();
        let registry = test_registry(Pubkey::new_unique());
        assert!(is_authorized_caregiver(&pet.owner, &pet, &registry));
    }

    #[test]
    fn registry_authority_is_authorized_caregiver() {
        let pet = test_pet();
        let authority = Pubkey::new_unique();
        let registry = test_registry(authority);
        assert!(is_authorized_caregiver(&authority, &pet, &registry));
    }

    #[test]
    fn stranger_is_rejected_caregiver() {
        let pet = test_pet();
        let registry = test_registry(Pubkey::new_unique());
        let stranger = Pubkey::new_unique();
        assert!(!is_authorized_caregiver(&stranger, &pet, &registry));
    }

    // --- Transfer authorization tests --------------------------------------------

    #[test]
    fn owner_is_authorized_transfer() {
        let pet = test_pet();
        let registry = test_registry(Pubkey::new_unique());
        assert!(is_authorized_transfer(&pet.owner, &pet, &registry));
    }

    #[test]
    fn registry_authority_is_authorized_transfer() {
        let pet = test_pet();
        let authority = Pubkey::new_unique();
        let registry = test_registry(authority);
        assert!(is_authorized_transfer(&authority, &pet, &registry));
    }

    #[test]
    fn stranger_is_rejected_transfer() {
        let pet = test_pet();
        let registry = test_registry(Pubkey::new_unique());
        let stranger = Pubkey::new_unique();
        // Stranger is neither the owner nor the authority.
        assert_ne!(stranger, pet.owner);
        assert_ne!(stranger, registry.authority);
        assert!(!is_authorized_transfer(&stranger, &pet, &registry));
    }

    // --- Canonical color pool tests ----------------------------------------------

    #[test]
    fn coat_pools_have_eight_slots_each() {
        for species in [
            Species::Otter,
            Species::Cat,
            Species::Dog,
            Species::Monkey,
        ] {
            for i in 0..COAT_POOL_SIZE {
                assert!(
                    coat_color_name(species, i).is_some(),
                    "{:?} missing coat slot {}",
                    species,
                    i
                );
            }
            assert_eq!(coat_color_name(species, COAT_POOL_SIZE), None);
        }
    }

    #[test]
    fn creator_pet_coat_genes_resolve() {
        // Spot-check the exact gene slots from Brad's character sheets.
        assert_eq!(coat_color_name(Species::Otter, 2), Some("C - Clay")); // Luna
        assert_eq!(coat_color_name(Species::Otter, 3), Some("D - Chestnut")); // Luna
        assert_eq!(coat_color_name(Species::Otter, 0), Some("A - River Brown")); // Lenny
        assert_eq!(coat_color_name(Species::Otter, 1), Some("B - Sand")); // Lenny
        assert_eq!(coat_color_name(Species::Dog, 1), Some("B - Brown")); // Lucy, Loboe
        assert_eq!(coat_color_name(Species::Dog, 3), Some("D - Dark Brown")); // Lucy
        assert_eq!(coat_color_name(Species::Dog, 4), Some("E - White")); // Loboe
        assert_eq!(coat_color_name(Species::Cat, 0), Some("A - Black")); // Leo
        assert_eq!(coat_color_name(Species::Cat, 4), Some("E - White")); // Leo
        assert_eq!(coat_color_name(Species::Cat, 3), Some("D - Dark Brown")); // Lilah
        assert_eq!(coat_color_name(Species::Monkey, 1), Some("B - Light Brown")); // Lola
        assert_eq!(coat_color_name(Species::Monkey, 3), Some("D - Dark Brown")); // Loki
    }

    #[test]
    fn eye_pool_has_six_slots() {
        assert_eq!(EYE_POOL_SIZE, 6);
        for i in 0..EYE_POOL_SIZE {
            assert!(eye_color_name(i).is_some(), "missing eye slot {}", i);
        }
        assert_eq!(eye_color_name(EYE_POOL_SIZE), None);
    }

    #[test]
    fn creator_pet_eye_genes_resolve() {
        assert_eq!(eye_color_name(0), Some("E-A Amber")); // Luna
        assert_eq!(eye_color_name(1), Some("E-B Ocean Blue")); // Lenny
        assert_eq!(eye_color_name(3), Some("E-D Warm Brown")); // Lucy
        assert_eq!(eye_color_name(2), Some("E-C Rose Amber")); // Lilah
        assert_eq!(eye_color_name(4), Some("E-G Green (Glowing)")); // Leo
        assert_eq!(eye_color_name(5), Some("E-P Pink (Glowing)")); // Lola
    }

    // --- Creator-pet trait canon tests -------------------------------------------

    #[test]
    fn trait_counts_match_canon() {
        assert_eq!(CREATOR_PET_COUNT, 8);
        assert_eq!(u32::from(CREATOR_PET_COUNT), CREATOR_IDS_END);
        assert_eq!(LEGENDARY_TRAITS_PER_CREATOR, 2);
        assert_eq!(MYTHIC_MUTATIONS_PER_CREATOR, 1);
    }

    #[test]
    fn all_eight_creator_ids_have_traits() {
        for id in 1..=8u32 {
            assert!(
                creator_pet_traits(id).is_some(),
                "creator ID {} missing from trait canon",
                id
            );
        }
    }

    #[test]
    fn non_creator_ids_have_no_traits() {
        assert_eq!(creator_pet_traits(0), None);
        assert_eq!(creator_pet_traits(9), None);
        assert_eq!(creator_pet_traits(15_000), None);
        assert_eq!(creator_pet_traits(u32::MAX), None);
    }

    #[test]
    fn luna_traits_are_canon() {
        let t = creator_pet_traits(1).unwrap(); // Luna
        assert_eq!(t.legendary_a, LegendaryTrait::CrescentMoonChest);
        assert_eq!(t.legendary_b, LegendaryTrait::GlowingTail);
        assert_eq!(t.mythic, MythicMutation::LunarAura);
    }

    #[test]
    fn lenny_traits_are_canon() {
        let t = creator_pet_traits(2).unwrap(); // Lenny
        assert_eq!(t.legendary_a, LegendaryTrait::FlowMarkTail);
        assert_eq!(t.legendary_b, LegendaryTrait::TidecallerEyes);
        assert_eq!(t.mythic, MythicMutation::RiverSpiritTrail);
    }

    #[test]
    fn lucy_traits_are_canon() {
        let t = creator_pet_traits(3).unwrap(); // Lucy
        assert_eq!(t.legendary_a, LegendaryTrait::HeartlightChest);
        assert_eq!(t.legendary_b, LegendaryTrait::SunsetPaws);
        assert_eq!(t.mythic, MythicMutation::GoldenHeartAura);
    }

    #[test]
    fn loboe_traits_are_canon() {
        let t = creator_pet_traits(4).unwrap(); // Loboe
        assert_eq!(t.legendary_a, LegendaryTrait::EmberEyes);
        assert_eq!(t.legendary_b, LegendaryTrait::StonehideMarkings);
        assert_eq!(t.mythic, MythicMutation::MountainsStandAura);
    }

    #[test]
    fn lilah_traits_are_canon() {
        let t = creator_pet_traits(5).unwrap(); // Lilah
        assert_eq!(t.legendary_a, LegendaryTrait::MoonbeamWhiskers);
        assert_eq!(t.legendary_b, LegendaryTrait::RoseGlowEyes);
        assert_eq!(t.mythic, MythicMutation::DreamweaverAura);
    }

    #[test]
    fn leo_traits_are_canon() {
        let t = creator_pet_traits(6).unwrap(); // Leo
        assert_eq!(t.legendary_a, LegendaryTrait::GlowingArrowheadChest);
        assert_eq!(t.legendary_b, LegendaryTrait::GlowingGreenEyes);
        assert_eq!(t.mythic, MythicMutation::ConstellationCoat);
    }

    #[test]
    fn lola_traits_are_canon() {
        let t = creator_pet_traits(7).unwrap(); // Lola
        assert_eq!(t.legendary_a, LegendaryTrait::GlowingBananaMark);
        assert_eq!(t.legendary_b, LegendaryTrait::Bioluminescence);
        assert_eq!(t.mythic, MythicMutation::JunglePrism);
    }

    #[test]
    fn loki_traits_are_canon() {
        let t = creator_pet_traits(8).unwrap(); // Loki
        assert_eq!(t.legendary_a, LegendaryTrait::GlowingRedTailTip);
        assert_eq!(t.legendary_b, LegendaryTrait::SparkEyes);
        assert_eq!(t.mythic, MythicMutation::EmberTrail);
    }

    #[test]
    fn validate_creator_traits_accepts_canon() {
        // Every founder's canonical set must validate.
        for id in 1..=8u32 {
            let t = creator_pet_traits(id).unwrap();
            assert!(
                validate_creator_traits(id, t.legendary_a, t.legendary_b, t.mythic).is_ok(),
                "canon rejected for creator ID {}",
                id
            );
        }
    }

    #[test]
    fn validate_creator_traits_rejects_wrong_traits() {
        // Luna's ID with Leo's traits must fail.
        let leo = creator_pet_traits(6).unwrap();
        let err = validate_creator_traits(
            1,
            leo.legendary_a,
            leo.legendary_b,
            leo.mythic,
        )
        .unwrap_err();
        assert_eq!(err, PetRegistryError::CreatorTraitsMismatch.into());
    }

    #[test]
    fn validate_creator_traits_rejects_non_creator_id() {
        let luna = creator_pet_traits(1).unwrap();
        let err = validate_creator_traits(
            9,
            luna.legendary_a,
            luna.legendary_b,
            luna.mythic,
        )
        .unwrap_err();
        assert_eq!(err, PetRegistryError::NotACreatorId.into());
    }

    #[test]
    fn all_legendary_traits_have_names() {
        // Exhaustive: if a variant is added without a name, this fails to compile.
        let all = [
            LegendaryTrait::CrescentMoonChest,
            LegendaryTrait::GlowingTail,
            LegendaryTrait::FlowMarkTail,
            LegendaryTrait::TidecallerEyes,
            LegendaryTrait::HeartlightChest,
            LegendaryTrait::SunsetPaws,
            LegendaryTrait::EmberEyes,
            LegendaryTrait::StonehideMarkings,
            LegendaryTrait::MoonbeamWhiskers,
            LegendaryTrait::RoseGlowEyes,
            LegendaryTrait::GlowingArrowheadChest,
            LegendaryTrait::GlowingGreenEyes,
            LegendaryTrait::GlowingBananaMark,
            LegendaryTrait::Bioluminescence,
            LegendaryTrait::GlowingRedTailTip,
            LegendaryTrait::SparkEyes,
        ];
        assert_eq!(all.len(), 16); // 2 per founder x 8 founders
        for t in all {
            assert!(!legendary_trait_name(t).is_empty());
        }
    }

    #[test]
    fn all_mythic_mutations_have_names() {
        let all = [
            MythicMutation::LunarAura,
            MythicMutation::RiverSpiritTrail,
            MythicMutation::GoldenHeartAura,
            MythicMutation::MountainsStandAura,
            MythicMutation::DreamweaverAura,
            MythicMutation::ConstellationCoat,
            MythicMutation::JunglePrism,
            MythicMutation::EmberTrail,
        ];
        assert_eq!(all.len(), 8); // 1 per founder
        for m in all {
            assert!(!mythic_mutation_name(m).is_empty());
        }
    }

    // --- Expressed-trait bitmask tests (Step 5) ----------------------------------

    #[test]
    fn legendary_discriminants_are_stable() {
        // The bitmask system depends on declaration order. If anyone
        // reorders these variants, this test fails loudly.
        assert_eq!(LegendaryTrait::CrescentMoonChest as u8, 0);
        assert_eq!(LegendaryTrait::GlowingTail as u8, 1);
        assert_eq!(LegendaryTrait::FlowMarkTail as u8, 2);
        assert_eq!(LegendaryTrait::TidecallerEyes as u8, 3);
        assert_eq!(LegendaryTrait::HeartlightChest as u8, 4);
        assert_eq!(LegendaryTrait::SunsetPaws as u8, 5);
        assert_eq!(LegendaryTrait::EmberEyes as u8, 6);
        assert_eq!(LegendaryTrait::StonehideMarkings as u8, 7);
        assert_eq!(LegendaryTrait::MoonbeamWhiskers as u8, 8);
        assert_eq!(LegendaryTrait::RoseGlowEyes as u8, 9);
        assert_eq!(LegendaryTrait::GlowingArrowheadChest as u8, 10);
        assert_eq!(LegendaryTrait::GlowingGreenEyes as u8, 11);
        assert_eq!(LegendaryTrait::GlowingBananaMark as u8, 12);
        assert_eq!(LegendaryTrait::Bioluminescence as u8, 13);
        assert_eq!(LegendaryTrait::GlowingRedTailTip as u8, 14);
        assert_eq!(LegendaryTrait::SparkEyes as u8, 15);
    }

    #[test]
    fn mythic_discriminants_are_stable() {
        assert_eq!(MythicMutation::LunarAura as u8, 0);
        assert_eq!(MythicMutation::RiverSpiritTrail as u8, 1);
        assert_eq!(MythicMutation::GoldenHeartAura as u8, 2);
        assert_eq!(MythicMutation::MountainsStandAura as u8, 3);
        assert_eq!(MythicMutation::DreamweaverAura as u8, 4);
        assert_eq!(MythicMutation::ConstellationCoat as u8, 5);
        assert_eq!(MythicMutation::JunglePrism as u8, 6);
        assert_eq!(MythicMutation::EmberTrail as u8, 7);
    }

    #[test]
    fn bitmask_round_trip() {
        let mask = legendary_trait_bit(LegendaryTrait::CrescentMoonChest)
            | legendary_trait_bit(LegendaryTrait::SparkEyes);
        assert!(expresses_legendary(mask, LegendaryTrait::CrescentMoonChest));
        assert!(expresses_legendary(mask, LegendaryTrait::SparkEyes));
        assert!(!expresses_legendary(mask, LegendaryTrait::GlowingTail));
        assert_eq!(count_legendary_expressed(mask), 2);

        let m_mask = mythic_mutation_bit(MythicMutation::LunarAura);
        assert!(expresses_mythic(m_mask, MythicMutation::LunarAura));
        assert!(!expresses_mythic(m_mask, MythicMutation::EmberTrail));
        assert_eq!(count_mythic_expressed(m_mask), 1);

        assert_eq!(count_legendary_expressed(0), 0);
        assert_eq!(count_mythic_expressed(0), 0);
        assert_eq!(count_legendary_expressed(u16::MAX), 16);
        assert_eq!(count_mythic_expressed(u8::MAX), 8);
    }

    #[test]
    fn index_mapping_matches_discriminants() {
        for i in 0..16u8 {
            let t = legendary_trait_from_index(i);
            assert_eq!(t as u8, i);
            assert_eq!(legendary_trait_bit(t), 1u16 << i);
        }
        for i in 0..8u8 {
            let m = mythic_mutation_from_index(i);
            assert_eq!(m as u8, i);
            assert_eq!(mythic_mutation_bit(m), 1u8 << i);
        }
    }

    #[test]
    fn creator_masks_match_canon() {
        // Luna: Crescent Moon (bit 0) + Glowing Tail (bit 1), Lunar Aura (bit 0)
        let luna_l = creator_legendary_mask(1).unwrap();
        assert_eq!(luna_l, 0b11);
        assert_eq!(creator_mythic_mask(1).unwrap(), 0b1);
        // Loki: Glowing Red Tail Tip (bit 14) + Spark Eyes (bit 15), Ember Trail (bit 7)
        let loki_l = creator_legendary_mask(8).unwrap();
        assert_eq!(loki_l, (1u16 << 14) | (1u16 << 15));
        assert_eq!(creator_mythic_mask(8).unwrap(), 1u8 << 7);
        // Non-creator IDs have no masks
        assert_eq!(creator_legendary_mask(9), None);
        assert_eq!(creator_mythic_mask(0), None);
        // Every founder's mask has exactly 2 legendary bits
        for id in 1..=8u32 {
            assert_eq!(count_legendary_expressed(creator_legendary_mask(id).unwrap()), 2);
            assert_eq!(count_mythic_expressed(creator_mythic_mask(id).unwrap()), 1);
        }
    }
}
