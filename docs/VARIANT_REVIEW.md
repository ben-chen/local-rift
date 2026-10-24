# Complete alternate-printing reviews

Reviewed **October 24, 2026**. This source review covers **29 existing playable identities and every one of their 90 gallery printings**. It adds no playable identity or card mechanic: coverage remains **315 of 1,079 Standard-legal identities**, with 764 unavailable. “Complete” means complete membership and coverage of the detected tag/text differences in these exact groups; it is not a complete visual audit of every statistic or proof of engine conformance.

The [identity-review manifest](../assets/data/catalog_identity_reviews.json) replaces the former temporary compatibility baseline. Each identity has an explicit disposition for every field in the union of its raw and effective tag/rules/equipment-text differences. The validator does not choose the newest or numbered face, strip parentheticals, union tags, or normalize punctuation to manufacture agreement.

## Three distinct kinds of conclusion

| Disposition | Identity-field cases | Meaning |
| --- | ---: | --- |
| Corrected transcription | 15 | Nine tag fields and six Rune text fields agree after individually evidenced feed corrections. |
| Reminder equivalent | 18 | Exact faces differ only in the reviewed nonfunctional reminder presentation, with Core authority. Distinct printed strings remain. |
| Rule-derived equivalent | 2 | Viktor and Lonely Poro retain genuinely different wording/separators, assessed under specific general rules. These are not named FAQ rulings. |

These **35 field dispositions** cover nine tag fields and 26 rules-text fields; an identity may have both. Akali Deadly Weapon’s separate one-period correction is recorded before assessing its remaining reminder difference. It is not an extra identity-field disposition.

## Evidence and applicability

The [variant image index](../assets/data/catalog_evidence/variant_images.json) contains all **90 exact original Riot CDN images, 86,696,094 bytes**. Each was individually inspected for matching identity/collector footer and the relevant tag or text feature. The index retains the original URL, exact bytes/SHA-256, and visual observations. No runtime thumbnail or OCR inference substitutes for the original image. Riftbound and the artwork belong to Riot Games; this project is unaffiliated.

The field-resolution index remains separate: 69 printing records, of which 31 also occur in the 90-member identity index. Shared image files are stored once. The complete field manifest contains 112 entries; the 73 earlier source corrections and 37 historical issue groups remain unchanged. [Catalog review](CATALOG_REVIEW.md) describes that layer.

Every identity review pins:

- Every current member ID, complete raw projection SHA-256, and complete effective projection SHA-256. The latter overlays only validated gameplay fields on the same full version-1 printing projection.
- Every applied field-resolution record by its full canonical hash, including its reason and evidence. A changed correction cannot silently inherit an earlier identity conclusion.
- Matching original image URL, bytes and hash; the independently reparsed immutable gallery origin; and exact full identity membership in both current and origin records.
- The exact official Core PDF URL/bytes and individual rule anchors; any applicable current erratum as a full parsed record independently verified in archived official page bytes.

New members, changed raw/effective facts, different relevant fields, missing evidence, changed Core links or added/changed errata make a review stale. A review with only part of an identity or only some differing fields is rejected. Source review grants no runtime support and does not bypass scalar/type/domain conflict checks.

The official [October 9 Core PDF](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf) is retained at [its content hash](../assets/data/catalog_evidence/rules/94d41fee86f88b61c4157c0389c0595d990b98e5b42555c99d9f649ee64d1fec.pdf). The relevant [Unleashed erratum page](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/) is separately archived at [its decompressed content hash](../assets/data/catalog_evidence/pages/8b3cb69fea7c499e435c6af0ac89b5c5013711477a172c6bbed3a782eff7ac7b.html.gz).

## Reviewed identities

