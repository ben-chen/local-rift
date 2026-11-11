# C28 source admission: movement and battlefield choices

Reviewed October 26, 2026. The [admission manifest](admissions.json) records four units and six battlefields, all eleven complete official faces, raw and effective gallery facts, bounded runtime art, legality, current errata, and immutable origin hashes. The [original visual review](visual-review.json) identifies the actual reviewer and observations. This metadata admission prepares executable definitions; it does not establish that their effects work or that the client matches Standard rules. Separate engine, API, and UI acceptance is required before publication.

| Runtime ID | Unit | Domain | Energy / Power / Might | Main-deck legend IDs |
| --- | --- | --- | --- | --- |
| 417 | Irresistible Faefolk | Body | 2 / absent / 1 | 1, 7, 8 |
| 418 | Kinkou Initiate | Body | 3 / absent / 3 | 1, 7, 8 |
| 419 | Nidalee, Cat Form | Body | 3 / 1 / 4 | 1, 7, 8 |
| 420 | Pyke, Dockside Butcher | Fury | 3 / absent / 2 | 3, 4, 8 |

Nidalee and Pyke are Champion Units, but neither matches a chosen champion for the ten registered legends. Missing printed Power remains null in the source projection and zero in the executable cost representation. All four retain ordinary Unit timing; Nidalee's Ambush and Pyke's Hidden/Ganking require their separate rules descriptors. No card timing is inferred merely from a keyword appearing in reminder text.

| Field ID | Battlefield | Printed and operative text |
| --- | --- | --- |
| 11 | Emperor's Dais | Preserve the printed face; use the current Vendetta erratum linking one Energy plus return of a controlled unit here as the optional cost to play a Sand Soldier here. |
| 12 | Star Spring | The first non-token unit played here by each player each turn permits moving another controlled unit here to its base. |
| 13 | Abandoned Hall | Playing a spell permits giving a controlled unit here +1 Might this turn. |
| 14 | Targon's Peak | Preserve the original “ready 2” face; current Origins errata says “ready up to 2” at the end of the turn. |
| 15 | Shadow Temple | Holding here Burns 3, with the complete printed reminder retained. |
| 16 | Zaun Warrens | Conquering here discards 1, then draws 1, in that printed order. |

The battlefield manifest keeps original gallery text and art references separately from `corrected_text` and `errata_url`. Dais and Peak definitions use the operative erratum, not a silently rewritten image or raw printing. A source record is not proof that costs, response windows, delayed triggers, or physical rune identities have been implemented correctly.

Pyke's two exact faces have a separate complete identity review. The ordinary face includes Hidden and Ganking reminders; the alternate omits those reminders while retaining the same keywords, optional Fury Power cost, and conditional Ready/+2 Might instruction. Core 135.2.d.1–3, 810, and 811 support this explicit reminder-equivalence disposition. Raw strings remain distinct; no generic text normalization manufactures agreement. That review has `enables_runtime_support: false` and alone leaves all new candidates unavailable at 417 main cards / 11 battlefields / 444 mapped legal identities / 635 unavailable.

After the separate metadata mappings, the candidate catalog records 421 main cards, 17 battlefields, 454 mapped Standard-legal identities, and 625 unavailable. Complete alternate-printing reviews become 30 identities / 92 printings, leaving 94 unreviewed identities. These are registry counts, not runtime acceptance results.

All preceding 417 card records, eleven battlefield records, four presets, legends, training recipes, 29 identity reviews, and 90 variant images are pinned as exact prefixes. The 138 field resolutions, 88 reviewed printings, and 53 source issue records are unchanged. No fifth preset is included. Original unit images remain complete portrait faces, and battlefield originals remain complete landscape faces; runtime thumbnails are proportionate 280×391 and 360×258 PNGs, respectively.
