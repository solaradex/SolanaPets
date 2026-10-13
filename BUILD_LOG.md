# SolanaPets Build Log

## Step 1 — Pet Registry Program (2026-10-13)

**Status:** ✅ Complete — compiles clean, all tests pass.

### What was built
The foundation of the SolanaPets NFP ecosystem: an Anchor (Rust) Solana program
that mints living on-chain pets. Every byte of a pet's identity — species,
genetics, personality, lineage, care stats, health — lives in its Pet account.

### Files
- `Anchor.toml` — Anchor workspace config (devnet, program ID registered)
- `Cargo.toml` — workspace manifest
- `programs/pet-registry/Cargo.toml` — program crate (anchor-lang 0.32.2)
- `programs/pet-registry/src/lib.rs` — the full program (~470 lines)

### Program design
- **Two instructions:**
  - `initialize_registry` — one-time setup; sets mint authority, starts public
    minting at ID 9 (IDs 1–8 permanently reserved for Brad's 8 creator pets).
  - `mint_nfp` — mints a new NFP with the next sequential ID. Only the
    registry authority can call it (`has_one` check). Newborns start at full
    care stats, Baby life stage, generation 0, no parents.
- **Pet account (90 bytes):** id, species (Otter/Cat/Dog/Monkey), gender,
  generation, color genes (0–7), marking (0–15), eye color (0–5),
  6 personality traits (0–100), rarity (Common→Mythic), parent IDs,
  owner, born_at, life stage, 5 care stats + health (0–100), bond_xp.
- **Registry account (41 bytes):** authority, next_id, total_minted.
- **PDAs:** `["registry"]` and `["pet", id_le_bytes]` — the pet's address
  itself proves ID sequencing.
- **Guards:** creator IDs (1–8) rejected, 750k max supply enforced,
  all trait ranges validated, unauthorized minters rejected.

### Test results
`cargo test -p pet-registry` — **12 passed, 0 failed:**
- valid_traits_pass, boundary_traits_pass
- rejects_bad_color_gene / _eye_color / _marking / _personality
- creator_ids_are_reserved, public_minting_starts_at_nine
- genesis_range_is_correct, max_supply_guard, pet_account_size_is_sane
- (plus Anchor's built-in `test_id`)

`cargo check` — clean (only standard `anchor-debug` cfg warnings from
building without the Anchor CLI; harmless).

### Notes
- Devnet program ID: `HZhS2qw6edrL7YKjUZopjJGdSfh9UJniVCAZ9ZMkQFkz`
- No mainnet deployment. Ever, until audit.
- Creator pets (IDs 1–8) get a dedicated `mint_creator_pet` instruction later
  with mythic traits — deliberately not in this step.

## Step 2 — Care Actions (2026-10-13)

**Status:** ✅ Complete — compiles clean, all tests pass.

### What was added
Five care-action instructions on the pet registry program — the heartbeat of
the "living NFP" vision. Every action restores a care stat, grants bond XP,
stamps `last_cared_at`, and may nudge `health_score` when all stats thrive.

- `feed_pet` — hunger +25 (clamped 100), +5 bond XP
- `play_with_pet` — happiness +20, energy −10 (floors at 0), +10 bond XP
- `clean_pet` — cleanliness restored to 100, +5 bond XP
- `rest_pet` — energy +30 (clamped 100), +3 bond XP. **Rejected at full
  energy** (`AlreadyRested`) — a rested pet has nothing to recover.
- `give_affection` — love +20, **+15 bond XP** (the biggest gain)

### Authorization
New `CareForPet` accounts struct: the `caregiver` signer must equal
`pet.owner` **or** `registry.authority` (pure fn `is_authorized_caregiver`,
enforced by `authorize_caregiver`). The registry is read-only in care
contexts — care never changes who can mint.

### Account size change
- Pet account grew **90 → 98 bytes**: added `last_cared_at: i64` (unix
  timestamp of the last care action), initialized at mint from the clock.
- `Pet::SIZE` updated; `MintNfp`'s `space = 8 + Pet::SIZE` picks it up
  automatically. **Note:** existing devnet pet accounts (if any) would need
  reallocation — fine, since nothing is deployed yet.

### Health model (simple v1)
`maybe_boost_health`: when all five care stats ≥ 80, `health_score` ticks
+1 (capped at 100). Small and steady — never a shortcut past real neglect.

### Tuning constants
All amounts live in named constants (`BOND_XP_FEED`, `FEED_AMOUNT`, etc.)
so the game client and future breeding gates read the same numbers.

### New errors
- `UnauthorizedCaregiver` — signer is neither owner nor registry authority
- `AlreadyRested` — rest attempted at full energy
- `MathOverflow` — bond_xp checked-add guard (bond never wraps)

### Test results
`cargo test -p pet-registry` — **28 passed, 0 failed** (12 from Step 1 + 16 new):
- feed_restores_hunger_and_grants_bond, feed_clamps_at_100
- play_boosts_happiness_costs_energy, play_energy_floors_at_zero
- clean_restores_fully
- rest_restores_energy, rest_rejected_at_full_energy, rest_clamps_at_100
- affection_grants_biggest_bond, bond_xp_accumulates_across_actions
- health_boosts_when_all_stats_thriving, no_health_boost_when_any_stat_low,
  health_never_exceeds_100
- owner_is_authorized_caregiver, registry_authority_is_authorized_caregiver,
  stranger_is_rejected_caregiver

`cargo check` — clean (only the standard `anchor-debug` cfg warnings from
building without the Anchor CLI; harmless, same as Step 1).
- Anchor CLI not installed; project scaffolded manually and verified with
  `cargo check` + `cargo test` (rustc 1.99.0, anchor-lang 0.32.2).

### Next steps
1. `mint_creator_pet` instruction for IDs 1–8 (mythic traits).
2. Breeding program (separate crate).

## Step 3 — Canonical Color Pools (2026-10-13)

**Status:** ✅ Complete — compiles clean, all tests pass.

### What was done
Established the canonical fur + eye color pools from Brad's 8 Genesis
Founder character sheets. Gene letters and color names are canonical from
the sheets; only genuinely empty slots were filled (themed per species).

### Files
- `COLOR_POOLS.md` — full tables: 8 coat colors per species (gene letter →
  name → approx. hex → which creator pet), 6 eye colors, plus natural
  markings + fantasy mutations reference.
- `programs/pet-registry/src/lib.rs` — added:
  - `COAT_POOL_SIZE` (8) / `EYE_POOL_SIZE` (6) constants (documented as
    the canonical pools; `MAX_COLOR_GENE`/`MAX_EYE_COLOR` remain the
    validation bounds).
  - `coat_color_name(species, index) -> Option<&'static str>` — pure lookup
    over the per-species 8-color tables.
  - `eye_color_name(index) -> Option<&'static str>` — pure lookup over the
    6-color table.

### The pools
- **Otters:** A-River Brown, B-Sand, C-Clay, D-Chestnut (from sheets);
  E-White, F-Driftwood, G-Pebble, H-Midnight (filled).
- **Dogs:** B-Brown (Neutral/Fawn shades), D-Dark Brown, E-White (from
  sheets); A-Black, C-Tan, F-Golden, G-Slate, H-Cream (filled).
- **Cats:** A-Black, B-Neutral Brown, D-Dark Brown, E-White (from sheets);
  C-Smoke Gray, F-Ginger, G-Cream, H-Silver (filled).
- **Monkeys:** B-Light Brown, D-Dark Brown, E-White (from sheets);
  A-Ebony, C-Copper, F-Golden, G-Ash Gray, H-Tan (filled).
- **Eyes (6):** E-A Amber (Luna, Loboe) · E-B Ocean Blue (Lenny, Loki) ·
  E-C Rose Amber (Lilah) · E-D Warm Brown (Lucy) · E-G Green, glowing
  (Leo) · E-P Pink, glowing (Lola).

### Sheet-code normalization
- **E-B was triple-booked** on the sheets: Ocean Blue (Lenny), Warm Brown
  (Lucy), Blue Bright (Loki). Consolidated: E-B = blue family;
  Lucy's Warm Brown normalized to its own slot **E-D** (fills the natural
  letter gap between C and G). All sheet color *names* preserved.
- E-A covers both Luna's Amber and Loboe's Hazel Brown as shades.
- Sheet gene letters E-G / E-P kept verbatim (not re-lettered).

### Test results
`cargo test -p pet-registry` — **32 passed, 0 failed** (28 existing + 4 new):
- coat_pools_have_eight_slots_each (all species × all 8 indices, None at 8)
- creator_pet_coat_genes_resolve (12 spot-checks against the sheets)
- eye_pool_has_six_slots (all 6 indices, None at 6)
- creator_pet_eye_genes_resolve (all 6 creator eye genes)

`cargo check` — clean (only the standard `anchor-debug` cfg warnings;
harmless, same as Steps 1–2).

## Step 4 — Creator Pet Trait Canon Locked (2026-10-13)

**Status:** ✅ Complete — compiles clean, all tests pass.

### What was done
Locked in Brad's final trait assignments for all 8 creator pets: each gets
exactly **2 legendary traits** + **1 mythic mutation**, all fully visual.
These traits seed the gene pool that every future pet carries (low-penetrance
expression model — genes are universal, visible expression is rare).

### Files
- `CREATOR_PETS.md` — canonical reference doc: full trait profile per pet
  (name, species, gender, genesis ID, 2 legendary traits with descriptions,
  mythic mutation with description, gene pool contribution note), summary
  table, gene pool model explanation, and on-chain mapping notes.
- `programs/pet-registry/src/lib.rs` — added:
  - `CREATOR_PET_COUNT` (8), `LEGENDARY_TRAITS_PER_CREATOR` (2),
    `MYTHIC_MUTATIONS_PER_CREATOR` (1) constants.
  - `LegendaryTrait` enum — all 16 legendary traits (2 per founder).
  - `MythicMutation` enum — all 8 mythic mutations (1 per founder).
  - `CreatorTraits` struct — (legendary_a, legendary_b, mythic).
  - `legendary_trait_name()` / `mythic_mutation_name()` — pure lookups.
  - `creator_pet_traits(pet_id) -> Option<CreatorTraits>` — the single
    source of truth mapping IDs 1–8 to their canonical traits. Returns
    `None` for any other ID.
  - `validate_creator_traits()` — pure fn the future `mint_creator_pet`
    instruction will call to enforce the canon on-chain.
  - New errors: `NotACreatorId`, `CreatorTraitsMismatch`.

### The canon (Brad's original names)
| ID | Name | Legendary 1 | Legendary 2 | Mythic |
|----|------|-------------|-------------|--------|
| 1 | Luna (F otter) | Crescent Moon chest | Glowing Tail | Lunar Aura |
| 2 | Lenny (M otter) | Flow Mark tail | Tidecaller Eyes | River Spirit Trail |
| 3 | Lucy (F dachshund) | Heartlight Chest | Sunset Paws | Golden Heart Aura |
| 4 | Loboe (M bulldog) | Ember Eyes | Stonehide Markings | Mountain's Stand Aura |
| 5 | Lilah (F cat) | Moonbeam Whiskers | Rose Glow Eyes | Dreamweaver Aura |
| 6 | Leo (M cat) | Glowing Arrowhead | Glowing Green Eyes | Constellation Coat |
| 7 | Lola (F capuchin) | Banana Mark | Bioluminescence | Jungle Prism |
| 8 | Loki (M capuchin) | Glowing Tail Tip | Spark Eyes | Ember Trail |

### Design notes
- Lucy, Loboe, Lilah, and Lenny show no visible mutations on their sheets
  ("natural beauty") but carry the full gene set — their legendary/mythic
  traits are now defined and fully visual on the founders.
- The `Pet` account layout was NOT changed (still 98 bytes). Trait fields
  will be added when the dedicated `mint_creator_pet` instruction is built;
  `validate_creator_traits` is ready for it.
- `COLOR_POOLS.md` was left untouched — no conflicts with the trait canon.

### Test results
`cargo test -p pet-registry` — **48 passed, 0 failed** (32 existing + 16 new):
- trait_counts_match_canon (counts + CREATOR_PET_COUNT == CREATOR_IDS_END)
- all_eight_creator_ids_have_traits, non_creator_ids_have_no_traits
- Per-pet canon tests: luna/lenny/lucy/loboe/lilah/leo/lola/loki_traits_are_canon
- validate_creator_traits_accepts_canon (all 8 founders)
- validate_creator_traits_rejects_wrong_traits (Luna ID + Leo traits → mismatch)
- validate_creator_traits_rejects_non_creator_id (ID 9 → NotACreatorId)
- all_legendary_traits_have_names (16/16), all_mythic_mutations_have_names (8/8)

`cargo check` — clean (only the standard `anchor-debug` cfg warnings;
harmless, same as Steps 1–3).

### Next steps
1. `mint_creator_pet` instruction for IDs 1–8 (uses `validate_creator_traits`).
2. Breeding program (separate crate) — carrier genetics + low-penetrance expression.

## Step 5 — Breeding Program (2026-10-13)

**Status:** ✅ Complete — compiles clean, all tests pass.

### What was built
A SEPARATE Anchor program crate (`programs/breeding/`) implementing Brad's
low-penetrance genetics model. It validates a breeding pair, rolls
inheritance with pure functions, then CPI-calls `mint_nfp` on the
pet-registry program to create the offspring. The registry authority
co-signs (it's the payer + mint authority inside `mint_nfp`).

### Files
- `programs/breeding/Cargo.toml` — program crate (anchor-lang 0.32.2,
  depends on `pet-registry` with the `cpi` feature for type-safe CPI)
- `programs/breeding/src/lib.rs` — the full program (~950 lines)
- `target/deploy/breeding-keypair.json` — devnet keypair (devnet only)

### pet-registry changes (same step — the two programs co-evolve)
- `Pet` account grew **98 → 109 bytes**: added `legendary_expressed: u16`
  (bitmask of the 16 `LegendaryTrait` variants), `mythic_expressed: u8`
  (bitmask of the 8 `MythicMutation` variants), and
  `breed_cooldown_until: i64` (unix timestamp; 0 = ready).
- `MintNfpParams` gained `legendary_expressed: u16` + `mythic_expressed: u8`
  (breeding passes the rolled masks; raffle mints pass 0; the future
  `mint_creator_pet` will pass the canonical founder masks).
- `#[repr(u8)]` added to `LegendaryTrait` / `MythicMutation` — locks
  discriminants so bit positions can never drift. New helpers:
  `legendary_trait_bit`, `mythic_mutation_bit`, `expresses_legendary`,
  `expresses_mythic`, `count_legendary_expressed`,
  `count_mythic_expressed`, `legendary_trait_from_index`,
  `mythic_mutation_from_index`, `creator_legendary_mask`,
  `creator_mythic_mask`.
- 5 new pet-registry tests (discriminant stability ×2, bitmask round-trip,
  index mapping, creator masks) — pet-registry now **53 passed, 0 failed**.

### Breeding rules (all enforced in `validate_breed_pair`)
- Same species, opposite genders, both Adults, two distinct pets
- Breeder owns BOTH parents, or breeder IS the registry authority
- Neither parent on cooldown (`breed_cooldown_until <= now`)
- Both parents `health_score >= 50` — neglected pets can't breed until
  they recover (Brad's original design requirement)

### Inheritance model (all pure functions — `cargo test`, no validator)
| Trait | Rule |
|---|---|
| Coat color (×2) | 5% mutation → random pool color; else 50/50 from parents |
| Eye color | 5% mutation → random pool color; else 50/50 from parents |
| Marking | 5% mutation → random 0–15; else 50/50 from parents |
| Personality (×6) | Parent average ± 10, clamped 0–100 |
| Gender | 50/50 |
| Legendary (per trait) | 5% base · 25% if one parent expresses it · 50% if both |
| Mythic (per mutation) | 1% base · 10% if either parent expresses it |
| Generation | max(parent generations) + 1 (saturates at 255) |
| Rarity | Mythic if any mythic expressed · Legendary if 2+ legendary · Epic if 1 legendary · else bloodline prestige capped at Rare |
| Lineage | parent_a / parent_b IDs recorded on the offspring |
| Cooldown | both parents stamped `now + 7 days` after breeding |

### Randomness — READ THIS
Devnet scaffold: `ChainRng` = splitmix64 seeded from FNV-1a folded
(clock.unix_timestamp + slot + parent IDs). PREDICTABLE — must NEVER
secure mainnet value. Production replaces every roll with Switchboard
VRF before the security audit. This is documented in-code and here.

### CPI design notes
- `breed_pets` takes `breeder` (Signer, future owner), `authority`
  (Signer, registry mint authority), `registry`, `parent_a`, `parent_b`,
  `offspring` (UncheckedAccount — created by the inner `mint_nfp` via its
  own `init`; a wrong account makes the CPI fail), `pet_registry_program`
  (constrained to the real program ID), `system_program`.
- The generated `pet_registry::cpi::accounts::MintNfp` struct takes plain
  `AccountInfo`s (Anchor flattens `Signer` in CPI account structs) —
  signers pass through with their `is_signer` flags intact.
- In production, the registry authority should be a breeding-program PDA
  (CPI with signer seeds) instead of a co-signing keypair.

### Test results
`cargo test -p breeding` — **32 passed, 0 failed:**
- Validation: valid_pair_passes, rejects_same_pet_twice,
  rejects_species_mismatch, rejects_same_gender, rejects_non_adult,
  rejects_stranger_breeder, rejects_half_ownership,
  registry_authority_may_breed_any_pair, rejects_parent_on_cooldown,
  cooldown_boundary_passes, rejects_low_health_parent,
  health_threshold_boundary_passes, cooldown_math
- Inheritance: inherit_gene_picks_parent_a/b_on_flip,
  inherit_gene_mutates_to_random_pool_value,
  inherit_personality_averages_with_variance,
  inherit_personality_respects_variance_bounds,
  inherit_personality_clamps_at_edges, legendary_chance_table_is_correct,
  mythic_chance_table_is_correct,
  expressing_parent_boosts_trait_inheritance,
  base_rate_can_still_express_without_parent_expression,
  mythic_can_express_at_base_rate, rarity_ladder_from_expression,
  parent_max_rarity_picks_higher, full_inheritance_is_deterministic
  (39-roll FixedRng end-to-end), generation_increments_from_higher_parent,
  generation_saturates_at_255, chain_rng_is_deterministic_for_same_inputs,
  chain_rng_differs_across_pairs

`cargo test --workspace` — **85 passed, 0 failed** (53 pet-registry + 32 breeding).
`cargo check` — clean (only the standard `anchor-debug` cfg warnings;
harmless, same as previous steps).

### Notes
- Devnet breeding program ID: `4XjtZUPzHMJTLCkAT3ZWqUB4g3viopH1R1XhFU9hBeXQ`
- No mainnet deployment. Ever, until audit.
- Aggregate expression math (for tuning later): 16 legendary traits × 5%
  base ≈ 56% of offspring express ≥1 legendary; 8 mythic × 1% base ≈ 7.7%
  express ≥1 mythic. Per-trait rates are as specified; global tuning is a
  game-design decision for Brad.

### Next steps
1. `mint_creator_pet` instruction for IDs 1–8 (uses `validate_creator_traits`
   + `creator_legendary_mask` / `creator_mythic_mask`).
2. Raffle program (Genesis distribution).
3. $PAW token program.

---

## Step 6 — $PAW Token Program (October 13, 2026)

**Program:** `paw-token` — the SolanaPets utility token.
- Devnet program ID: `HEHEXUJfmQ3S5i2vxH6SqmZVBQo78vmygD3dVx1BZWSk`
- Real SPL token (9 decimals), mint authority = program PDA.

**Instructions:**
| Instruction | Who calls | What it does |
|---|---|---|
| `initialize_paw` | deployer | One-time setup: creates $PAW mint + config PDA, sets caps |
| `earn_paw` | earner_authority (game server) | Mints $PAW reward for verified gameplay |
| `spend_paw` | player | Burns $PAW for in-game purchases (deflationary sink) |
| `set_earner_authority` | config authority | Rotates the game server key |

**Anti-gaming controls (all enforced on-chain):**
1. Only `earner_authority` may mint — game server verifies real gameplay first
2. Per-pet daily cap: 50 $PAW (tracked in `PetEarnings` PDA)
3. Min 1-hour cooldown between claims per pet
4. Global supply cap: 1B $PAW — minting halts forever at cap
5. Checked math everywhere

**Economics (see PAW_ECONOMICS.md):**
- Base: 1 $PAW per care action, scaled by bond level (1x–3x)
- Bond: every 1,000 XP = +1 level, +0.1x multiplier, capped at 3x (20k XP)
- Year-one realistic emission: ~55M $PAW (<6% of supply)
- Burn sinks: marketplace fees, breeding fees, accessories

**Branding:** Luna (Genesis Founder #1, the Moonlit Otter) is the face of $PAW.
Token logo, imagery, and metadata feature Luna. (Brad decided Oct 13, 2026.)

**Design notes:**
- `PetEarnings` PDA: one per pet (`["earnings", pet_id]`), day rollover handled
  in-account. Pet account layout untouched (still 98 bytes).
- `init_if_needed` used for earnings PDA; safe because seeds are deterministic
  per pet_id and only the trusted earner_authority pays/initializes.
- Pure reward-math fns (`bond_multiplier_scaled`, `reward_for_care_action`,
  `day_bucket`) shared between game server and tests.

**Test results:**
`cargo test -p paw-token` — **15 passed, 0 failed:**
- Reward math: bond multiplier 1x→3x scaling and cap, per-action rewards,
  daily-cap sanity, exact base-unit math
- Day buckets: midnight split, pre-epoch (negative timestamp) handling
- Economic invariants: 1B supply cap, 50/day cap, 1hr cooldown
- Simulations: Genesis year <6% of cap, max-theoretical documents cap as circuit breaker
- Account sizes: PawConfig 137 bytes, PetEarnings 37 bytes

`cargo test --workspace` — **100 passed, 0 failed** (53 pet-registry + 32 breeding + 15 paw-token).
`cargo check` — clean (only standard anchor-debug cfg warnings).

**Files:**
- [programs/paw-token/Cargo.toml](sandbox://workspace/solanapets/programs/paw-token/Cargo.toml)
- [programs/paw-token/src/lib.rs](sandbox://workspace/solanapets/programs/paw-token/src/lib.rs) (~630 lines)
- [PAW_ECONOMICS.md](sandbox://workspace/solanapets/PAW_ECONOMICS.md) — plain-English token model

### Next steps
1. `mint_creator_pet` instruction for IDs 1–8.
2. Raffle program (Genesis distribution).
3. Marketplace program ($PAW burn integration).

---

## Step 7 — Marketplace Program (October 13, 2026)

**Status:** ✅ Complete — compiles clean, all tests pass.

**Program:** `marketplace` — the trading heart of the NFP ecosystem.
- Devnet program ID: `246z2Vaqyd4j8QnMW9cCQtSbYC9LeGUC61CRKEWbveWS`
- Real keypair at `target/deploy/marketplace-keypair.json` (devnet only)

**Instructions:**
| Instruction | Who calls | What it does |
|---|---|---|
| `initialize_marketplace` | deployer | One-time setup: config PDA + treasury PDA, fee/adoption/restoration params, dev wallet, $PAW mint |
| `update_config` | marketplace authority | Tune fee_bps, adoption fee, restoration period, dev wallet (all optional) |
| `list_pet` | pet owner | Escrows pet to vault PDA via CPI `transfer_pet`; creates Listing PDA. Rejects pets on cooldown |
| `buy_pet` | buyer | Pays price (SOL or $PAW); seller gets price − 3% fee, treasury gets fee; pet → buyer; listing closed |
| `cancel_listing` | seller only | Pet returns from vault; listing closed, rent to seller |
| `rescue_pet` | REGISTRY authority only | Force-transfers neglected pet to rescue vault; creates RescueRecord. Rejects listed/already-rescued pets |
| `adopt_pet` | adopter | Pays adoption fee to dev wallet; pet → adopter with restoration cooldown stamped; record closed |

**Fee structure (Brad-approved):**
- Marketplace fee: 3% (300 bps) on every sale → treasury. Max 10% sanity cap.
- Adoption fee: 0.1 SOL (100M lamports) ≈ $10 at $100/SOL. Configurable by authority as SOL moves. **Production MUST wire a Pyth SOL/USD oracle before mainnet** (documented in-code).
- Listing/cancel: free (rent returned on close).

**Escrow design:**
- Listed pets: owner = `["vault"]` PDA. Only the marketplace program can sign for it (invoke_signed).
- Rescued pets: owner = `["rescue-vault"]` PDA.
- All pet movement goes through pet-registry's `transfer_pet` via CPI — the marketplace never touches Pet account bytes directly.

**pet-registry changes (same step — additive only, no layout change):**
- New `transfer_pet` instruction: signer must be pet owner OR registry authority (authority override = rescue flow). Takes `new_owner` + optional `breed_cooldown_until`.
- New `TransferPet` accounts struct; pure helper `is_authorized_transfer` (mirrors the `is_authorized_caregiver` pattern).
- New errors: `UnauthorizedTransfer`, `TransferToSelf`, `InvalidCooldown`.
- 3 new tests: owner/authority authorized, stranger rejected. Pet account still 109 bytes.

**Restoration cooldown enforcement:**
- `adopt_pet` stamps `breed_cooldown_until = now + 7 days` via the transfer CPI.
- The breeding program already rejects pets on cooldown; `list_pet` checks it too. One field, both gates.

**$PAW sales:**
- Buyer/seller/treasury $PAW token accounts passed as `Option<...>` (None for SOL sales).
- Mints validated against `marketplace.paw_mint`; buyer must own their token account.
- Treasury's $PAW ATA (of the treasury PDA) must be created once at setup — documented in-code.

**Test results:**
`cargo test -p marketplace` — **12 passed, 0 failed**: fee math (3% split, dust rounding, u64::MAX no-overflow, sums-to-price), fee validation (1–1000 bps), adoption fee/restoration sanity, account sizes (116/46/13 bytes), currency enum discriminants.

`cargo test --workspace` — **115 passed, 0 failed** (56 pet-registry + 32 breeding + 15 paw-token + 12 marketplace).
`cargo check` — clean (only standard anchor-debug cfg warnings).

**Files:**
- [programs/marketplace/Cargo.toml](sandbox://workspace/solanapets/programs/marketplace/Cargo.toml)
- [programs/marketplace/src/lib.rs](sandbox://workspace/solanapets/programs/marketplace/src/lib.rs) (~700 lines)

### Next steps
1. `mint_creator_pet` instruction for IDs 1–8.
2. Raffle program (Genesis distribution).

---

## Step 7b: Accessories Store (Oct 13)

Brad asked for the accessories store — in-game digital items purchased with $PAW, built into the marketplace program.

**Design decision — burn, don't pay out:** Every accessory purchase BURNS the $PAW via CPI to the paw-token program's `spend_paw` (the buyer signs, so the inner instruction sees them as player). This is the deflationary sink Brad approved in PAW_ECONOMICS.md: "marketplace, breeding, accessories all destroy $PAW permanently." The creator gets storefront attribution and delisting rights, not a revenue cut — the value destruction is what keeps $PAW scarce as the game grows.

**4 instructions:**
| Instruction | Who | What |
|---|---|---|
| `list_accessory` | any creator (signer) | Lists an accessory: creator-chosen u64 ID, name (≤64 bytes), type (Habitat/Pet), $PAW price, metadata URI (≤200 bytes, Arweave/IPFS). Open creator marketplace. |
| `delist_accessory` | original creator only | Deactivates the listing. Existing owners keep their items. |
| `buy_accessory` | buyer (signer) | Pays price in $PAW → BURNED via `spend_paw` CPI. Creates `AccessoryOwnership` PDA. |
| `use_accessory` | owner (signer) | Binds the accessory to a pet (verifies pet PDA + ownership on-chain) or to the owner's habitat. Re-binding overwrites. |

**New accounts:**
- `AccessoryListing` (323 bytes): creator, accessory_id, name, type, price_paw, metadata_uri, active, bump. PDA: `["accessory-listing", id]`.
- `AccessoryOwnership` (47 bytes): owner, accessory_id, bound_pet_id (Option), bound_to_habitat, bump. PDA: `["accessory-owned", owner, id]`.

**New errors:** `AccessoryNameTooLong`, `MetadataUriTooLong`, `AccessoryNotActive`, `NotAccessoryCreator`, `NotAccessoryOwner`, `AccessoryTargetMismatch`.

**Game client note:** All visuals are client-side. On-chain records what the item is (type + metadata URI), who owns it, and what it's bound to. The client reads the binding to render.

**Physical merchandise is OUT OF SCOPE for this program.** Plushies, apparel, and other real-world goods are a future off-chain phase with a fulfillment partner — noted here so nobody confuses the two.

**Test results:**
`cargo test -p marketplace` — **18 passed, 0 failed** (12 existing + 6 new: listing validation accept/reject paths, name/URI length bounds, enum discriminant stability).
`cargo test --workspace` — **121 passed, 0 failed** (56 pet-registry + 32 breeding + 15 paw-token + 18 marketplace).
`cargo check` — clean.

**Files:**
- [programs/marketplace/src/lib.rs](sandbox://workspace/solanapets/programs/marketplace/src/lib.rs) (~1,450 lines)
- [programs/marketplace/Cargo.toml](sandbox://workspace/solanapets/programs/marketplace/Cargo.toml) (added paw-token cpi dependency)

---

## Step 8: Genesis Raffle Program (Oct 13)

The on-chain raffle for distributing the 15,000 Genesis NFPs — the fundraiser for the security audit and development (see the marketing plan).

**5 instructions:**
| Instruction | Who | What |
|---|---|---|
| `initialize_raffle` | deployer | One-time setup: sale window, dev wallet (Brad's), quotas. |
| `buy_tickets` | any buyer | Purchases Single (0.25 SOL/1 ticket), 5-Pack (1 SOL/5), or Whale (1.25 SOL/10, capped at 500 packs). SOL → treasury PDA. Per-buyer `TicketPurchase` PDA records the global ticket index range. Sale-window and whale-cap enforced. |
| `draw_winners` | raffle authority, once, after sale ends | Fisher-Yates shuffles the 15,000-slot prize pool (3,750/species, 50/50 M/F) and the ticket order. First 15,000 shuffled tickets win. Results stored publicly in `DrawResult`. |
| `claim_pet` | winner | O(1) verification: `winner_order[rank] == ticket_index`, ticket ownership check, double-claim bitmap. CPI-mints the Genesis pet with drawn species/gender + deterministically rolled traits (colors, personality, weighted rarity — Mythic pulls possible). |
| `distribute_funds` | raffle authority, once, after draw | 20% of treasury → Brad's dev wallet (founder compensation, non-negotiable). 80% stays in treasury. Emits amounts for transparency. |

**Draw algorithm:** Fisher-Yates on two arrays — the prize pool (15,000 packed species/gender bytes honoring quotas) and the ticket index order — both seeded by `RaffleRng` (splitmix64 over slot + timestamp + ticket count). Winner rank r gets prize slot r. Claims verify in O(1): the client looks up the rank from the public winner list off-chain.

**RANDOMNESS WARNING (in-code):** `RaffleRng` is predictable — a validator can reproduce it. Devnet scaffolding only. Production MUST use Switchboard VRF before mainnet/audit. The marketing plan promises "provably fair."

**Genesis trait rolls:** colors from canonical pools, personality 20–90, rarity weighted (Common 40 / Uncommon 30 / Rare 20 / Epic 7 / Legendary 2.5 / Mythic 0.5). Legendary rolls express 1–2 random legendary traits; Mythic rolls express 1 mythic mutation — seeding the gene pool from day one.

**New accounts:** `RaffleConfig` (92 bytes), `TicketPurchase` (53 bytes), `DrawResult` (~76.9KB: winner order + packed prizes + claim bitmap).

**Test results:**
`cargo test -p raffle` — **15 passed, 0 failed**: prize pool quotas (3,750/species, 1,875/gender, 50/50 split), shuffle preserves quotas + uniqueness, 20/80 fund split math (incl. dust rounding), tier pricing matches marketing plan, trait roll ranges, rarity distribution bands, trait expression on Legendary/Mythic, determinism, account sizes, prize packing round-trip.
`cargo test --workspace` — **136 passed, 0 failed** (56 pet-registry + 32 breeding + 15 paw-token + 18 marketplace + 15 raffle).
`cargo check` — clean.

**Files:**
- [programs/raffle/src/lib.rs](sandbox://workspace/solanapets/programs/raffle/src/lib.rs) (~1,200 lines)
- [programs/raffle/Cargo.toml](sandbox://workspace/solanapets/programs/raffle/Cargo.toml)
- Anchor.toml (registered raffle devnet program ID)

**Note:** The 8 creator pets (IDs 1–8) are excluded — minted separately via the future `mint_creator_pet`. Raffle mints start at registry `next_id` (9+).
