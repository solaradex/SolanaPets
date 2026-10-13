//! # SolanaPets Marketplace Program
//!
//! The trading heart of the SolanaPets NFP ecosystem.
//!
//! ## What it does
//! - **Listings**: owners list pets at fixed prices (SOL or $PAW). The pet is
//!   escrowed — ownership transfers to a program vault PDA, so the seller
//!   can't move it mid-sale.
//! - **Sales**: buyers pay the listing price. The seller receives price minus
//!   the marketplace fee (3%); the fee goes to the treasury. The pet transfers
//!   to the buyer.
//! - **Rescue**: the registry authority can reclaim a neglected pet into the
//!   rescue vault. This is the enforcement arm of the neglect system —
//!   reclaiming someone's pet is a big deal, so it requires the REGISTRY
//!   authority's signature, not just the marketplace authority's.
//! - **Adoption**: anyone can adopt a rescued pet by paying the adoption fee
//!   ($10 SOL equivalent, configurable). The fee goes straight to the dev
//!   wallet. The pet enters a restoration cooldown — it can't breed or be
//!   re-listed until the cooldown expires.
//!
//! ## Trust model
//! - The marketplace NEVER holds user funds beyond the instant of a sale.
//!   SOL/$PAW move directly buyer → seller and buyer → treasury.
//! - Escrowed pets are owned by program PDAs (`vault`, `rescue_vault`).
//!   Only this program can sign for those PDAs (via `invoke_signed`).
//! - Pet movement always goes through pet-registry's `transfer_pet` via CPI.
//!   The vault PDA is the recorded `owner` while a pet is escrowed.
//! - Rescue requires the registry authority (the multisig in production).
//!   The marketplace authority alone cannot seize pets.
//!
//! ## Fee structure (Brad-approved, changeable only by authority)
//! - Marketplace fee: 3% (300 bps) on every sale. Goes to the treasury.
//! - Adoption fee: $10 SOL equivalent (configurable lamport amount — see note).
//! - Listing/cancel: free (just rent, returned on close).
//!
//! ## $PAW sales — setup note
//! For $PAW-denominated sales, the treasury's $PAW token account (the ATA of
//! the `treasury` PDA for the $PAW mint) must exist before the first $PAW
//! sale. Create it once at deploy time with the associated-token program.
//! The buyer/seller pass their own $PAW token accounts per sale.
//!
//! ## The $10 peg — READ THIS
//! The adoption fee is stored as a fixed lamport amount
//! (`DEFAULT_ADOPTION_FEE_LAMPORTS` = 0.1 SOL ≈ $10 at $100/SOL). A true
//! USD peg needs a price oracle (e.g. Pyth). For devnet scaffolding the
//! authority updates the lamport amount via `update_config` as SOL moves.
//! Production MUST wire a Pyth SOL/USD feed before mainnet.
//!
//! ## DEVNET ONLY. No mainnet deployment until audit.
//!
//! Program ID (devnet): 246z2Vaqyd4j8QnMW9cCQtSbYC9LeGUC61CRKEWbveWS

use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use paw_token::{self, PawConfig, PAW_CONFIG_SEED};
use pet_registry::{self, Pet, Registry, PET_SEED, REGISTRY_SEED};

// Devnet program ID — generated for this deployment, never mainnet.
declare_id!("246z2Vaqyd4j8QnMW9cCQtSbYC9LeGUC61CRKEWbveWS");

// ============================================================================
// Constants — the numbers Brad approved. Change only with his sign-off.
// ============================================================================

/// Basis points denominator (100% = 10,000 bps).
pub const BPS_DENOMINATOR: u64 = 10_000;
/// Marketplace fee: 3% of every sale goes to the treasury.
pub const DEFAULT_FEE_BPS: u16 = 300;
/// Maximum fee the authority may set (10% — sanity cap).
pub const MAX_FEE_BPS: u16 = 1_000;
/// Default adoption fee: 0.1 SOL ≈ $10 at $100/SOL.
/// UPDATED BY AUTHORITY as SOL price moves (see $10 peg note above).
pub const DEFAULT_ADOPTION_FEE_LAMPORTS: u64 = 100_000_000; // 0.1 SOL
/// Default restoration cooldown after adoption: 7 days. The pet can't
/// breed or be re-listed until this expires.
pub const DEFAULT_RESTORATION_SECONDS: i64 = 7 * 86_400;
/// Seconds in a day.
pub const SECONDS_PER_DAY: i64 = 86_400;

/// PDA seeds.
pub const MARKETPLACE_SEED: &[u8] = b"marketplace";
pub const TREASURY_SEED: &[u8] = b"treasury";
pub const VAULT_SEED: &[u8] = b"vault";
pub const RESCUE_VAULT_SEED: &[u8] = b"rescue-vault";
pub const LISTING_SEED: &[u8] = b"listing";
pub const RESCUE_SEED: &[u8] = b"rescue";
pub const ACCESSORY_LISTING_SEED: &[u8] = b"accessory-listing";
pub const ACCESSORY_OWNED_SEED: &[u8] = b"accessory-owned";

/// Max bytes for an accessory display name.
pub const MAX_ACCESSORY_NAME_LEN: usize = 64;
/// Max bytes for an accessory metadata URI (Arweave/IPFS link).
pub const MAX_METADATA_URI_LEN: usize = 200;

// ============================================================================
// Program entrypoints
// ============================================================================

#[program]
pub mod marketplace {
    use super::*;

