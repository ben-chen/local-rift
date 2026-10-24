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

All 69 card names, energy costs, power costs, printed might, domains, card types, rules text, and keyword annotations were extracted from the embedded public card data in the [official Riftbound Card Gallery](https://playriftbound.com/en-us/card-gallery/) on October 24, 2026. The manifest records the original card identifier and CDN image URL for each card. Alternate printings share base stats; an ordinary printing was preferred where available.

Images under `assets/art/` are the gallery's official Riot CDN card images, resized by its image service to 360 pixels wide as PNG (supported by the default Macroquad image decoder). `card_0.png` through `card_68.png` match catalog IDs; `champion_0.png` through `champion_3.png` are aliases of cards 0, 22, 33, and 49. The game does not need a network connection to load the bundled assets. These images are not original art created for this project.

The card-fact manifest and runtime `CardDef.domains` preserve every printed domain on signature cards. A card's colored power cost can use any of its printed domains; for example, Defiant Dance accepts Calm or Chaos, as described in Core Rules 135.2.e.6.c. Published rune splits are stored in the manifest and used by the runtime presets.

## Official rules and interaction reference

All 69 runtime cards now include **`official_text`**, extracted from the same official gallery snapshot. Both the primary ability panel and the separate equipment-effect panel are included. Resource icons are expanded into readable names such as `[2 Energy]`, `[Body Power]`, `[Any Power]`, and `[Exhaust]`; these expansions change presentation, not card instructions. Bracketed keyword annotations are also available in `CardDef.keywords`.

The gallery snapshot contains some older printed wording. The runtime and manifest apply the [official October 21, 2025 card errata](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5bcbb23cb6131680ec8d469de6c87a3966a7622d.pdf) for three cards in these lists: Zhonya's Hourglass heals the saved unit before recalling it exhausted; Tideturner chooses a controlled unit at another location; and Salvage may kill up to one gear before drawing. The original artwork can still show pre-errata wording. The erratum takes precedence.

The October 24 audit also checked every later errata article linked from the Rules Hub. [Spiritforged errata](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-errata/) supplies Falling Star's two separate damage instructions. [Unleashed errata](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/) explicitly allows Rengar to use Ambush at a battlefield occupied only by enemies. [Vendetta errata](https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/) updates Fizz's recycling wording. Those corrections are reflected in the readable rules text; the manifest keeps original gallery HTML as provenance. [Radiance errata](https://playriftbound.com/en-us/news/announcements/radiance-errata-updates/) does not change a card in the four bundled main lists.

The JSON manifest preserves the original ability HTML, separate equipment-effect HTML, tags, domains, and might bonus. Each preset includes its official legend name, card identifier, and full legend text. `learning_notes` are authored strategy explanations inferred from those actual abilities; they are not quotations or tournament matchup advice.

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

`src/cards.rs` is the runtime source of truth. `assets/data/decks.json` is a readable provenance export, not dynamically loaded at startup. Keep these in sync when adding cards or changing presets. Check the official gallery for card facts and the official tournament articles for lists; retain the event date, pilot, chosen champion, original counts, and source URL. Reassess legality separately from historical list provenance.
