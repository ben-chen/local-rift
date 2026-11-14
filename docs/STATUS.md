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

The next acceptance priorities are complete Irelia match practice with active
opposing responses, independently expected rules scenarios, opponent decisions
outside the development benchmark, and beginner learning sessions. Expanding to
all legal cards remains part of the goal; completing these priorities does not
replace that scope.

The [Strategic combat improvement](validation/strategic_combat.json) passed its
separate installed acceptance: 2,018 Rust tests, 334 source checks, 1,176 simulated
games, 23 UI cases and ten complete API comparisons (63 records). Its scoped
Block decision benchmark improves to 26/26; this does not establish expert strength.
