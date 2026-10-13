# SolanaPets — Canonical Color Pools

> Sourced from Brad's 8 Genesis Founder character sheets. Gene letters and color
> names are canonical from the sheets; only genuinely empty slots were filled.
> Pet names use Brad's originals (not the sheet typos):
> **Luna** (F otter), **Lenny** (M otter), **Lucy** (F dachshund), **Loboe** (M bulldog),
> **Lilah** (F cat), **Leo** (M tuxedo cat), **Lola** (F capuchin), **Loki** (M capuchin).

On-chain, a pet stores `color_primary` / `color_secondary` as `u8` 0–7 and
`eye_color` as `u8` 0–5. The tables below map those indices to Brad's gene
letters and color names. See `programs/pet-registry/src/lib.rs`
(`coat_color_name`, `eye_color_name`) for the in-code reference.

---

## 🦦 Otters — 8 coat colors

| Idx | Gene | Color name | Approx. hex | On |
|-----|------|------------|-------------|----|
| 0 | A | River Brown | `#6B4A2F` | Lenny |
| 1 | B | Sand | `#C9A876` | Lenny |
| 2 | C | Clay | `#8A5A3B` | Luna |
| 3 | D | Chestnut | `#5C3A21` | Luna |
| 4 | E | White | `#F5F0E6` | *(filled — otter chin/chest cream)* |
| 5 | F | Driftwood | `#7A6A5A` | *(filled)* |
| 6 | G | Pebble | `#A8A49E` | *(filled)* |
| 7 | H | Midnight | `#2E2018` | *(filled)* |

Slots E–H were empty on the sheets and filled to complete the 8-color pool,
themed to the otters' river/coast world.

## 🐶 Dogs — 8 coat colors

| Idx | Gene | Color name | Approx. hex | On |
|-----|------|------------|-------------|----|
| 0 | A | Black | `#1E1A17` | *(filled — classic dog coat)* |
| 1 | B | Brown | `#8B5E34` | Lucy (Neutral Brown) · Loboe (Fawn Brown) |
| 2 | C | Tan | `#C68E5A` | *(filled)* |
| 3 | D | Dark Brown | `#4A2E1A` | Lucy |
| 4 | E | White | `#F7F3EA` | Loboe |
| 5 | F | Golden | `#C9973F` | *(filled)* |
| 6 | G | Slate | `#6E6E6E` | *(filled)* |
| 7 | H | Cream | `#EFE3CC` | *(filled)* |

Note: the sheets use **B** for both "Neutral Brown" (Lucy) and "Fawn Brown"
(Loboe) — same gene slot, breed-shade variants. Canonical slot B = Brown.

## 🐱 Cats — 8 coat colors

| Idx | Gene | Color name | Approx. hex | On |
|-----|------|------------|-------------|----|
| 0 | A | Black | `#141114` | Leo (primary) |
| 1 | B | Neutral Brown | `#7A5230` | Lilah |
| 2 | C | Smoke Gray | `#8C8C94` | *(filled)* |
| 3 | D | Dark Brown | `#3E2A1C` | Lilah |
| 4 | E | White | `#FAFAF7` | Leo (mask, paws, chest) |
| 5 | F | Ginger | `#D97B2E` | *(filled)* |
| 6 | G | Cream | `#F2E4C9` | *(filled)* |
| 7 | H | Silver | `#C0C4CC` | *(filled)* |

## 🐵 Monkeys — 8 coat colors

| Idx | Gene | Color name | Approx. hex | On |
|-----|------|------------|-------------|----|
| 0 | A | Ebony | `#211A14` | *(filled — dark capuchin coat)* |
| 1 | B | Light Brown | `#A9743F` | Lola |
| 2 | C | Copper | `#B06030` | *(filled)* |
| 3 | D | Dark Brown | `#4A2E1A` | Loki (solid) |
| 4 | E | White | `#F6F1E6` | Lola |
| 5 | F | Golden | `#D9A441` | *(filled)* |
| 6 | G | Ash Gray | `#8A857C` | *(filled)* |
| 7 | H | Tan | `#C49A6C` | *(filled)* |

---

## 👁️ Eye colors — 6 canonical

| Idx | Gene | Color name | Approx. hex | Trait | On |
|-----|------|------------|-------------|-------|----|
| 0 | E-A | Amber | `#D9932C` | — | Luna · Loboe (Hazel Brown shade) |
| 1 | E-B | Ocean Blue | `#2E9BD6` | — | Lenny · Loki (Blue Bright shade) |
| 2 | E-C | Rose Amber | `#C25E5E` | — | Lilah |
| 3 | E-D | Warm Brown | `#7A4A22` | — | Lucy |
| 4 | E-G | Green | `#39FF7A` | ✨ Glowing | Leo |
| 5 | E-P | Pink | `#FF6EC7` | ✨ Glowing | Lola |

**Sheet-code notes (kept canonical where possible):**
- Sheets use gene letters **E-A, E-B, E-C, E-G, E-P**. All preserved.
- **E-B was triple-booked** on the sheets: Ocean Blue (Lenny), Warm Brown (Lucy),
  Blue Bright (Loki). Consolidated: E-B = the blue family (Lenny, Loki);
  Lucy's Warm Brown gets its own slot **E-D** (fills the natural letter gap).
  Amber family (E-A) covers Luna's Amber and Loboe's Hazel Brown as shades.
- **Glowing eyes** (Leo's green, Lola's pink) are fantasy-mutation traits —
  rarer in the breeding pool and tied to the glowing-effect system.

---

## Markings reference (0–15, species-specific)

Natural markings (from sheets):

| Species | Natural markings |
|---------|------------------|
| Otter | Chest Patch (Luna) · Chin Blaze (Lenny) |
| Dog | Mask — head/eyes (Lucy) · White Blaze — face/chest (Loboe) |
| Cat | Mask — face / White Chest (Lilah) · White Mask / White Paws (Leo) |
| Monkey | White Face / White Chest (Lola) · Solid Coat — none (Loki) |

Fantasy mutations (creator-pet tier, extremely rare in breeding):

| Pet | Mutation |
|-----|----------|
| Luna | Crescent Moon (chest) · Glowing Tail (luminescent) |
| Lenny | Flow Mark (tail) · Bioluminescent streak |
| Leo | Arrowhead (chest) · Glowing Eyes (luminescent) |
| Lola | Banana Mark (stomach) · Bioluminescent (luminescent) |
| Loki | Glowing Tail Tip — red (luminescent) |
| Lucy · Loboe · Lilah | None — natural beauty |

Full 0–15 marking index per species to be finalized with the breeding program.
