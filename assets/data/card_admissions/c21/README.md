# C21: Ornn, Clairvoyance, and Scryer's Bloom

The [admission proof](admissions.json) registers three selected official identities as main-card IDs **395–397** and preserves their complete **four-printing** English gallery cohorts. Both original Ornn, Blacksmith faces were independently inspected, as were Clairvoyance and Scryer's Bloom. All source gameplay facts match the faces; there are no new transcription corrections, variant dispositions, or applicable errata.

| Runtime ID | Official printing | Card | Printed facts | Current main-deck homes |
|---|---|---|---|---|
| 395 | `sfd-058-221`, alternate `sfd-058a-221` | Ornn, Blacksmith | Champion Unit; Calm; 5 Energy / 1 Power / 5 Might; Ornn, Freljord | Irelia, Master Yi |
| 396 | `ven-056-166` | Clairvoyance | Spell; Mind; 7 Energy / no Power; Reaction, Predict 5 then Draw 2 | Kai'Sa, Viktor, Lux, Ezreal |
| 397 | `unl-136-219` | Scryer's Bloom | Gear; Chaos; 1 Energy / no Power; enters exhausted | Irelia, Kennen, Ezreal |

Ornn is not a Signature and can be included in a compatible Calm main deck, but **no current legend may choose him as its champion**. All ten legends, ten authored training recipes and four sourced presets remain unchanged. Original nullable Power/Might values are preserved in the source proof; runtime numeric zeroes are representation choices.

Original images retain exact downloaded bytes, source URLs, full raw projection hashes and independent visual observations. The four originals total **3,867,454 bytes**. Runtime images are complete, proportional **280 × 391** PNG faces under 300 KB each, fetched from the same official asset URLs with explicit size parameters. Neither these four faces nor their admission status alter the field-correction or whole-identity review manifests.

The original gallery and all five errata authorities remain pinned. Current page data was rechecked on 2026-10-24: all candidate records, complete printing cohorts, parsed errata entries, Standard legality and the current Core PDF link match the archived authorities. This check does not claim a newly downloaded or re-reviewed PDF. Bloom's doubled feed space before its Predict reminder is retained verbatim; visual comparisons ignore physical line wrapping and repeated whitespace, without changing operative words.

The proof pins the previous **395 runtime cards**, authored lists, and the **122 field resolutions / 77 reviewed printings / 47 issue groups**. The complete 29-identity/90-printing variant review remains unchanged. A source-only reconstruction with the old 395-card runtime prefix leaves all three candidates unavailable. Admission metadata is distinct from runtime correctness: Ornn's optional exact-card Draw occurs during mandatory trigger resolution; Clairvoyance and Bloom require ordered, resumable Predict→Draw and Predict→Draw→XP effects, respectively. Those behaviors belong to explicit engine descriptors and focused runtime tests, not inferred text support.

Reproduce the source checks with `python -m unittest discover -s tools -p 'test_c21_admissions.py'` and the metadata/deck tests with `cargo test --locked --test card_batch_twenty_two`. The importer separately validates the generated full inventory and current legality. All art, names and printed rules are Riot Games intellectual property; this unofficial noncommercial client does not claim Riot affiliation.
