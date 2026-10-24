# Reviewed catalog evidence

This is a source-data review, dated **October 24, 2026**. It adds no playable cards or rule implementations. At C7 publication, the client mapped **315 of 1,079 Standard-legal identities**, including 290 main-deck cards, eight legends, eleven battlefields, and six rune types; 764 identities were unavailable. Those are historical review-checkpoint counts. Later runtime admissions are tracked in [Card coverage](CARD_COVERAGE.md); the source review itself does not register cards or change presets, training recipes, stable IDs, or existing eligibility.

## What was reviewed

The first review covers every group in the catalog's original **37 source-quality issue records**, representing **38 official printings**. Matching official card images corroborate 13 missing-type fallbacks, resolve 23 structured/accessibility type contradictions, and show Might 5 on both printings of Kai'Sa, Rebel. The same images reveal additional domain, tag, and text defects on these records. This is a bounded review of those records, not a complete visual audit of the gallery or a count of every possible source error.

The [resolution manifest](../assets/data/catalog_resolutions.json) now records **114 field resolutions across 70 printings: 87 effective changes and 27 confirmations**. The original 73 entries, 38 raw printings, and all 37 historical issue records remain unchanged. A subsequent variant review adds 25 tag observations, one paired Akali punctuation correction, and six paired Rune-placeholder corrections. Two subsequent artwork discoveries correct Bilgewater Tracker’s domain and tag. Of the complete 114 entries, 110 are direct artwork observations; four exclusions of an unprinted Device classification remain explicitly rule-derived. The separate [identity review](VARIANT_REVIEW.md) assesses whole printing groups without rewriting genuine printed text.

| Field | Entries | Review |
| --- | ---: | --- |
| Types | 36 | 23 contradictory structured types; 13 corroborated accessibility fallbacks |
| Domains | 17 | Six incorrect nonempty domains, including Tracker; 11 missing domains |
| Tags | 41 | Original 15 entries; 13 further missing-tag corrections and 13 confirmations |
| Rules HTML | 9 | Activation glyph, stray suffix, duplicate Akali period, six Rune placeholders |
| Readable rules | 9 | The same nine corrections in readable text |
| Might | 2 | Numbered Kai'Sa 5 confirmed; showcase 4 corrected to 5 |

A confirmation leaves the existing normalized value unchanged while recording why it is supported, including corroborated missing-type fallbacks and complete printed tag sets. Empty domain arrays mean incomplete data; they do not make a card colorless. Genuine colorless identities retain an explicit `colorless` value.

## Raw values and effective facts

The catalog keeps original printing fields, including contradictory types, absent domains, raw HTML, accessibility labels, nullable statistics, and the old Kai'Sa 5/4 disagreement. Each printing additionally has an `effective` object for reviewed gameplay facts, `resolution_ids`, and `unresolved_gameplay_fields`. Identity-level `source_review` connects those facts to review status and remaining diagnostics. Consumers must use the effective object and reject unresolved gameplay fields; they must not silently fall back to incomplete raw data.

Historical `source_quality_issues`, `conflicting_fields`, and the original identity kind remain evidence. Effective identity kind and keyword information are separate. Resolving a source issue does not claim that the publisher changed its feed, that a new erratum exists, or that a card's mechanics have been implemented. A supported identity still needs its own reviewed runtime registration.

Each resolution pins its exact official printing ID, field, typed raw value, original source-field value, effective value, review date, basis, and evidence. Boolean values cannot substitute for numeric values. The complete original printing is pinned by canonical JSON SHA-256 under `printing_projection_version:1`: UTF-8, sorted keys, no ASCII escaping, and compact separators. Changing the projected field set requires a new explicit projection version.

## Evidence retained in the repository

- [Field-resolution image index](../assets/data/catalog_evidence/images.json): 70 original Riot CDN images, 67,033,713 bytes in total, with exact URLs, file sizes, and SHA-256 values. The separate [variant image index](../assets/data/catalog_evidence/variant_images.json) covers all 90 members of 29 complete identity reviews. The indexes share 31 files; together they reference 129 distinct originals. These unchanged evidence files are separate from the smaller artwork embedded in the game. Riftbound and this artwork belong to Riot Games; the project is unaffiliated.
- [Immutable origin gallery](../assets/data/catalog_evidence/pages/97ee26fd9a7780b234c7744cf2008abdafd2566f51effdc6864bc096089d33b9.html.gz): the original gzip bytes. Its decompressed SHA-256 is `97ee26fd9a7780b234c7744cf2008abdafd2566f51effdc6864bc096089d33b9`. The importer independently reparses this origin rather than trusting a copied prerequisite in the manifest.
- [October 9 Core Rules PDF](../assets/data/catalog_evidence/rules/94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec.pdf): unchanged official bytes with SHA-256 `94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec`. [Official PDF URL](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).

