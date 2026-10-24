# C19 original-face and errata review

Four separate Standard-legal, single-printing identities are appended at IDs 385–388. The original 385 runtime records, their exact Rust definitions, ten-legend manifest, training recipes, four tournament presets and complete existing source-review cohorts are pinned in `admissions.json`. This evidence does not assert executable rules conformance.

All four original official images were independently inspected on 2026-10-24. Name, collector code, type, costs, domain symbols, absence of tags, rules and blank equipment panels were checked. The exact original PNG bytes are retained under `originals/`; proportional official-CDN runtime images are under `assets/art/`, each below 300,000 bytes. Artwork remains copyright Riot Games and its credited artists.

| ID | Card | Current text authority | Physical/gallery distinction |
| ---: | --- | --- | --- |
| 385 | Baited Hook | Origins errata | The original face omits the Banish instruction; current gallery already includes it. |
| 386 | Dazzling Aurora | Origins errata | Both original face and current gallery omit Banish; current erratum includes it. |
| 387 | Reinforce | Spiritforged errata | Original face has the earlier play-and-reduce wording; current gallery already includes Banish. |
| 388 | Wild Claw | Current gallery and original face | Both agree; no applicable erratum found in the five current official errata articles. |

Raw and effective gallery fields are retained exactly, including Aurora's outdated text. Runtime readable text uses the separately recorded erratum verbatim where applicable, preserving Riot's symbol shorthand. Runtime source HTML remains the gallery HTML. These are genuine printed-wording changes, not new transcription corrections; no field-resolution or identity-review entry is added.

The five official errata pages were fetched independently and parsed on the review date. Every parsed entry matched the existing archived authority. The proof pins each archived page, its complete parsed-entry set, and the dated live response/parsed hashes. Routine offline checks verify archived evidence; they do not imply a new live fetch. The original faces have exact source URL, byte length and SHA-256 pins.

Run the bounded source checks with `python3 -m unittest discover -s tools -p test_c19_admissions.py` and metadata/deck checks with `cargo test --test card_batch_twenty` after the associated runtime mappings are available. `tools/import_catalog.py --check-live` separately checks current official page facts. Future changes to any source need a fresh review, rather than replacing original evidence.