    /// One-time setup: creates the Marketplace config PDA and the treasury
    /// PDA. Sets the fee, adoption fee, restoration period, dev wallet, and
    /// the $PAW mint accepted for $PAW-denominated sales.
    pub fn initialize_marketplace(
        ctx: Context<InitializeMarketplace>,
        dev_wallet: Pubkey,
        paw_mint: Pubkey,
        fee_bps: u16,
        adoption_fee_lamports: u64,
        restoration_seconds: i64,
    ) -> Result<()> {
        validate_fee_bps(fee_bps)?;
        require!(adoption_fee_lamports > 0, MarketplaceError::InvalidAdoptionFee);
        require!(
            restoration_seconds >= 0,
            MarketplaceError::InvalidRestorationPeriod
        );

        let marketplace = &mut ctx.accounts.marketplace;
        marketplace.authority = ctx.accounts.authority.key();
        marketplace.dev_wallet = dev_wallet;
        marketplace.paw_mint = paw_mint;
        marketplace.fee_bps = fee_bps;
        marketplace.adoption_fee_lamports = adoption_fee_lamports;
        marketplace.restoration_seconds = restoration_seconds;
        marketplace.bump = ctx.bumps.marketplace;
        marketplace.treasury_bump = ctx.bumps.treasury;

        msg!(
            "Marketplace initialized: fee={}bps, adoption_fee={} lamports, restoration={}s, dev_wallet={}",
            fee_bps,
            adoption_fee_lamports,
            restoration_seconds,
            dev_wallet
        );
        Ok(())
    }