Every artwork observation matched the printed name and collector number before reading the relevant type line, domain glyph, cost, Might box, or rules panel. Review did not infer these facts from illustration subjects, names, collector-number ordering, or general mechanics. `origin_archives` identifies the immutable reviewed page; `source_archives` retains historical retrieval metadata.

## Reviewed corrections

The table below retains the original 73 manifest entries; the later 41 entries are summarized after it. A type shown as ∅ means the structured type was absent; the current accessibility-derived type is corroborated. Every image link points to the exact bytes used for review. Complete raw/effective strings and hash prerequisites are in the manifest.

| Official printing | Card | Reviewed values | Evidence |
| --- | --- | --- | --- |
| `rad-006-167` | Scorched Earth | domains: ∅ → fury; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-006-167.png) |
| `rad-007-167` | Rally for War | type: gear → spell | [official image](../assets/data/catalog_evidence/images/rad-007-167.png) |
| `rad-009-167` | Antagonistic Apparatus | domains: ∅ → fury; tags: ∅ → Bandle City, Yordle, Mech; type: ∅ → unit | [official image](../assets/data/catalog_evidence/images/rad-009-167.png) |
| `rad-012-167` | Runeblade Anvil | domains: chaos → fury; activation power: Chaos → Fury; type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-012-167.png) |
| `rad-016-167` | Coalesced Energy | domains: mind → fury; tags: ∅ → Shurima; type: spell → unit | [official image](../assets/data/catalog_evidence/images/rad-016-167.png) |
| `rad-022-167` | Whirling Twirligig | domains: ∅ → fury; type: ∅ → gear | [official image](../assets/data/catalog_evidence/images/rad-022-167.png) |
| `rad-025-167` | Consult the Heavens | type: unit → spell | [official image](../assets/data/catalog_evidence/images/rad-025-167.png) |
| `rad-026-167` | Entrenched Boar | domains: ∅ → calm; tags: ∅ → Freljord; type: ∅ → unit | [official image](../assets/data/catalog_evidence/images/rad-026-167.png) |
| `rad-033-167` | Command: Attack | type: gear → spell | [official image](../assets/data/catalog_evidence/images/rad-033-167.png) |
| `rad-034-167` | Hunker Down | type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-034-167.png) |
| `rad-035-167` | Invigorating Bloom | type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-035-167.png) |
| `rad-039-167` | Ephemeral Ward | type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-039-167.png) |
| `rad-041-167` | Hush | domains: ∅ → calm; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-041-167.png) |
| `rad-044-167` | Hunting Blind | tags: Device → ∅; type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-044-167.png) |
| `rad-046-167` | Resplendent Rostrum | type: spell → gear | [official image](../assets/data/catalog_evidence/images/rad-046-167.png) |
| `rad-047-167` | Fanfare | domains: ∅ → mind; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-047-167.png) |
| `rad-049-167` | Industrious Mechsmith | domains: ∅ → mind; tags: ∅ → Zaun; type: ∅ → unit | [official image](../assets/data/catalog_evidence/images/rad-049-167.png) |
| `rad-056-167` | Improvised Projectile | type: gear → spell | [official image](../assets/data/catalog_evidence/images/rad-056-167.png) |
| `rad-059-167` | Supernova | domains: order → mind; type: unit → spell | [official image](../assets/data/catalog_evidence/images/rad-059-167.png) |
| `rad-063-167` | Kai'Sa, Rebel | might: 5 confirmed | [official image](../assets/data/catalog_evidence/images/rad-063-167.png) |
| `rad-064-167` | Scrutinizing Scope | type: spell → gear | [official image](../assets/data/catalog_evidence/images/rad-064-167.png) |
| `rad-069-167` | Zero Point Inflection | type: spell → gear | [official image](../assets/data/catalog_evidence/images/rad-069-167.png) |
| `rad-072-167` | Overloaded Traveler | tags: ∅ → Bandle City, Yordle; type: spell → unit | [official image](../assets/data/catalog_evidence/images/rad-072-167.png) |
| `rad-087-167` | Portable Portal | tags: Device → ∅; type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-087-167.png) |
| `rad-097-167` | Siren of the Shallows | tags: ∅ → Bilgewater; type: spell → unit | [official image](../assets/data/catalog_evidence/images/rad-097-167.png) |
| `rad-101-167` | An Offer You Can't Refuse | type: gear → spell | [official image](../assets/data/catalog_evidence/images/rad-101-167.png) |
| `rad-104-167` | Pitiless Stalker | domains: ∅ → chaos; tags: ∅ → Shadow Isles, Cat; type: ∅ → unit | [official image](../assets/data/catalog_evidence/images/rad-104-167.png) |
| `rad-108-167` | Bushwhack Trap | domains: fury → chaos; tags: Device → ∅; type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-108-167.png) |
| `rad-120-167` | Meticulous Restorationist | tags: Device → Piltover; type: gear → unit | [official image](../assets/data/catalog_evidence/images/rad-120-167.png) |
| `rad-121-167` | One-Two Punch | domains: ∅ → order; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-121-167.png) |
| `rad-123-167` | Protector's Shield | domains: ∅ → order; tags: ∅ → Equipment; type: ∅ → gear | [official image](../assets/data/catalog_evidence/images/rad-123-167.png) |
| `rad-124-167` | Burn Away | domains: body → order; type: gear → spell | [official image](../assets/data/catalog_evidence/images/rad-124-167.png) |
| `rad-127-167` | Rousing Display | rules suffix: `instead.disa` → `instead.`; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-127-167.png) |
| `rad-130-167` | Battlefield Armorer | tags: ∅ → Demacia; type: spell → unit | [official image](../assets/data/catalog_evidence/images/rad-130-167.png) |
| `rad-131-167` | Demacian Illuminators | tags: ∅ → Demacia; type: spell → unit | [official image](../assets/data/catalog_evidence/images/rad-131-167.png) |
| `rad-134-167` | Raucous Anthem | domains: ∅ → order; type: ∅ → spell | [official image](../assets/data/catalog_evidence/images/rad-134-167.png) |
| `rad-136-167` | Dockside Lock-Up | tags: Device → ∅; type: unit → gear | [official image](../assets/data/catalog_evidence/images/rad-136-167.png) |
| `rad-sp4-005` | Kai'Sa, Rebel | might: 4 → 5 | [official image](../assets/data/catalog_evidence/images/rad-sp4-005.png) |

