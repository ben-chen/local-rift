# C24: Deploy and battlefield gear source admission

The [admission proof](admissions.json) covers the ten complete original English gallery faces below and preserves the prior 401-card registry. It is source evidence, not proof that the rules engine executes the effects. This isolated admission must be combined with the complete, separately reviewed Deploy runtime, native controls and API before enabling these cards.

| ID | Official printing | Name | Printed domain | Energy / Power / Might |
|---|---|---|---|---|
| 401 | `rad-002-167` | Banner of Conquest | Fury | 1 / 1 / — |
| 402 | `rad-024-167` | Blossoming Dreamroot | Calm | 2 / none / — |
| 403 | `rad-116-167` | Concert Poster | Order | 1 / none / — |
| 404 | `rad-044-167` | Hunting Blind | Calm | 4 / 2 / — |
| 405 | `rad-073-167` | Pillaged Armory | Body | 1 / none / — |
| 406 | `rad-087-167` | Portable Portal | Body | 2 / none / — |
| 407 | `rad-052-167` | Resource Extractor | Mind | 1 / none / — |
| 408 | `rad-106-167` | The Good Stuff | Chaos | 3 / 1 / — |
| 409 | `rad-072-167` | Overloaded Traveler | Body | 2 / none / 2 |
| 410 | `rad-033-167` | Command: Attack | Calm | 1 / 1 / — |

The first eight are ordinary Gear with Deploy, without Equipment panels. Overloaded Traveler is a Unit with Bandle City and Yordle tags; its text refers to friendly Deploy gear but does not give the Traveler Deploy. Command: Attack is an ordinary Spell, without Action or Reaction. None has a Champion or Signature supertype, so none is a chosen-champion candidate. Exact eligible homes among the ten supported legends are recorded per identity. Four sourced presets and all ten authored training recipes remain unchanged.

All ten identities have one complete current English gallery printing and no applicable erratum or unresolved variant discrepancy. Original source values remain alongside the effective facts. Earlier source review corrected Dreamroot's Calm domain, Armory's Body domain and Good Stuff's Chaos domain, plus individually reviewed type and tag errors. Unprinted Device exclusions are explicitly rule-derived under Core133.8,763.1 and830; they are not presented as card-specific errata. The independent [source-only cohort](../../catalog_evidence/reviews/deploy-source.json) preserves all sixteen inspected candidates, including six that this admission leaves unavailable.

Original 744×1039 faces reuse the exact existing PNGs in `catalog_evidence/images/`, avoiding duplicate copies. Their paths, byte counts and hashes are pinned individually. Runtime art is full-face 280×391 PNG from each same official asset URL, under300KB per image. Nothing was redrawn or cropped. Riot Games owns the art, text and names; this is an unofficial noncommercial client.

Fresh October25 official-page comparisons checked the complete cohorts, current Standard legality, all67 errata and the Core Rules link. The freshly downloaded October9 Core PDF matched the archived bytes. The admission preserves137 field resolutions across87 reviewed printings and52 historical issue groups, plus29 alternate-printing identity reviews and90 printing members. Reconstructing the catalog with the previous401-card runtime leaves all ten candidates unavailable. After complete integration the intended inventory is411 main cards /438 mapped legal identities /641 unavailable; source metadata alone does not establish executable support.

Runtime acceptance must exercise independent battlefield gear location, any-field placement without control or combat, ordinary gear Recall versus Deploy's exemption, one respondable destruction trigger per physical gear on opponent Hold, and no destruction on opponent Conquer. Local effects use current source locations and active printed text. Dreamroot needs the actual combat result before control/Conquer; recall or mutual-wipe No Result and uncontested conquest do not trigger it. Poster and Portal require explicit optional trigger base-cost payments before finalization and responses (Core204.1.c/404.1.b.1), legal resource generation, exact source selection, preserved targets and rollback. Their effects execute only after the paid triggers resolve. Armory can buff a friendly unit elsewhere; Portal can ready an enemy unit here. Command: Attack selects its mode and exact physical gear/destination or uses private top-five Gear Draw. Traveler's bonus is existential. Extractor creates actual Gold at base; Good Stuff discards before drawing and returns its exact source on friendly Hold.

Bushwhack Trap, Demacian Standard, Detonation Amplifier, Dockside Lock-Up, Gravity Field and The Ball remain unavailable with their separate unimplemented dependencies. Source readiness is not gameplay support.

Source checks: `PYTHONPATH=tools python3 -m unittest test_c24_admissions`. Deck/metadata checks: `cargo test --locked --test card_batch_twenty_four`. Neither certifies full Standard compliance.