    /// Update marketplace parameters. Authority only.
    /// Each field is optional — pass `None` to leave it unchanged.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        fee_bps: Option<u16>,
        adoption_fee_lamports: Option<u64>,
        restoration_seconds: Option<i64>,
        dev_wallet: Option<Pubkey>,
    ) -> Result<()> {
        let marketplace = &mut ctx.accounts.marketplace;
        require_keys_eq!(
            ctx.accounts.authority.key(),
            marketplace.authority,
            MarketplaceError::UnauthorizedAuthority
        );

        if let Some(fee) = fee_bps {
            validate_fee_bps(fee)?;
            marketplace.fee_bps = fee;
        }
        if let Some(fee) = adoption_fee_lamports {
            require!(fee > 0, MarketplaceError::InvalidAdoptionFee);
            marketplace.adoption_fee_lamports = fee;
        }
        if let Some(secs) = restoration_seconds {
            require!(secs >= 0, MarketplaceError::InvalidRestorationPeriod);
            marketplace.restoration_seconds = secs;
        }
        if let Some(wallet) = dev_wallet {
            marketplace.dev_wallet = wallet;
        }

        msg!("Marketplace config updated");
        Ok(())
    }

    /// List a pet for sale at a fixed price.
    ///
    /// The pet is escrowed: ownership transfers to the program vault PDA via
    /// CPI into pet-registry's `transfer_pet`. The seller can't move the pet
    /// while it's listed. A `Listing` PDA records the sale terms.
    ///
    /// Guards: seller must own the pet; the pet must not be in a breeding /
    /// restoration cooldown (a rescued pet can't be flipped immediately).
    pub fn list_pet(
        ctx: Context<ListPet>,
        price: u64,
        currency: SaleCurrency,
    ) -> Result<()> {
        require!(price > 0, MarketplaceError::ZeroPrice);

        let pet = &ctx.accounts.pet;
        require_keys_eq!(
            pet.owner,
            ctx.accounts.seller.key(),
            MarketplaceError::NotPetOwner
        );

        // A pet in restoration (or breeding cooldown) can't be listed.
        let now = Clock::get()?.unix_timestamp;
        require!(
            pet.breed_cooldown_until <= now,
            MarketplaceError::PetOnCooldown
        );

        // Escrow: seller -> vault PDA.
        let cpi_accounts = pet_registry::cpi::accounts::TransferPet {
            owner: ctx.accounts.seller.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
        );
        pet_registry::cpi::transfer_pet(cpi_ctx, ctx.accounts.vault.key(), None)?;

        // Record the listing.
        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.pet_id = pet.id;
        listing.price = price;
        listing.currency = currency;
        listing.bump = ctx.bumps.listing;

        msg!(
            "NFP #{} listed for {} {}",
            pet.id,
            price,
            match currency {
                SaleCurrency::Sol => "lamports",
                SaleCurrency::Paw => "$PAW base units",
            }
        );
        Ok(())
    }

    /// Buy a listed pet.
    ///
    /// The buyer pays the full listing price. The seller receives
    /// price − fee; the treasury receives the fee (3%). The pet transfers
    /// from the vault PDA to the buyer, and the listing is closed (rent
    /// returned to the seller).
    pub fn buy_pet(ctx: Context<BuyPet>) -> Result<()> {
        let listing = &ctx.accounts.listing;
        let pet = &ctx.accounts.pet;

        // The pet must still be escrowed — if it moved (e.g. rescued),
        // the listing is stale and the sale is rejected.
        require_keys_eq!(
            pet.owner,
            ctx.accounts.vault.key(),
            MarketplaceError::PetNotEscrowed
        );
        require!(
            listing.pet_id == pet.id,
            MarketplaceError::ListingPetMismatch
        );

        let (fee, seller_amount) =
            split_fee(listing.price, ctx.accounts.marketplace.fee_bps)?;

        match listing.currency {
            SaleCurrency::Sol => {
                // Buyer -> seller (97%).
                let cpi_ctx = CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    system_program::Transfer {
                        from: ctx.accounts.buyer.to_account_info(),
                        to: ctx.accounts.seller.to_account_info(),
                    },
                );
                system_program::transfer(cpi_ctx, seller_amount)?;

                // Buyer -> treasury (3%), if the fee is non-zero.
                if fee > 0 {
                    let cpi_ctx = CpiContext::new(
                        ctx.accounts.system_program.to_account_info(),
                        system_program::Transfer {
                            from: ctx.accounts.buyer.to_account_info(),
                            to: ctx.accounts.treasury.to_account_info(),
                        },
                    );
                    system_program::transfer(cpi_ctx, fee)?;
                }
            }
            SaleCurrency::Paw => {
                // All three $PAW accounts must be provided for $PAW sales.
                // Their mints must match the marketplace's $PAW mint, and the
                // treasury account must be the treasury PDA's ATA (created
                // once at setup — see module docs).
                let buyer_paw = ctx.accounts.buyer_paw_account.as_ref().ok_or(
                    MarketplaceError::MissingPawAccounts,
                )?;
                let seller_paw = ctx.accounts.seller_paw_account.as_ref().ok_or(
                    MarketplaceError::MissingPawAccounts,
                )?;
                let treasury_paw = ctx.accounts.treasury_paw_account.as_ref().ok_or(
                    MarketplaceError::MissingPawAccounts,
                )?;
                let paw_mint = ctx.accounts.marketplace.paw_mint;
                require_keys_eq!(buyer_paw.mint, paw_mint, MarketplaceError::WrongPawMint);
                require_keys_eq!(seller_paw.mint, paw_mint, MarketplaceError::WrongPawMint);
                require_keys_eq!(
                    treasury_paw.mint,
                    paw_mint,
                    MarketplaceError::WrongPawMint
                );
                // The buyer must own their $PAW account.
                require_keys_eq!(
                    buyer_paw.owner,
                    ctx.accounts.buyer.key(),
                    MarketplaceError::WrongPawAccountOwner
                );

                // Buyer -> seller (97%).
                let cpi_ctx = CpiContext::new(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: buyer_paw.to_account_info(),
                        to: seller_paw.to_account_info(),
                        authority: ctx.accounts.buyer.to_account_info(),
                    },
                );
                token::transfer(cpi_ctx, seller_amount)?;

                // Buyer -> treasury (3%), if the fee is non-zero.
                if fee > 0 {
                    let cpi_ctx = CpiContext::new(
                        ctx.accounts.token_program.to_account_info(),
                        token::Transfer {
                            from: buyer_paw.to_account_info(),
                            to: treasury_paw.to_account_info(),
                            authority: ctx.accounts.buyer.to_account_info(),
                        },
                    );
                    token::transfer(cpi_ctx, fee)?;
                }
            }
        }

        // Vault PDA -> buyer (program signs for the vault).
        let vault_bump = ctx.bumps.vault;
        let vault_seeds: &[&[u8]] = &[VAULT_SEED, &[vault_bump]];
        let signer_seeds = &[vault_seeds];
        let cpi_accounts = pet_registry::cpi::accounts::TransferPet {
            owner: ctx.accounts.vault.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        pet_registry::cpi::transfer_pet(cpi_ctx, ctx.accounts.buyer.key(), None)?;

        msg!(
            "NFP #{} sold for {} (fee {} to treasury)",
            pet.id,
            listing.price,
            fee
        );
        Ok(())
        // `listing` is closed via `close = seller` — rent returns to seller.
    }

    /// Cancel a listing. Only the seller may cancel.
    ///
    /// The pet returns from the vault PDA to the seller, and the listing
    /// is closed (rent returned to the seller).
    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        let listing = &ctx.accounts.listing;
        require_keys_eq!(
            listing.seller,
            ctx.accounts.seller.key(),
            MarketplaceError::NotListingSeller
        );

        // Sanity: the pet should still be escrowed.
        require_keys_eq!(
            ctx.accounts.pet.owner,
            ctx.accounts.vault.key(),
            MarketplaceError::PetNotEscrowed
        );

        // Vault PDA -> seller.
        let vault_bump = ctx.bumps.vault;
        let vault_seeds: &[&[u8]] = &[VAULT_SEED, &[vault_bump]];
        let signer_seeds = &[vault_seeds];
        let cpi_accounts = pet_registry::cpi::accounts::TransferPet {
            owner: ctx.accounts.vault.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        pet_registry::cpi::transfer_pet(cpi_ctx, ctx.accounts.seller.key(), None)?;

        msg!("Listing for NFP #{} cancelled", listing.pet_id);
        Ok(())
        // `listing` closed via `close = seller`.
    }

    /// Rescue a neglected pet. REGISTRY AUTHORITY ONLY.
    ///
    /// This is the enforcement arm of the neglect system. The registry
    /// authority (multisig in production) reclaims the pet into the rescue
    /// vault. A `RescueRecord` PDA tracks it until adoption.
    ///
    /// Guards: the pet must not be actively listed (a listed pet has an
    /// engaged seller — cancel the listing first) and must not already
    /// be rescued.
    pub fn rescue_pet(ctx: Context<RescuePet>) -> Result<()> {
        // Only the REGISTRY authority can seize a pet — not the
        // marketplace authority. Reclaiming someone's pet is a big deal.
        require_keys_eq!(
            ctx.accounts.authority.key(),
            ctx.accounts.registry.authority,
            MarketplaceError::UnauthorizedAuthority
        );

        let pet = &ctx.accounts.pet;
        require!(
            pet.owner != ctx.accounts.vault.key(),
            MarketplaceError::PetIsListed
        );
        require!(
            pet.owner != ctx.accounts.rescue_vault.key(),
            MarketplaceError::AlreadyRescued
        );

        // Authority -> rescue vault (pet-registry's authority override).
        let cpi_accounts = pet_registry::cpi::accounts::TransferPet {
            owner: ctx.accounts.authority.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
        );
        pet_registry::cpi::transfer_pet(cpi_ctx, ctx.accounts.rescue_vault.key(), None)?;

        let record = &mut ctx.accounts.rescue_record;
        record.pet_id = pet.id;
        record.rescued_at = Clock::get()?.unix_timestamp;
        record.bump = ctx.bumps.rescue_record;

        msg!(
            "NFP #{} rescued by the marketplace — awaiting adoption",
            pet.id
        );
        Ok(())
    }

    /// Adopt a rescued pet.
    ///
    /// The adopter pays the adoption fee ($10 SOL equivalent) directly to
    /// the dev wallet. The pet transfers from the rescue vault to the
    /// adopter with a restoration cooldown stamped — it can't breed or be
    /// re-listed until the cooldown expires. The rescue record is closed
    /// (rent returned to the adopter).
    pub fn adopt_pet(ctx: Context<AdoptPet>) -> Result<()> {
        let marketplace = &ctx.accounts.marketplace;
        let pet = &ctx.accounts.pet;

        // The pet must actually be in rescue.
        require_keys_eq!(
            pet.owner,
            ctx.accounts.rescue_vault.key(),
            MarketplaceError::PetNotInRescue
        );
        require!(
            ctx.accounts.rescue_record.pet_id == pet.id,
            MarketplaceError::RescueRecordMismatch
        );

        // Adopter -> dev wallet: the adoption fee.
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.adopter.to_account_info(),
                to: ctx.accounts.dev_wallet.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, marketplace.adoption_fee_lamports)?;

        // Rescue vault -> adopter, with the restoration cooldown stamped.
        // The cooldown blocks breeding (breeding program checks
        // breed_cooldown_until) and re-listing (list_pet checks it too).
        let now = Clock::get()?.unix_timestamp;
        let cooldown_until = now
            .checked_add(marketplace.restoration_seconds)
            .ok_or(MarketplaceError::MathOverflow)?;

        let vault_bump = ctx.bumps.rescue_vault;
        let vault_seeds: &[&[u8]] = &[RESCUE_VAULT_SEED, &[vault_bump]];
        let signer_seeds = &[vault_seeds];
        let cpi_accounts = pet_registry::cpi::accounts::TransferPet {
            owner: ctx.accounts.rescue_vault.to_account_info(),
            pet: ctx.accounts.pet.to_account_info(),
            registry: ctx.accounts.registry.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.pet_registry_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        pet_registry::cpi::transfer_pet(
            cpi_ctx,
            ctx.accounts.adopter.key(),
            Some(cooldown_until),
        )?;

        msg!(
            "NFP #{} adopted — restoration cooldown until {}",
            pet.id,
            cooldown_until
        );
        Ok(())
        // `rescue_record` closed via `close = adopter`.
    }

    // ========================================================================
    // Accessories store — in-game digital items (habitat + pet accessories)
    // ========================================================================
    //
    // The accessories store sells DIGITAL in-game items for $PAW: habitat
    // accessories (waterfall upgrades, decorations) and pet accessories
    // (collars, bandanas). Every purchase BURNS the $PAW via the paw-token
    // program's `spend_paw` — this is the deflationary sink Brad approved in
    // PAW_ECONOMICS.md ("marketplace, breeding, accessories all destroy
    // $PAW permanently"). Nothing goes to a treasury; the value is
    // destroyed, which is what keeps $PAW scarce as the game grows.
    //
    // Physical merchandise (plushies, apparel) is a FUTURE OFF-CHAIN phase
    // with a fulfillment partner — it is NOT part of this program.
    //
    // The game client handles all visuals. On-chain we record: what the
    // item is (type + metadata URI), who owns it, and what it's bound to
    // (a pet or a habitat).

    /// List a new accessory for sale. Any creator can list — this is an open
    /// creator marketplace. The `accessory_id` is chosen by the creator
    /// (must be unique; the PDA init fails on collision).
    pub fn list_accessory(
        ctx: Context<ListAccessory>,
        accessory_id: u64,
        name: String,
        accessory_type: AccessoryType,
        price_paw: u64,
        metadata_uri: String,
    ) -> Result<()> {
        validate_accessory_listing(&name, &metadata_uri, price_paw)?;

        let listing = &mut ctx.accounts.accessory_listing;
        listing.creator = ctx.accounts.creator.key();
        listing.accessory_id = accessory_id;
        listing.name = name;
        listing.accessory_type = accessory_type;
        listing.price_paw = price_paw;
        listing.metadata_uri = metadata_uri;
        listing.active = true;
        listing.bump = ctx.bumps.accessory_listing;

        msg!(
            "Accessory #{} ({:?}) listed by {} for {} $PAW",
            accessory_id,
            listing.accessory_type,
            listing.creator,
            price_paw,
        );
        Ok(())
    }

    /// Deactivate an accessory listing. Only the original creator may delist.
    /// Existing owners keep their items — delisting only stops new sales.
    pub fn delist_accessory(ctx: Context<DelistAccessory>) -> Result<()> {
        let listing = &mut ctx.accounts.accessory_listing;
        require_keys_eq!(
            listing.creator,
            ctx.accounts.creator.key(),
            MarketplaceError::NotAccessoryCreator
        );
        listing.active = false;
        msg!("Accessory #{} delisted", listing.accessory_id);
        Ok(())
    }

    /// Buy an accessory. The buyer pays `price_paw` in $PAW, which is BURNED
    /// via the paw-token program (deflationary sink — see module docs).
    /// An `AccessoryOwnership` PDA is created for the buyer.
    pub fn buy_accessory(ctx: Context<BuyAccessory>) -> Result<()> {
        let listing = &ctx.accounts.accessory_listing;
        require!(listing.active, MarketplaceError::AccessoryNotActive);

        // Burn the $PAW via the paw-token program. The buyer signs, so the
        // inner `spend_paw` sees them as the player/authority.
        let cpi_accounts = paw_token::cpi::accounts::SpendPaw {
            player: ctx.accounts.buyer.to_account_info(),
            paw_mint: ctx.accounts.paw_mint.to_account_info(),
            source: ctx.accounts.buyer_paw_account.to_account_info(),
            paw_config: ctx.accounts.paw_config.to_account_info(),
            token_program: ctx.accounts.token_program.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(
            ctx.accounts.paw_token_program.to_account_info(),
            cpi_accounts,
        );
        paw_token::cpi::spend_paw(cpi_ctx, listing.price_paw)?;

        // Record ownership.
        let ownership = &mut ctx.accounts.accessory_ownership;
        ownership.owner = ctx.accounts.buyer.key();
        ownership.accessory_id = listing.accessory_id;
        ownership.bound_pet_id = None;
        ownership.bound_to_habitat = false;
        ownership.bump = ctx.bumps.accessory_ownership;

        msg!(
            "Accessory #{} purchased by {} — {} $PAW burned",
            listing.accessory_id,
            ownership.owner,
            listing.price_paw,
        );
        Ok(())
    }

    /// Bind an owned accessory to a pet or a habitat. The game client reads
    /// this binding to render the accessory. Re-binding overwrites the
    /// previous binding (one active binding per owned accessory).
    pub fn use_accessory(
        ctx: Context<UseAccessory>,
        target: AccessoryTarget,
    ) -> Result<()> {
        let ownership = &mut ctx.accounts.accessory_ownership;
        require_keys_eq!(
            ownership.owner,
            ctx.accounts.owner.key(),
            MarketplaceError::NotAccessoryOwner
        );

        match target {
            AccessoryTarget::Pet(pet_id) => {
                // Deserialize the pet and verify: correct PDA, correct ID,
                // and owned by the accessory owner.
                let pet_ai = ctx.accounts.pet.to_account_info();
                let (expected_key, _) = Pubkey::find_program_address(
                    &[PET_SEED, pet_id.to_le_bytes().as_ref()],
                    &pet_registry::ID,
                );
                require_keys_eq!(
                    pet_ai.key(),
                    expected_key,
                    MarketplaceError::AccessoryTargetMismatch
                );
                let pet_data = pet_ai.try_borrow_data()?;
                let mut pet_slice: &[u8] = &pet_data;
                let pet = Pet::try_deserialize(&mut pet_slice)?;
                require!(
                    pet.id == pet_id,
                    MarketplaceError::AccessoryTargetMismatch
                );
                require_keys_eq!(
                    pet.owner,
                    ctx.accounts.owner.key(),
                    MarketplaceError::NotPetOwner
                );
                ownership.bound_pet_id = Some(pet_id);
                ownership.bound_to_habitat = false;
                msg!(
                    "Accessory #{} bound to NFP #{}",
                    ownership.accessory_id,
                    pet_id
                );
            }
            AccessoryTarget::Habitat => {
                ownership.bound_pet_id = None;
                ownership.bound_to_habitat = true;
                msg!(
                    "Accessory #{} bound to {}'s habitat",
                    ownership.accessory_id,
                    ownership.owner
                );
            }
        }
        Ok(())
    }
}

