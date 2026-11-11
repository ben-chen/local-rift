# C26: Rengar and Ezreal combat-card source admission

The [admission proof](admissions.json) covers proposed IDs **411–412**: Rengar, Pouncing and Ezreal, Dashing. All four current English printing faces were directly inspected. This source patch requires the separately reviewed engine, API and interface changes before these identities are enabled for play; metadata is not executable-behavior proof.

| ID | Identity | Energy / Power / Might | Main-deck legend IDs | Chosen-champion legend ID |
|---|---|---|---|---|
| 411 | `sfd-025-221` — Rengar, Pouncing | 3 / 1 Fury / 3 | 3, 4, 8 | 8 (Rengar) |
| 412 | `sfd-082-221` — Ezreal, Dashing | 4 / 1 Mind / 3 | 3, 5, 6, 9 | 9 (Ezreal) |

Both are Champion Units. Rengar's face prints Rengar, Ixtal, Cat in that order; the gallery orders the same tags Cat, Rengar, Ixtal. Ezreal's three faces print Ezreal and Piltover. No additional tags are inferred from artwork. Ezreal's alternate cohort is `sfd-082a-221` and `rad-180-167`, with the same complete operative text and stats. No applicable erratum, unresolved field discrepancy or unreviewed variant is present in these two cohorts.

Rengar's Reaction, Assault 2 and attacking-battlefield permission are separate clauses. Ezreal's Action marker belongs to his paid self-Move activation; it does not grant Action timing to the Unit card. His combat-damage restriction does not remove his printed Might or grant Stunned. These distinctions need runtime tests, including current Might, target/source movement, first designation, retained player designation and damage-source attribution. The source proof does not establish those behaviors.

Four exact original official PNGs total **3,928,808 bytes**. The runtime faces are proportional **280 × 391** PNGs fetched from the same official assets with sizing parameters, each below 300 KB. They are neither cropped nor redrawn. All names, card art and text belong to Riot Games; this is an unofficial noncommercial client.

The evidence pins raw/effective records, original and runtime image bytes, complete cohort membership, all five current errata archives, the October 25 live gallery/Rules Hub comparison and the current Core link. It preserves the previous **411 main-card definitions**, all ten legends, four presets, authored training recipes and the existing source reviews: **137 field resolutions / 87 printings / 52 historical issue groups**. No correction or variant disposition is added by this source admission. Source-only reconstruction with the old runtime prefix keeps both identities unavailable.

After separate runtime integration, the projected inventory is **413 main cards / 440 mapped legal identities / 639 unavailable** of 1,079 legal identities. These counts describe coverage, not complete Standard conformance. Source checks are `python3 -m unittest discover -s tools -p test_c26_admissions.py`; Rust metadata/deck checks are `cargo test --locked --test card_batch_twenty_five`. Behavioral evidence is recorded separately.
