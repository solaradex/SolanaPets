//! # SolanaPets Breeding Program
//!
//! Step 5 of the SolanaPets build. Mints new living NFPs from two parent
//! pets, implementing Brad's low-penetrance genetics model:
//!
//! - EVERY pet carries the full legendary + mythic gene set in its DNA.
//! - Expression is rare: most offspring never visually show special traits.
//! - Offspring can express traits NEITHER parent shows (carrier genetics).
//! - The 8 founders (IDs 1–8) express everything — they ARE the gene pool.
//!
//! ## Architecture
//! This program owns no pet data. It validates a breeding pair, rolls
//! inheritance with pure functions, then CPI-calls `mint_nfp` on the
//! pet-registry program to create the offspring. The registry authority
//! must co-sign (it is the payer and mint authority inside `mint_nfp`).
//!
//! ## Randomness — READ THIS
//! Devnet scaffold only: inheritance rolls use a splitmix64 PRNG seeded
//! from clock + slot + parent IDs. This is PREDICTABLE and must NEVER
//! secure mainnet value. Production replaces every roll with
//! Switchboard VRF before the security audit.
//!
//! DEVNET ONLY. No mainnet deployment until audit.
//!
//! Program ID (devnet): 4XjtZUPzHMJTLCkAT3ZWqUB4g3viopH1R1XhFU9hBeXQ
//! Keypair: target/deploy/breeding-keypair.json (devnet only)

use anchor_lang::prelude::*;
use pet_registry::{
    self, expresses_legendary, expresses_mythic, Gender, LegendaryTrait, LifeStage,
    MintNfpParams, MythicMutation, Pet, Rarity, Registry, Species, count_legendary_expressed,
    count_mythic_expressed, legendary_trait_bit, legendary_trait_from_index,
    mythic_mutation_bit, mythic_mutation_from_index,
};

// Devnet program ID — generated for this deployment, never mainnet.
declare_id!("4XjtZUPzHMJTLCkAT3ZWqUB4g3viopH1R1XhFU9hBeXQ");

// ============================================================================
// Constants — all tuning lives here, in the open.
// ============================================================================

/// Breeding cooldown applied to BOTH parents after a successful breeding:
/// 7 days real time.
pub const BREED_COOLDOWN_SECS: i64 = 7 * 24 * 60 * 60;

/// Pets below this health_score cannot breed. Neglected pets must recover
/// first — care is the way back.
pub const MIN_BREED_HEALTH: u8 = 50;

/// Mutation chances (percent): a mutated gene rolls a random pool value
/// instead of inheriting from a parent.
pub const COAT_MUTATION_CHANCE: u64 = 5;
pub const EYE_MUTATION_CHANCE: u64 = 5;
pub const MARKING_MUTATION_CHANCE: u64 = 5;

/// Legendary expression chance per trait (percent):
/// - 5% base (the genes are universal, expression is rare)
/// - 25% if ONE parent visually expresses the trait
/// - 50% if BOTH parents visually express it
pub const LEGENDARY_BASE_CHANCE: u64 = 5;
pub const LEGENDARY_ONE_PARENT_CHANCE: u64 = 25;
pub const LEGENDARY_TWO_PARENT_CHANCE: u64 = 50;

/// Mythic expression chance per mutation (percent):
/// - 1% base — witnessing a mythic should feel like witnessing something
/// - 10% if EITHER parent visually expresses the mutation
pub const MYTHIC_BASE_CHANCE: u64 = 1;
pub const MYTHIC_PARENT_CHANCE: u64 = 10;

/// Personality inheritance: offspring = parent average ± this (clamped 0–100).
pub const PERSONALITY_VARIANCE: u8 = 10;

// ============================================================================
// Program entrypoint
// ============================================================================

#[program]
pub mod breeding {
    use super::*;