// ============================================================================
// Account validation
// ============================================================================

#[derive(Accounts)]
pub struct InitializeMarketplace<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Marketplace::SIZE,
        seeds = [MARKETPLACE_SEED],
        bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    /// Treasury PDA — receives the 3% marketplace fee (SOL).
    /// CHECK: PDA with seeds ["treasury"]; no data, just an address that
    /// can hold SOL. Only SOL is ever sent here directly.
    #[account(
        seeds = [TREASURY_SEED],
        bump,
    )]
    pub treasury: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKETPLACE_SEED],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,
}

#[derive(Accounts)]
pub struct ListPet<'info> {
    /// The seller — must own the pet.
    #[account(mut)]
    pub seller: Signer<'info>,

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

    #[account(
        init,
        payer = seller,
        space = 8 + Listing::SIZE,
        seeds = [LISTING_SEED, pet.id.to_le_bytes().as_ref()],
        bump,
    )]
    pub listing: Account<'info, Listing>,

    /// CHECK: escrow vault PDA — becomes the pet's owner while listed.
    /// Only this program can sign for it (via invoke_signed).
    #[account(
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyPet<'info> {
    /// The buyer — pays the listing price.
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: the seller receives price − fee. Any system account.
    #[account(mut)]
    pub seller: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [MARKETPLACE_SEED],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        mut,
        seeds = [LISTING_SEED, pet.id.to_le_bytes().as_ref()],
        bump = listing.bump,
        close = seller,
    )]
    pub listing: Account<'info, Listing>,

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

    /// CHECK: escrow vault PDA — current owner of the pet.
    #[account(
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: treasury PDA — receives the 3% fee (SOL).
    #[account(
        mut,
        seeds = [TREASURY_SEED],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: UncheckedAccount<'info>,

    // --- $PAW payment accounts (Some(..) only when currency == Paw) ---
    // For SOL sales, pass None for all three. For $PAW sales, the mints
    // are validated against marketplace.paw_mint in the handler, and the
    // treasury's $PAW ATA must already exist (created once at setup —
    // see the module docs).
    pub buyer_paw_account: Option<Account<'info, TokenAccount>>,
    pub seller_paw_account: Option<Account<'info, TokenAccount>>,
    pub treasury_paw_account: Option<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelListing<'info> {
    /// The seller — only they may cancel.
    #[account(mut)]
    pub seller: Signer<'info>,

    #[account(
        mut,
        seeds = [LISTING_SEED, pet.id.to_le_bytes().as_ref()],
        bump = listing.bump,
        close = seller,
    )]
    pub listing: Account<'info, Listing>,

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

    /// CHECK: escrow vault PDA — current owner of the pet.
    #[account(
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RescuePet<'info> {
    /// Must be the REGISTRY authority (checked in the handler) — the
    /// multisig in production. Reclaiming a pet is a big deal.
    #[account(mut)]
    pub authority: Signer<'info>,

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

    #[account(
        init,
        payer = authority,
        space = 8 + RescueRecord::SIZE,
        seeds = [RESCUE_SEED, pet.id.to_le_bytes().as_ref()],
        bump,
    )]
    pub rescue_record: Account<'info, RescueRecord>,

    /// CHECK: rescue vault PDA — becomes the pet's owner while in rescue.
    #[account(
        seeds = [RESCUE_VAULT_SEED],
        bump,
    )]
    pub rescue_vault: UncheckedAccount<'info>,

    /// CHECK: escrow vault PDA — used only to REJECT rescuing a listed pet.
    /// The pet's owner is compared against this address.
    #[account(
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault: UncheckedAccount<'info>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AdoptPet<'info> {
    /// The adopter — pays the adoption fee, receives the pet.
    #[account(mut)]
    pub adopter: Signer<'info>,

    #[account(
        mut,
        seeds = [MARKETPLACE_SEED],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

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

    #[account(
        mut,
        seeds = [RESCUE_SEED, pet.id.to_le_bytes().as_ref()],
        bump = rescue_record.bump,
        close = adopter,
    )]
    pub rescue_record: Account<'info, RescueRecord>,

    /// CHECK: rescue vault PDA — current owner of the pet.
    #[account(
        seeds = [RESCUE_VAULT_SEED],
        bump,
    )]
    pub rescue_vault: UncheckedAccount<'info>,

    /// CHECK: the dev wallet receives the adoption fee. Must match config.
    #[account(
        mut,
        address = marketplace.dev_wallet @ MarketplaceError::WrongDevWallet,
    )]
    pub dev_wallet: UncheckedAccount<'info>,

    /// CHECK: constrained to the real pet-registry program ID.
    #[account(address = pet_registry::ID)]
    pub pet_registry_program: Program<'info, pet_registry::program::PetRegistry>,

    pub system_program: Program<'info, System>,
}