Runeblade Anvil's correction includes its activation glyph as well as its domain. Merely changing the domain would leave a Chaos-powered instruction where the printed card requires Fury. Rousing Display's correction removes only the observed `disa` suffix; it is not a general text-cleanup rule. Protector's Shield has Equipment, an attached Might bonus of 1, and Tank; its raw `might:1` is not interpreted as a unit base stat, and its existing equipment panel and bonus remain unchanged.

Hunting Blind, Portable Portal, Bushwhack Trap, and Dockside Lock-Up print Gear without a Device tag. Core 133.8 identifies tags after the printed type; 763.1's existing tag list excludes Device; 830 does not grant such a tag to Deploy gear. Their effective tag lists are therefore empty. This is a **rule-derived interpretation**, supported by the card faces and Core text, not a card-specific erratum. Meticulous Restorationist is different: its printed Unit/Piltover line directly contradicts the feed's Device classification.

None of the 37 reviewed identities has an indexed erratum in the archived five errata articles. The dated live Radiance errata check listed Elnuk Herd, Last Caress, Forbidden Technique, and Neeko, Blending In, and did not adjudicate these discrepancies. Future official errata or changed card faces require another review.

## Later field corrections and complete variant review

The subsequent review inspects all **90 printings of 29 already-playable identities**. Its [complete identity manifest](../assets/data/catalog_identity_reviews.json) pins each raw and effective printing, applied field resolutions, exact image, immutable gallery origin, and official Core/errata authority. Its 35 field dispositions cover nine tag differences and 26 rules-text differences: **15 corrected transcriptions, 18 reminder-equivalent cases, and two rule-derived equivalences**. These are explicit identity-specific conclusions, not text-normalization rules. See [Variant review](VARIANT_REVIEW.md) for the full scope and reasoning.

The additional 39 field-resolution entries are:

- **25 tag observations across nine identities:** 12 omissions corrected and 13 existing sets confirmed. Every exact face prints its identity's full tag set. Kennen's selected base record `ven-113-166` omits Ionia even though both Kennen faces print it; the runtime tag metadata now follows that reviewed fact. No union of different printed tag sets is used.
- **Akali, Deadly Weapon, `ven-021a-166`:** its image has one final period, while the feed has two. Paired HTML/readable corrections remove only that duplicate period. Its real reminder omission remains printed and preserved.
- **Six Radiance alternate Runes, `rad-r01a` through `rad-r06a`:** their exact faces have blank rules areas. Paired HTML/readable corrections remove the unprinted `[NO TEXT]` feed placeholder. Basic Rune abilities remain supplied by Core 164.2; empty printed text does not remove those abilities.

## Artwork discoveries outside the original parser diagnostics

The C9 admission review found two feed errors on **Bilgewater Tracker**, `rad-003-167`. Its exact [original face](../assets/data/catalog_evidence/images/rad-003-167.png) shows the Fury domain and a printed Bilgewater tag; the structured feed says Body and an empty tag list. The original PNG is 826,331 bytes with SHA-256 `52810b6ac0f04b2b7501f1a8d5f269869d2d7e422904f0684d639b7e934c7570`. A fresh fetch of that exact URL reproduced the same bytes. No domain or region was inferred from the name or illustration.

The two new field overrides pin the entire original printing and immutable origin, retain the exact raw values, and link to two new `official_artwork_field_mismatch` history records. The importer validates the discovery basis, printing, field, typed raw/printed values, date, and unique linked override before projecting those observations into the generated source-quality list. Invented or mismatched observations, missing overrides, changed raw prerequisites, or stale evidence fail closed. This makes **39 visible source-issue groups: the unchanged original 37 plus two explicit visual discoveries**. It does not change what the original parser detected or imply that every unflagged card is correct.

The original 112 resolutions, 69 preserved printings, 37 historical issue records, and all 29 identity reviews remain unchanged. Tracker has one printing and requires no alternate-printing equivalence disposition. The source-only regeneration retained **308 main cards, 333 mapped legal identities and 746 unavailable**, with Tracker unavailable. Its later runtime admission is separately recorded in the [C9 admission evidence](../assets/data/card_admissions/c9/admissions.json). This correction does not admit a card by itself or change Standard legality.

## Remaining source review

The raw snapshot still retains **42 tag-set divergences and 110 rules-text divergences across 124 identities**. These counts include reviewed history, not just remaining uncertainty. Of those identities, 29 now have complete evidence-backed reviews. The other **95 remain unreviewed: 94 Standard-legal identities and the auxiliary Gold token**. A detected difference alone is not proof of a gameplay error. Missing reminder text, presentation changes, and actual operative differences must be distinguished individually.

The former compatibility baseline has been retired. There is no exception admitting a mapped identity with unreviewed variant fields. A new or changed member, raw/effective fact, field resolution, image, Core link, or applicable erratum must pass the review prerequisites. Whole-identity review does not register a card or prove its engine implementation correct.

The original 37 issue groups remain after resolution so a reader can reproduce what the feed said. Their count does not imply that every other record has been visually audited. An unreviewed field on another record remains unreviewed even if it has no current source-quality flag.

## Offline checks, live checks, and refresh

Run `python3 tools/import_catalog.py --check` to verify archived sources, immutable origin data, complete printing prerequisites, every local evidence hash, and generated catalog/report agreement. A stale candidate printing, changed image URL, missing evidence file, or corrupt bytes fails before generated outputs are replaced.

`--check-live` checks current gallery, legality and errata pages, and the current Rules Hub Core PDF link. A markup-only page change can pass when the reviewed complete printing projection is unchanged: the origin page hash identifies historical evidence, not a requirement that the current website retain identical markup. The retained origin still verifies independently. Refresh validates the complete candidate before replacing current source files; it never overwrites the immutable reviewed origin as part of an ordinary refresh.

**A live page check does not download the images or Core PDF again.** It verifies current image URLs, parsed errata and the Core link against the review, then rechecks stored evidence bytes. The visual/rules review remains dated October 24; it cannot detect silently replaced remote bytes at an unchanged image or PDF URL without a separate fetch and review.

These checks establish traceable source facts and safe metadata admission. They do not establish complete Standard rules parity or make an unavailable identity playable without a separate runtime implementation.
