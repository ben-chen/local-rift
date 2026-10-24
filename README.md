# Local Rift · The Practice Grounds

A native Rust client for practicing Riftbound against three levels of AI. It includes a clear playmat over a quiet 3D backdrop, with art-faced unit cards, matching rune reserves and side victory-point tracks, four tournament-derived presets, eight playable legends, a custom deck editor, full card previews, and Mote, a small companion with Beginner and Advanced advice. A local Rust and JSON API supports custom automated opponents.

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

1. Choose a deck for yourself and another for the AI in **Practice** or **Deck library**. All four presets can be used on either side. The legend picker can create editable training lists for eight legends, and the deck editor saves a custom list for you and one for the AI.
2. Choose the battlefields, who starts, and **Relaxed**, **Tactical**, or **Strategic** difficulty, then **Start practice**.
3. Select up to two opening cards to replace, then begin.
4. Deploy units at base or a controlled battlefield. Move ready units, individually or as a group, to contest points.
5. During a showdown or spell chain, use your legal responses or **Pass priority**. In an open turn, the same control becomes **End turn**.
6. Hover over a card or unit to read its full text; right-click to pin the inspector. Mote can explain a current decision or offer another thought.

Relaxed uses straightforward plays. Tactical weighs combat and resources. Strategic compares bounded sequences of possible plays and their public board outcomes. It samples unknown draws from cards legal for the public legend identity; it cannot read the opponent’s hidden hand, deck list, facedown identities, or future deck order. Its short search assumes no further discretionary responses while resolving a candidate, so it remains a practice opponent rather than a tournament-strength benchmark.

Both players use the same rune layout: upright runes are ready, sideways runes are exhausted, and the rune deck shows how many remain to be channeled. Use your rune controls to exhaust or recycle a chosen domain before a play, or let payment activate runes automatically. Enable **Power: choose sources** to choose exactly which generated power and runes pay for each action. Generated energy, colored power, flexible power, and spell-only power are shown separately. Battlefield effects remain visible on the mat, with a **Rules** button for the full explanation. A gold strip beneath each equipped unit shows its gear. Click a gear name to inspect it, or the **N gear** count to open all attachments. Sparse unit rows expand to make names and artwork easier to read. Board thumbnails crop the artwork proportionally instead of stretching it; hover for the full card.

Victory points climb the board’s outer edges: your teal track is on the left, and the opponent’s gold track is on the right. Filled steps show points earned; the diamond marks the current total. Both tracks share the same scale from zero to eight. The decorative scenery is clipped to the board so it stays clear of the companion and controls.

Abilities that recycle trash show a separate cost tray, so those cards are not confused with effect targets. Resource conversions have an amount control and can use another resource ability before finishing. **Generate resources** keeps the chosen play and its calculated cost fixed while you pay. A card being played appears as pending and leaves its original zone. If the attempt can no longer finish, rollback restores the position before it began, including the original mandatory choice.

Gear with **Empower** has a separate ability button and a visible Empowered state. Its second ability may spend that state as a Disempower cost. Costs and use restrictions appear before confirmation; ordinary Empower takes effect after responses. The same controls handle gear with multiple printed abilities.

Empowered units have a visible status marker. Their inspector shows current Might and keywords, plus active and inactive conditional effects with their sources. Ability controls show the current payable cost, including rune discounts. Apprentice Mage's Predict2 lets you keep or recycle each card and order the kept cards before confirming. Ambessa is available as an eighth legend with an editable practice list.

XP costs are shown separately from Energy and Power. Crowd Favorite and Enthralling Protector spend2XP before their self-Buff can be responded to. Honeyfruit shows both of its resource abilities at Level6, including the exact Energy/Any Power output. Shepherd's Heirloom gains1XP when played and spends1XP to Equip; its gold attachment strip then shows the bearer’s gear as usual. Fixed payments preserve the declared XP cost, and failed-attempt rollback restores XP and generated resources together.

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

The three Los Angeles lists are recent competitive examples as of **October 24, 2026**. Kai'Sa is an older competitive archetype included for variety. These are dated source lists, not an assertion about the newly released Radiance metagame. The full 40-card main lists are preserved. Their 69 distinct cards and 303 additional cards for custom decks have executable printed-effect mappings. All eight registered legend powers, persistent equipment, Hidden, Flow, Repeat, Accelerate, and Empower are implemented. The additional unit and gear activations include explicit targets, trash-recycling, sacrifice and Disempower costs, response windows, and resource conversions during permitted payments. The engine exposes trigger order, on-play targets, death replacements, and rune activation as choices. Coverage and remaining interface limits are described in [RULES.md](docs/RULES.md); this is still an unofficial implementation under development.

Counts, card IDs, original rune splits, and source URLs are recorded in [`assets/data/decks.json`](assets/data/decks.json). The [official tournament sources](docs/SOURCES.md) explain provenance and historical legality.

