# C20 original-face and source review

Six Standard-legal single-printing identities are appended at IDs 389–394: Double Trouble, Fate Weaver, Trifarian Captain, Apprentice Smith, Raving Witness and Pakaa Protector. This proof pins the previous 389 card records and exact Rust definitions, ten-legend manifest, training recipes, four tournament presets and all earlier source-review objects. Source evidence does not establish executable rules conformance.

All six exact original official faces were independently inspected on 2026-10-24 for name, collector code, type/supertype, costs, Might, domain symbol, tags, rules and equipment panel. Original images are retained under `originals/`, except the two shared correction originals in `assets/data/catalog_evidence/images/`. Full-face proportional runtime images come directly from the official CDN and are each below 300,000 bytes. Artwork remains copyright Riot Games and its credited artists.

Two gallery transcription omissions were corrected before admission:

| Card | Preserved raw tags | Printed and reviewed effective tags |
| --- | --- | --- |
| Trifarian Captain, RAD-129/167 | `[]` | Noxus, Trifarian |
| Raving Witness, RAD-074/167 | `[]` | Freljord |

The separate `assets/data/catalog_evidence/reviews/c20-tags.json` pins complete original records, immutable gallery origin, exact original image URL/bytes/SHA-256, independent observations, and prior cohort hashes. Only effective tags change. No tags are inferred from illustrations or names. Pakaa Protector prints Shurima then Cat; the feed's sorted tag set has the same members, so it needs no correction.

Source-only regeneration before the card append retained 389 main cards / 416 mapped legal identities / 663 unavailable and changed only those two printing projections. The previous 120 field resolutions / 75 reviewed raw printings / 45 historical issues were preserved exactly; the two new corrections give 122 / 77 / 47. The 29 complete identity reviews and 90 variant members remain unchanged. Field corrections neither add playable cards nor adjudicate any new variant cohort.

Five current official errata pages were independently fetched and parsed on the review date; all parsed entries match the immutable archived authorities, and none applies to these six identities. `admissions.json` preserves archived and dated live parsed hashes. Raw null statistics are retained in source records; runtime numeric zero is used for absent Power and non-unit Might. Printed Repeat and Mighty words are source metadata, not substitutes for effect implementation.

Focused checks: `python3 -m unittest discover -s tools -p test_c20_admissions.py` and `cargo test --test card_batch_twenty_one` after runtime integration and inventory regeneration. The separate importer live check verifies refreshed official page facts; offline checks do not imply a new network fetch.