// --- Accessories store ------------------------------------------------------

#[derive(Accounts)]
#[instruction(accessory_id: u64)]
pub struct ListAccessory<'info> {
    /// The creator listing the accessory.
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        payer = creator,
        space = 8 + AccessoryListing::SIZE,
        seeds = [ACCESSORY_LISTING_SEED, accessory_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub accessory_listing: Account<'info, AccessoryListing>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DelistAccessory<'info> {
    /// Must be the original creator (checked in the handler).
    pub creator: Signer<'info>,

    #[account(
        mut,
        seeds = [ACCESSORY_LISTING_SEED, accessory_listing.accessory_id.to_le_bytes().as_ref()],
        bump = accessory_listing.bump,
    )]
    pub accessory_listing: Account<'info, AccessoryListing>,
}

#[derive(Accounts)]
pub struct BuyAccessory<'info> {
    /// The buyer — pays $PAW (burned) and receives the ownership record.
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE_SEED],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, Marketplace>,

    #[account(
        seeds = [ACCESSORY_LISTING_SEED, accessory_listing.accessory_id.to_le_bytes().as_ref()],
        bump = accessory_listing.bump,
    )]
    pub accessory_listing: Account<'info, AccessoryListing>,

    #[account(
        init,
        payer = buyer,
        space = 8 + AccessoryOwnership::SIZE,
        seeds = [
            ACCESSORY_OWNED_SEED,
            buyer.key().as_ref(),
            accessory_listing.accessory_id.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub accessory_ownership: Account<'info, AccessoryOwnership>,

    /// The buyer's $PAW token account — $PAW is burned from here.
    #[account(
        mut,
        token::mint = paw_mint,
        token::authority = buyer,
    )]
    pub buyer_paw_account: Account<'info, TokenAccount>,

    /// The $PAW mint — must match the marketplace's configured mint.
    #[account(
        mut,
        address = marketplace.paw_mint @ MarketplaceError::WrongPawMint,
    )]
    pub paw_mint: Account<'info, Mint>,

    #[account(
        seeds = [PAW_CONFIG_SEED],
        bump = paw_config.bump,
    )]
    pub paw_config: Account<'info, PawConfig>,

    /// CHECK: constrained to the real paw-token program ID.
    #[account(address = paw_token::ID)]
    pub paw_token_program: Program<'info, paw_token::program::PawToken>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UseAccessory<'info> {
    /// The accessory owner — must own both the accessory and the pet
    /// (pet ownership checked in the handler).
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [
            ACCESSORY_OWNED_SEED,
            owner.key().as_ref(),
            accessory_ownership.accessory_id.to_le_bytes().as_ref(),
        ],
        bump = accessory_ownership.bump,
    )]
    pub accessory_ownership: Account<'info, AccessoryOwnership>,

    /// The pet to bind to (only read when target is Pet; for Habitat
    /// bindings the client passes any owned pet or the check is skipped —
    /// the handler only reads it for Pet targets).
    /// CHECK: validated in the handler when the target is a pet.
    pub pet: UncheckedAccount<'info>,
}