The legend picker adds **Darius / Hand of Noxus**, **Viktor / Herald of the Arcane**, **Lux / Lady of Luminosity, Starter**, and **Ambessa / Matriarch of War** alongside the original four. It shows each legend’s official portrait and full rules. Create an editable 40-card training list, adjust it in the editor, and save it for either seat. These four lists are authored practice starting points, not sourced meta decks. Darius practices Legion sequencing, Viktor develops Recruit boards, Lux rewards spells with a printed Energy cost of five or more, and Ambessa practices Empowering objects and readying units.

Legend identity is independent of preset choice and chosen-champion variant. Existing saved decks keep their numeric identities; invalid saved files now produce a visible load message without being overwritten.

The [full card inventory](docs/CARD_COVERAGE.md) accounts for 1,079 legal Standard identities as of October 24, 2026. Current support maps 397 identities, including 372 main-deck cards, legends, battlefields, and runes; 682 remain unavailable. The reproducible importer preserves official printings, bans, and errata without registering unsupported cards as playable. Its [source review](docs/CATALOG_REVIEW.md) records 117 field corrections or confirmations, backed by archived official artwork and rules. All 37 original issue groups are retained alongside five later artwork-confirmed discrepancies. Original source values remain available alongside the reviewed facts. Complete [alternate-printing reviews](docs/VARIANT_REVIEW.md) now cover 29 playable identities and all 90 of their gallery printings, replacing the previous compatibility exceptions. Kennen’s missing Ionia tag is corrected in gameplay and the bot API. Another 95 identities have alternate-printing differences still awaiting review; those identities remain unavailable. Unresolved gameplay facts or variant differences prevent admission. Source corrections alone do not register playable cards. The latest gameplay batch adds ten separately reviewed cards for gear destruction, Conquer triggers, readying and Mech creation. Its [admission evidence](assets/data/card_admissions/c13/README.md) preserves the official faces and exact source facts. Patched Porobot remains unavailable until its combined Unit/Gear type is fully supported. The [independent rules audit](docs/RULES_AUDIT.md) records reproduced fixes and remaining interaction work.

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
| `src/deck.rs` / `src/deck_editor.rs` | Constructed-deck validation, native editor, and separate saved lists |
| `src/cards.rs` | Card facts, official text, and preset compositions |
| `src/battlefields.rs` | Eleven official battlefield choices, triggers, and cost or damage modifiers |
| `src/legends.rs` | Eight legend powers, optional trigger costs, and combat bonuses |
| `src/hidden.rs` | Facedown slots, Hidden timing, target restrictions, and visibility |
| `src/gear.rs` | Persistent gear, attachment abilities, resource generation, and death prevention |
| `src/activated.rs` / `src/ability_ui.rs` | Printed unit and gear abilities, targets, costs, and desktop controls |
| `src/payment.rs` / `src/payment_plan.rs` | Exact source choices, fixed payment obligations, pending cards, and rollback |
| `src/play.rs` / `src/play_ui.rs` | Delayed play grants, independent Repeat choices, typed cost plans, and desktop controls |
| `src/damage.rs` | Spell, ability, and unit damage provenance with live and captured bonuses |
| `src/resources.rs` | Resource-ability search for mandatory payments, with executable action sequences |
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