    /// Breed two Adult pets of the same species and opposite genders.
    ///
    /// Flow:
    /// 1. Validate the pair (species, gender, life stage, ownership,
    ///    cooldown, health).
    /// 2. Roll inheritance (pure functions — the genetics model).
    /// 3. CPI into pet-registry `mint_nfp` to create the offspring.
    /// 4. Put both parents on breeding cooldown.
    pub fn breed_pets(ctx: Context<BreedPets>) -> Result<()> {
        let clock = Clock::get()?;
        let now = clock.unix_timestamp;
        let breeder_key = ctx.accounts.breeder.key();

        // --- 1. Validate ------------------------------------------------------
        validate_breed_pair(
            &ctx.accounts.parent_a,
            &ctx.accounts.parent_b,
            &breeder_key,
            &ctx.accounts.registry,
            now,
        )?;

        // --- 2. Roll inheritance -----------------------------------------------
        let mut rng = ChainRng::new(&clock, ctx.accounts.parent_a.id, ctx.accounts.parent_b.id);
        let offspring = inherit_offspring_traits(
            &ctx.accounts.parent_a,
            &ctx.accounts.parent_b,
            &mut rng,
        );

        let params = MintNfpParams {
            species: offspring.species,
            gender: offspring.gender,
            color_primary: offspring.color_primary,
            color_secondary: offspring.color_secondary,
            marking: offspring.marking,
            eye_color: offspring.eye_color,
            curiosity: offspring.curiosity,
            playfulness: offspring.playfulness,
            affection: offspring.affection,
            loyalty: offspring.loyalty,
            intelligence: offspring.intelligence,
            calmness: offspring.calmness,
            rarity: offspring.rarity,
            owner: breeder_key,
            legendary_expressed: offspring.legendary_expressed,
            mythic_expressed: offspring.mythic_expressed,
        };

        // The offspring's ID is the registry's next_id BEFORE the CPI mints.
        let offspring_id = ctx.accounts.registry.next_id;

        // --- 3. CPI into pet-registry -------------------------------------------
        let cpi_program = ctx.accounts.pet_registry_program.to_account_info();
        let cpi_accounts = pet_registry::cpi::accounts::MintNfp {
            authority: ctx.accounts.authority.as_ref().clone(),
            registry: ctx.accounts.registry.to_account_info(),
            pet: ctx.accounts.offspring.as_ref().clone(),
            system_program: ctx.accounts.system_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        pet_registry::cpi::mint_nfp(cpi_ctx, params)?;

        // --- 4. Cooldowns ----------------------------------------------------------
        ctx.accounts.parent_a.breed_cooldown_until = now
            .checked_add(BREED_COOLDOWN_SECS)
            .ok_or(BreedingError::MathOverflow)?;
        ctx.accounts.parent_b.breed_cooldown_until = now
            .checked_add(BREED_COOLDOWN_SECS)
            .ok_or(BreedingError::MathOverflow)?;

        msg!(
            "Bred NFP #{} ({:?} {:?}, gen {}) from #{} x #{}. Rarity: {:?} | legendary expressed: {} | mythic expressed: {}",
            offspring_id,
            offspring.species,
            offspring.gender,
            offspring.generation,
            ctx.accounts.parent_a.id,
            ctx.accounts.parent_b.id,
            offspring.rarity,
            count_legendary_expressed(offspring.legendary_expressed),
            count_mythic_expressed(offspring.mythic_expressed),
        );
        Ok(())
    }
}

// ============================================================================
// Account validation
// ============================================================================

#[derive(Accounts)]
pub struct BreedPets<'info> {
    /// The breeder — must own both parents (or be the registry authority).
    /// Becomes the offspring's owner. (Pays nothing in this scaffold; the
    /// registry authority funds the offspring account inside `mint_nfp`.)
    #[account(mut)]
    pub breeder: Signer<'info>,

    /// Registry mint authority — co-signs so the CPI into `mint_nfp`
    /// satisfies its `has_one = authority` check and pays for the new
    /// pet account.
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [pet_registry::REGISTRY_SEED],
        bump = registry.bump,
        has_one = authority @ BreedingError::UnauthorizedAuthority,
    )]
    pub registry: Account<'info, Registry>,

    #[account(
        mut,
        seeds = [pet_registry::PET_SEED, parent_a.id.to_le_bytes().as_ref()],
        bump = parent_a.bump,
    )]
    pub parent_a: Account<'info, Pet>,

    #[account(
        mut,
        seeds = [pet_registry::PET_SEED, parent_b.id.to_le_bytes().as_ref()],
        bump = parent_b.bump,
    )]
    pub parent_b: Account<'info, Pet>,

    /// CHECK: Created by the pet-registry program via CPI into `mint_nfp`.
    /// The inner program enforces seeds ["pet", registry.next_id] with its
    /// own `init` — a wrong account makes the CPI fail. We only require
    /// writability here.
    #[account(mut)]
    pub offspring: UncheckedAccount<'info>,

    /// The pet-registry program — constrained to its real devnet program ID.
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum BreedingError {
    #[msg("Breeding parents must be the same species")]
    SpeciesMismatch,

    #[msg("Breeding parents must be opposite genders")]
    SameGender,

    #[msg("Both breeding parents must be Adults")]
    NotAdult,

    #[msg("The two parents must be distinct pets")]
    SamePet,

    #[msg("Breeder must own both parents or be the registry authority")]
    UnauthorizedBreeder,

    #[msg("A parent is still on breeding cooldown")]
    ParentOnCooldown,

    #[msg("A parent's health is too low to breed — care for them first")]
    LowHealth,

    #[msg("Signer is not the registry mint authority")]
    UnauthorizedAuthority,

    #[msg("Arithmetic overflow in breeding math")]
    MathOverflow,
}

