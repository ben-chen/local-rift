# Local Rift · The Practice Grounds

A native Rust client for practicing Riftbound against three levels of AI. It includes a clear playmat over a quiet 3D backdrop, with art-faced unit cards, matching rune reserves and side victory-point tracks, four tournament-derived presets, ten playable legends, a custom deck editor, full card previews, and Mote, a small companion with Beginner and Advanced advice. A local Rust and JSON API supports custom automated opponents.

![The Practice Grounds with card art and matching rune reserves](docs/preview.png)

**This is an unofficial rules implementation under development, not yet identical to standard Riftbound.** The client models turns, resources, movement, scoring, showdowns, priority, and printed card, legend, equipment, and selected battlefield effects. Hover previews and inspectors show complete official text, followed by specific practice notes where needed. Coverage and remaining choice differences are recorded in [Implemented rules and limitations](docs/RULES.md); [Sources](docs/SOURCES.md) records the official references and errata. Eleven battlefield choices are available, and simulated match results do not establish real deck strength.

## Run

Install a current stable Rust toolchain, then run from this directory:

```sh
./run.sh
```

The launcher builds the release client and selects the host graphics libraries when needed. On a normally configured desktop, `cargo run --release` also works. Use `./run.sh --small` for the smaller window.

The first build needs access to the Rust package registry. Card art is embedded into the executable at build time; normal play is offline and needs no account, API key, or server. A graphical desktop and an OpenGL-capable driver are required.

On Debian/Ubuntu, the usual native development packages are:

```sh
sudo apt-get install build-essential pkg-config libx11-dev libxi-dev libgl1-mesa-dev
```

Macroquad uses its native X11/OpenGL backend on Linux. XWayland can provide X11 compatibility on a Wayland desktop. A headless terminal alone cannot display the client.

On this development host, the client launches with the system graphics-library directory selected explicitly:

```sh
LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu cargo run --release
```

That path is a workaround for this x86-64 Debian/Ubuntu host, not a portable requirement. Prefer your system's normal graphics configuration elsewhere. `cargo run --release -- --small` opens a 1152×720 window; the interface also scales when resized.

## Nix

The flake pins Nixpkgs and builds from `Cargo.lock`, including the embedded artwork and fonts. With flakes enabled:

```sh
nix run                       # Build and launch the game
nix run . -- --small           # Launch the smaller window
nix build                     # Create result/bin/local-rift, simulate, and rift-api
nix develop                   # Rust, Cargo, rustfmt, Clippy, rust-analyzer, and native libraries
```

Inside the development shell, use `cargo run --release` and `cargo test --locked` normally. The Linux package and shell supply the X11, input, keyboard, and OpenGL libraries that Miniquad loads at runtime. The packaged game does not depend on the host-library workaround in `run.sh`.

```sh
nix flake check               # Build, run the Rust tests, then simulate 384 games
nix run .#simulate -- --seeds 128
nix run .#api                  # Local JSONL interface for bots
nix fmt                       # Format flake.nix
```