| Identity | Complete member IDs | Field dispositions |
| --- | --- | --- |
| Ahri, Inquisitive | `ogn-119-298`; `ogn-119a-298`; `sfd-227-221`; `sfd-227-star-221`; `ven-sp3-006` | tags: Corrected transcription |
| Akali, Deadly Weapon | `ven-021-166`; `ven-021a-166` | tags: Corrected transcription; rules_text: Reminder equivalent |
| Akali, Silent | `ven-038-166`; `ven-038a-166` | tags: Corrected transcription |
| Ambessa, Respected and Feared | `ven-136-166`; `ven-136a-166`; `ven-187-166` | tags: Corrected transcription; rules_text: Reminder equivalent |
| Body Rune | `ogn-126-298`; `ogn-126a-298`; `ven-r04`; `rad-r04`; `rad-r04a` | rules_text: Corrected transcription |
| Calm Rune | `ogn-042-298`; `ogn-042a-298`; `ven-r02`; `rad-r02`; `rad-r02a` | rules_text: Corrected transcription |
| Chaos Rune | `ogn-166-298`; `ogn-166a-298`; `ven-r05`; `rad-r05`; `rad-r05a` | rules_text: Corrected transcription |
| Daring Poro | `ogn-210-298`; `unl-225-219` | rules_text: Reminder equivalent |
| Diana, No Longer Human | `unl-149-219`; `unl-149a-219`; `ven-183-166` | rules_text: Reminder equivalent |
| Fury Rune | `ogn-007-298`; `ogn-007a-298`; `ven-r01`; `rad-r01`; `rad-r01a` | rules_text: Corrected transcription |
| Heart of the Tempest | `ven-155-166`; `ven-197-166`; `ven-197-star-166` | rules_text: Reminder equivalent |
| Irelia, Fervent | `sfd-057-221`; `sfd-057a-221`; `sfd-225-221`; `sfd-225-star-221`; `ven-174-166` | rules_text: Reminder equivalent |
| Kai'Sa, Survivor | `ogn-039-298`; `ogn-039a-298`; `ven-sp1-006` | tags: Corrected transcription; rules_text: Reminder equivalent |
| Kennen, Storm of Shuriken | `ven-113-166`; `ven-113a-166` | tags: Corrected transcription; rules_text: Reminder equivalent |
| Lonely Poro | `sfd-036-221`; `unl-221-219` | rules_text: Rule-derived equivalent |
| Master Yi, Tempered | `unl-113-219`; `unl-113a-219` | rules_text: Reminder equivalent |
| Matriarch of War | `ven-153-166`; `ven-196-166`; `ven-196-star-166` | rules_text: Reminder equivalent |
| Mind Rune | `ogn-089-298`; `ogn-089a-298`; `ven-r03`; `rad-r03`; `rad-r03a` | rules_text: Corrected transcription |
| Mystic Poro | `ogn-171-298`; `unl-224-219` | rules_text: Reminder equivalent |
| Nasus, Ascended | `ven-046-166`; `ven-046a-166` | tags: Corrected transcription; rules_text: Reminder equivalent |
| Order Rune | `ogn-214-298`; `ogn-214a-298`; `ven-r06`; `rad-r06`; `rad-r06a` | rules_text: Corrected transcription |
| Pouty Poro | `ogn-013-298`; `unl-220-219` | rules_text: Reminder equivalent |
| Pyke, Returned | `unl-145-219`; `unl-145a-219` | rules_text: Reminder equivalent |
| Rengar, Trophy Hunter | `unl-120-219`; `unl-120a-219`; `ven-179-166` | rules_text: Reminder equivalent |
| Rengar, Unseen | `unl-024-219`; `unl-024a-219` | rules_text: Reminder equivalent |
| Shen, Leader of the Kinkou Order | `ven-138-166`; `ven-138a-166` | rules_text: Reminder equivalent |
| Shen, Scourge of Shadows | `ven-042-166`; `ven-042a-166`; `ven-170-166` | tags: Corrected transcription |
| Vex, Apathetic | `unl-150-219`; `unl-150a-219` | rules_text: Reminder equivalent |
| Viktor, Innovator | `ogn-117-298`; `ogn-117a-298`; `ven-176-166` | tags: Corrected transcription; rules_text: Rule-derived equivalent |

## Specific conclusions and their limits

**Printed tags.** All 25 faces in the nine tag-divergent identities actually print the same full tag set within each identity. Twelve feed records omit a printed region tag; thirteen are confirmed. Both Kennen faces print `Kennen · Ionia · Yordle`; the numbered source record alone omitted Ionia. Its selected runtime tag metadata now follows the direct evidence. This conclusion uses each face, not lore, majority vote, a tag union, or an assumed showcase exception. Core 133.8/143.1 locate and define the tags.

**Akali punctuation.** The image for `ven-021a-166` has one final period after its +1 Might instruction; the feed has two. Paired HTML/readable corrections remove precisely the extra final period. The actual Empower reminder omission remains intact. Other punctuation is not globally ignored.

**Rune placeholders.** The six alternate Radiance Rune faces `rad-r01a` through `rad-r06a` have blank rules areas. `[NO TEXT]` appears only in the feed. Paired corrections remove that literal placeholder and its false observed-keyword implication. Core 164.2 still supplies the normal Basic Rune abilities. This is a feed correction, not permission to omit real rules text.

**Reminder differences.** Core 135.2.c.2 and 135.2.d.1–3 make the individually identified reminder presentation nonfunctional. Each review also identifies the relevant keyword/action rules. Explicit keyword numbers, costs, targets, durations and dependent instructions remain operative; only the actual italic/parenthetical reminders observed on those faces receive this disposition. No generic regular expression determines equivalence.

**Lonely Poro.** `sfd-036-221` prints a Deathknell em dash and a death/alone reminder; `unl-221-219` prints the associated keyword arrow/tab without the reminder. Both associate the exact “If I died alone, draw 1” effect with Deathknell. Core 808.1.b/c supplies that association, alongside the keyword-boundary rules in 135.2.e.7. The review is a bounded rule-derived interpretation of these two exact forms, not a blanket dash/arrow replacement or card-specific erratum. Both printed forms remain.

**Viktor, Innovator.** The OGN faces genuinely say “in your base”; VEN176 says “to your base”. Neither is a feed mistake. Under Core 185.2.a, 420.3.b.1 and 355.2, both instruct a Recruit token Play with the base supplied as its location. Pending-token sequencing follows 354.3, and this is not a separate Move under 421.1. This is a general-rule interpretation, not an identified Viktor-specific FAQ ruling. All three distinct printed texts remain in the catalog.

**Rengar, Trophy Hunter.** The inspected printed variants share the historical “I can be played” placement instruction, with reminder omissions on some faces. The current [Unleashed erratum](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/) instead specifies that the special enemy-field placement is available through **Ambush**. The identity review pins that full current erratum separately. It does not rewrite old printed faces, mistake old text for current authority, or grant ordinary-speed placement. The runtime already uses the erratum.

## Remaining review and checks

The raw history retains 124 identities with variant diagnostics: 123 Standard-legal identities and auxiliary Gold. These 29 identity reviews leave **95 unreviewed identities: 94 Standard-legal identities plus Gold**. Other source facts are not declared clean merely because they lack a diagnostic. Every new admission still needs accurate facts and a separate executable implementation.

Run the offline importer check and the Python review tests to verify the exact local evidence and current applicability. Live page checks additionally verify current parsed facts/errata and the Rules Hub Core link, ignoring unrelated website markup. **They do not freshly download remote image or PDF bytes.** An unchanged URL can conceal changed remote content, so the visual/rules evidence remains dated and a separate fresh fetch is needed to reassess those bytes.

The complete review mechanism establishes explicit source conclusions for exact evidence. It does not establish full Standard rules parity.