// ============================================================================
// Randomness
// ============================================================================

/// Anything that can produce u64 rolls. The on-chain path uses `ChainRng`;
/// unit tests use `FixedRng` for determinism.
pub trait RollSource {
    fn next_u64(&mut self) -> u64;
}

/// Roll 0–99.
pub fn roll_d100(rng: &mut impl RollSource) -> u64 {
    rng.next_u64() % 100
}

/// Roll 0..bound.
pub fn roll_below(rng: &mut impl RollSource, bound: u64) -> u64 {
    debug_assert!(bound > 0);
    rng.next_u64() % bound
}

/// Devnet-scaffold RNG: splitmix64 seeded from clock + slot + parent IDs.
///
/// PREDICTABLE — a validator or observer can reproduce these rolls.
/// Production MUST replace this with Switchboard VRF before mainnet.
pub struct ChainRng {
    seed: u64,
    counter: u64,
}

impl ChainRng {
    pub fn new(clock: &Clock, parent_a_id: u32, parent_b_id: u32) -> Self {
        // Fold the seed material with FNV-1a, then avalanche once through
        // splitmix64 before first use. NOT cryptographic — devnet scaffold.
        let mut bytes = [0u8; 24];
        bytes[0..8].copy_from_slice(&clock.unix_timestamp.to_le_bytes());
        bytes[8..16].copy_from_slice(&clock.slot.to_le_bytes());
        bytes[16..20].copy_from_slice(&parent_a_id.to_le_bytes());
        bytes[20..24].copy_from_slice(&parent_b_id.to_le_bytes());

        let mut seed: u64 = 0xCBF2_9CE4_4842_2252; // FNV offset basis
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            seed ^= u64::from_le_bytes(word);
            seed = seed.wrapping_mul(0x100_0000_01B3); // FNV prime
        }
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        seed = z ^ (z >> 31);
        Self { seed, counter: 0 }
    }
}