Outputs are provided for x86-64 and ARM64 Linux, plus Apple Silicon macOS. Linux uses X11 or XWayland; launching the game still needs a graphical session. On non-NixOS Linux, matching the host GPU drivers may require [nixGL](https://github.com/nix-community/nixGL). macOS uses the system graphics frameworks. Only x86-64 Linux has been tested here.

`flake.lock` pins the toolchain and system dependencies; update them intentionally with `nix flake update`. Git-based flakes include tracked files, so remember to add new source files and assets to Git when extending the game. Build outputs such as `target/` and `result` are excluded from the package source.

## First match

1. Choose a deck for yourself and another for the AI in **Practice** or **Deck library**. All four presets can be used on either side. The legend picker can create editable training lists for ten legends, and the deck editor saves a custom list for you and one for the AI.
2. Choose the battlefields, who starts, and **Relaxed**, **Tactical**, or **Strategic** difficulty, then **Start practice**.
3. Select up to two opening cards to replace, then begin.
4. Play units at base or a controlled battlefield. Move ready units, individually or as a group, to contest points.
5. During a showdown or spell chain, use your legal responses or **Pass priority**. In an open turn, the same control becomes **End turn**.
6. Hover over a card or unit to read its full text; right-click to pin the inspector. Mote can explain a current decision or offer another thought.

Relaxed uses straightforward plays. Tactical weighs combat and resources. Strategic compares bounded sequences of possible plays and their public board outcomes. It samples unknown draws from cards legal for the public legend identity; it cannot read the opponent’s hidden hand, deck list, facedown identities, or future deck order. Its short search assumes no further discretionary responses while resolving a candidate, so it remains a practice opponent rather than a tournament-strength benchmark.

Both players use the same rune layout: upright runes are ready, sideways runes are exhausted, and the rune deck shows how many remain to be channeled. Use your rune controls to exhaust or recycle a chosen domain before a play, or let payment activate runes automatically. Enable **Power: choose sources** to choose exactly which generated power and runes pay for each action. Generated energy, colored power, flexible power, and spell-only power are shown separately. Battlefield effects remain visible on the mat, with a **Rules** button for the full explanation. A gold strip beneath each equipped unit shows its gear. Click a gear name to inspect it, or the **N gear** count to open all attachments. Sparse unit rows expand to make names and artwork easier to read. Board thumbnails crop the artwork proportionally instead of stretching it; hover for the full card.

Deploy gear has its own illustrated row at each battlefield. Click the field’s gear count to open those exact copies in Loadout; attached equipment remains visible beneath its bearer. Gear can occupy a battlefield without controlling or contesting it. Command: Attack offers separate Move and Gear-draw modes, and each Repeat keeps its own choices. Optional paid gear triggers show their source and targets before payment. **Undo payment** returns to that choice; **Decline** skips the optional trigger. Mote explains each step at your selected advice level.

Victory points climb the board’s outer edges: your teal track is on the left, and the opponent’s gold track is on the right. Filled steps show points earned; the diamond marks the current total. Both tracks share the same scale from zero to eight. The decorative scenery is clipped to the board so it stays clear of the companion and controls.

Abilities that recycle trash show a separate cost tray, so those cards are not confused with effect targets. Resource conversions have an amount control and can use another resource ability before finishing. **Generate resources** keeps the chosen play and its calculated cost fixed while you pay. A card being played appears as pending and leaves its original zone. If the attempt can no longer finish, rollback restores the position before it began, including the original mandatory choice.

Gear with **Empower** has a separate ability button and a visible Empowered state. Its second ability may spend that state as a Disempower cost. Costs and use restrictions appear before confirmation; ordinary Empower takes effect after responses. The same controls handle gear with multiple printed abilities.

Empowered units have a visible status marker. Their inspector shows current Might and keywords, plus active and inactive conditional effects with their sources. Ability controls show the current payable cost, including rune discounts. Apprentice Mage's Predict2 lets you keep or recycle each card and order the kept cards before confirming. The legend picker includes editable training lists for Ambessa, Rengar and Ezreal. Use its page controls to browse all ten legends. Rengar asks you to choose a unit after each unit play; Ezreal’s legend panel shows progress toward its two enemy choices and enables the draw ability when its prerequisite is met.

XP costs are shown separately from Energy and Power. Crowd Favorite and Enthralling Protector spend2XP before their self-Buff can be responded to. Honeyfruit shows both of its resource abilities at Level6, including the exact Energy/Any Power output. Shepherd's Heirloom gains1XP when played and spends1XP to Equip; its gold attachment strip then shows the bearer’s gear as usual. Fixed payments preserve the declared XP cost, and failed-attempt rollback restores XP and generated resources together.

When an effect lets you look through your deck, choose a highlighted card or **Recycle all**, then confirm. The tray identifies each physical copy and explains why other cards cannot be selected. Only you see your private look; Aurora’s reveals are public and can be paged through. Wild Claw offers its separate Empower after the chosen unit or gear enters, with time for responses.

Open **Trash → Banish** to inspect banished cards. The owner button switches between your cards and the opponent’s. Clicking a banished card opens its full text. Click **Banish** again to return to trash.

Gear-targeting abilities open the Loadout, highlight legal equipment, and retain your chosen target when switching players. Printed Energy limits appear beside the confirmation controls; optional abilities have a separate decline button.

Temporary units have a visible marker and a hover explanation of their expiry. Sprite previews show the token’s own identity. Petal Pixie shows its live bonus, and ordered target guidance explains which unit to choose next. Mixed unit/gear spells keep their draft when switching to Loadout.

The battlefield picker previews each location's art and full rules before selection. **Turn effects** shows delayed grants and their expiry. **Play choices** lets you select independent Repeat costs and targets, while **Review costs** handles Energy-versus-Power discounts and their order. The same choices are available to automated opponents through the API. Firestorm selects a named battlefield, even when it is empty; each Repeat execution can select a different field. Those field choices remain visible while players can respond. Area damage follows the selected area, and the unit inspector explains bonuses such as Taric’s nearby Shield or Wielder of Water fighting alone.

| Control | Action |
|---|---|
| **Power: choose sources** | Review each power payment; adjust sources, use the suggestion, or close the editor before submitting |
| Click one of your rune cards | Choose to exhaust it for energy or recycle it for power |
| Click a unit's gold equipment strip | Inspect the named gear; click **N gear** to open all attachments |
| Battlefield **Rules** button | Pin the location's full effect and scoring reminder |
| Left-click a hand card | Select it; use its play/deploy button, or click an eligible target |
| Drag a hand card | Drop on a highlighted destination or target; plays needing more choices remain selected for confirmation |
| Left-click a friendly unit | Select its movement controls; use **Abilities** for printed activations |
| Conversion controls | Choose how much energy or power to convert; use another resource ability and return to the same payment |
| Unit or gear ability controls | Choose an ability and targets, then confirm its cost; cancelling an unsubmitted activation spends nothing |
| Drag a friendly unit or selected group | Move to a legal highlighted battlefield or your base |
| Click a battlefield with a unit selected | Move there if legal |
| Click or drop a battlefield-targeting spell on a field | Choose that physical field; use the execution tabs to set each Repeat target |
| **Group all … ready here** / Shift-click friendly units | Group ready units for one move to a common destination |
| Move / return buttons | Move the selected unit or group |
| **Pass priority** / `Space` | Pass during a chain or showdown; end the turn when no window is open |
| Decision buttons and card choices | Resolve Predict, optional payments, discard, revealed cards, trash plays, and unit choices |
| Combat damage controls | Reorder targets or use **Exact amounts** to assign damage to each unit before simultaneous combat damage |
| Repeat target toggle | Choose separate targets for the repeated spell effect |
| **Board** / **Loadout** / **Trash** | Browse all units, inspect public loadouts and trash for either player, and use your eligible cards |
| Right-click a card or unit | Open its inspector and official rules text |
| Hover over a card or unit | Read its complete preview; scroll inside a long preview or inspector |
| `1`–`9` | Select the corresponding card in your hand |
| `H` | Open the guide from Practice |
| `Esc` | Cancel selection, close an overlay, or return to Practice |
| On-screen arrows | Browse decks, scroll the hand/list, or adjust the board view |
| Mouse wheel over the deck list | Scroll the selected preset's cards |

An invalid drop or a cancelled drag spends no resources. Click controls remain available alongside dragging. Closing the source editor does not undo a payment already started. Choosing zero for a resource conversion finishes it but keeps the exhaustion cost already paid; a failed payment attempt can be rolled back with its separate rollback control.

Choose Mote's **Beginner** or **Advanced** level in the sidebar, or turn advice off in Settings. Mote considers your hand, the public board, and information effects explicitly let you see; it does not peek at private opponent cards. Advice is optional and never interrupts an action.

The client saves completed wins/losses and coach, advice level, animation, and difficulty preferences to `$XDG_DATA_HOME/local-rift/preferences.txt`, or `~/.local/share/local-rift/preferences.txt` when `XDG_DATA_HOME` is unset. **New match → Choose matchup** returns to setup and re-enables deck selection. Custom lists are stored beside those preferences as `custom-you.json` and `custom-ai.json`. The editor validates before saving, and saved lists can also be passed to the example bot. There is no resumable match-save format.

## Included presets

| Deck | Real archetype reference | Published source |
|---|---|---|
| Irelia / Blade Dance | Movement, choosing friendly units, and conquest | Xeno, Los Angeles runner-up, September 2026 |
| Master Yi / Wuju | Buffs and efficient combat | SWAGALISK, Los Angeles sixth place, September 2026 |
| Kennen / Tempest | Repeated actions, tokens, and resource loops | DSG AwesumRosum, Los Angeles thirteenth place, September 2026 |
| Kai'Sa / Void Control | Spell resources, removal, and finishers | Sinzari, Vancouver fifteenth place, May 2026; historical list |

The three Los Angeles lists are recent competitive examples as of **October 24, 2026**. Kai'Sa is an older competitive archetype included for variety. These are dated source lists, not an assertion about the newly released Radiance metagame. The full 40-card main lists are preserved. Their 69 distinct cards and 342 additional cards for custom decks have executable printed-effect mappings. All ten registered legend powers, persistent equipment, Hidden, Flow, Repeat, Accelerate, and Empower are implemented. The additional unit and gear activations include explicit targets, trash-recycling, sacrifice and Disempower costs, response windows, and resource conversions during permitted payments. The engine exposes trigger order, on-play targets, death replacements, and rune activation as choices. Coverage and remaining interface limits are described in [RULES.md](docs/RULES.md); this is still an unofficial implementation under development.

Counts, card IDs, original rune splits, and source URLs are recorded in [`assets/data/decks.json`](assets/data/decks.json). The [official tournament sources](docs/SOURCES.md) explain provenance and historical legality.

The legend picker adds **Darius / Hand of Noxus**, **Viktor / Herald of the Arcane**, **Lux / Lady of Luminosity, Starter**, and **Ambessa / Matriarch of War**, **Rengar / Pridestalker**, and **Ezreal / Prodigal Explorer** alongside the original four. It shows each legend’s official portrait and full rules. Create an editable 40-card training list, adjust it in the editor, and save it for either seat. These six lists are authored practice starting points, not sourced meta decks. Darius practices Legion sequencing, Viktor develops Recruit boards, Lux rewards spells with a printed Energy cost of five or more, Ambessa practices Empowering objects and readying units, Rengar turns unit plays into temporary Might, and Ezreal rewards choosing enemy units or gear with spells and unit abilities. Rengar’s and Ezreal’s Signature spells are still unavailable; the lists use supported cards and make no claim to reproduce current meta decks.

Legend identity is independent of preset choice and chosen-champion variant. Existing saved decks keep their numeric identities; invalid saved files now produce a visible load message without being overwritten.

The [full card inventory](docs/CARD_COVERAGE.md) accounts for 1,079 legal Standard identities, rechecked on October 25, 2026. Current support maps 438 identities, including 411 main-deck cards, ten legends, eleven battlefields, and six runes; 641 remain unavailable. The reproducible importer preserves official printings, bans, and errata without registering unsupported cards as playable. Its [source review](docs/CATALOG_REVIEW.md) records 137 field corrections or confirmations, backed by archived official artwork and rules. All 37 original issue groups are retained alongside fifteen later artwork-confirmed discrepancies. Original source values remain available alongside the reviewed facts. Complete [alternate-printing reviews](docs/VARIANT_REVIEW.md) now cover 29 playable identities and all 90 of their gallery printings, replacing the previous compatibility exceptions. Kennen’s missing Ionia tag is corrected in gameplay and the bot API. Another 95 identities have alternate-printing differences still awaiting review; those identities remain unavailable. Unresolved gameplay facts or variant differences prevent admission. Source corrections alone do not register playable cards. The latest admission adds eight Deploy gear cards, Overloaded Traveler and Command: Attack, with independent battlefield gear, optional paid triggers and explicit Move or private Gear-draw modes. Their [source evidence](assets/data/card_admissions/c24/README.md) preserves all ten complete official faces and their printing cohorts. The preceding gameplay admission added Gemcraft Seer, Forecaster and Jeweled Colossus, with source-attributed Vision grants, independent entry triggers and actual token identity. Their [source evidence](assets/data/card_admissions/c22/README.md) preserves all three complete original faces. The preceding gameplay admission added Ornn, Blacksmith; Clairvoyance; and Scryer's Bloom, with private Gear selection and ordered Predict, Draw and XP instructions. Their [source evidence](assets/data/card_admissions/c21/README.md) preserves all four original gallery faces, including both Ornn printings. The preceding admission added Double Trouble, Fate Weaver, Trifarian Captain, Apprentice Smith, Raving Witness and Pakaa Protector, with private inspected draws and public Move-triggered reveals. Their [source evidence](assets/data/card_admissions/c20/README.md) preserves all six complete faces and separately records two printed-tag corrections. The preceding admission added Baited Hook, Dazzling Aurora, Reinforce and Wild Claw, with private deck selection, public reveals, precise cost reductions and linked Empower. Their [source evidence](assets/data/card_admissions/c19/README.md) keeps original faces and current errata separate. The preceding admission added Clocktower Guardian, with private Predict3 and an optional play of its banished top card. Its [source and rule record](assets/data/card_admissions/c18/README.md) separates the corrected Mind/Shurima facts from the preserved raw gallery values. The preceding batch added Burn Away, Portal Rescue, Temporal Breach, Thrill of the Hunt and Arcane Shift. Their [admission evidence](assets/data/card_admissions/c17/README.md) preserves the complete official faces, existing Burn Away corrections and Portal Rescue’s separate owner erratum. Linked replays retain the physical card, prescribed destination and base-cost waiver, with fresh unit identities and ordinary extra costs. The [Rengar and Ezreal source review](assets/data/card_admissions/c16/README.md) adds their two legend identities after checking all five official printings. Patched Porobot remains unavailable until its combined Unit/Gear type is fully supported. The [independent rules audit](docs/RULES_AUDIT.md) records reproduced fixes and remaining interaction work.

## Automated opponents

Run `cargo run --locked --bin rift-api` or `nix run .#api` for a local JSONL process: send one JSON request per line on standard input and read one response per line from standard output. No server or API key is required. The Rust facade is `local_rift::api::Session`.

The API supports validated custom decks, seeded setup, both players' mulligans, private player observations, action validation, explicit targets and optional costs, exact power payments and quotes, group movement, combat assignments, and stepping the built-in AI. The catalog describes whether a card play selects units, gear, a spell, or a battlefield; field targets use the same validated action and payment paths as other targets. Every mutation checks a revision and commits only after validation. Suggested actions are a convenience subset; custom bots can submit structured actions directly. See [the protocol and example bot](docs/API.md).

This process is a trusted match coordinator that can request either player's view. Keep opponent bots separated from that coordinator if hidden-information isolation matters.

## Development and verification

The implementation uses **Rust + Macroquad**, with a pure Rust rules layer separated from rendering.

| File | Responsibility |
|---|---|
| `src/main.rs` | Screens, input, selection, match flow, and preferences |
| `src/game.rs` | Deterministic game state, legal actions, priority, effect execution, and AI |
| `src/combat.rs` / `src/combat_ui.rs` | Resumable combat resolution, captured results, closed focus and visible progress |
| `src/deck.rs` / `src/deck_editor.rs` | Constructed-deck validation, native editor, and separate saved lists |
| `src/cards.rs` | Card facts, official text, and preset compositions |
| `src/battlefields.rs` | Eleven official battlefield choices, triggers, and cost or damage modifiers |
| `src/legends.rs` | Ten legend powers, optional trigger costs, and combat bonuses |
| `src/hidden.rs` | Facedown slots, Hidden timing, target restrictions, and visibility |
| `src/gear.rs` | Persistent gear, attachment abilities, resource generation, and death prevention |
| `src/deploy.rs` / `src/deploy_ui.rs` | Battlefield gear locations, Deploy rules, paid triggers and native controls |
| `src/activated.rs` / `src/ability_ui.rs` | Printed unit and gear abilities, targets, costs, and desktop controls |
| `src/payment.rs` / `src/payment_plan.rs` | Exact source choices, fixed payment obligations, pending cards, and rollback |
| `src/play.rs` / `src/play_ui.rs` | Delayed play grants, independent Repeat choices, typed cost plans, and desktop controls |
| `src/damage.rs` | Spell, ability, and unit damage provenance with live and captured bonuses |
| `src/resources.rs` | Resource-ability search for mandatory payments, with executable action sequences |
| `src/deck_plays.rs` / `src/deck_look_ui.rs` | Private deck selections, public reveals, linked Banish plays and Empower, and their desktop controls |
| `src/deck_draws.rs` / `src/draw_look_ui.rs` | Exact inspected draws, Move reveals, private occurrence choices and public inspection controls |
| `src/rules.rs` | Declarative official effect and keyword reference; metadata alone does not imply engine support |
| `src/companion.rs` | Mote's advice and procedural character drawing |
| `src/scene.rs` | Tabletop backdrop, unit positions, and picking regions |
| `src/ui.rs` | Typography, controls, and card presentation |
| `src/smoke.rs` | Native UI interaction checks |
| `src/bin/simulate.rs` | Deterministic all-matchup simulation checks |
| `src/api.rs` / `src/bin/api.rs` | Player views, validated Rust interface, and local JSONL bot protocol |
| `tests/conformance.rs` | Independent examples from official rules and errata, plus catalog consistency |
| `build.rs` | Embeds local card art into the executable |
| `assets/data/decks.json` | Data provenance; not a runtime import format |

Run checks without a graphics window:

```sh
cargo test --locked
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo run --locked --bin simulate
```

The simulator runs **1,536 matches** by default: all 16 ordered deck pairings, all three AI levels, and 32 seeds. Starting players alternate across seeds, so each pairing and AI level includes 16 games from each starting position. It checks card/rune conservation, energy accounting, unit validity, and winner consistency after every action, and fails on an illegal action or stalled game. Example options:

```sh
cargo run --bin simulate -- --seeds 128 --seed 1 --max-steps 5000
cargo run --bin simulate -- --difficulty strategic --seeds 8
```

Run the native UI smoke check on a graphical desktop, isolating its test preferences:

```sh
XDG_DATA_HOME=/tmp/local-rift-ui-smoke-data ./run.sh --ui-smoke
```

The interaction script exercises match setup, deck editing, mulligans, deployment, drag/drop, grouped movement, responses, combat assignment, restart and victory replay. It checks illustrated cards and battlefields, hover inspectors, equipment, rune controls, both players’ public zones, companion settings and side victory tracks. Rules-specific controls cover optional costs, exact resource payments and rollback, Repeat targets, trash plays, ordered choices, numeric limits, triggered abilities and token placement. The latest checks cover battlefield gear rows and exact-copy inspectors, Deploy placement, both Command modes, private Gear choices, funded optional-trigger Undo and decline, and Beginner/Advanced companion advice. They retain pending combat results, Deathknell choices before control and scoring, retained combat designations during Conquer responses and closed focus. They retain visible Vision counts and granting sources, separate stacked Predict choices, actual Mech tokens and retained abilities after the grant source leaves. They retain four- and five-card Predict ordering, Ornn’s private Gear choices on Play and Hold, Bloom’s paid costs and deferred Draw/XP, empty-Predict continuation, and private opponent choices. Resetting a draft resets its choices without implying a refund of already paid costs. Earlier checks retain private inspected draws, exact duplicate occurrences, printed eligibility limits, explicit recycling, public Draw/Recycle/Trash outcomes, fresh Repeat choices and live unit inspection after an immediate right-click. Printed copies in a hand, deck, trash, or reveal clear the prior board-unit context; board and combat trays inspect the exact clicked unit. They retain private five-card choices, reduced child costs, linked Unit/Gear Empower, and public opponent reveals that wait for inspection. They retain Clocktower Guardian’s Predict3 ordering, optional Banish offer, and unit/spell finalization. Earlier checks cover Banish targets, prescribed replay destinations, fresh unit entry, Rengar after replay, Arcane Shift’s parent completion, and unavailable destination controls. They also retain the paginated legend picker, Rengar’s required target selection, and Ezreal’s visible prerequisite and Reaction draw. They retain the prior death-location, mixed unit/gear, Banish-browser, equipment, rune and side-score controls. The `--small` option exercises scaled coordinates. Apply the graphics-library workaround above if this host requires it.

C24 checkpoint, verified on October 26, 2026: **1,297 automated Rust tests and 211 importer/review tests pass.** The card-family matrices complete **7,656 games with per-action conservation checks**, including 240 new exact-40-card Deploy matchups across ten legends, both package seats, both starters and all three AI levels. Every new card and both Command modes are played from both seats. A further **384 preset matches** cover all 16 ordered preset pairings, all three AI levels and both starters. Formatting and Clippy with warnings denied pass. Native and Nix run the same test definitions; their counts are not added together. The first package attempt caught three outdated aggregate catalog-count assertions in two historical tests. The corrected revision changes only those expected counts; the failed attempt is retained, and the full suite is rerun.

This admission adds Banner of Conquest, Blossoming Dreamroot, Concert Poster, Hunting Blind, Pillaged Armory, Portable Portal, Resource Extractor, The Good Stuff, Overloaded Traveler and Command: Attack. Deploy gear occupies its own battlefield location, has visible art and a Loadout entry, and retains its printed opponent-Hold trigger. It does not control or contest a field. Attached gear follows its bearer; on detachment, ordinary gear stays at the departure location until cleanup recalls it, while Deploy provides its exception. Optional trigger costs and pinned targets are handled before the ability becomes respondable. The API exposes the same location, mode and payment choices. See [the rules audit](docs/RULES_AUDIT.md).

Each final API build completes **264 full games plus 32 retained opening baselines**. The 120 new Deploy matrix games and 12 new targeted scenarios use validated legal 40-card decks. The retained 120-game matrix uses 40–55-card practice decks, and the retained 12 combat scenarios use 46/43 cards; those establish public-action flow rather than Standard-legal constructed setup. The new checks observe all ten cards, both Command modes, actual Recruit and Gold tokens, exact trigger targets, payment and refusal, private draws, and invalid-command rollback. A first external-harness attempt incorrectly expected every Recruit to enter exhausted; an exact replay showed paid Accelerate from Firelights’ Hideout. The corrected checks explicitly select, validate and quote token finalization, then check its entry state against that intent. The failed attempt is retained; the final full runs use the corrected harness.

The separate C23 comparison completes **40 Easy/Tactical matches**: 39 match the asserted old schema, and one retains an independently reviewed timing difference. Cloth Armor detached during Baited Hook’s resolving effect stays at the battlefield while its private choice is pending, then returns to Base at the next cleanup. All 258 requests and 1,032 before/after viewer observations in that divergent game were reviewed; only that pending-resolution location differs. Both branches finish with the same winner, 8–7 scores, turn and action count. The raw compatibility flag remains false, and the difference is preserved. Only exact validated additive fields and the domain-eligible new-card catalog suffix are projected out; existing locations, actions, timing, resources and outcomes remain compared. The prior 401 card entries, ten legends, four presets and training recipes remain unchanged.

The source cohort now contains **137 field resolutions across 87 printings and 52 historical issue groups**. Ten original complete faces support this admission. The inventory still includes 1,408 printings and 1,079 legal identities; 438 identities are mapped and 641 remain unavailable. Source corrections alone do not make a card playable. The October 25 live gallery, Rules Hub and errata comparison, and the archived October 9 Core PDF comparison, remain recorded in the source audit.

The native release passes **1,132 interaction steps at four window sizes** (1440×900, 1152×720, 1280×800 and 1600×1000), with 246 captures at each size. The Nix package passes the same script at the first two sizes. Visual review covers battlefield gear, physical-copy inspection, full card text, Command choices, optional payments, private information, matching rune areas, equipment and side victory tracks. The complete flake check passes, including all 1,297 Rust tests, 211 catalog tests, the offline catalog check and its separate 384-game preset simulation. Native/Nix API outcomes and all saved Deploy/combat trace hashes agree, including the retained raw comparison difference. The main checkout’s release executables match the independently checked frozen draft. Desktop acceptance uses isolated profiles and software-rendered displays; only x86-64 Linux has been exercised.

Known remaining issues include a reproduced general payment restriction: some fully funded positive-resource card or ability costs cannot currently be refused after entering their payment plans, although Core 445.2 permits this. A separate correction is in progress. The inspector also still labels non-Unit cards with zero Might; a presentation fix is queued. Same-location gear-Move rejection is a documented narrow interpretation of the Move framework, not a located gear-specific ruling. Strategic AI search can also be slow in resource-heavy positions; one retained scenario exceeded a 60-second API deadline during concurrent builds in both compared versions and remains under investigation. Complete Standard coverage remains in progress.

These tests verify implemented examples and application consistency; they do not certify full Standard compliance. Lightning Rush now notifies ordinary Draw observers before trashing its inspected remainder. Draw history now uses checked 64-bit counts. A seeded boundary audit reproduced the old 16-bit panic and release wrapping at 65,535, including a false Frigid Jewel second-Draw trigger; ordinary-match reachability of that boundary was not established. The correction preserves exact ordinary and specified-card Draw counts, exposes a decimal-string API companion, and rolls back the whole attempted command at the finite host limit. XP, dynamic Might and damage cross their previous numeric caps without truncation. The bot API supplies exact decimal strings for those fields, and the native combat editor accepts exact allocations. Signed Might retains negative values for calculations while references floor them at zero. Exceeding their documented 64-bit representation returns a distinct capacity error without changing the attempted action, including associated payment, card movement and RNG changes. These are finite host limits, not game-rule maxima or arbitrary-precision support.

## Artwork and attribution

The bundled card thumbnails come from Riot's official card gallery. Riftbound, its characters, card names, and artwork are Riot Games property. This personal offline prototype is unaffiliated with Riot Games. The tabletop scenery and Mote are drawn procedurally. The two battlefield panels show the selected official fields and their full effect text. See [SOURCES.md](docs/SOURCES.md) for attribution and exact source URLs.
