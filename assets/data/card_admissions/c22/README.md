# C22: granted Vision source admission

The [admission proof](admissions.json) covers Gemcraft Seer, Forecaster and Jeweled Colossus as proposed main-card IDs **398–400**, with all three original English gallery faces. It preserves exact source facts and the prior 398-card registry; it is not proof of executable behavior. Integration requires the separately reviewed runtime that captures each active Vision instance on entry and resolves them independently. The source patch must be combined with that runtime before these identities are enabled for play.

| ID | Official printing | Name | Energy / Power / Might | Actual printed tags |
|---|---|---|---|---|
| 398 | `ogn-100-298` | Gemcraft Seer | 3 / 1 / 3 | Mount Targon |
| 399 | `sfd-065-221` | Forecaster | 2 / none / 2 | Bandle City, Yordle, Mech |
| 400 | `ogn-086-298` | Jeweled Colossus | 5 / none / 5 | Mount Targon |

All three are ordinary Mind units, legal main-deck candidates for the current Kai'Sa, Viktor, Lux and Ezreal legends. None has Champion or Signature supertype, so none is a chosen-champion candidate. Ten legends, ten authored training recipes and four sourced presets are unchanged.

Each identity has exactly one current English gallery printing and no applicable erratum or unresolved source/variant discrepancy. Names, collector numbers, type, tags, statistics, domain and text were independently checked against each original face. Forecaster's face and gallery list the same three tags in a different display order; both representations are retained. Colossus's face uses the Might icon in its Shield reminder; canonical `rules_html` correctly records that icon, separately from the accessibility field's `[S]` encoding. No source correction or semantic identity review is added.

The three exact original PNGs total **2,498,918 bytes**. Full-face proportional runtime PNGs are **280 × 391**, each under 300 KB, downloaded from the same official asset URLs with explicit sizing parameters. No image was redrawn or cropped. All art, names and card text belong to Riot Games; this is an unofficial noncommercial client.

The gallery origin, complete raw/effective printing hashes, visual observations, original bytes, thumbnail bytes and all five current errata authorities are pinned. The fresh page comparison on 2026-10-24 checked the complete candidate cohorts, parsed errata, Standard legality and current Core PDF link. It does not claim a fresh PDF download. Raw `power:null` remains in the source records; runtime numeric zero is only a representation choice.

The proof preserves the previous **398 main cards**, every earlier definition, presets/recipes, and **122 field resolutions / 77 reviewed printings / 47 historical issue groups**, plus all **29 identity reviews / 90 printing members**. Reconstructing with the old runtime prefix keeps these candidates unavailable. The projected full inventory after source and runtime integration is **401 main / 428 mapped legal identities / 651 unavailable**; source metadata alone does not establish those cards' executable support.

Runtime acceptance must distinguish Seer's intrinsic Vision from its grant to **other** friendly units, Forecaster's self-applicable actual-Mech grant from an intrinsic keyword line, and Colossus's intrinsic Vision plus defender-only Shield 1. Core 817.2 requires separate Vision triggers and private Predict choices, not one Predict N operation. Actual token tags determine Forecaster eligibility. Source departure after a trigger has been captured must not erase that ability; acquiring an aura alone is not a new entry event.

Source checks: `python -m unittest discover -s tools -p 'test_c22_admissions.py'`. Metadata/deck acceptance after runtime integration: `cargo test --locked --test card_batch_twenty_three`. These checks do not assert full Standard conformance.