// ============================================================================
// Account data
// ============================================================================

/// Marketplace configuration. PDA seeds: ["marketplace"]
#[account]
pub struct Marketplace {
    /// Marketplace admin — can update config. NOT the rescue authority
    /// (rescue requires the registry authority's signature).
    pub authority: Pubkey, // 32
    /// Receives adoption fees.
    pub dev_wallet: Pubkey, // 32
    /// The $PAW SPL mint accepted for $PAW-denominated sales.
    pub paw_mint: Pubkey, // 32
    /// Marketplace fee in basis points (300 = 3%).
    pub fee_bps: u16, // 2
    /// Adoption fee in lamports ($10 SOL equivalent — see peg note).
    pub adoption_fee_lamports: u64, // 8
    /// Restoration cooldown after adoption, in seconds.
    pub restoration_seconds: i64, // 8
    /// PDA bump for ["treasury"] (cached so buy_pet can derive it).
    pub treasury_bump: u8, // 1
    /// PDA bump.
    pub bump: u8, // 1
}

impl Marketplace {
    pub const SIZE: usize = 32 + 32 + 32 + 2 + 8 + 8 + 1 + 1; // 116
}

/// A fixed-price sale listing. PDA seeds: ["listing", pet_id.to_le_bytes()]
#[account]
pub struct Listing {
    /// The seller — receives price − fee, and the rent on close/cancel.
    pub seller: Pubkey, // 32
    /// The listed pet's ID.
    pub pet_id: u32, // 4
    /// Asking price (lamports if Sol, $PAW base units if Paw).
    pub price: u64, // 8
    /// Which currency the price is denominated in.
    pub currency: SaleCurrency, // 1
    /// PDA bump.
    pub bump: u8, // 1
}

impl Listing {
    pub const SIZE: usize = 32 + 4 + 8 + 1 + 1; // 46
}

/// Tracks a rescued pet awaiting adoption. PDA seeds: ["rescue", pet_id]
#[account]
pub struct RescueRecord {
    /// The rescued pet's ID.
    pub pet_id: u32, // 4
    /// Unix timestamp of the rescue.
    pub rescued_at: i64, // 8
    /// PDA bump.
    pub bump: u8, // 1
}

