# Local Rift · The Practice Grounds

A native Rust client for practicing Riftbound against three levels of AI. It includes a clear playmat over a quiet 3D backdrop, with art-faced unit cards, matching rune reserves and side victory-point tracks, seven tournament-derived presets, eleven playable legends, a custom deck editor, full card previews, and Mote, a small companion with Beginner and Advanced advice. A local Rust and JSON API supports custom automated opponents.

![The Practice Grounds with card art and matching rune reserves](docs/preview.png)

**This is an unofficial rules implementation under development, not yet identical to standard Riftbound.** The client models turns, resources, movement, scoring, showdowns, priority, and printed card, legend, equipment, and selected battlefield effects. Hover previews and inspectors show complete official text, followed by specific practice notes where needed. See [current practice scope](docs/STATUS.md) for a concise feature and acceptance summary. Coverage and remaining choice differences are recorded in [Implemented rules and limitations](docs/RULES.md); [Sources](docs/SOURCES.md) records the official references and errata. Eighteen battlefield choices are available, and simulated match results do not establish real deck strength.

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
nix develop                   # Rust, Cargo, nextest, rustfmt, Clippy, rust-analyzer, native libraries
```

Inside the development shell, use `cargo run --release` and `cargo test --locked` normally. The Linux package and shell supply the X11, input, keyboard, and OpenGL libraries that Miniquad loads at runtime. The packaged game does not depend on the host-library workaround in `run.sh`.

```sh
nix flake check               # Build, test, and simulate every preset pairing
nix run .#simulate -- --seeds 128
nix run .#api                  # Local JSONL interface for bots
nix fmt                       # Format flake.nix
```

Outputs are provided for x86-64 and ARM64 Linux, plus Apple Silicon macOS. Linux uses X11 or XWayland; launching the game still needs a graphical session. On non-NixOS Linux, matching the host GPU drivers may require [nixGL](https://github.com/nix-community/nixGL). macOS uses the system graphics frameworks. Only x86-64 Linux has been tested here.

`flake.lock` pins the toolchain and system dependencies; update them intentionally with `nix flake update`. Git-based flakes include tracked files, so remember to add new source files and assets to Git when extending the game. Build outputs such as `target/` and `result` are excluded from the package source.

## First match

1. Choose a deck for yourself and another for the AI in **Practice** or **Deck library**. All seven presets can be used on either side. The legend picker can create editable training lists for eleven legends, and the deck editor saves a custom list for you and one for the AI.
2. Choose the battlefields, who starts, and **Relaxed**, **Tactical**, or **Strategic** difficulty, then **Start practice**.
3. Select up to two opening cards to replace, then begin.
4. Play units at base or a controlled battlefield. Move ready units, individually or as a group, to contest points.
5. During a showdown or spell chain, use your legal responses or **Pass priority**. In an open turn, the same control becomes **End turn**.
6. Hover over a card or unit to read its full text; right-click to pin the inspector. Mote can explain a current decision or offer another thought.

**Practice format** opens the date and deck checks for either single games or best-of-three matches. New installations use **Standard · today**, resolved from your computer's local date when the match starts. The accepted date stays fixed through sideboarding and every game of that match. Setup shows the last source-review date; this is an offline rules bundle, not a live metagame service. Its scope is global Standard 1v1, not regional release schedules.

Choose **Historical snapshot** to practice the original dated tournament lists under the bundled October 24 catalog, or use the announced-date button to preview scheduled restrictions. The panel checks both players, names banned cards and battlefields, and warns about upcoming restrictions. Best-of-three checks include the complete sideboard and all registered fields. Rejected starts preserve an existing match; no cards are silently substituted. The format remains visible during play and between games. For repeatable launches, `--format today`, `--format snapshot`, or `--format YYYY-MM-DD` selects the format at launch. Existing native smoke scenarios explicitly use historical practice; `--ui-smoke-format` exercises the date controls and rejection paths.

Relaxed uses straightforward plays. Tactical weighs combat and resources. Strategic compares bounded sequences of possible plays and their public board outcomes, including alternative combat damage assignments that respect Tank and Backline. Its combat forecasts also consider which units the opponent can defeat. It samples unknown draws from cards legal for the public legend identity; it cannot read the opponent’s hidden hand, deck list, facedown identities, or future deck order. Its short search assumes no further discretionary responses while resolving a candidate, so it remains a practice opponent rather than a tournament-strength benchmark.

The opponent thinks in the background. You can inspect cards, open Learn to play or Settings, and choose a new match while a calculation is running. A completed move waits while an inspection or menu pauses play. Starting a new match discards any result from the old one. If a calculation fails, the match stays unchanged and an explicit **Retry opponent** control lets you try again.

Both players use the same rune layout: upright runes are ready, sideways runes are exhausted, and the rune deck shows how many remain to be channeled. Use your rune controls to exhaust or recycle a chosen domain before a play, or let payment activate runes automatically. Enable **Power: choose sources** to choose exactly which generated power and runes pay for each action. Generated energy, colored power, flexible power, and spell-only power are shown separately. Battlefield effects remain visible on the mat, with a **Rules** button for the full explanation. A gold strip beneath each equipped unit shows its gear. Click a gear name to inspect it, or the **N gear** count to open all attachments. Sparse unit rows expand to make names and artwork easier to read. Board thumbnails crop the artwork proportionally instead of stretching it; hover for the full card.

Sand Soldier, Sprite, Bird, Tentacle, Mech, Recruit and Gold tokens have their own
official artwork on the board and in previews. Right-click a token to keep its
details open: current Might, damage, keywords, attached equipment and its own
rules. Dragging a unit token keeps that token's artwork. A token that leaves the
board is identified as departed in its inspector.

Deploy gear has its own illustrated row at each battlefield. Click the field’s gear count to open those exact copies in Loadout; attached equipment remains visible beneath its bearer. Gear can occupy a battlefield without controlling or contesting it. Command: Attack offers separate Move and Gear-draw modes, and each Repeat keeps its own choices. Optional paid gear triggers show their source and targets before payment. **Undo payment** returns to that choice; **Decline** skips the optional trigger. Mote explains each step at your selected advice level.

A unit’s **Might** and **combat damage** are shown separately when they differ. Ezreal, Dashing keeps his Might for lethal damage and his printed ability while contributing zero combat damage. Orianna, Exquisite deals combat damage only while friendly gear shares her location; gear attached to another friendly unit there counts. Her inspector shows whether that requirement is met. Stun is shown as its own reason for contributing zero. Hover or pin the unit to read the complete explanation. Rengar, Pouncing’s destination controls retain an attacked battlefield while you remain its attacking player, even after your last unit leaves.

Victory points climb the board’s outer edges: your teal track is on the left, and the opponent’s gold track is on the right. Filled steps show points earned; the diamond marks the current total. Both tracks share the same scale from zero to eight. The decorative scenery is clipped to the board so it stays clear of the companion and controls.

Abilities that recycle trash show a separate cost tray, so those cards are not confused with effect targets. Resource conversions have an amount control and can use another resource ability before finishing. **Generate resources** keeps the chosen play and its calculated cost fixed while you pay. A card being played appears as pending and leaves its original zone. Before finalization, **Undo payment** restores the staged payment boundary even when a positive resource cost is affordable. **Decline** abandons that unfinished positive cost and restores resources generated for the same pending attempt. A zero-cost mandatory finalization cannot be refused through this control. Resource instructions already under way must finish before either option is available.

Gear with **Empower** has a separate ability button and a visible Empowered state. Its second ability may spend that state as a Disempower cost. Costs and use restrictions appear before confirmation; ordinary Empower takes effect after responses. The same controls handle gear with multiple printed abilities.

Empowered units have a visible status marker. Their inspector shows current Might and keywords, plus active and inactive conditional effects with their sources. Ability controls show the current payable cost, including rune discounts. Apprentice Mage's Predict2 lets you keep or recycle each card and order the kept cards before confirming. The legend picker includes editable training lists for Ambessa, Rengar and Ezreal. Use its page controls to browse all eleven legends. Rengar asks you to choose a unit after each unit play; Ezreal’s legend panel shows progress toward its two enemy choices and enables the draw ability when its prerequisite is met.

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

The [Arito validation](docs/validation/irelia_arito.json) covers the exact
Arito Convergence IV preset and Edge of Night: **1,598 Rust tests**, **269 source
checks**, **72 exact-list games**, **24 bot API games**, and **eight native UI
runs** passed. Sideboard and battlefield tabs expose the registered choices.
The native **Best of three** mode now uses that complete registration for match
practice, including legal swaps between games.

Registered best-of-three matches are available in the **native app, Rust engine
and bot API**, including legal sideboarding, private battlefield selection and
match scores. In Practice, choose **Best of three**, then select Xeno Irelia,
Arito Irelia, Master Yi, Kennen, Rengar, or Sonecon Riven for either seat and follow the setup steps. Mote explains match
rules or planning at your chosen advice level. See
[match practice](docs/MATCH_SERIES.md) for instructions and current limits.
The [match validation](docs/validation/match_series.json) passed **1,609 Rust
tests**, **269 source checks**, **36 engine matches (91 games)** and **12 release
bot API matches (29 games)**. The subsequent native interface is documented in
[its validation record](docs/validation/native_match_series.json): **1,612 Rust
tests**, **269 source checks**, **six controller matches (14 games)** and **20
release UI runs** passed. The match UI runs cover three AI levels and two window
sizes; twelve runs each play a complete match. Card coverage is unchanged.

Tactical and Strategic match opponents now remember revealed cards and consider
legal sideboard swaps while preserving Irelia's core engine. **Mote: review my
sideboard** offers an optional suggestion without moving your cards. The bot API
exposes the same planner and scouting history. The [match-AI validation
record](docs/validation/match_ai.json) includes a fresh 32-pair comparison: the
adaptive policy won 12 matches and the unchanged-deck baseline won 9. This limited
simulation does not establish tournament or expert strength. Card coverage and
the remaining rules limitations are unchanged.

Unyielding Spirit and Repulse are available in the card library and deck editor.
Unyielding Spirit prevents spell and ability damage for both players for the turn;
it leaves unit-dealt damage and independent instructions intact. Repulse first
chooses your battlefield unit, then an eligible enemy spell or ability. Its
choices account for every Repeat execution. Both cards include complete previews
and optional advice. See the [reaction-card validation](docs/validation/master_yi_reactions.json)
for the source, rules, game, UI and package checks. The complete Master Yi
registration is now available for best-of-three practice, with its exact ten-card
sideboard and three battlefields. Its main deck is unchanged. The
[registration validation](docs/validation/master_yi_registration.json) records
1,819 Rust tests, 48 native and packaged API matches (119 games each), and six
packaged interface runs. Every packaged API trace matches the native build.
Mote and the stronger AI levels consider sideboard options using revealed cards;
these heuristics do not establish expert-level strategy.

Alpha Strike, Brutalizer and Hand Hammer are available in the card library and deck editor.
Alpha Strike asks you to choose a friendly source and enemy battlefield units,
then divide damage when it resolves. Its XP rewards resolve separately.
Brutalizer's temporary attachment bonus and Hand Hammer's two-unit condition
appear in the unit inspector, alongside the visible equipment strip. These
three additions passed 1,815 Rust tests, 282 source checks, 1,176 preset simulations
and installed-client checks. All 432 installed API game traces match the native
build. See the [release validation](docs/validation/alpha_equipment.json) and
[Alpha Strike rules audit](docs/ALPHA_STRIKE_AUDIT.md).

Hidden Blade is available for custom practice, including Hidden play, death
replacement choices, complete card inspection and optional Mote advice. Invalid
targets stop its instructions; a valid target saved by a death replacement still
lets its controller draw two. Successful drag-and-drop now clears an earlier
invalid-drop message.

The engine and API also record physical damage sources and supported Kill
credits. Resource abilities reject an unrepresentable total before consuming
their source. These changes passed 1,842 Rust tests and 284 source checks;
84 packaged card-test games and 48 registered matches reproduce native traces.
See [the validation](docs/validation/hidden_blade.json) and the remaining
attribution scope in [the rules audit](docs/RULES_AUDIT.md).

Deathgrip is available in custom practice decks. Choose two different friendly
units before responses; an actual death transfers the first unit’s current Might
for the turn, and drawing a card is a separate instruction. Death replacements
finish before the draw. Beginner and Advanced Mote advice explain these choices.
Its [validation record](docs/validation/deathgrip.json) includes 1,852 Rust tests,
286 source checks, and matching native/packaged traces across 84 card-test games
and 48 registered matches.

Guards! is available for custom practice. It creates a Sand Soldier, then offers
a separate optional Order Power payment to ready that exact token. Opponents get
a response opportunity after payment. Its Hidden and Repeat behavior follows
the official erratum; Mote explains the choices at both advice levels. This adds
a card to the supported pool, without changing the seven tournament recipes.
The [combined validation record](docs/validation/guards_tokens.json) distinguishes
the accepted Guards rules from the final token-art package checks.

## Included presets

| Deck | Real archetype reference | Published source |
|---|---|---|
| Irelia / Blade Dance | Movement, choosing friendly units, and conquest | Xeno, Los Angeles runner-up, September 2026 |
| Master Yi / Wuju | Buffs and efficient combat | SWAGALISK, Los Angeles sixth place, September 2026 |
| Kennen / Tempest | Repeated actions, tokens, and resource loops | DSG AwesumRosum, Los Angeles thirteenth place, September 2026 |
| Kai'Sa / Void Control | Spell resources, removal, and finishers | Sinzari, Vancouver fifteenth place, May 2026; historical list |
| Rengar / Pridestalker | Temporary Might, unit sequencing, and movement | DSG Prismaticism, Los Angeles winner, September 2026 |
| Irelia / Arito Convergence IV | Orianna, equipment, ready effects, and movement | Arito, Convergence IV runner-up, October 31, 2026 |
| Riven / Sonecon Stuttgart | Ready entrants, equipment, movement, and Baron Pit | Sonecon, European Regional Championship runner-up, November 2026 |

The four Los Angeles lists were rechecked against Riot’s published source on **October 26, 2026**. Kai'Sa is an older competitive archetype included for variety. These are dated source lists. The Riven preset separately reproduces Sonecon’s exact Stuttgart main deck, sideboard, runes and three fields from Riot’s November 11 article, rechecked on November 12. Baron can add a third battlefield during play; the board, movement choices, scoring and bot API include it. The full 40-card main lists are preserved. Playable cards have executable effect mappings; the generated coverage report lists the current supported pool. Arito’s exact 40-card list was independently checked against its original organizer deck page on November 11, including two Edge of Night, three Orianna and one Gust Monk. All eleven registered legend powers, persistent equipment, Hidden, Flow, Repeat, Accelerate, and Empower are implemented. The additional unit and gear activations include explicit targets, trash-recycling, sacrifice and Disempower costs, response windows, and resource conversions during permitted payments. The engine exposes trigger order, on-play targets, death replacements, and rune activation as choices. Coverage and remaining interface limits are described in [RULES.md](docs/RULES.md); this is still an unofficial implementation under development.

Use **Prev** and **Next** in the deck library to browse all seven presets. Each preset can be selected for either seat or copied into the editor. The complete Xeno Irelia, Arito Irelia, Kennen, Rengar, and Sonecon Riven registrations have **Main deck**, **Sideboard**, and **Battlefields** tabs. Inspect all ten sideboard cards and all three battlefield rules; after selecting the preset for a seat, choose its battlefield there. Choose **Best of three** in Practice and **Next registered deck** for either seat to practice these complete lists with sideboarding. Other presets and saved custom lists remain available for single-game practice.

Counts, card IDs, original rune splits, and source URLs are recorded in [`assets/data/decks.json`](assets/data/decks.json). The [official tournament sources](docs/SOURCES.md) explain provenance and historical legality.

The legend picker adds **Darius / Hand of Noxus**, **Viktor / Herald of the Arcane**, **Lux / Lady of Luminosity, Starter**, and **Ambessa / Matriarch of War**, **Rengar / Pridestalker**, and **Ezreal / Prodigal Explorer** alongside the original four. It shows each legend’s official portrait and full rules. Create an editable 40-card training list, adjust it in the editor, and save it for either seat. These six lists are authored practice starting points, not sourced meta decks. Darius practices Legion sequencing, Viktor develops Recruit boards, Lux rewards spells with a printed Energy cost of five or more, Ambessa practices Empowering objects and readying units, Rengar turns unit plays into temporary Might, and Ezreal rewards choosing enemy units or gear with spells and unit abilities. Thrill of the Hunt and Arcane Shift are available. These authored training recipes stay separate from the tournament presets, including the sourced Rengar winner’s list.

Legend identity is independent of preset choice and chosen-champion variant. Existing saved decks keep their numeric identities; invalid saved files now produce a visible load message without being overwritten.

The generated [card coverage report](docs/CARD_COVERAGE.md) is the source for
current supported counts, missing identities and the catalog's review date.
It distinguishes imported official text from playable cards. The [source
review](docs/CATALOG_REVIEW.md) and [printing review](docs/VARIANT_REVIEW.md)
preserve official facts, errata, artwork-confirmed corrections and unresolved
differences. Unresolved gameplay facts or printing differences prevent admission;
a source correction alone never registers a playable card.

The [rules audit](docs/RULES_AUDIT.md) records reproduced fixes and remaining
interaction questions. The supported pool includes the cards in both Irelia
presets; their dated event registrations do not establish today's metagame
ranking. Full Standard coverage and exhaustive interaction fidelity remain in
progress. Historical card-admission evidence is retained under
[`assets/data/card_admissions`](assets/data/card_admissions), and release checks
are retained under [`docs/validation`](docs/validation).

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
| `src/battlefields.rs` | Eighteen official battlefield choices, triggers, and cost or damage modifiers |
| `src/legends.rs` | Legend powers, optional trigger costs, and combat bonuses |
| `src/hidden.rs` | Facedown slots, Hidden timing, target restrictions, and visibility |
| `src/gear.rs` | Persistent gear, attachment abilities, resource generation, and death prevention |
| `src/deploy.rs` / `src/deploy_ui.rs` | Battlefield gear locations, Deploy rules, paid triggers and native controls |
| `src/activated.rs` / `src/ability_ui.rs` | Printed unit and gear abilities, targets, costs, and desktop controls |
| `src/payment.rs` / `src/payment_plan.rs` | Exact source choices, fixed payment obligations, pending cards, and rollback |
| `src/play.rs` / `src/play_ui.rs` | Delayed play grants, independent Repeat choices, typed cost plans, and desktop controls |
| `src/damage.rs` | Spell, ability, and unit damage provenance with live and captured bonuses |
| `src/resources.rs` | Resource-ability search for mandatory payments, with executable action sequences |
| `src/ai_worker.rs` | Background opponent calculations and stale-result protection for the desktop |
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

For a quicker full Rust run, `nix develop` includes nextest:

```sh
cargo nextest run --locked --all-targets --profile ci
cargo test --locked --doc
```

The `ci` profile runs at most four tests concurrently, reports all failures, and
does not retry failed tests. Tests outside the complete-match binaries run first,
so focused rules failures surface before the long simulation matrices. This only
changes scheduling; no tests are omitted. It retains a JUnit report in
`target/nextest/ci/junit.xml`. This runs the full suite; the separate Cargo command
keeps documentation tests covered. During implementation, use a focused test
filter for the changed mechanic, then run full regression on the final candidate.

Keep each checkout's default `target/` directory, or give each worktree its own
`CARGO_TARGET_DIR`. Sharing project build outputs between old and new revisions
caused an instrumented baseline library to be linked into newer tests during a
local comparison. The package checks use separate immutable Nix builds.

When a seeded AI game exposes a rules bug, preserve the relevant legal actions
as a replay. A rule regression should keep testing that interaction when the AI
learns a different line; policy tests should assert the AI's decisions separately.

The Kha'Zix practice check uses an authored legal Irelia registration in both
seats, with both starting players and all three difficulty levels. It records
actual Attack/Defend triggers and XP resolution separately from completed games:

```sh
python3 tools/check_irelia_khazix_api.py target/release/rift-api /tmp/khazix-matches.json
./target/release/local-rift --ui-smoke-khazix --small
```

The native check exercises invalid and valid hand drops, unit dragging, full
inspection, both advice levels and the resolved combat bonus. Like the other
UI smoke modes, it uses controlled positions rather than replaying full games.

For Irelia development, one focused command runs the related rules, counter,
opponent, companion and registered-match tests. It retains the exact test list,
source identity, logs and decision measurements in a new output directory:

```sh
python3 tools/check_irelia_practice.py --output /tmp/irelia-check-1
# Also build the API and exercise complete dated matches at all three levels:
python3 tools/check_irelia_practice.py --output /tmp/irelia-check-2 --api
```

Jae Medarda's spell-choice coverage also includes a dated Standard match campaign:

```sh
python3 tools/check_spell_choice_api.py target/release/rift-api /tmp/jae-matches.json
python3 tools/check_spirit_wheel_api.py target/release/rift-api /tmp/wheel-matches.json
./target/release/local-rift --ui-smoke-irelia --small
```

The campaign uses an authored Irelia list at the pinned November 13, 2026 date,
with both seats and all three difficulty levels. It records actual draw triggers
and preserves zero-use cases. The native sequence plays and inspects Jae, switches
advice levels, chooses him with a spell, and passes on his separate draw ability.

Use a new directory for each run. The runner uses this checkout's `target/`
directory even when `CARGO_TARGET_DIR` is set elsewhere, and rejects a result if
sources change while it runs. It does not run the full regression suite or the
installed UI checks; those remain release gates.

The focused selection also includes global card classification, catalog mapping
and source-review checks. Run it before freezing a release candidate so an
inconsistent card admission fails before the expensive complete-game matrices.

The decision benchmark pairs explicit legal solutions with opponent execution
and Mote's Beginner/Advanced commentary. Its twelve authored positions run in
both seats at all three difficulty levels. `decisions.json` retains successes,
known misses, decision times and required regression gates; a passing check does
not mean every measured objective was achieved. It checks hidden-information
independence and read-only, brief advice. The scripted opponent passes except in
the separate response regression. These are development positions, not held-out
matchup evidence, complete games, human learning tests or a strength rating.
Timing is observational and has no machine-dependent pass threshold. The optional
API campaign separately measures complete-match and dated-format stability.

The same focused run writes `responses.json` for four additional Irelia positions
with an opponent that recasts returned Gust whenever legal. Each position has
an independently specified legal winning line, both seats and all three levels.
The report separates winning from winning without unnecessary spending, and
retains misses. Mote explains Gust’s resolution-time Might check, the counter’s
return-to-hand risk and when passing preserves resources. Additional regressions
cover replay departures and Syndra's Repeat with different targets. The
[response validation](docs/validation/irelia_responses.json) records the scoped
evidence. These development scenarios do not rate general opponent strength.

`python3 tools/check_release.py --plan plan.json --output /path/to/new-results`
builds and verifies one committed revision. The JSON plan has a full `revision`
SHA, `ui` cases (`name`, smoke-mode `args`, optional `captures` environment-variable
names), and `api` cases (`name`, a `check_*.py` script in `tools/`, a passing native
`baseline` JSON path, and its `records_key`). Series UI cases require
`minimum_games` of at least two. Optional UI `env` settings accept only `RIFT_`
variables; every run gets isolated preferences. Use `--xvfb /path/to/xvfb-run`
when it is not on PATH.

The runner builds the package, catalog and simulation checks from that immutable
revision, then uses the installed binaries. It preserves the plan and baselines,
checks actual UI completion and exact capture names, and compares full API records
including trace hashes. `status.json` records the live stage and final outcome.
Existing result directories are refused. Visual review and independently expected
rules scenarios remain necessary; matching builds do not establish rules parity.

GitHub checks cache Rust dependencies. Documentation-only changes validate changed
JSON receipts and whitespace without rebuilding the game. The generated
`docs/CARD_COVERAGE.md`, code, data, assets, tooling, unknown paths, and unavailable
history all require full checks. Superseded branch checks can be cancelled; main
checks run to completion. Manual workflow dispatch always runs the full checks.
`python3 tools/ci_scope.py --base <commit>` shows the decision locally.

Nix keeps Rust build inputs separate from Python catalog checks and documentation.
Both checks remain in `nix flake check`; changing a source report or test tool no
longer invalidates an otherwise unchanged Rust package.
The [workflow validation](docs/validation/validation_workflow.json) records the
local comparison, preserved checks, and the GitHub results for its recorded revisions.

The simulator runs every ordered pairing of the currently included presets, all three AI levels, and 32 seeds by default. Starting players alternate across seeds, so each pairing and AI level includes 16 games from each starting position. It checks card/rune conservation, energy accounting, unit validity, and winner consistency after every action, and fails on an illegal action or stalled game. Example options:

```sh
cargo run --bin simulate -- --seeds 128 --seed 1 --max-steps 5000
cargo run --bin simulate -- --difficulty strategic --seeds 8
```

Run the native UI smoke check on a graphical desktop, isolating its test preferences:

```sh
XDG_DATA_HOME=/tmp/local-rift-ui-smoke-data ./run.sh --ui-smoke
```

The core interaction check exercises setup, mulligans, drag/drop, grouped movement,
responses, exact payments, combat, inspection, equipment, runes, scoring and Mote.
Focused smoke modes cover specific cards and match workflows. Use `--small` to
check the scaled layout. Screenshot output is optional; the same render and
interaction checkpoints run with or without captures. Visual review complements
the assertions. Apply the graphics-library workaround above if this host needs it.

See [current practice scope](docs/STATUS.md) for supported workflows and remaining
limits. [Historical validation checkpoints](docs/VALIDATION_HISTORY.md) retain
older results; their test counts and feature limitations apply to those revisions.

## Artwork and attribution

The bundled card thumbnails come from Riot's official card gallery. Riftbound, its characters, card names, and artwork are Riot Games property. This personal offline prototype is unaffiliated with Riot Games. The tabletop scenery and Mote are drawn procedurally. The battlefield panels show the selected official fields and their full effect text, including Baron Pit when it joins the board. See [SOURCES.md](docs/SOURCES.md) for attribution and exact source URLs.
