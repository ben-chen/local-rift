# C8 card admission evidence

This dated record covers only the eighteen gameplay additions at runtime IDs 290–307. It records source facts; the corresponding runtime effects and interactions are verified separately in Rust tests.

`admissions.json` pins the pre-admission catalog, complete raw printing projections, selected effective facts, original image URLs/bytes/hashes, individual visual observations, resized runtime artwork, and existing field-resolution references. All original faces were viewed individually. Readable observations expand icon glyphs using the gallery convention and ignore cosmetic line wrapping; they preserve operative wording and actual type/tag facts.

The seventeen files in `images/` are unedited original official PNGs. Supernova reuses its already archived C6 original through a repository-relative reference. Its raw Unit/Order data remains intact; C8 uses the separately reviewed effective Spell/Mind facts. No existing field resolution, identity review, raw source archive, historical issue, or source-review image index is changed.

The review uses the October 24, 2026 gallery, Rules Hub and errata snapshot. Each candidate has one gallery printing, is Standard legal, and has no indexed erratum or unresolved gameplay/variant field. This is not a continuing live-policy guarantee or a new blanket proof layer.

Offline checks:

- `python3 -m unittest discover -s tools -p 'test_import_catalog.py'` checks pinned raw/effective facts, image bytes, mappings, and preservation of the original 290 runtime entries, four presets and legend manifest.
- `cargo test --test card_batch_ten` checks card facts, legal deck homes, Lux signature restrictions, Taric's chosen-champion restriction, and unchanged authored training recipes.
- `python3 tools/import_catalog.py --check` checks the independently generated complete catalog and coverage report against the archived source pages and registered mappings.

Card names, rules text and artwork belong to Riot Games. Exact image URLs and source provenance are retained in the admission record and `docs/SOURCES.md`; these images are not original artwork made for this project.