impl RescueRecord {
    pub const SIZE: usize = 4 + 8 + 1; // 13
}

/// An accessory listed for sale in the accessories store.
/// PDA seeds: ["accessory-listing", accessory_id.to_le_bytes()]
#[account]
pub struct AccessoryListing {
    /// The creator who listed it — receives nothing on sale (the $PAW is
    /// burned, not paid out — see module docs). Listed here for delisting
    /// rights and storefront attribution.
    pub creator: Pubkey, // 32
    /// Creator-chosen unique ID.
    pub accessory_id: u64, // 8
    /// Display name (max 64 bytes).
    pub name: String, // 4 + 64
    /// Habitat or Pet accessory.
    pub accessory_type: AccessoryType, // 1
    /// Price in $PAW base units (burned on purchase).
    pub price_paw: u64, // 8
    /// Metadata URI (Arweave/IPFS link to art + attributes, max 200 bytes).
    pub metadata_uri: String, // 4 + 200
    /// False after the creator delists. Existing owners keep their items.
    pub active: bool, // 1
    /// PDA bump.
    pub bump: u8, // 1
}

impl AccessoryListing {
    pub const SIZE: usize = 32 + 8 + (4 + 64) + 1 + 8 + (4 + 200) + 1 + 1; // 323
}

/// Records that a wallet owns an accessory and what it's bound to.
/// PDA seeds: ["accessory-owned", owner, accessory_id.to_le_bytes()]
#[account]
pub struct AccessoryOwnership {
    /// The owner.
    pub owner: Pubkey, // 32
    /// Which accessory.
    pub accessory_id: u64, // 8
    /// Bound pet ID, if bound to a pet.
    pub bound_pet_id: Option<u32>, // 1 + 4
    /// True if bound to the owner's habitat.
    pub bound_to_habitat: bool, // 1
    /// PDA bump.
    pub bump: u8, // 1
}

impl AccessoryOwnership {
    pub const SIZE: usize = 32 + 8 + (1 + 4) + 1 + 1; // 47
}

/// What kind of in-game item an accessory is.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessoryType {
    /// Habitat items: waterfall upgrades, decorations, lighting, etc.
    Habitat,
    /// Pet items: collars, bandanas, hats, etc.
    Pet,
}

/// Where an owned accessory is applied.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum AccessoryTarget {
    /// Bind to a specific pet (must be owned by the accessory owner).
    Pet(u32),
    /// Bind to the owner's habitat.
    Habitat,
}

/// Sale currency for a listing.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SaleCurrency {
    Sol,
    Paw,
}

// ============================================================================
// Pure helpers — shared with tests and (later) the game client.
// ============================================================================

/// Validate the marketplace fee: 1 bps minimum (0 would be pointless),
/// MAX_FEE_BPS maximum (10% sanity cap).
pub fn validate_fee_bps(fee_bps: u16) -> Result<()> {
    require!(fee_bps > 0, MarketplaceError::InvalidFeeBps);
    require!(fee_bps <= MAX_FEE_BPS, MarketplaceError::InvalidFeeBps);
    Ok(())
}

/// Validate an accessory listing's metadata: name and URI length bounds,
/// non-zero $PAW price. Pure function — unit-tested.
pub fn validate_accessory_listing(
    name: &str,
    metadata_uri: &str,
    price_paw: u64,
) -> Result<()> {
    require!(price_paw > 0, MarketplaceError::ZeroPrice);
    require!(
        name.len() <= MAX_ACCESSORY_NAME_LEN,
        MarketplaceError::AccessoryNameTooLong
    );
    require!(
        metadata_uri.len() <= MAX_METADATA_URI_LEN,
        MarketplaceError::MetadataUriTooLong
    );
    Ok(())
}
/// Split a sale price into (treasury_fee, seller_amount).
/// Uses u128 internally so price * fee_bps can never overflow u64.
pub fn split_fee(price: u64, fee_bps: u16) -> Result<(u64, u64)> {    let fee = (price as u128)
        .checked_mul(fee_bps as u128)
        .and_then(|v| v.checked_div(BPS_DENOMINATOR as u128))
        .and_then(|v| u64::try_from(v).ok())
        .ok_or(MarketplaceError::MathOverflow)?;
    let seller_amount = price
        .checked_sub(fee)
        .ok_or(MarketplaceError::MathOverflow)?;
    Ok((fee, seller_amount))
}

// ============================================================================
// Errors
// ============================================================================

#[error_code]
pub enum MarketplaceError {
    #[msg("Listing price must be greater than zero")]
    ZeroPrice,

    #[msg("Signer does not own this pet")]
    NotPetOwner,

    #[msg("Pet is in a breeding/restoration cooldown and cannot be listed")]
    PetOnCooldown,

    #[msg("Pet is not escrowed in the marketplace vault")]
    PetNotEscrowed,

    #[msg("Listing does not match this pet")]
    ListingPetMismatch,

    #[msg("Only the listing seller may cancel it")]
    NotListingSeller,

    #[msg("Pet is actively listed — cancel the listing before rescue")]
    PetIsListed,

    #[msg("Pet is already in rescue")]
    AlreadyRescued,

    #[msg("Pet is not in the rescue vault")]
    PetNotInRescue,

    #[msg("Rescue record does not match this pet")]
    RescueRecordMismatch,

    #[msg("Signer is not the marketplace authority")]
    UnauthorizedAuthority,

    #[msg("Fee must be between 1 and 1000 basis points (0.01%–10%)")]
    InvalidFeeBps,

    #[msg("Adoption fee must be greater than zero")]
    InvalidAdoptionFee,

    #[msg("Restoration period cannot be negative")]
    InvalidRestorationPeriod,

    #[msg("$PAW mint does not match marketplace config")]
    WrongPawMint,

    #[msg("$PAW accounts required for $PAW-denominated sales")]
    MissingPawAccounts,

    #[msg("Buyer's $PAW account is not owned by the buyer")]
    WrongPawAccountOwner,