impl RollSource for ChainRng {
    fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.counter = self.counter.wrapping_add(1);
        let mut z = self
            .seed
            .wrapping_add(self.counter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

// ============================================================================
// Breeding validation (pure — unit-testable)
// ============================================================================

/// Enforces every rule for a breeding pair. Pure function: no accounts
/// needed beyond the data itself.
pub fn validate_breed_pair(
    parent_a: &Pet,
    parent_b: &Pet,
    breeder: &Pubkey,
    registry: &Registry,
    now: i64,
) -> Result<()> {
    require!(parent_a.id != parent_b.id, BreedingError::SamePet);
    require!(
        parent_a.species == parent_b.species,
        BreedingError::SpeciesMismatch
    );
    require!(
        parent_a.gender != parent_b.gender,
        BreedingError::SameGender
    );
    require!(
        parent_a.life_stage == LifeStage::Adult && parent_b.life_stage == LifeStage::Adult,
        BreedingError::NotAdult
    );

    // Ownership: the breeder owns both parents, or is the registry
    // authority (e.g. sanctioned breeding events, rescue flows).
    let is_authority = breeder == &registry.authority;
    require!(
        is_authority || (breeder == &parent_a.owner && breeder == &parent_b.owner),
        BreedingError::UnauthorizedBreeder
    );

    // Cooldowns.
    require!(
        parent_a.breed_cooldown_until <= now,
        BreedingError::ParentOnCooldown
    );
    require!(
        parent_b.breed_cooldown_until <= now,
        BreedingError::ParentOnCooldown
    );

    // Health gate — neglected pets cannot breed until they recover.
    require!(
        parent_a.health_score >= MIN_BREED_HEALTH
            && parent_b.health_score >= MIN_BREED_HEALTH,
        BreedingError::LowHealth
    );

    Ok(())
}

// ============================================================================
// Inheritance (pure — unit-testable)
// ============================================================================

/// The complete rolled trait set for an offspring, ready to become
/// `MintNfpParams`.
pub struct OffspringTraits {
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
    pub generation: u8,
    pub legendary_expressed: u16,
    pub mythic_expressed: u8,
}

/// Inherit one discrete gene: `mutation_chance`% to roll a fresh random
/// pool value, otherwise 50/50 from the parents.
pub fn inherit_gene(
    pa: u8,
    pb: u8,
    pool_size: u8,
    mutation_chance: u64,
    rng: &mut impl RollSource,
) -> u8 {
    if roll_d100(rng) < mutation_chance {
        return roll_below(rng, pool_size as u64) as u8;
    }
    if roll_d100(rng) < 50 {
        pa
    } else {
        pb
    }
}

/// Personality: the parents' average ± variance, clamped to 0–100.
/// Temperament runs in families, but every pet is its own soul.
pub fn inherit_personality(pa: u8, pb: u8, rng: &mut impl RollSource) -> u8 {
    let avg = (pa as u16 + pb as u16) / 2;
    let span = (PERSONALITY_VARIANCE as u64) * 2 + 1;
    let variance = roll_below(rng, span) as i16 - PERSONALITY_VARIANCE as i16;
    ((avg as i16 + variance).clamp(0, 100)) as u8
}

/// Per-trait legendary expression chance from the parents' expression.
pub fn legendary_expression_chance(pa_expresses: bool, pb_expresses: bool) -> u64 {
    match (pa_expresses, pb_expresses) {
        (true, true) => LEGENDARY_TWO_PARENT_CHANCE,
        (true, false) | (false, true) => LEGENDARY_ONE_PARENT_CHANCE,
        (false, false) => LEGENDARY_BASE_CHANCE,
    }
}

/// Per-mutation mythic expression chance from the parents' expression.
pub fn mythic_expression_chance(pa_expresses: bool, pb_expresses: bool) -> u64 {
    if pa_expresses || pb_expresses {
        MYTHIC_PARENT_CHANCE
    } else {
        MYTHIC_BASE_CHANCE
    }
}

/// Roll expression for all 16 legendary traits + 8 mythic mutations.
/// Every pet carries every gene — these rolls decide which ones wake up.
pub fn inherit_special_traits(
    pa: &Pet,
    pb: &Pet,
    rng: &mut impl RollSource,
) -> (u16, u8) {
    let mut legendary_mask: u16 = 0;
    for i in 0..16u8 {
        let t: LegendaryTrait = legendary_trait_from_index(i);
        let chance = legendary_expression_chance(
            expresses_legendary(pa.legendary_expressed, t),
            expresses_legendary(pb.legendary_expressed, t),
        );
        if roll_d100(rng) < chance {
            legendary_mask |= legendary_trait_bit(t);
        }
    }

    let mut mythic_mask: u8 = 0;
    for i in 0..8u8 {
        let m: MythicMutation = mythic_mutation_from_index(i);
        let chance = mythic_expression_chance(
            expresses_mythic(pa.mythic_expressed, m),
            expresses_mythic(pb.mythic_expressed, m),
        );
        if roll_d100(rng) < chance {
            mythic_mask |= mythic_mutation_bit(m);
        }
    }

    (legendary_mask, mythic_mask)
}

/// The higher of the two parents' rarities (enum order = ladder order).
pub fn parent_max_rarity(pa: &Pet, pb: &Pet) -> Rarity {
    if (pa.rarity as u8) >= (pb.rarity as u8) {
        pa.rarity
    } else {
        pb.rarity
    }
}

/// Rarity is earned through expression:
/// - any mythic expressed → Mythic
/// - 2+ legendary expressed → Legendary
/// - 1 legendary expressed → Epic
/// - none → at most Rare (bloodline prestige without new expression)
pub fn calculate_rarity(
    legendary_count: u32,
    mythic_count: u32,
    parent_max: Rarity,
) -> Rarity {
    if mythic_count > 0 {
        return Rarity::Mythic;
    }
    if legendary_count >= 2 {
        return Rarity::Legendary;
    }
    if legendary_count == 1 {
        return Rarity::Epic;
    }
    match parent_max {
        Rarity::Mythic | Rarity::Legendary | Rarity::Epic | Rarity::Rare => Rarity::Rare,
        Rarity::Uncommon => Rarity::Uncommon,
        Rarity::Common => Rarity::Common,
    }
}

/// Roll the full offspring trait set. Deterministic given the parents
/// and the roll source — which is exactly what makes it unit-testable.
pub fn inherit_offspring_traits(
    pa: &Pet,
    pb: &Pet,
    rng: &mut impl RollSource,
) -> OffspringTraits {
    let (legendary_expressed, mythic_expressed) = inherit_special_traits(pa, pb, rng);
    let rarity = calculate_rarity(
        count_legendary_expressed(legendary_expressed),
        count_mythic_expressed(mythic_expressed),
        parent_max_rarity(pa, pb),
    );

    OffspringTraits {
        species: pa.species, // validated: same species
        gender: if roll_d100(rng) < 50 {
            Gender::Male
        } else {
            Gender::Female
        },
        color_primary: inherit_gene(
            pa.color_primary,
            pb.color_primary,
            pet_registry::COAT_POOL_SIZE,
            COAT_MUTATION_CHANCE,
            rng,
        ),
        color_secondary: inherit_gene(
            pa.color_secondary,
            pb.color_secondary,
            pet_registry::COAT_POOL_SIZE,
            COAT_MUTATION_CHANCE,
            rng,
        ),
        marking: inherit_gene(
            pa.marking,
            pb.marking,
            16,
            MARKING_MUTATION_CHANCE,
            rng,
        ),
        eye_color: inherit_gene(
            pa.eye_color,
            pb.eye_color,
            pet_registry::EYE_POOL_SIZE,
            EYE_MUTATION_CHANCE,
            rng,
        ),
        curiosity: inherit_personality(pa.curiosity, pb.curiosity, rng),
        playfulness: inherit_personality(pa.playfulness, pb.playfulness, rng),
        affection: inherit_personality(pa.affection, pb.affection, rng),
        loyalty: inherit_personality(pa.loyalty, pb.loyalty, rng),
        intelligence: inherit_personality(pa.intelligence, pb.intelligence, rng),
        calmness: inherit_personality(pa.calmness, pb.calmness, rng),
        rarity,
        generation: pa.generation.max(pb.generation).saturating_add(1),
        legendary_expressed,
        mythic_expressed,
    }
}

/// Cooldown timestamp to stamp on both parents after breeding.
pub fn cooldown_until(now: i64) -> Result<i64> {
    now.checked_add(BREED_COOLDOWN_SECS)
        .ok_or(BreedingError::MathOverflow.into())
}

// ============================================================================
// Unit tests — `cargo test` (pure logic; no validator needed)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Fixtures ---------------------------------------------------------------

    fn test_registry(authority: Pubkey) -> Registry {
        Registry {
            authority,
            next_id: 100,
            total_minted: 91,
            bump: 254,
        }
    }

    /// An Adult pet owned by `owner`, healthy and off cooldown.
    fn test_parent(id: u32, species: Species, gender: Gender, owner: Pubkey) -> Pet {
        Pet {
            id,
            species,
            gender,
            generation: 0,
            color_primary: 2,
            color_secondary: 3,
            marking: 1,
            eye_color: 4,
            curiosity: 60,
            playfulness: 70,
            affection: 80,
            loyalty: 75,
            intelligence: 65,
            calmness: 55,
            rarity: Rarity::Common,
            parent_a: 0,
            parent_b: 0,
            owner,
            born_at: 1_000_000,
            life_stage: LifeStage::Adult,
            hunger: 90,
            happiness: 90,
            cleanliness: 90,
            energy: 90,
            love: 90,
            health_score: 90,
            bond_xp: 100,
            last_cared_at: 2_000_000,
            legendary_expressed: 0,
            mythic_expressed: 0,
            breed_cooldown_until: 0,
            bump: 255,
        }
    }

    /// Deterministic roll source: cycles through canned values.
    struct FixedRng {
        values: Vec<u64>,
        idx: usize,
    }

    impl FixedRng {
        fn new(values: Vec<u64>) -> Self {
            assert!(!values.is_empty());
            Self { values, idx: 0 }
        }
    }

    impl RollSource for FixedRng {
        fn next_u64(&mut self) -> u64 {
            let v = self.values[self.idx % self.values.len()];
            self.idx += 1;
            v
        }
    }

    // --- Validation tests ----------------------------------------------------------

    #[test]
    fn valid_pair_passes() {
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        assert!(validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).is_ok());
    }

