# Card and preset provenance

Checked **October 24, 2026**. This is a personal, offline, unofficial practice prototype. Riftbound, its characters, card names, and official card artwork belong to Riot Games. The client is not endorsed by Riot Games.

## What the presets contain

Each preset reproduces the published **40-card main-deck composition**, including the chosen champion. At setup, the chosen champion is removed from the shuffled deck into its own playable zone. The published rune split is also used at setup. The three original tournament battlefield choices remain provenance metadata. Before a match, each player selects one of four implemented official fields: Sunken Temple, Seat of Power, Minefield, or Void Gate. Each preset defaults to a field from its published list. Runtime support for legend and card abilities is identified separately from these source facts.

| Preset | Published pilot and result | Source |
|---|---|---|
| Irelia / Blade Dance | Xeno, 2nd, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Master Yi / Wuju | SWAGALISK, 6th, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Kennen / Tempest | DSG AwesumRosum, 13th and best Kennen, Los Angeles Regional Qualifier, September 2026 | [Riot: Los Angeles' Top Decks](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/) |
| Kai'Sa / Void Control | Sinzari, 15th, Vancouver Regional Qualifier, May 2026 | [Riot: Vancouver's Top Decks](https://playriftbound.com/en-us/news/organizedplay/vancouvers-top-decks/) |

Irelia, Master Yi, and Kennen are recent competitive archetypes with primary-source tournament evidence. Kai'Sa is a **historical competitive example** included for a different play style, not a claim that this May list is currently top tier. Radiance released October 23; these are dated, reproducible presets, not a forecast of that new metagame.

The original lists' exact counts and reference metadata are in `assets/data/decks.json`. No sideboard changes are applied. Kai'Sa's original battlefield metadata includes **The Arena's Greatest**, which is now banned; it is retained only as historical provenance and is not a selectable battlefield. Consult the [official Rules Hub](https://playriftbound.com/en-us/rules-hub/) for current legality.

## Card facts and artwork

All 230 runtime card names, energy costs, power costs, printed might, domains, card types, rules text, and keyword annotations were extracted from the embedded public card data in the [official Riftbound Card Gallery](https://playriftbound.com/en-us/card-gallery/) on October 24, 2026. The manifest records the original card identifier and CDN image URL for each card. An ordinary printing was preferred where available. The full inventory preserves conflicting printing values rather than assuming alternate printings always agree. It also flags 23 official records whose structured type contradicts their accessibility label, preserves both claims, and blocks those unresolved identities from runtime mapping. By-kind inventory totals are therefore provisional; total legal identity counts are unaffected. See `docs/CARD_COVERAGE.md`.

Images under `assets/art/` are the gallery's official Riot CDN card images, resized by its image service to 280–360 pixels wide as PNG (supported by the default Macroquad image decoder). `card_0.png` through `card_229.png` match catalog IDs; `champion_0.png` through `champion_3.png` are aliases of cards 0, 22, 33, and 49. The game does not need a network connection to load the bundled assets. These images are not original art created for this project.

Runtime IDs 0–68 preserve the original preset catalog. IDs 69–80 append five vanilla units (Mega-Mech, Mountain Drake, Playful Phantom, Shipyard Skulker, and Vanguard Sergeant) and seven spells (Incinerate, Falling Comet, Primal Strength, Progress Day, Premonition, Moonlight Affliction, and Rebuke). These additions come from the same official gallery snapshot and do not alter the sourced tournament lists. Their five empty rules-text fields are intentional: those vanilla units have no printed abilities. `CardDef.official_id` exposes each stable official printing identifier independently of its runtime index.

Runtime IDs 81–102 append seven spells, eleven units whose only abilities use existing combat keywords, and four units with Legion on-play abilities. They retain the official gallery text and costs; none requires a new erratum. Their artwork follows the same Riot CDN source at 360 pixels wide (320 for Clash of Giants). Explicit runtime mappings, keyword evaluation, target choices, and triggered-ability timing are tested separately from those source facts. No tournament preset composition changes accompany this batch.

Runtime IDs 103–151 append fourteen units with static keywords or ready entry, eight pieces of equipment, six resource gear, Battering Ram, seven spells, eight units with immediate play abilities, and five token makers. The source snapshot has no errata or unresolved printing conflicts for these identities. Both equipment panels remain visible; equipment grants and printed Flow costs retain their exact types, including Iterative Design’s Mind power. The complete 49-card expansion preserves the original presets and prior runtime IDs. Cards imported from Radiance retain only the tags supplied by the official structured source; absent tags are not invented.

Runtime IDs 152–229 append 64 units and gear with reviewed event-triggered abilities and 14 spells. Each source identity is Standard legal in the archived October 24 snapshot and has no unresolved printing conflict or erratum. `CardDef.tags` now preserves the official structured tags for every runtime card, including empty arrays where the source omits tags. `src/trigger_rules.rs` records explicit per-card events, conditions, chosen targets, and ordered effects; `src/triggers.rs` executes them. These descriptors and source facts do not independently establish interaction conformance. The original tournament lists remain unchanged.

Legend IDs 0–6 are independently recorded in `assets/data/legends.json` and `src/legend_catalog.rs`. The first four retain their original Irelia, Master Yi, Kennen, and Kai'Sa identities; IDs 4–6 add **Hand of Noxus** (`ogn-253-298`, Darius), **Herald of the Arcane** (`ogn-265-298`, Viktor), and **Lady of Luminosity, Starter** (`ogs-021-024`, Lux). These three source identities are Standard legal in the archived snapshot, with no listed errata or printing conflicts. Their matching default chosen champions already exist as runtime cards 183, 175, and 182. Legend availability is independent of the four historical tournament presets; no new tournament lists or results are implied.

The registry keeps exact gallery rules text, names, domain sets, source artwork URLs, and stable official IDs. It uses the original four presets' domain display order where that differs from the gallery order. `legend_0.png` through `legend_6.png` contain the actual legend card faces from the same Riot CDN, bundled for offline use. Their source illustrators are retained in the independent manifest. The importer verifies the legend source facts, matching champion tags and champion-unit type, and compatible chosen-champion domains before creating an implementation mapping. Those validations establish identity and provenance, not comprehensive interaction conformance.

The card-fact manifest and runtime `CardDef.domains` preserve every printed domain on signature cards. A card's colored power cost can use any of its printed domains; for example, Defiant Dance accepts Calm or Chaos, as described in Core Rules 135.2.e.6.c. Published rune splits are stored in the manifest and used by the runtime presets.

## Official rules and interaction reference

All 230 runtime cards now include **`official_text`**, extracted from the same official gallery snapshot. Both the primary ability panel and the separate equipment-effect panel are included. Resource icons are expanded into readable names such as `[2 Energy]`, `[Body Power]`, `[Any Power]`, and `[Exhaust]`; these expansions change presentation, not card instructions. Bracketed keyword annotations are also available in `CardDef.keywords`.

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

`assets/data/battlefields.json` preserves the four selectable fields' original gallery text, identifiers, and art references. Void Gate uses its corrected official wording: bonus damage applies to spells and abilities dealing damage to units there. `src/hidden.rs` implements the facedown-zone and Hidden rules from Core Rules 107.3, 323.6a, 422, and 811, including the later-turn restriction, ordinary hide timing, the base-cost waiver, restricted targets, and revealing lost facedown cards.

The authoritative rules are the [official Rules Hub](https://playriftbound.com/en-us/rules-hub/) and its [October 9, 2026 Core Rules PDF](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf), including subsequent errata. The source snapshot is not a guarantee that every printed interaction is implemented or that later errata have been incorporated.

## Refreshing the data

`src/cards.rs` and `src/legend_catalog.rs` define the reviewed runtime cards and independent legend registry. `assets/data/decks.json` and `assets/data/legends.json` are readable provenance exports, not dynamically loaded at startup. Keep these in sync when adding cards or changing presets. Check the official gallery for card facts and the official tournament articles for lists; retain the event date, pilot, chosen champion, original counts, and source URL. Reassess legality separately from historical list provenance.