    #[msg("Dev wallet does not match marketplace config")]
    WrongDevWallet,

    #[msg("Arithmetic overflow in fee math")]
    MathOverflow,

    #[msg("Accessory name is too long (max 64 bytes)")]
    AccessoryNameTooLong,

    #[msg("Accessory metadata URI is too long (max 200 bytes)")]
    MetadataUriTooLong,

    #[msg("Accessory listing is not active")]
    AccessoryNotActive,

    #[msg("Only the accessory creator may delist it")]
    NotAccessoryCreator,

    #[msg("Only the accessory owner may use it")]
    NotAccessoryOwner,

    #[msg("Accessory target does not match the provided pet")]
    AccessoryTargetMismatch,
}

// ============================================================================
// Tests — pure-function unit tests (no validator needed).
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- Fee math ----------------------------------------------------------

    #[test]
    fn fee_split_3_percent_is_correct() {
        // 1 SOL sale at 300 bps: fee 0.03 SOL, seller 0.97 SOL.
        let (fee, seller) = split_fee(1_000_000_000, 300).unwrap();
        assert_eq!(fee, 30_000_000);
        assert_eq!(seller, 970_000_000);
    }

    #[test]
    fn fee_split_sums_to_price() {
        for price in [1u64, 100, 999, 1_000_000, u64::MAX / 2] {
            let (fee, seller) = split_fee(price, 300).unwrap();
            assert_eq!(fee + seller, price, "price {}", price);
        }
    }

    #[test]
    fn fee_split_rounds_down_on_dust() {
        // 1 lamport at 3%: fee rounds to 0, seller keeps it all.
        let (fee, seller) = split_fee(1, 300).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(seller, 1);
    }

    #[test]
    fn fee_split_max_price_no_overflow() {
        // u64::MAX at 10%: must not overflow (u128 math).
        let (fee, seller) = split_fee(u64::MAX, 1_000).unwrap();
        assert_eq!(fee + seller, u64::MAX);
        assert!(fee > 0);
    }

    #[test]
    fn fee_split_zero_bps_gives_all_to_seller() {
        let (fee, seller) = split_fee(1_000_000, 0).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(seller, 1_000_000);
    }

    // --- Fee validation ----------------------------------------------------

    #[test]
    fn validate_fee_bps_accepts_valid_range() {
        assert!(validate_fee_bps(1).is_ok());
        assert!(validate_fee_bps(300).is_ok());
        assert!(validate_fee_bps(1_000).is_ok());
    }

    #[test]
    fn validate_fee_bps_rejects_zero_and_over_max() {
        assert!(validate_fee_bps(0).is_err());
        assert!(validate_fee_bps(1_001).is_err());
        assert!(validate_fee_bps(u16::MAX).is_err());
    }

    // --- Adoption fee sanity -------------------------------------------------

    #[test]
    fn default_adoption_fee_is_sane() {
        // 0.1 SOL = 100M lamports ≈ $10 at $100/SOL.
        assert_eq!(DEFAULT_ADOPTION_FEE_LAMPORTS, 100_000_000);
        assert!(DEFAULT_ADOPTION_FEE_LAMPORTS > 0);
    }

    #[test]
    fn default_restoration_is_seven_days() {
        assert_eq!(DEFAULT_RESTORATION_SECONDS, 7 * SECONDS_PER_DAY);
    }

    // --- Account sizes -------------------------------------------------------

    #[test]
    fn account_sizes_are_sane() {
        assert_eq!(Marketplace::SIZE, 116);
        assert_eq!(Listing::SIZE, 46);
        assert_eq!(RescueRecord::SIZE, 13);
        assert_eq!(AccessoryListing::SIZE, 323);
        assert_eq!(AccessoryOwnership::SIZE, 47);
    }

    // --- Accessories store -----------------------------------------------------

    #[test]
    fn validate_accessory_listing_accepts_valid() {
        assert!(validate_accessory_listing(
            "Moonlit Waterfall",
            "https://arweave.net/abc123",
            100_000_000_000, // 100 $PAW
        )
        .is_ok());
    }

    #[test]
    fn validate_accessory_listing_rejects_zero_price() {
        assert!(validate_accessory_listing("Collar", "https://x.co/a", 0).is_err());
    }

    #[test]
    fn validate_accessory_listing_rejects_long_name() {
        let long_name = "x".repeat(MAX_ACCESSORY_NAME_LEN + 1);
        assert!(validate_accessory_listing(&long_name, "https://x.co/a", 1).is_err());
        // Exactly at the limit is fine.
        let ok_name = "x".repeat(MAX_ACCESSORY_NAME_LEN);
        assert!(validate_accessory_listing(&ok_name, "https://x.co/a", 1).is_ok());
    }

    #[test]
    fn validate_accessory_listing_rejects_long_uri() {
        let long_uri = "x".repeat(MAX_METADATA_URI_LEN + 1);
        assert!(validate_accessory_listing("Hat", &long_uri, 1).is_err());
        let ok_uri = "x".repeat(MAX_METADATA_URI_LEN);
        assert!(validate_accessory_listing("Hat", &ok_uri, 1).is_ok());
    }

    #[test]
    fn accessory_type_discriminants_stable() {
        assert_eq!(AccessoryType::Habitat as u8, 0);
        assert_eq!(AccessoryType::Pet as u8, 1);
    }

    #[test]
    fn accessory_target_variants_distinct() {
        let pet_target = AccessoryTarget::Pet(42);
        let habitat_target = AccessoryTarget::Habitat;
        assert_ne!(pet_target, habitat_target);
        assert_eq!(pet_target, AccessoryTarget::Pet(42));
    }

    // --- Currency enum ---------------------------------------------------------

    #[test]
    fn sale_currency_round_trips() {
        let sol = SaleCurrency::Sol;
        let paw = SaleCurrency::Paw;
        assert_ne!(sol, paw);
        // Discriminants are stable for on-chain compat.
        assert_eq!(sol as u8, 0);
        assert_eq!(paw as u8, 1);
    }
}