    #[test]
    fn rejects_same_pet_twice() {
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(9, Species::Otter, Gender::Male, owner); // same id
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::SamePet.into());
    }

    #[test]
    fn rejects_species_mismatch() {
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Cat, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::SpeciesMismatch.into());
    }

    #[test]
    fn rejects_same_gender() {
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Female, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::SameGender.into());
    }

    #[test]
    fn rejects_non_adult_parent() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.life_stage = LifeStage::Baby;
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::NotAdult.into());
    }

    #[test]
    fn rejects_stranger_breeder() {
        let owner = Pubkey::new_unique();
        let stranger = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &stranger, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::UnauthorizedBreeder.into());
    }

    #[test]
    fn rejects_half_ownership() {
        // Breeder owns only one parent — not enough.
        let owner_a = Pubkey::new_unique();
        let owner_b = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner_a);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner_b);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner_a, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::UnauthorizedBreeder.into());
    }

    #[test]
    fn registry_authority_may_breed_any_pair() {
        // The authority acts with breeding permission (sanctioned events,
        // rescue flows) even without owning the parents.
        let authority = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, Pubkey::new_unique());
        let pb = test_parent(10, Species::Otter, Gender::Male, Pubkey::new_unique());
        let registry = test_registry(authority);
        assert!(validate_breed_pair(&pa, &pb, &authority, &registry, 3_000_000).is_ok());
    }

    #[test]
    fn rejects_parent_on_cooldown() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.breed_cooldown_until = 3_000_000 + 100; // still cooling down
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::ParentOnCooldown.into());
    }

    #[test]
    fn cooldown_boundary_passes() {
        // Cooldown expiring exactly now is ready.
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.breed_cooldown_until = 3_000_000;
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        assert!(validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).is_ok());
    }

    #[test]
    fn rejects_low_health_parent() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.health_score = MIN_BREED_HEALTH - 1;
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        let err = validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).unwrap_err();
        assert_eq!(err, BreedingError::LowHealth.into());
    }

    #[test]
    fn health_threshold_boundary_passes() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.health_score = MIN_BREED_HEALTH; // exactly at the gate
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let registry = test_registry(Pubkey::new_unique());
        assert!(validate_breed_pair(&pa, &pb, &owner, &registry, 3_000_000).is_ok());
    }

    #[test]
    fn cooldown_math() {
        let until = cooldown_until(1_000_000).unwrap();
        assert_eq!(until, 1_000_000 + BREED_COOLDOWN_SECS);
        assert_eq!(BREED_COOLDOWN_SECS, 7 * 24 * 60 * 60);
    }

    // --- Inheritance tests -----------------------------------------------------------

    #[test]
    fn inherit_gene_picks_parent_a_on_low_flip() {
        // mutation roll 50 (>= 5, no mutation), coin flip 10 (< 50) → pa
        let mut rng = FixedRng::new(vec![50, 10]);
        assert_eq!(inherit_gene(2, 7, 8, COAT_MUTATION_CHANCE, &mut rng), 2);
    }

    #[test]
    fn inherit_gene_picks_parent_b_on_high_flip() {
        // mutation roll 50 (no mutation), coin flip 90 (>= 50) → pb
        let mut rng = FixedRng::new(vec![50, 90]);
        assert_eq!(inherit_gene(2, 7, 8, COAT_MUTATION_CHANCE, &mut rng), 7);
    }

    #[test]
    fn inherit_gene_mutates_to_random_pool_value() {
        // mutation roll 3 (< 5) → random pool value: roll_below(8) with 6 → 6
        let mut rng = FixedRng::new(vec![3, 6]);
        assert_eq!(inherit_gene(2, 7, 8, COAT_MUTATION_CHANCE, &mut rng), 6);
    }

    #[test]
    fn inherit_personality_averages_with_variance() {
        // roll_below(21) with 10 → variance 10 - 10 = 0 → exact average
        let mut rng = FixedRng::new(vec![10]);
        assert_eq!(inherit_personality(60, 80, &mut rng), 70);
    }

    #[test]
    fn inherit_personality_respects_variance_bounds() {
        // roll 0 → variance -10; roll 20 → variance +10
        let mut rng = FixedRng::new(vec![0]);
        assert_eq!(inherit_personality(60, 80, &mut rng), 60);
        let mut rng = FixedRng::new(vec![20]);
        assert_eq!(inherit_personality(60, 80, &mut rng), 80);
    }

    #[test]
    fn inherit_personality_clamps_at_edges() {
        // avg 95, variance +10 → would be 105 → clamps to 100
        let mut rng = FixedRng::new(vec![20]);
        assert_eq!(inherit_personality(90, 100, &mut rng), 100);
        // avg 5, variance -10 → would be -5 → clamps to 0
        let mut rng = FixedRng::new(vec![0]);
        assert_eq!(inherit_personality(0, 10, &mut rng), 0);
    }

    #[test]
    fn legendary_chance_table_is_correct() {
        assert_eq!(legendary_expression_chance(false, false), LEGENDARY_BASE_CHANCE);
        assert_eq!(legendary_expression_chance(false, false), 5);
        assert_eq!(legendary_expression_chance(true, false), 25);
        assert_eq!(legendary_expression_chance(false, true), 25);
        assert_eq!(legendary_expression_chance(true, true), 50);
    }

    #[test]
    fn mythic_chance_table_is_correct() {
        assert_eq!(mythic_expression_chance(false, false), MYTHIC_BASE_CHANCE);
        assert_eq!(mythic_expression_chance(false, false), 1);
        assert_eq!(mythic_expression_chance(true, false), 10);
        assert_eq!(mythic_expression_chance(false, true), 10);
        assert_eq!(mythic_expression_chance(true, true), 10);
    }

    #[test]
    fn expressing_parent_boosts_trait_inheritance() {
        // Parent A expresses CrescentMoonChest (bit 0). Rolls: 10 for trait 0
        // (< 25 → expressed), 50 for the other 15 (>= their chances → not).
        // Mythic rolls: all 50 (>= 1 → none).
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.legendary_expressed = legendary_trait_bit(LegendaryTrait::CrescentMoonChest);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);

        let mut rolls = vec![10u64];
        rolls.extend(vec![50u64; 15]); // remaining 15 legendary
        rolls.extend(vec![50u64; 8]); // 8 mythic
        let mut rng = FixedRng::new(rolls);

        let (l_mask, m_mask) = inherit_special_traits(&pa, &pb, &mut rng);
        assert!(expresses_legendary(l_mask, LegendaryTrait::CrescentMoonChest));
        assert_eq!(count_legendary_expressed(l_mask), 1);
        assert_eq!(m_mask, 0);
    }

    #[test]
    fn base_rate_can_still_express_without_parent_expression() {
        // No parent expresses anything. Trait 5 rolls 3 (< 5 base) → expressed.
        // Everything else rolls 50. Mythic all 50 (>= 1) → none.
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);

        let mut rolls = vec![50u64; 5];
        rolls.push(3);
        rolls.extend(vec![50u64; 10]); // 16 legendary total
        rolls.extend(vec![50u64; 8]); // 8 mythic
        let mut rng = FixedRng::new(rolls);

        let (l_mask, m_mask) = inherit_special_traits(&pa, &pb, &mut rng);
        // trait index 5 = SunsetPaws
        assert!(expresses_legendary(l_mask, LegendaryTrait::SunsetPaws));
        assert_eq!(count_legendary_expressed(l_mask), 1);
        assert_eq!(m_mask, 0);
    }

    #[test]
    fn mythic_can_express_at_base_rate() {
        // Mythic mutation index 2 rolls 0 (< 1 base) → expressed. The genes
        // wake up even when neither parent shows them — carrier genetics.
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);

        let mut rolls = vec![50u64; 16]; // no legendary
        rolls.push(50);
        rolls.push(50);
        rolls.push(0); // mythic index 2 < 1 → GoldenHeartAura expresses
        rolls.extend(vec![50u64; 5]);
        let mut rng = FixedRng::new(rolls);

        let (l_mask, m_mask) = inherit_special_traits(&pa, &pb, &mut rng);
        assert_eq!(l_mask, 0);
        assert!(expresses_mythic(m_mask, MythicMutation::GoldenHeartAura));
        assert_eq!(count_mythic_expressed(m_mask), 1);
    }

    #[test]
    fn rarity_ladder_from_expression() {
        // Mythic expressed → Mythic, regardless of anything else.
        assert_eq!(calculate_rarity(0, 1, Rarity::Common), Rarity::Mythic);
        assert_eq!(calculate_rarity(3, 2, Rarity::Common), Rarity::Mythic);
        // 2+ legendary → Legendary.
        assert_eq!(calculate_rarity(2, 0, Rarity::Common), Rarity::Legendary);
        assert_eq!(calculate_rarity(5, 0, Rarity::Common), Rarity::Legendary);
        // Exactly 1 legendary → Epic.
        assert_eq!(calculate_rarity(1, 0, Rarity::Common), Rarity::Epic);
        // No expression → bloodline prestige caps at Rare.
        assert_eq!(calculate_rarity(0, 0, Rarity::Mythic), Rarity::Rare);
        assert_eq!(calculate_rarity(0, 0, Rarity::Legendary), Rarity::Rare);
        assert_eq!(calculate_rarity(0, 0, Rarity::Epic), Rarity::Rare);
        assert_eq!(calculate_rarity(0, 0, Rarity::Rare), Rarity::Rare);
        assert_eq!(calculate_rarity(0, 0, Rarity::Uncommon), Rarity::Uncommon);
        assert_eq!(calculate_rarity(0, 0, Rarity::Common), Rarity::Common);
    }

    #[test]
    fn parent_max_rarity_picks_higher() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.rarity = Rarity::Rare;
        let mut pb = test_parent(10, Species::Otter, Gender::Male, owner);
        pb.rarity = Rarity::Epic;
        assert_eq!(parent_max_rarity(&pa, &pb), Rarity::Epic);
        assert_eq!(parent_max_rarity(&pb, &pa), Rarity::Epic);
    }

    #[test]
    fn full_inheritance_is_deterministic() {
        let owner = Pubkey::new_unique();
        let pa = test_parent(9, Species::Otter, Gender::Female, owner);
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);

        // Roll plan (39 rolls total):
        // 16 legendary: all 50 → none expressed (base 5)
        // 8 mythic: all 50 → none expressed (base 1)
        // gender: 10 → Male
        // color_primary: 50 (no mut), 10 → pa (2)
        // color_secondary: 50 (no mut), 90 → pb (3)
        // marking: 50 (no mut), 10 → pa (1)
        // eye_color: 50 (no mut), 90 → pb (4)
        // 6 personality: 10 each → variance 0 → exact averages
        let mut rolls = vec![50u64; 16];
        rolls.extend(vec![50u64; 8]);
        rolls.push(10); // gender → Male
        rolls.extend(vec![50, 10]); // color_primary → pa
        rolls.extend(vec![50, 90]); // color_secondary → pb
        rolls.extend(vec![50, 10]); // marking → pa
        rolls.extend(vec![50, 90]); // eye_color → pb
        rolls.extend(vec![10u64; 6]); // personalities → averages
        assert_eq!(rolls.len(), 39);

        let mut rng = FixedRng::new(rolls);
        let t = inherit_offspring_traits(&pa, &pb, &mut rng);

        assert_eq!(t.species, Species::Otter);
        assert_eq!(t.gender, Gender::Male);
        assert_eq!(t.color_primary, 2);
        assert_eq!(t.color_secondary, 3);
        assert_eq!(t.marking, 1);
        assert_eq!(t.eye_color, 4);
        assert_eq!(t.curiosity, 60);
        assert_eq!(t.playfulness, 70);
        assert_eq!(t.affection, 80);
        assert_eq!(t.loyalty, 75);
        assert_eq!(t.intelligence, 65);
        assert_eq!(t.calmness, 55);
        assert_eq!(t.rarity, Rarity::Common); // no traits, Common parents
        assert_eq!(t.generation, 1);
        assert_eq!(t.legendary_expressed, 0);
        assert_eq!(t.mythic_expressed, 0);
    }

    #[test]
    fn generation_increments_from_higher_parent() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.generation = 3;
        let mut pb = test_parent(10, Species::Otter, Gender::Male, owner);
        pb.generation = 5;
        // all rolls neutral: no traits, averages
        let mut rng = FixedRng::new(vec![50]);
        let t = inherit_offspring_traits(&pa, &pb, &mut rng);
        assert_eq!(t.generation, 6);
    }

    #[test]
    fn generation_saturates_at_255() {
        let owner = Pubkey::new_unique();
        let mut pa = test_parent(9, Species::Otter, Gender::Female, owner);
        pa.generation = 255;
        let pb = test_parent(10, Species::Otter, Gender::Male, owner);
        let mut rng = FixedRng::new(vec![50]);
        let t = inherit_offspring_traits(&pa, &pb, &mut rng);
        assert_eq!(t.generation, 255); // saturates, never wraps
    }

    #[test]
    fn chain_rng_is_deterministic_for_same_inputs() {
        let clock = Clock {
            unix_timestamp: 1_700_000_000,
            slot: 123_456,
            ..Default::default()
        };
        let mut a = ChainRng::new(&clock, 9, 10);
        let mut b = ChainRng::new(&clock, 9, 10);
        for _ in 0..20 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn chain_rng_differs_across_pairs() {
        let clock = Clock {
            unix_timestamp: 1_700_000_000,
            slot: 123_456,
            ..Default::default()
        };
        let mut a = ChainRng::new(&clock, 9, 10);
        let mut b = ChainRng::new(&clock, 11, 12);
        // At least one of the first few rolls should differ (overwhelmingly likely).
        let same = (0..5).all(|_| a.next_u64() == b.next_u64());
        assert!(!same, "different pairs should seed different streams");
    }
}
