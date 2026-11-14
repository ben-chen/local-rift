# Card and preset provenance

The base inventory was captured on **October 24, 2026**; later source admissions and reviews are dated below. This is a personal, offline, unofficial practice client. Riftbound, its characters, card names, and official card artwork belong to Riot Games. The client is not endorsed by Riot Games.

Dated sections preserve the scope and remaining work at each admission checkpoint. For current support, use [Implemented rules](RULES.md), [Card coverage](CARD_COVERAGE.md), and the linked validation records. A source-backed identity or deck list is distinct from verified gameplay behavior.

## What the presets contain

Each preset reproduces the published **40-card main-deck composition**, including the chosen champion. At setup, the chosen champion is removed from the shuffled deck into its own playable zone. The published rune split is also used at setup. Complete registered presets include their published sideboard and three battlefield choices; the deck library and match setup expose these separately from single-game field selection. Before a game, each player selects one of eighteen reviewed official fields. The original choices and later additions are recorded below. Each preset defaults to a field from its published list. Runtime support for legend and card abilities is identified separately from these source facts.

| Preset | Published pilot and result | Source |
|---|---|---|
| Irelia / Blade Dance | Xeno, 2nd, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Master Yi / Wuju | SWAGALISK, 6th, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Kennen / Tempest | DSG AwesumRosum, 13th and best Kennen, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Kai'Sa / Void Control | Sinzari, 15th, Vancouver Regional Qualifier, May 2026 | [Riot: Vancouver's Top Decks](https://playriftbound.com/en-us/news/organizedplay/vancouvers-top-decks/) |
| Rengar / Pridestalker | DSG Prismaticism, 1st, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles’ Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Irelia / Arito Convergence IV | Arito, 2nd (8–2–0), Convergence IV, October 31, 2026 | [Organizer deck page](https://topdeck.gg/deck/convergence-4/3Mb0hLsUTwgBYrpAauGOaRvDhnq1) and [standings](https://topdeck.gg/bracket/convergence-4) |
| Riven / Sonecon Stuttgart | Sonecon, 2nd, European Regional Championship, Stuttgart, November 2026 | [Riot: European Championship highlights](https://playriftbound.com/en-us/news/organizedplay/best-ofs-for-the-european-championship/) |

Irelia, Master Yi, and Kennen have primary-source tournament evidence for the dated events above. Kai'Sa is a **historical competitive example** included for a different play style, not a claim that this May list is currently top tier. Radiance released October 23; these are dated, reproducible presets, not a forecast of that new metagame.

The original lists' exact counts and reference metadata are in `assets/data/decks.json`. Saved tournament recipes remain intact. Registered best-of-three practice allows legal sideboarding between games and restores the original registration for a new match. Kai'Sa's original battlefield metadata includes **The Arena's Greatest**, which is banned in the accepted catalog snapshot; it is retained only as historical provenance and is not a selectable battlefield. Consult the [official Rules Hub](https://playriftbound.com/en-us/rules-hub/) for current legality.

## Card facts and artwork

Runtime card facts come from the archived October 24, 2026 [official Riftbound Card Gallery](https://playriftbound.com/en-us/card-gallery/), reviewed source corrections, and the card admissions and operative errata recorded below. [Card coverage](CARD_COVERAGE.md) owns the current counts and identity mapping. The manifest records the original card identifier and CDN image URL for each card. An ordinary printing was preferred where available. The full inventory preserves conflicting printing values rather than assuming alternate printings always agree. It preserves 23 official records whose structured type contradicts their accessibility label. A separate, dated visual review now supplies explicit effective facts for those records and the other original source-issue groups, while keeping both raw claims. Every resolution has exact raw prerequisites and archived artwork evidence; an unresolved gameplay field still blocks runtime mapping. The source review does not add playable cards. See [Catalog review](CATALOG_REVIEW.md) and [Card coverage](CARD_COVERAGE.md).

Card images under `assets/art/` use official Riot CDN artwork; the manifests and individual admissions record their source URLs and any image-service resizing. `card_<id>.png` names match runtime card IDs; `champion_0.png` through `champion_3.png` are aliases of cards 0, 22, 33, and 49. The token-art section below documents token images separately. The game does not need a network connection to load the bundled assets. These images are not original art created for this project.

Runtime IDs 0–68 preserve the original preset catalog. IDs 69–80 append five vanilla units (Mega-Mech, Mountain Drake, Playful Phantom, Shipyard Skulker, and Vanguard Sergeant) and seven spells (Incinerate, Falling Comet, Primal Strength, Progress Day, Premonition, Moonlight Affliction, and Rebuke). These additions come from the same official gallery snapshot and do not alter the sourced tournament lists. Their five empty rules-text fields are intentional: those vanilla units have no printed abilities. `CardDef.official_id` exposes each stable official printing identifier independently of its runtime index.

Runtime IDs 81–102 append seven spells, eleven units whose only abilities use existing combat keywords, and four units with Legion on-play abilities. They retain the official gallery text and costs; none requires a new erratum. Their artwork follows the same Riot CDN source at 360 pixels wide (320 for Clash of Giants). Explicit runtime mappings, keyword evaluation, target choices, and triggered-ability timing are tested separately from those source facts. No tournament preset composition changes accompany this batch.

Runtime IDs 103–151 append fourteen units with static keywords or ready entry, eight pieces of equipment, six resource gear, Battering Ram, seven spells, eight units with immediate play abilities, and five token makers. The source snapshot has no errata or unresolved printing conflicts for these identities. Both equipment panels remain visible; equipment grants and printed Flow costs retain their exact types, including Iterative Design’s Mind power. The complete 49-card expansion preserves the original presets and prior runtime IDs. Cards imported from Radiance retain only the tags supplied by the official structured source; absent tags are not invented.

Runtime IDs 152–229 append 64 units and gear with reviewed event-triggered abilities and 14 spells. Each source identity is Standard legal in the archived October 24 snapshot and has no unresolved type, domain, or statistic conflict or indexed erratum. Unreviewed tag and rules-text variant diagnostics remain recorded separately. `CardDef.tags` preserves the selected source tags for every runtime card, with explicit evidence-backed corrections recorded separately; the later variant review supplies Kennen’s visibly printed Ionia tag. `src/trigger_rules.rs` records explicit per-card events, conditions, chosen targets, and ordered effects; `src/triggers.rs` executes them. These descriptors and source facts do not independently establish interaction conformance. The original tournament lists remain unchanged.

Runtime IDs 230–239 append five gear and five units with explicitly reviewed activated abilities: Heart of Dark Ice, Orb of Regret, Iron Ballista, Arena Kingpin, Arena Bar, Vanguard Armory, Dragonsoul Sage, Xerath, Freed, Caitlyn, Patrolling, and Renata Glasc, Mastermind. Their identities are Standard legal in the same archived snapshot, with no listed errata or unresolved source conflicts. The source archive hashes were verified again before import. They preserve the exact official text and original tags; Caitlyn's printed wording states the combat-assignment restriction rather than using a bracketed Backline label. Iron Ballista's exhausted entry and Arena Kingpin's ready entry are intrinsic entry characteristics. Their other instructions are activated abilities, not automatic effects of playing those cards. `src/activated.rs` supplies the separate reviewed activation descriptors and execution; these source mappings do not establish comprehensive rules conformance. Artwork `card_230.png` through `card_239.png` comes from the same official Riot CDN at 360 pixels wide. The four tournament presets and seven training recipes remain unchanged.

Runtime IDs 240–245 append six ordinary gear: Garbage Grabber (`ogn-099-298`), Assembly Rig (`sfd-019-221`), Divining Shells (`unl-161-219`), Hextech Formula (`ven-062-166`), Ancient Henge (`sfd-117-221`), and Hextech Anomaly (`sfd-083-221`). All six are Standard legal in the same archived snapshot, without listed errata, conflicting source fields, creature tags, or equipment panels. Source archive hashes were checked again before import. Artwork `card_240.png` through `card_245.png` retains the official Riot CDN source at 360 pixels wide, except Ancient Henge at 320. Hextech Formula enters exhausted; Divining Shells has a separate Vision trigger. Other printed instructions belong to activated abilities and must not run automatically when the gear is played. The converter's payment occurs during ability resolution, as the Core's Ancient Henge example states (204.3.a); Garbage Grabber recycles its three cost cards in random order, as its explicit Core example states (417.5). These source facts and declarative metadata do not establish conformance of the runtime cost, target, or nested-payment systems. Existing presets, training recipes, and IDs remain unchanged.

Runtime IDs 246–253 append Poro Snax (`sfd-046-221`), The Syren (`ogn-184-298`), Iridescent Avian (`rad-027-167`), Ultrasoft Poro (`unl-160-219`), Questionable Tome (`ven-054-166`), Hextech Disc (`ven-087-166`), Platewyrm Egg (`ven-075-166`), and Tools of Empire (`ven-077-166`). The eight identities are Standard legal in the same dated snapshot, without unresolved source-field conflicts; all archived source hashes were checked again before import. The Syren uses its [Origins erratum](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/): move the chosen friendly battlefield unit to **its base**. The original gallery HTML retains the older “your base” wording. The other seven have no listed errata. Their original tags and equipment panels remain unchanged; none is Equipment.

Ultrasoft Poro has a narrow, explicitly recorded gallery transcription correction. Its HTML incorrectly marks the token-Might numeral as an Energy icon. The [official card image](https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/36900508e2ce61196450a4bf7e27722ae36521d9-744x1039.png) and the Core Rules’ direct example in **377.2.b** both specify two **1-Might Bird unit tokens with Deflect**, with no token-play Energy payment. The runtime readable text follows those primary sources; the manifest’s `text_correction` records the evidence, while the original gallery HTML and inventory text remain intact. Deflect belongs to the created Birds, not the Poro itself.

These eight cards use the typed activated-ability registry. Poro Snax separately triggers Draw1 when played; Platewyrm Egg enters exhausted. Self-Empower is a respondable activation whose source is not a target (827.1.b.1); Disempower costs and live Empowered alternatives are separate from catalog facts. Artwork `card_246.png` through `card_253.png` uses the official Riot CDN at 360 pixels wide, except Iridescent Avian at 320. Existing presets, training recipes, and earlier runtime IDs remain unchanged; source metadata alone is not a claim of interaction conformance.

Runtime IDs 254–271 append eighteen cards involving next-play grants, ready entry, Bonus Damage, additional costs, and live cost modifiers: Sun Disc, Ravenborn Tome, Temporal Portal, Raging Firebrand, Astral Heron, Annie, Fiery, Consuming Curse, Nami, Headstrong, Bushwhack, Confront, Magma Wurm, Rek'Sai, Breacher, Syndra, Transcendent, Applied Researchers, Helm of Suppression, Stargazer, Vex, Cheerless, and Irelia, Graceful. All eighteen are Standard legal in the dated snapshot without unresolved source conflicts. Every archive hash was checked again before import; printed costs, Might, domains, tags, and original HTML remain intact. Ravenborn Tome uses its [Origins erratum](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/), with which the current gallery text already agrees. Astral Heron uses the [Vendetta erratum](https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/), which explicitly limits the next-card discount to this turn; the manifest also retains the older gallery HTML. No other card in this batch has listed errata. These are reviewed source mappings, not a claim of comprehensive interaction conformance.

Artwork `card_254.png` through `card_271.png` uses the same official Riot CDN at 360 pixels wide, except Astral Heron, Annie, Rek'Sai, and Helm of Suppression at 280 and Nami and Syndra at 320. The lower dimensions keep each bundled PNG under 300 KB. The existing four tournament lists, seven training recipes, and earlier runtime IDs remain unchanged.

Battlefield IDs 4–10 append **The Academy** (`unl-216-219`), **Firelights' Hideout** (`rad-162-167`), **Ornn's Forge** (`sfd-213-221`), **Hunters' Circle** (`rad-163-167`), **Piltovan Forge** (`ven-161-166`), **Risen Altar** (`ven-163-166`), and **Vaults of Helia** (`unl-219-219`). All seven are colorless, Standard legal, and have no listed errata or unresolved source conflict. `assets/data/battlefields.json` retains the exact official identifiers, original HTML, readable text, and image references. All eleven actual field faces are bundled as `battlefield_0.png` through `battlefield_10.png` at 360 pixels wide. Original preset defaults remain IDs 0–3. The importer validates field identities, source text, artwork, and contiguous IDs independently of those defaults.

First-play battlefield discounts require a rules interpretation distinct from delayed next-play grants. The [official Vendetta rules notes](https://playriftbound.com/en-us/news/announcements/core-rules-vendetta-patch-notes/), under “Play Definitions,” and Core 420.4.b tie non-triggered history checks to finalization; Core 420.5 instead ties next-play grants to queueing the next object. The client therefore checks prior finalized history for Ornn's Forge, Hunters' Circle, and Piltovan Forge. A Piltovan-discounted outer activation keeps its determined cost while a nested Add activation can also qualify before either has finalized. This is a rule-derived interpretation of the general rules, not a card-specific Piltovan FAQ ruling. Core 190.6.d also makes Risen Altar's “your units” refer to its controller; an uncontrolled Altar supplies no such discount.

Runtime IDs **272–289** append seventeen units and one gear from Vendetta, preserving all earlier indices: Shadow Fiend, Steel Paws, Serene Ascetic, Kinkou Lifeblade, Brutal Hunter, Solari Sunhawk, Frostcoat Mother, Grumpy Rockbear, Baccai Sandspinner, Rage Amplifier, Aurok General, Apprentice Mage, Covert Informant, Noxian Emissary, Baccai Witherclaw, Nasus, Ascended, Akali, Deadly Weapon, and Ambessa, Respected and Feared. All eighteen printing records are Standard legal with populated domains/types, no indexed errata, and no unresolved type, domain, or statistic conflicts. Unreviewed tag and rules-text variant diagnostics remain recorded separately. All eight archived source hashes were revalidated before import; the nineteen new main/legend images were also checked against their printed names, collector numbers, type/tag lines, costs, and rules. Their original rules HTML and nullable source statistics remain in the full inventory; runtime cost fields represent no Energy or Power requirement as numeric zero. Artwork `card_272.png` through `card_289.png` is 360 pixels wide except Aurok General at 320 and Nasus, Akali, and Ambessa at 280, keeping every bundled image below 300 KB. That runtime batch did not change the 37 historical source-quality issue groups; the separate source review below records their later reconciliation. Conditional characteristics, Empower event provenance, Predict 2 choices, rune-dependent costs, and the new legend are runtime responsibilities tested separately from these factual mappings.

Runtime IDs **290–307** append ten spells and eight units: Final Spark, Lux; Supernova; Rune Prison; Cleave; Vengeance; Soul Harvest; Blast of Power; Flurry of Blades; Avalanche; Firestorm; Soaring Scout; Black Rose Dignitary; Stormclaw Ursine; Dramatic Visionary; Crimson Pigeons; Wielder of Water; Captain Farron; and Taric, Protector. Each identity has one gallery printing, is Standard legal in the dated snapshot, has no indexed erratum, and has no unresolved gameplay or variant field. All eighteen exact original official faces were individually checked against collector IDs, type/supertype and tag lines, Energy, colored Power, Might, domain symbols, and full rules text. These source checks are separate from the effects, targeting, continuous effects, and interaction tests needed for runtime support.

Supernova (`rad-059-167`, runtime 291) uses the already-reviewed C6 **Spell/Mind** facts. Its preserved raw structured feed still says **Unit/Order**; neither the raw printing nor either existing field-resolution object is rewritten. The original face was reused only after its URL, bytes, hash, and raw-printing hash were reconfirmed. Final Spark is a **Lux signature**, so compatible domains alone do not permit it in another legend's deck. Taric is a **Champion Unit** and can occupy a regular main-deck slot in a compatible Calm deck, but neither current Calm legend permits Taric as its chosen champion.

The bounded [C8 admission record](../assets/data/card_admissions/c8/admissions.json) retains the eighteen raw/effective fact sets, exact original image URLs and hashes, individual visual observations, and the pre-admission catalog hash. Seventeen new original PNGs are archived beside that record; Supernova references the existing C6 original. This record is specific admission evidence, not a new blanket equivalence policy. It does not add or alter any of the 112 field resolutions or 29 complete identity reviews. Offline metadata tests verify these pinned source/image facts and preservation of the original 290 runtime entries, published lists, and legend manifest. The existing eight training recipes are unchanged. Runtime artwork `card_290.png` through `card_307.png` uses the official CDN at 360 pixels wide; each is below 300 KB and is bundled offline.

Runtime IDs **308–333** add six spells, fifteen units, and five gears. They cover simultaneous Kill, Channel with a partial-result fallback, group Ready/Exhaust, explicit ready token entry, live board and attached-unit modifiers, Hunt aggregation, and Level thresholds. These behaviors have separate runtime tests; the metadata alone does not establish them. All 26 selected identities are Standard legal in the dated snapshot, have one gallery printing, and have no indexed erratum or unresolved gameplay/variant field after their reviewed corrections. Master Yi, Meditative is an additional legal chosen champion for the existing Master Yi legend. Garen, Commander remains a legal Order main-deck unit, but there is no matching Garen legend in this client. All earlier card IDs, four published lists, eight authored training recipes, and legend identities remain unchanged.

The [C9 admission record](../assets/data/card_admissions/c9/admissions.json) pins each complete raw printing, effective facts, original face URL/bytes/hash, visual observations, and runtime artwork. Twenty-one original images are archived beside the record; five reuse earlier evidence. Every selected face was inspected for collector ID, name, type/tags, costs, domains, Might, and both rules panels. The raw uppercase “ALL” on Unchecked Power and the printed lowercase “all” are retained as a cosmetic case distinction. The original images total 21,678,747 bytes. Runtime images `card_308.png` through `card_333.png` total 6,746,840 bytes; each is an official CDN resize below 300 KB and is bundled offline.

Bilgewater Tracker (`rad-003-167`, runtime 333) required a new, narrow source correction before admission: the feed says **Body** with no tags, while its exact face prints **Fury** and **Bilgewater**. Both raw claims remain. Two independently linked artwork-observation records extend the source history without changing its original 37 parser-detected groups. A source-only regeneration was checked at **308 main cards / 333 mapped legal identities / 746 unavailable**, with Tracker still unavailable; only the later executable admission raises coverage to **334 / 359 / 720**. The source-only checkpoint and preservation hashes are retained in the admission record. No new card mechanic is inferred from a corrected domain or tag.

Runtime IDs 334–344 append the first eleven XP/Level cards: Bandle Soldier, Combat Experience, Concentrate, Gemhand Hunter, Gustwalker, Mosstomper, Scorchclaw, Wuju Apprentice, Wily Newtfish, Mister Root, and Nilah, Joyful Ascetic. These nine units and two spells each have one Standard-legal printing in the dated snapshot, with no indexed erratum or unresolved source conflict. They use existing Hunt, Ganking, Deflect, Accelerate, and movement abilities together with reviewed XP thresholds, conditional ready entry, live Might, and energy reductions. Concentrate's Level 11 reduction replaces its Level 6 reduction; it does not add both reductions. Source facts and metadata are separate from the runtime interaction tests. Nilah is a legal Body main-deck unit, but is not an eligible chosen champion because the client has no matching Nilah legend. All prior IDs, presets, eight training recipes, and legend identities remain unchanged.

The bounded [C10 admission record](../assets/data/card_admissions/c10/admissions.json) pins all eleven complete raw printings, effective facts, exact original face URLs/bytes/hashes, individual visual observations, and runtime artwork. The eleven original PNGs total 8,856,899 bytes. Runtime images `card_334.png` through `card_344.png` total 2,960,739 bytes, using the official CDN at 360 pixels wide except Nilah at 320; each is below 300 KB and bundled offline. No source correction or identity review is added: the 114 field resolutions, 70 preserved raw printing records, 39 historical issue groups, and 29 complete identity reviews remain unchanged. That C10 checkpoint raised the implemented main-card count to 345 and mapped legal identities to 370 of 1,079, leaving 709 unavailable. The later XP-spending and Temporary/Sprite cards were outside that admission.

Runtime IDs 345–348 add Crowd Favorite, Enthralling Protector, Honeyfruit, and Shepherd’s Heirloom. All four are single-print Standard-legal identities with no indexed erratum or unresolved source or variant issue in the dated inventory. The two units spend XP to Buff themselves; Honeyfruit has two separately indexed Add abilities, with the stronger ability available at Level 6; Heirloom gains XP through its printed Rules panel and equips for XP. Its Equipment effect-text panel is empty. The physical +2 attached Might bonus is preserved as `might_bonus=2`; the raw gallery also records `might=2`. Runtime Gear `might` follows the existing attachment-bonus convention, without inventing a Unit stat or source correction. These source facts are distinct from the runtime payment, event, and timing tests.

The bounded [C11 admission record](../assets/data/card_admissions/c11/admissions.json) pins all four original faces and complete raw/effective records. Original PNGs total 3,198,855 bytes. Runtime images `card_345.png` through `card_348.png` total 1,065,020 bytes, all 360 pixels wide and individually below 300 KB. Earlier IDs, presets, eight training recipes, legends, and chosen-champion eligibility remain unchanged. All 114 field resolutions, 70 preserved raw records, 39 historical source-issue groups, and 29 complete identity reviews are preserved. This checkpoint has 349 main cards and 374 mapped legal identities of 1,079, with 705 unavailable. The prepared Temporary/Sprite family is not included; this remains partial Standard coverage.

A subsequent **source-only** review corrects Momentary Companion (`rad-095-167`) from raw empty tags to the printed **Shadow Isles / Demon** tags. Its [exact original face](../assets/data/catalog_evidence/images/rad-095-167.png) and [review proof](../assets/data/catalog_evidence/reviews/momentary-companion.json) preserve the URL, original bytes and hash, complete raw facts, immutable gallery origin, and the explicitly superseded preliminary tag observation. The prior 114 field entries, 70 raw records, 39 issue groups, and every complete identity review remain intact; the new totals are 115 / 71 / 40. At that source-only checkpoint, the 349 / 374 / 705 gameplay totals stayed unchanged: Momentary Companion was unavailable, with no runtime image or implementation added. The subsequent admission below is separate from that correction.

Runtime IDs 349–361 add Fading Memories, Last Stand, Momentary Companion, Petal Pixie, Shadow's Call, Smoke and Mirrors, Sprite Burst, Sprite Call, Sprite Mother, Sprite Queen, Trevor Snoozebottom, Sprite Fountain and Tasty Faefolk. The [C12 admission proof](../assets/data/card_admissions/c12/admissions.json) pins thirteen original faces totaling 10,782,362 bytes, complete raw/effective facts, visual observations and the smaller runtime PNGs. Momentary reuses its prior reviewed original and tag correction. The other twelve source records are unchanged. All are single-print legal identities in the dated inventory, without indexed errata or unresolved source conflicts; none adds chosen-champion eligibility. Fountain is Rules-panel gear with no Equipment text or attached Might bonus. Sprite auxiliary printings remain outside constructed-card registration.

The earlier 349 runtime entries, presets, eight training recipes, legend manifest, 115 field resolutions, 71 preserved printings/images, 40 historical issue groups and 29 identity reviews remain pinned. This admission maps 387 legal identities including 362 main cards, leaving 692 unavailable. The official Rules Hub was checked again on October 24 and still points to the October 9 Core Rules. Source proof, runtime interactions and current tournament metagame recommendations are separate claims.

A later [source-only gear review](CATALOG_REVIEW.md#stalwart-goatherd-and-patched-porobot-two-source-corrections) corrects Stalwart Goatherd’s missing Mount Targon tag and Patched Porobot’s incomplete Unit/Gear type. Both exact original faces and full raw records are archived. At that source-only checkpoint, field-review totals were 117 resolutions / 73 printings / 42 historical issue groups, while playable coverage stayed 362 main cards / 387 legal identities / 692 unavailable. Neither card becomes playable through this correction.

Legend IDs 0–7 are independently recorded in `assets/data/legends.json` and `src/legend_catalog.rs`. The first four retain their original Irelia, Master Yi, Kennen, and Kai'Sa identities; IDs 4–6 add **Hand of Noxus** (`ogn-253-298`, Darius), **Herald of the Arcane** (`ogn-265-298`, Viktor), and **Lady of Luminosity, Starter** (`ogs-021-024`, Lux). These three source identities are Standard legal in the archived snapshot, with no listed errata or printing conflicts. Their matching default chosen champions already exist as runtime cards 183, 175, and 182. Legend availability is independent of the four historical tournament presets; no new tournament lists or results are implied.

Legend **7** adds **Matriarch of War** (`ven-153-166`, Ambessa), with Body/Order identity and **Ambessa, Respected and Feared** (runtime289, `ven-136-166`) as its matching default chosen champion. Both exact source identities have no indexed errata or unresolved type, domain, or statistic conflicts. Unreviewed tag and rules-text variant diagnostics remain recorded separately. `CustomDeck::from_legend(7)` supplies an **authored 40-card training list**, including its chosen champion, five Body and seven Order runes, and Risen Altar. It demonstrates the six new units legal in Body/Order alongside previously reviewed resource, Empower, and combat cards. It is not a tournament list or a metagame claim. All seven older training recipes and the four published preset lists remain unchanged.

The registry keeps exact gallery rules text, names, domain sets, source artwork URLs, and stable official IDs. It uses the original four presets' domain display order where that differs from the gallery order. `legend_0.png` through `legend_7.png` contain the actual legend card faces from the same Riot CDN, bundled for offline use. Their source illustrators are retained in the independent manifest. The importer verifies the legend source facts, matching champion tags and champion-unit type, and compatible chosen-champion domains before creating an implementation mapping. Those validations establish identity and provenance, not comprehensive interaction conformance.

The card-fact manifest and runtime `CardDef.domains` preserve every printed domain on signature cards. A card's colored power cost can use any of its printed domains; for example, Defiant Dance accepts Calm or Chaos, as described in Core Rules 135.2.e.6.c. Published rune splits are stored in the manifest and used by the runtime presets.

## Reviewed source corrections

The [field-resolution manifest](../assets/data/catalog_resolutions.json) retains all 37 original historical issue groups and now covers **122 field entries across 77 original printings**, plus ten explicitly evidenced artwork-discovery issue records. The original 112 entries, 69 raw-printing records, and 37 issue records are unchanged. Later evidence adds 25 tag observations, paired removal of Akali Deadly Weapon's extra final period, and paired removal of six unprinted Radiance Rune `[NO TEXT]` placeholders. Two later entries correct Bilgewater Tracker’s domain and tag; another corrects Momentary Companion’s missing tags, followed by Goatherd’s Mount Targon tag and Porobot’s compound type. Mournful Spirit’s omitted Shadow Isles tag is separately corrected from its original face. Clocktower Guardian’s Mind domain and Shurima tag are independently corrected from its original face. Trifarian Captain’s Noxus/Trifarian tags and Raving Witness’s Freljord tag are separately preserved as C20 discoveries. There are now 118 artwork-based entries and four explicitly rule-derived Device-tag exclusions. These source records changed neither Standard legality nor the 315 mapped identities present at C7 publication. Subsequent runtime admissions are separate from the source review.

The original cohort in the [complete identity manifest](../assets/data/catalog_identity_reviews.json) reviewed **29 already-playable identities and all 90 of their printings**, including all nine mapped tag-divergent identities. Each pins exact raw/effective printing hashes, complete membership, field-resolution records, original image bytes, immutable gallery origin, and official Core/errata evidence. All inspected tag variants actually print matching full sets; 12 feed omissions are corrected and 13 sets confirmed. Kennen's selected base printing visibly includes Ionia, so its runtime tags now retain that fact. Genuine reminder and wording differences remain distinct in the inventory.

The review records 15 corrected-transcription field dispositions, 18 reminder equivalences, and two bounded rule-derived equivalences. Viktor's “in your base” versus “to your base” and Lonely Poro's Deathknell separator are interpreted under their cited general rules, not presented as named official FAQ rulings or rewritten into identical text. Rengar's historical printed variants remain separate from its pinned current Unleashed erratum. [Variant review](VARIANT_REVIEW.md) details these distinctions.

Raw gallery values and historical conflicts remain preserved. A reviewed source correction never makes an identity playable by itself; reviewing existing playable variants adds no new admission. Supernova is subsequently admitted in the separately implemented C8 batch using its existing correction, while every other inventory-only identity still requires its own runtime implementation. The 42 raw tag-set and 110 raw rules-text divergences span 124 identities. Following the November 11 Orianna review, 31 are completely reviewed; 93 remain unreviewed, comprising 92 Standard-legal identities plus auxiliary Gold. The temporary compatibility baseline has been removed. Unresolved variant fields block mapping rather than inheriting an exception. [Catalog review](CATALOG_REVIEW.md) describes field evidence and its limits.

The original visual review is dated October 24, 2026; later additions retain their own dated observations. The [field image index](../assets/data/catalog_evidence/images.json) now covers 88 images; the [identity image index](../assets/data/catalog_evidence/variant_images.json) covers 92, sharing 31 files. Both retain exact official Riot CDN URLs, hashes and sizes. The immutable gallery, October 9 Core PDF and applicable Unleashed erratum page are retained separately. Live checks verify current page facts, image URLs, errata records and the Rules Hub Core link, while rechecking local evidence bytes. They do not freshly download remote images or PDFs. A changed prerequisite or corrupted evidence fails before refresh replaces outputs.

## Official rules and interaction reference

All 379 runtime cards now include **`official_text`**, extracted from the same official gallery snapshot. Both the primary ability panel and the separate equipment-effect panel are included. Resource icons are expanded into readable names such as `[2 Energy]`, `[Body Power]`, `[Any Power]`, and `[Exhaust]`; these expansions change presentation, not card instructions. Bracketed keyword annotations are also available in `CardDef.keywords`.

The gallery snapshot contains some older printed wording. The runtime and manifest apply the [official October 21, 2025 card errata](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5bcbb23cb6131680ec8d469de6c87a3966a7622d.pdf) for three cards in these lists: Zhonya's Hourglass heals the saved unit before recalling it exhausted; Tideturner chooses a controlled unit at another location; and Salvage may kill up to one gear before drawing. The original artwork can still show pre-errata wording. The erratum takes precedence.

The October 24 audit also checked every later errata article linked from the Rules Hub. [Spiritforged errata](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-errata/) supplies Falling Star's two separate damage instructions. [Unleashed errata](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/) explicitly allows Rengar to use Ambush at a battlefield occupied only by enemies. [Vendetta errata](https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/) updates Fizz's recycling wording. Those corrections are reflected in the readable rules text; the manifest keeps original gallery HTML as provenance. [Radiance errata](https://playriftbound.com/en-us/news/announcements/radiance-errata-updates/) does not change a card in the four bundled main lists.

The JSON manifest preserves the original ability HTML, separate equipment-effect HTML, tags, domains, and might bonus. Each preset includes its official legend name, card identifier, and full legend text. `learning_notes` are authored strategy explanations inferred from those actual abilities; they are not quotations or tournament matchup advice.

The three added legends use the same current Core Rules. Viktor's Recruit instruction queues a token play until the activated ability finishes (354.3), followed by the normal base or controlled-battlefield placement choice (355.2); it is an ordinary Main-phase activation (381.1). Lux's trigger observes successful spell completion (420.4.a), including spells paid for by alternative or discounted costs, and checks their printed Energy cost (131.2 and 356.1.c). A countered spell does not trigger Lux.

For Darius, Legion checks a different card's finalization during the turn (812.1.c and 420.4.b), including a subsequently countered spell. The client allows exhausting Hand of Noxus before Legion is active; it adds no energy and retains immediate Add-ability timing. This is a **rule-derived interpretation**, not an explicit Darius ruling: 401.2 classifies an ability containing Add, while 727.1.b.1 preserves inactive dependent text for reference/evaluation. The [official Vendetta FAQ](https://playriftbound.com/en-us/news/rules-and-releases/vendetta-rules-faq-and-clarifications/), in its Heimerdinger/Jayce example, confirms that inactive dependent text can still be evaluated. Legion's inactive instruction itself is not executed (721.2).

The four legend abilities are the foundation of their deck plans: Irelia links choosing friendly units, readying, and conquest; Master Yi strengthens a lone defender; Kennen links playing from outside the hand to Empower and Assault; Kai'Sa generates flexible power reserved for spells. Those abilities must be included before a simulation can teach the archetypes' real sequencing.

| Interaction group | Examples in these exact lists |
|---|---|
| Chain, priority, and counters | Defy, Abandon, Hard Bargain, Not So Fast, Reaction/Action timing |
| Choosing, readying, moving, and bouncing | Irelia, First Mate, Ride the Wind, Flash, Charm, Gust, Star-Crossed |
| Combat modifiers and assignment | Discipline, En Garde, Defiant Dance, Eclipse, Stupefy, Tank, Backline, Assault |
| Triggers at the correct event | Conquer, hold, move, play, death, choose, and ready triggers |
| Costs and resources | Legion discounts, Rhasa's trash discount, Accelerate, Repeat, rainbow power, Seal of Discord |
| Zone-aware play | Hidden, Flow, Burn, Fizz replay, Matriarch revival, Last Rites, banishment |
| Equipment and prevention | Equip, Quick-Draw, Boots' Ganking, Guardian Angel, Zhonya's Hourglass |
| Additional state | XP/Hunt/Level, Empower, stun, protection, tokens, extra turns |

A printed ability and its implementation are different artifacts. `CardDef.official_text` and the JSON manifest preserve sourced rules text; `src/rules.rs` maps that text to operations and the game modules execute them. Earlier prototype substitutions have been removed from the catalog and manifest. Consult the client’s implementation note and `docs/RULES.md` for remaining choices or timing that differ from tabletop play. An official description alone is not proof of complete implementation.

`assets/data/battlefields.json` preserves the original gallery text, identifiers and art references for seventeen source-registered fields. C28's six added fields require separate executable acceptance. Void Gate uses its corrected official wording: bonus damage applies to spells and abilities dealing damage to units there. `src/hidden.rs` implements the facedown-zone and Hidden rules from Core Rules 107.3, 323.6a, 422, and 811, including the later-turn restriction, ordinary hide timing, the base-cost waiver, restricted targets, and revealing lost facedown cards.

The authoritative rules are the [official Rules Hub](https://playriftbound.com/en-us/rules-hub/) and its [October 9, 2026 Core Rules PDF](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf), including subsequent errata. The source snapshot is not a guarantee that every printed interaction is implemented or that later errata have been incorporated.

## Refreshing the data

`src/cards.rs` and `src/legend_catalog.rs` define the reviewed runtime cards and independent legend registry. `assets/data/decks.json` and `assets/data/legends.json` are readable provenance exports, not dynamically loaded at startup. Keep these in sync when adding cards or changing presets. Check the official gallery for card facts and the official tournament articles for lists; retain the event date, pilot, chosen champion, original counts, and source URL. Reassess legality separately from historical list provenance.


Runtime IDs 362–371 add the ten single-type cards in the [C13 admission record](../assets/data/card_admissions/c13/README.md). Ten original faces total 8,216,520 bytes; Goatherd reuses its separately reviewed tag correction and original image. Raw source fields, prior gameplay entries, lists, legends, 117 field resolutions / 73 printings / 42 issue groups, and all 29 identity reviews are preserved. Coverage becomes 372 main cards / 397 legal mappings / 682 unavailable. Patched Porobot's reviewed Unit/Gear type remains outside runtime support.


Runtime IDs 372–375 add Factory Recall, Downwell, Mournful Spirit and Angler Beast with [C14 source evidence](../assets/data/card_admissions/c14/README.md). The four original faces total 3,114,750 bytes; Spirit reuses its preserved source correction. All preceding runtime entries and source-review cohorts remain unchanged. Coverage at this checkpoint was 376 main cards / 401 mapped legal identities / 678 unavailable.

Runtime IDs 376–378 add Grim Apothecary, Retreat and Wind and Ghosts with [C15 source evidence](../assets/data/card_admissions/c15/README.md). All three original faces are archived (2,283,529 bytes total), with independent gameplay-fact observations and an additional Mind Rune icon reference for Retreat. No source correction or alternate-print exception is needed. Prior runtime definitions, recipes and source-review cohorts remain pinned. Current coverage is 379 main cards / 404 mapped legal identities / 675 unavailable.

## C16 legend expansion

The [C16 admission record](../assets/data/card_admissions/c16/README.md) appends Rengar / Pridestalker and Ezreal / Prodigal Explorer as legend IDs 8 and 9. It preserves all five original official faces, printing membership, raw/effective facts, source hashes and independent visual observations. The two runtime portraits reuse the exact preferred original PNG bytes. The original eight legend entries and training recipes are preserved separately and checked against the preceding published API. The two new 40-card lists are authored teaching lists, not current competitive recommendations; their unavailable Signature cards are not substituted or represented as implemented. Runtime behavior has separate tests.


C17 admits Burn Away (`rad-124-167`), Portal Rescue (`ogn-102-298`), Temporal Breach (`ven-066-166`), Thrill of the Hunt (`unl-184-219`) and Arcane Shift (`sfd-200-221`). [Admission evidence](../assets/data/card_admissions/c17/README.md) preserves every complete raw/effective printing, independent face observation and image hash. Burn Away reuses the existing reviewed Order/Spell corrections. Portal Rescue retains its original gallery HTML and original face separately from the [official owner/base erratum](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/). The [Rules Hub](https://playriftbound.com/en-us/rules-hub/) and October 9, 2026 Core rules were rechecked for parent/child timing, linked instructions, rollback, Hidden target revalidation and terminating resolution when a spell leaves the chain.

The [Clocktower Guardian source review](../assets/data/catalog_evidence/reviews/clocktower-guardian.json) preserves the original Fury/Mech gallery fields and corrects only the effective domain/tag to printed Mind/Shurima. Its exact RAD-067/167 face is archived and independently inspected; no domain or regional tag is inferred from the illustration. The source-only checkpoint remains at 384 main cards, 411 mapped legal identities and 668 unavailable. All prior 118 field resolutions, 74 raw records/images, 43 issue groups and 29 complete identity reviews remain intact. The new source totals are 120 / 75 / 45; executable admission is separate.

Runtime ID **384** separately admits Clocktower Guardian (`rad-067-167`) using the [C18 admission evidence](../assets/data/card_admissions/c18/README.md). Its effective Mind/Shurima facts come from the preceding source correction, while raw Fury/Mech values remain preserved. The exact original face is reused without modification; the bundled 280×391 official thumbnail is 239,200 bytes. All previous runtime definitions, legends, recipes, presets and source-review cohorts remain pinned. This checkpoint has **385 main cards / 412 mapped legal identities / 667 unavailable**. Its three triggers, optional Banish play and base-Energy waiver are separately implemented and tested; the source review alone did not admit it.

C19 adds Baited Hook, Dazzling Aurora, Reinforce and Wild Claw. Its [admission record](../assets/data/card_admissions/c19/README.md) pins four independently inspected original faces, the exact current errata and four bounded runtime thumbnails, while preserving all preceding385 runtime card records and the120/75/45 source-review cohorts. The gallery and original faces are not silently rewritten to match errata.

C20 adds Double Trouble, Fate Weaver, Trifarian Captain, Apprentice Smith, Raving Witness and Pakaa Protector. Its [admission record](../assets/data/card_admissions/c20/README.md) preserves six independently inspected complete faces, proportional runtime thumbnails, the preceding 389 card definitions and all earlier source-review objects. All five current official errata pages were compared with the archived authorities; none changes these six cards. The separate [tag review](../assets/data/catalog_evidence/reviews/c20-tags.json) corrects the gallery's missing Noxus/Trifarian tags for Captain and missing Freljord tag for Witness. Source-only regeneration changed those two printing projections while leaving playable counts unchanged. Gameplay admission then brings support to 395 main cards / 422 mapped legal identities / 657 unavailable. Source review now contains 122 field resolutions / 77 raw printings / 47 historical issues; the 29 complete identity reviews and 90 variant members are unchanged.

C21 adds Ornn, Blacksmith; Clairvoyance; and Scryer’s Bloom. Its [admission record](../assets/data/card_admissions/c21/README.md) preserves the complete four-printing gallery cohorts, including both original Ornn faces, and three proportional offline thumbnails. All raw/effective gameplay facts agree; no field correction, whole-identity review or erratum is added. All five current official errata articles, the current gallery records, legality and Core link were rechecked against the archived authority. The preceding 395 cards, ten legends, four presets, ten authored recipes, and 122/77/47 source cohort remain unchanged. Ornn is a Champion Unit legal in compatible Calm main decks, but none of the ten current legends can choose him as its champion. Source evidence establishes facts; the separately tested runtime must preserve exact-card Draw and ordered Predict/Draw/XP resolution.

C22's [source admission proof](../assets/data/card_admissions/c22/README.md) covers Gemcraft Seer, Forecaster and Jeweled Colossus with three independently inspected complete original faces and three bounded proportional offline thumbnails. Their single-printing cohorts have no applicable errata or unresolved source/variant gates. All are ordinary Mind units compatible with Kai'Sa, Viktor, Lux and Ezreal main decks; none adds chosen-champion eligibility. The prior 398 cards, ten legends, four presets, ten authored recipes and 122/77/47 source-review cohort remain unchanged. Forecaster's physical tag order and gallery order are preserved separately but contain the same Mech/Yordle/Bandle City set. The canonical Colossus rules field correctly records the printed Might icon; accessibility encoding is retained separately. No correction or whole-identity disposition is added. Source proof is distinct from the required runtime: separate Vision instances, entry-time grants, actual token tags and Shield 1 must execute through explicit implementations.


## October 26 source and preset recheck

The live gallery and current errata still contain 1,408 printings and 1,079 legal Standard identities. The current Core PDF remains the October 9 edition, effective October 16. Runtime IDs 413–416 add Yasuo, Remorseful; Graves, Blasting Through; Lucian, Gunslinger; and Armored Tusker. The [C27 source record](../assets/data/card_admissions/c27/README.md) pins all six original faces and their effective facts. Tusker's missing Noxus tag raises the explicit field review to 138 resolutions across 88 printings and 53 historical issue groups. Earlier raw values, runtime IDs, four preset lists and alternate-printing reviews are preserved.

The [preset audit](PRESET_AUDIT.md) compares all four bundled lists with fresh official articles, separating their exact 40-card compositions from missing sideboards, battlefield options and current-meta claims. It also identifies missing cards in the Los Angeles winning list to guide the next admissions.


C28’s [source evidence](../assets/data/card_admissions/c28/README.md) covers Irresistible Faefolk, Kinkou Initiate, Nidalee, Cat Form and Pyke, Dockside Butcher (IDs 417–420), plus Emperor’s Dais, Star Spring, Abandoned Hall, Targon’s Peak, Shadow Temple and Zaun Warrens (field IDs 11–16). All eleven original faces, including both Pyke printings, were individually inspected by the recorded reviewer. Ten proportional offline thumbnails retain the complete card faces. The dated gallery, legality, Core link and five errata articles match the separately archived October 26 audit.

The Pyke identity review covers only the observed Hidden/Ganking reminder omission; the optional Fury Power remains an additional play cost while its raw printed base Power field remains null. Dais and Peak retain original text separately from current official errata. No field correction, legend, chosen champion or preset is added. The source-only review keeps 444 legal mappings; the explicit metadata registration reaches 454, leaving 625 unavailable. These are registry counts, not engine or interaction acceptance claims. Exact predecessor files preserve historical whole-file evidence hashes while tests independently verify their live prefixes.


## Reckless Innovator: November 11, 2026

Runtime ID429 uses the official RAD-105/167 face and the accepted October 24
gallery snapshot. The complete face visibly prints Piltover although its raw
gallery record has an empty tag list. The [independent source review](../assets/data/catalog_evidence/reviews/reckless-innovator.json)
corrects only the effective tag and preserves the original raw projection,
image hashes, prior review prefixes and archived preceding ledger. Its single
printing is legal in the dated catalog and has no indexed erratum.

The [admission evidence](../assets/data/card_admissions/irelia_innovator/README.md)
keeps the full face and resized runtime artwork, exact costs, rules text, and
unchanged predecessor cards and five tournament recipes. Availability in custom
Irelia decks is separate from evidence that a particular newer list is a current
meta preset.


## Orianna, Exquisite: November 11, 2026

Runtime ID430 uses both official RAD-042/167 and RAD-042a/167 faces from the
accepted gallery snapshot. Both complete originals were individually inspected;
the alternate omits only Shield's reminder. The identity review preserves both
printed strings, raw facts and all preceding review objects. No new field
correction or erratum is applied. The current Rules Hub's legality facts and
Core Rules link, plus the Radiance errata article, were compared with the
accepted sources again on November 11; their parsed facts remain unchanged.

The [admission evidence](../assets/data/card_admissions/irelia_orianna/README.md)
records the full faces, offline artwork, and unchanged predecessor cards and
five preset recipes. The [validation record](validation/irelia_orianna.json)
separates source review, focused rules tests, complete games, API observations
and native interface checks. Primary provenance for Arito’s newer Convergence
IV list is now verified below; the sixth preset is documented in its own admission.


## Gust Monk: November 11, 2026

Runtime ID431 is VEN-101/166, a Chaos unit costing 2 Energy and no Power with
2 Might and Bandle City/Yordle tags. Its complete original official face was
individually inspected. The accepted gallery has one printing, no unresolved
source conflict and no indexed erratum. No field or variant review is rewritten.
The [admission evidence](../assets/data/card_admissions/irelia_gust/README.md)
pins the raw/effective facts, both artwork sizes and every predecessor source
review, runtime definition and preset recipe. The current Rules Hub still links
the October 9 Core Rules; the Vendetta errata does not alter Gust Monk.

The optional extra play Energy and the separate pre-response banish cost follow
Core 204.1.c, 355.10.c, 383.2–3 and 404–407. The ability grants Assault to its
chosen unit rather than making Assault intrinsic to Gust Monk. The prior claim
that this completed Arito’s main card pool overlooked its two Edge of Night
copies; the Edge of Night admission below corrects that gap. The original
[organizer standings](https://topdeck.gg/bracket/convergence-4) and linked
[deck page](https://topdeck.gg/deck/convergence-4/3Mb0hLsUTwgBYrpAauGOaRvDhnq1)
were read independently on November 11, confirming second place and the complete
list. The [validation record](validation/irelia_gust.json) preserves those facts.
The sixth preset now preserves this exact list, as documented below.

## Edge of Night and Arito’s exact list: November 11, 2026

The [Edge of Night admission](../assets/data/card_admissions/irelia_edge/README.md)
pins SFD-139/221, its complete original official face, raw gallery facts and
separate operative Spiritforged erratum. The erratum and official FAQ distinguish
cleanup recall to base from Hidden’s original-field targeting. The source review
retains every prior card, preset and source-correction record.

The [sixth preset admission](../assets/data/preset_admissions/irelia_arito_2026_10/README.md)
records Arito’s exact 40-card main composition, ten sideboard cards, twelve runes
and three battlefields. Both the organizer standings and the complete rendered
deck entries were read, with no exported raw deck file claimed. This corrects the
previous overlooked Edge of Night gap. The [validation record](validation/irelia_arito.json)
distinguishes that checkpoint’s source facts and UI checks from tournament
match support added later; current registered play is described in [Match practice](MATCH_SERIES.md).

The [Master Yi reaction-card admission](../assets/data/card_admissions/master_yi/README.md)
adds Unyielding Spirit and Repulse from complete official faces, preserving raw
gallery facts, current effective text, printing cohorts and image hashes. Neither
has a reviewed erratum. The November 12 rules audit uses the current October 9
Core Rules: 418.6 and 438 for source-specific damage prevention, and 359.3.e.9.a
for live physical targets across all Repeat executions. Retired FAQs supply no
text override. The preceding 441 main cards and all seven sourced main recipes
remain unchanged. The [validation record](validation/master_yi_reactions.json)
separates authored card-use games from preset simulations and packaged UI/API
checks. At that checkpoint, complete SWAGALISK registration and Alpha Strike
were outstanding; their later acceptance is recorded in [the registration record](validation/master_yi_registration.json).


## Token artwork

The [token artwork manifest](../assets/data/token_art.json) records the original
URLs, printing identifiers, language, dimensions and SHA-256 hashes for Sand
Soldier, Sprite, Bird, Tentacle, Mech, Recruit and Gold. Each complete original
face was inspected. Sprite, Bird, Recruit and Gold use the official English
gallery; Sand Soldier, Mech and Tentacle use original images from the official
Chinese card CDN, located through the manifest’s linked card pages.

The renderer crops the artwork area and uses the engine’s English token identity
and live rules. It does not display the Chinese printed text as English rules,
copy the creating spell’s cost, or register these auxiliary tokens as new legal
main-deck cards. The downloaded originals remain unchanged.

## Irelia-compatible resource cards

The [resource-card manifest](../assets/data/card_admissions/irelia_resources/admissions.json)
preserves the original English faces and gallery records for Clockwork Keeper,
Meditation and Life from Stone, plus image hashes, current effective facts and
the preceding 451-card/seven-preset hashes. Keeper's current rules text follows
the [official Origins erratum](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/);
its original printing and gallery HTML remain intact. The inspector labels the
effective wording as rules text because it may differ from the printed image.
The other two cards have no erratum in the reviewed source bundle. This is a
custom-pool admission, not a new sourced meta deck.

## Spirit Wheel

The [Spirit Wheel admission](../assets/data/card_admissions/irelia_wheel/admissions.json)
preserves its complete English face, gallery record, original and runtime image
hashes, and the preceding 454-card/seven-preset hashes. No erratum is present in
the reviewed source bundle. The October 9 Core Rules 383.3.b/b.1 classify its
initial optional cost as a base cost paid before responses; the admission
records the rejected earlier resolution-payment assumption.

## Royal Entourage

The [Royal Entourage admission](../assets/data/card_admissions/irelia_entourage/admissions.json)
preserves the complete official English face of `sfd-039-221`, original gallery
record, image hashes, and preceding 455-card/seven-preset hashes. The printed Calm
unit costs 3 Energy and 1 Power, has 4 Might and the Shurima tag. No erratum appears
in the reviewed bundle. The official Rules Hub was rechecked November 14 and still
links the October 9 Core Rules. Sections 355.3, 355.5 and 355.18 distinguish
choosing the legend before responses from choosing ready/exhaust on resolution.
Sections 415.1 and 416.1 allow readiness effects that leave the state unchanged;
107.4 places Champion Legends in their own fixed zone.

## Crumbling Sands

The [admission](../assets/data/card_admissions/irelia_sands/admissions.json)
preserves the official `ven-039-166` face and gallery record, image hashes and
preceding 456-card/seven-preset hashes. It is a Calm Reaction spell costing
1 Energy and 1 Power, with no reviewed erratum. The
[Vendetta FAQ](https://playriftbound.com/en-us/news/rules-and-releases/vendetta-rules-faq-and-clarifications/)
explicitly allows countering the first spell after an opponent has played a
second, including when that second spell was countered. October 9 Core Rules
420.4.a/b distinguish successful-resolution play triggers from finalization
checks. The condition is evaluated on resolution, not used as a target filter.

Rockfall Path appends battlefield ID 17 (`sfd-216-221`). Its [admission record](../assets/data/card_admissions/irelia_rockfall/admissions.json) pins the archived official printing, original face and runtime thumbnail; all were checked against the printed prohibition on unit plays. The [official Vendetta FAQ](https://playriftbound.com/en-us/news/rules-and-releases/vendetta-rules-faq-and-clarifications/) distinguishes this restriction from permissions and mandatory location instructions. The [Murcia Irelia list](https://riftwatch.gg/decks/murcia-4/) is secondary evidence for prioritizing this field, not a newly admitted preset or independently verified tournament result. Previous cards, fields and all seven presets are unchanged.

## Consult the Heavens

The [admission](../assets/data/card_admissions/irelia_heavens/admissions.json)
preserves the official `rad-025-167` printing, reviewed effective facts, original
face, thumbnail hashes and preceding 457-card/seven-preset hashes. The complete
face confirms Spell rather than the raw structured Unit label. No erratum is
present in the reviewed bundle. The [official Rules Hub](https://playriftbound.com/en-us/rules-hub/)
was checked November 14 and still links the [October 9 Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).
Sections 814, 317.2.c, 359.3.e.7 and 437 govern Shield, expiry, invalid targets
and Predict respectively. The Radiance FAQ contains no named ruling for this
card; expected interactions derive from the printed text and those general rules.