It clicks through setup, mulligan, deployment, board selection, group movement, priority, the inspector, advice/settings, restarting with a different deck, legend and gear resource abilities, the trash browser, empty-card choices, victory replay, on-play targets, separate Repeat targets, manual combat target order, journal navigation, and the second-player opening. It also exercises the saved deck editor, invalid-save preservation, custom lists for both seats, new-card deployment, printed/granted Flow selection, staged battlefield order, drag/drop and cancellation, multi-target staging, long-text scrolling, rune activation, attached gear previews, battlefield inspection, and opponent-trash browsing. The expanded checks include friendly and unrestricted two-unit targeting, four incremental Recruit placements after the parent spell finishes, discard/draw choices, Accelerate and revival payments, live keyword previews, target limits, and Legion triggers. The latest checks also cover three saved AI levels, Shield and equipped gear, the new Repeat cards, Energy Conduit and Gold, and targeted/pending trash plays. The unit/gear ability checks cover explicit targets, both Renata choices, cancellation before spending, Dragonsoul during an actual pending payment, opponent inspection, and choosing Caitlyn’s first-or-last combat priority while equipped. Resource-payment checks cover separate trash costs, another-gear targeting, nested conversions, fixed payment obligations, exact source selection, and failed-attempt rollback. The eight-card Empower expansion adds checks for visible source state, multiple abilities, targetless Empower, Disempower costs, cancellation, reaction resource generation, and separate Bird placements. The latest checks cover the illustrated battlefield picker, visible turn effects, independent Repeat executions and targets, staged Energy/Power discounts, exact Power-reduction allocation, and pending token Accelerate. Further checks cover Empowered unit status and live passive effects, rune-dependent ability costs, ordered Predict choices, and Ambessa’s target and payment controls. The latest battlefield-targeting checks cover empty-field selection, invalid drops, separate fields for repeated executions, exact payment and pending-spell display, and live local/alone effects. Further checks exercise two attached Machetes, captured Hunt, Soul Sword’s XP bonus, rune recycling across a Might threshold, ready and exhausted token entry, printed and granted Temporary expiration after Equip, preserved gear-target drafts in Loadout, distinguishable death-replacement choices, partial Channel, and actual Buff/Ganking updates. XP checks cover exact values above 255, both players’ displays, conditional entry, positive-gain history and its reset, intrinsic/Level/history Ganking movement, Concentrate’s discount and cancellation, Wuju’s draw, and Combat Experience’s target. XP-cost checks cover both self-Buff abilities, visible costs and cancellation, losing Level before responses, Honeyfruit’s distinct outputs and shared Exhaust cost, and Shepherd’s Heirloom’s play trigger and XP Equip cost. Numeric checks cover exact decimal damage entry, rejected over-allocation without mutation, an alternate allocation above 2^53, complete Might/damage hover values, signed negative Might, explicit capacity errors, and a full Might-derived discount. Temporary/Sprite checks cover mixed unit/gear targets across Loadout, ordered Hidden slots and editing, distinct pending children, live Petal Might, physical expiry sources, and new Sprites surviving the same Beginning event. Gear-trigger checks cover attached targets, choices retained across both Loadouts, printed Energy limits, optional decline and companion guidance. Victory-track captures cover both seats at zero, midgame, last-point pressure and eight points. The `--small` option exercises scaled coordinates. Apply the graphics-library workaround above if this host requires it.

Verified on October 24, 2026: **967 automated Rust tests, 129 importer/review tests, and 3,792 custom-deck games with per-action conservation checks.** A further **384 preset matches** cover all 16 ordered preset pairings, all three AI levels, and both starting positions. Formatting and Clippy with warnings denied pass.

The native release API completes **48 expanded-deck matches** with all ten gear-interaction additions, covering every supported legend at each difficulty from both starting positions. These games check 13,596 player observations, 169 explicitly submitted new-card plays and target queries, and 322 pending-token views. Every new card is played. All 32 original opening-state baselines remain preserved. A separate comparison against the preceding Temporary/Sprite release covers **32 Easy/Tactical matches and 17,929 paired responses**, normalizing only the additive `target_rule` field. Strategic choices are not expected to remain identical because its hypothetical hidden cards come from the expanded public eligible-card pool; its complete matches and privacy tests are checked separately. The prior 362 catalog entries, IDs, presets and training recipes are preserved; eligibility gains only the new cards.

The catalog review checks all 1,408 original printing records, preserved issue history, complete variant membership, raw/effective and evidence hashes, stale corrections, current Core links and applicable errata, alternate-printing admission, and refresh failures. The ten new admissions pin every selected printing's facts and official face evidence. Goatherd's prior tag correction is reused; the source-only and Temporary/Sprite checkpoints remain preserved as historical evidence. The latest live source comparison passes.

The native release and Nix package both pass **800-step interaction checks at 1440×900 and 1152×720**, with 107 captures at each size per build. Selected victory-track, gear-target, equipment, rune and battlefield screens were visually reviewed, including the scenery clipping fix. The Nix package passes all 967 Rust tests, 129 Python tests, the offline catalog check, and a 384-game simulation check. Its API reproduces the 48 expanded-deck game records, 32 Easy/Tactical comparison games and 32 original opening baselines above. Final packaged desktop checks use fresh profiles on an isolated software-rendered display, without the host graphics-library workaround. Only x86-64 Linux has been exercised here.

These tests verify implemented behavior and application consistency; they do not certify official rules compliance. XP, dynamic Might and damage now cross their previous numeric caps without truncation. The bot API supplies exact decimal strings for clients with limited integer precision, and the native combat editor accepts exact allocations. Signed Might retains negative values for calculations while references floor them at zero. Exceeding the documented 64-bit representation returns a distinct capacity error without changing the attempted action, including pending choices, payment, draws and RNG. These are finite host limits, not game-rule maxima or arbitrary-precision support.

## Artwork and attribution

The bundled card thumbnails come from Riot's official card gallery. Riftbound, its characters, card names, and artwork are Riot Games property. This personal offline prototype is unaffiliated with Riot Games. The tabletop scenery and Mote are drawn procedurally. The two battlefield panels show the selected official fields and their full effect text. See [SOURCES.md](docs/SOURCES.md) for attribution and exact source URLs.
