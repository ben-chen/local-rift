# Reviewed catalog evidence

This is a source-data review, dated **October 24, 2026**. It adds no playable cards or rule implementations. The client still maps **315 of 1,079 Standard-legal identities**, including 290 main-deck cards, eight legends, eleven battlefields, and six rune types. The other **764 identities remain unavailable**. Presets, training recipes, stable IDs, and eligibility for existing runtime cards are unchanged.

## What was reviewed

The review covers every group in the catalog's original **37 source-quality issue records**, representing **38 official printings**. Matching official card images corroborate 13 missing-type fallbacks, resolve 23 structured/accessibility type contradictions, and show Might 5 on both printings of Kai'Sa, Rebel. The same images reveal additional domain, tag, and text defects on these records. This is a bounded review of those records, not a complete visual audit of the gallery or a count of every possible source error.

The [resolution manifest](../assets/data/catalog_resolutions.json) records **73 field resolutions: 59 effective changes and 14 confirmations**. It retains all 37 historical issue records and all 38 original normalized printings. 69 resolutions are direct artwork observations; four exclusions of an unprinted Device classification are rule-derived and explicitly marked separately.

| Field | Entries | Review |
| --- | ---: | --- |
| Types | 36 | 23 contradictory structured types; 13 corroborated accessibility fallbacks |
| Domains | 16 | Five incorrect nonempty domains; 11 missing domains |
| Tags | 15 | 11 printed-tag corrections; four rule-derived Device exclusions |
| Rules HTML | 2 | One colored activation glyph; one stray suffix |
| Readable rules | 2 | The same two corrections in readable text |
| Might | 2 | Numbered Kai'Sa 5 confirmed; showcase 4 corrected to 5 |

A confirmation leaves the existing normalized value unchanged while recording why a missing structured source field is acceptable. Empty domain arrays mean incomplete data; they do not make a card colorless. Genuine colorless identities retain an explicit `colorless` value.

## Raw values and effective facts

The catalog keeps original printing fields, including contradictory types, absent domains, raw HTML, accessibility labels, nullable statistics, and the old Kai'Sa 5/4 disagreement. Each printing additionally has an `effective` object for reviewed gameplay facts, `resolution_ids`, and `unresolved_gameplay_fields`. Identity-level `source_review` connects those facts to review status and remaining diagnostics. Consumers must use the effective object and reject unresolved gameplay fields; they must not silently fall back to incomplete raw data.

Historical `source_quality_issues`, `conflicting_fields`, and the original identity kind remain evidence. Effective identity kind and keyword information are separate. Resolving a source issue does not claim that the publisher changed its feed, that a new erratum exists, or that a card's mechanics have been implemented. A supported identity still needs its own reviewed runtime registration.

Each resolution pins its exact official printing ID, field, typed raw value, original source-field value, effective value, review date, basis, and evidence. Boolean values cannot substitute for numeric values. The complete original printing is pinned by canonical JSON SHA-256 under `printing_projection_version:1`: UTF-8, sorted keys, no ASCII escaping, and compact separators. Changing the projected field set requires a new explicit projection version.

## Evidence retained in the repository

- [Image index](../assets/data/catalog_evidence/images.json): 38 original Riot CDN images, 31,744,181 bytes in total, with exact URLs, file sizes, and SHA-256 values. These are unchanged evidence files, separate from the smaller artwork embedded in the game. Riftbound and this artwork belong to Riot Games; the project is unaffiliated.
- [Immutable origin gallery](../assets/data/catalog_evidence/pages/97ee26fd9a7780b234c7744cf2008abdafd2566f51effdc6864bc096089d33b9.html.gz): the original gzip bytes. Its decompressed SHA-256 is `97ee26fd9a7780b234c7744cf2008abdafd2566f51effdc6864bc096089d33b9`. The importer independently reparses this origin rather than trusting a copied prerequisite in the manifest.
- [October 9 Core Rules PDF](../assets/data/catalog_evidence/rules/94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec.pdf): unchanged official bytes with SHA-256 `94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec`. [Official PDF URL](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).

Every artwork observation matched the printed name and collector number before reading the relevant type line, domain glyph, cost, Might box, or rules panel. Review did not infer these facts from illustration subjects, names, collector-number ordering, or general mechanics. `origin_archives` identifies the immutable reviewed page; `source_archives` retains historical retrieval metadata.

## Reviewed corrections

The table summarizes the 73 exact manifest entries. A type shown as ∅ means the structured type was absent; the current accessibility-derived type is corroborated. Every image link points to the exact bytes used for review. Complete raw/effective strings and hash prerequisites are in the manifest.

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

## Remaining source review

Alternate printings still show **42 tag-set divergences** and **110 rules-text divergences** in the raw snapshot. These are unreviewed diagnostics, not 110 proven gameplay errors or automatic resolutions. Reminder-text omission, showcase presentation, and real differences must be distinguished individually. Existing reviewed runtime mappings retain their known semantics; this increment does not declare all their alternate printings visually audited. New mappings with unresolved variant differences require explicit review. The [variant compatibility baseline](../assets/data/catalog_variant_baseline.json) pins only existing playable mappings with these diagnostics, their selected runtime source, and every complete normalized printing hash. It preserves compatibility; it does not adjudicate the variant differences. A new mapping or a changed pinned variant must receive explicit review rather than inheriting this exception. The diagnostics remain visible in the generated report, and later complete variant review is still required.

The original 37 issue groups are retained after resolution so a reader can reproduce what the feed said. Their count does not become a claim that all other records are clean. An unreviewed field on another record remains unreviewed, even if that record has no current source-quality flag.

## Offline checks, live checks, and refresh

Run `python3 tools/import_catalog.py --check` to verify archived sources, immutable origin data, complete printing prerequisites, every local evidence hash, and generated catalog/report agreement. A stale candidate printing, changed image URL, missing evidence file, or corrupt bytes fails before generated outputs are replaced.

`--check-live` checks current gallery and legality pages. A markup-only page change can pass when the reviewed complete printing projection is unchanged: the origin page hash identifies historical evidence, not a requirement that the current website retain identical markup. The retained origin still verifies independently. Refresh validates the complete candidate before replacing current source files; it never overwrites the immutable reviewed origin as part of an ordinary refresh.

**A live page check does not download the images again.** It verifies the current page's image URLs against the review and rechecks the stored image bytes. The visual review remains dated October 24; it cannot detect a publisher silently replacing bytes at an unchanged remote image URL without a separate image fetch and review.

These checks establish traceable source facts and safe metadata admission. They do not establish complete Standard rules parity or make any of the 764 unavailable identities playable.
