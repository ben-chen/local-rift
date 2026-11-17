# Current practice scope

Local Rift is an offline Rust practice client with a shared rules engine for the
desktop app and bot API. It is under development; full Standard rules and card
coverage, expert opponent strength, and novice learning outcomes are not yet
established. This page describes capabilities, not a percentage of completion.

| Area | Available | Evidence and remaining work |
| --- | --- | --- |
| Match flow | Single games and untimed best-of-three, mulligans, priority and responses, combat, results, legal sideboarding, field selection and play order | [Registered match rules](RULES.md#registered-match-practice). Timed tournament rounds and multiplayer remain unfinished. |
| Irelia practice | Sourced Xeno and Arito registrations, registered opponents, sideboarding and relevant additional cards in the editor | [Priority and provenance](IRELIA_PRIORITY.md). Presets are dated tournament lists, not a live meta ranking. |
| Card coverage | Reviewed cards have explicit engine behavior, artwork and official text; unsupported identities cannot be played | [Generated inventory](CARD_COVERAGE.md) is the source for counts and its snapshot date. A mapping does not prove every interaction. |
| Rules fidelity | Shared legality and payment checks, response chains, exact damage allocation, trigger choices and action rollback | [Implemented rules](RULES.md) and [rules audit](RULES_AUDIT.md) record tested examples and open sequencing/interaction questions. Full parity remains unproven. |
| Format legality | Date-based Standard checks, historical practice, announced restrictions, and a fixed format throughout a match | [Format validation](validation/format_calendar.json). Sources are bundled offline and need intentional review when rules or legality change. |
| Opponents | Relaxed, Tactical and Strategic, background thinking and public-information decisions | [Decision benchmark](validation/irelia_practice.json) retains known misses. [Combat search validation](validation/strategic_combat.json) covers alternative damage assignments and Block tradeoffs. Authored positions and completed games do not establish tournament strength. |
| Mote | Optional Beginner and Advanced commentary, another-thought control, rules and strategy tips without taking actions | Companion regressions cover selected states, brevity and private-information independence. Human learning and matchup instruction still need evaluation. |
| Board clarity | Illustrated units and tokens, hover/pinned inspectors with scrolling, visible gear, matching rune reserves, battlefield rules and side victory tracks | Native interaction checks exercise both standard and small windows. Screenshots supplement behavioral assertions; learner playtesting remains necessary. |
| Bots | Local Rust interface and JSONL protocol using the same engine and player-visible observations | [API reference](API.md). Deterministic replay comparisons check compatibility, not independent rules correctness. |
| Packaging | Cargo launcher and a pinned Nix flake | Installed acceptance has exercised x86-64 Linux. Other declared platforms have not been verified here. |

## Reading validation results

[Validation records](validation/) identify the tested revision, checks, failed
attempts, limitations and remaining work. A completed result applies to that
revision and scope; it does not certify later untested edits. The
[Irelia practice record](validation/irelia_practice.json) contains installed
client/API acceptance, and [workflow evidence](validation/validation_workflow.json)
separately covers the focused runner and source checks. Historical checkpoint
notes live in [validation history](VALIDATION_HISTORY.md).

The [Disarm, Block and Yuumi release](validation/turn_keywords_release.json)
passed 2,012 Rust tests, 334 source checks, 1,176 simulated games, 23 installed
UI cases and ten exact API comparisons. Four of its 148 captures were visually
reviewed. This acceptance is separate from later Strategic AI changes.

During implementation, run focused rule, rollback and interface regressions for
the changed behavior. Before a commit, include affected interactions, formatting
and lint checks. Run the full regression, source, simulation and installed-client
gates against a fixed release candidate. Keep build outputs isolated per worktree.
Commands and evidence collection are documented in
[Development and verification](../README.md#development-and-verification).

The native client and bot API share binary-answer labels and availability,
including Irelia's distinct Power and Energy payments. Unavailable answers show
the engine's reason, while legal decline and resource actions remain available.
The [choice validation record](validation/binary_choices.json) separates local
checks from pending installed acceptance. This does not complete the broader
choice API or increase card coverage.

The next acceptance priorities are complete Irelia match practice with active
opposing responses, independently expected rules scenarios, opponent decisions
outside the development benchmark, and beginner learning sessions. Expanding to
all legal cards remains part of the goal; completing these priorities does not
replace that scope.

The [Strategic combat improvement](validation/strategic_combat.json) passed its
separate installed acceptance: 2,018 Rust tests, 334 source checks, 1,176 simulated
games, 23 UI cases and ten complete API comparisons (63 records). Its scoped
Block decision benchmark improves to 26/26; this does not establish expert strength.

Flurry of Feathers passed installed acceptance, with explicit modes shared with
Command: Attack, separate Bird placement choices, and concise advice at both
levels. A payment-undo regression found during that work also fixes response
priority restoration for other cards. The fixed candidate passed 2,032 Rust
tests, 336 source checks, 1,176 simulated games, 27 installed UI cases with
218 captures, and eleven complete API comparisons (69 records). Four installed
captures were visually reviewed. See [Flurry validation](validation/irelia_flurry.json).

Mageseeker Warden passed installed acceptance with live battlefield restrictions,
unit and gear inspection notes, public API source information, and both Mote
advice levels. The fixed candidate passed 2,045 Rust tests, 338 source checks,
1,176 simulated games, 29 installed UI cases (234 captures), and 12 API
campaigns with 75 full records matching the local baselines. Three installed
Warden captures were visually reviewed; see [Warden validation](validation/irelia_warden.json).

Decree of Discord passed installed acceptance with explicit remaining-target
choices when opposing responses break its collective Might limit. The fixed
candidate passed 2,055 Rust tests, 340 source checks, 1,176 simulated games,
31 installed UI cases (252 captures), and 13 API campaigns with 81 complete
records matching the local baselines. Four installed Decree captures were
visually reviewed; see [Decree validation](validation/irelia_discord.json).

Forgotten Monument, Back-Alley Bar and The Candlelit Sanctum passed installed
acceptance. The fixed candidate passed 2,070 Rust tests, 341 source checks,
1,176 simulated matches, 33 installed interface cases (272 captures), and
14 API campaigns with 99 full records matching local baselines. Four installed
captures were visually reviewed, including private-choice concealment and
small-window explanations. See [battlefield validation](validation/irelia_fields.json).

Crescent Guardian and Entrenched Boar have separate optional entry choices,
shared native/API behavior and both Mote advice levels. Local checks passed
1,145 library tests, 171 client tests, 14 catalog integration tests, 342 source
checks and 18 native captures at two sizes. Six authored series completed
13 games across both seats and all three AI levels. The 14 prior API campaigns
also passed, with nine changed Strategic records retained for review rather
than claimed identical. Installed acceptance of the fixed candidate passed
2,084 Rust tests, 342 source checks, 1,176 simulated matches, 35 interface
cases (290 captures and two actual games), and 15 API campaigns with 105
complete records matching fresh local baselines. Four installed captures were
visually reviewed; see [entry validation](validation/irelia_entry.json).


Possession and Conscription passed installed acceptance at acd3e67. Their
controls include Conscription's optional 5 XP payment, full cost totals, and
clear ownership and equipment information for taken units. The fixed candidate
passed 2,148 Rust tests, 343 source checks, 1,176 simulations, 37 native cases
(318 captures), and 16 API campaigns matching 111 full records. Three new
installed captures were visually reviewed. Control-specific coverage includes
24 authored engine games and six API series (17 games); the paid Conscription
path is covered by focused scenarios. See [validation](validation/control_cards.json).


Guardian of the Passage, Soulgorger, The Harrowing and Shadows of the Past
passed installed acceptance at 3c6861d. The same candidate preserves all
selected targets for automatic multi-target plays and fixes Strategic AI
valuing a speculative draw above an available winning Irelia readiness line.
It passed 2,206 Rust tests, 346 source checks, 1,176 simulations, 43 installed
interface cases (458 captures), and 19 API campaigns matching 153 complete
local records. Three installed captures were visually reviewed. The initial
failed candidate and the AI policy's conservative limits remain recorded in
[trash recovery validation](validation/irelia_trash_targets.json).


Acceptable Losses and Whirlwind passed immutable installed acceptance at
0b7a38b: 2,220 Rust tests, 349 source checks, 1,176 simulations, 45 interface
cases with 488 captures, and 20 API campaigns matching 159 complete records.
Three fresh installed captures were visually reviewed. This verifies the
recorded scenarios; full Standard parity and expert strength remain unproven.
See [player-choice acceptance](validation/irelia_player_choices.json).


Morbid Return, Existential Dread, Thwonk! and Friendship passed combined
immutable installed acceptance at f9eade5. The candidate passed 2,242 Rust tests,
355 source checks, 1,176 simulated games, 49 installed interface cases with
534 captures, and 22 API campaigns matching all 255 complete local records.
Six fresh installed captures were visually reviewed across both card batches.
Repeat displays its full resource total, attacker targeting distinguishes the
two Stun spells, and Friendship counts distinct live animal tags at resolution.
Both Mote levels and full card inspection are available. These results verify
the recorded scenarios; full Standard parity and expert strength remain unproven.
See [return-option acceptance](validation/irelia_return_options.json) and
[tag/stun acceptance](validation/irelia_tag_stun.json).


The shared binary-choice view passed immutable installed acceptance at e74c69d.
The native client disables unavailable answers and explains why; the revision-bound
`binary_choices` API exposes the same labels, actions and availability to bots.
Explicit Power selection and response timing remain available. The candidate
passed 2,249 Rust tests, 355 source checks, 1,176 simulations, 51 interface cases
(542 captures), and 23 API campaigns matching all 267 complete records. Three
fresh installed captures were visually reviewed. This covers ten binary question
shapes, not every decision or a complete legal-action API. See
[shared-choice acceptance](validation/binary_choices.json).


The movement correction separates destination checks when a spell is played
from target checks when it resolves. Ride the Wind now readies a legal friendly
unit even if Flash or an opposing Siren's Song has already moved it to the chosen
destination. Same-location declarations remain illegal. Its fixed installed
candidate passed 2,254 Rust tests, 355 source checks, 1,176 simulations,
51 native cases (542 captures), and 23 API campaigns matching all 267 records.
Two fresh installed captures were visually reviewed. See
[movement validation](validation/movement_resolution.json).

Twilight Step passed immutable installed acceptance at 96d6602: 2,261 Rust
tests, 358 source checks, 1,176 simulations, 53 interface cases (556 captures),
and 24 API campaigns matching all 279 complete local records. Three installed
captures were visually reviewed. Focused coverage includes live 3-Might targeting,
ordinary timing, printed Flow, retained physical base destinations and both Mote
levels. Its authored Irelia deck replaces two Charms; sourced presets are unchanged.
[Twilight validation](validation/twilight_step.json) records the evidence.
The snapshot maps 526 legal identities, with 553 still unimplemented; complete
Standard parity and expert opponent strength remain unproven.
