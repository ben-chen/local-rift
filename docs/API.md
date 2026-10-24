# Bot API

`rift-api` runs the same Rust engine as the graphical client without opening a window. It accepts one JSON object per line on standard input and writes exactly one JSON response per line to standard output. It has no server, ports, account system, or network dependency. This API shares the engine's supported card pool and rule limitations; see [RULES.md](RULES.md).

```sh
cargo build --release --bin rift-api
./target/release/rift-api
```

With Nix, run `nix run .#api`. After `nix build`, the example accepts `--binary result/bin/rift-api`.

The included standard-library Python bot plays seat 0 against the built-in AI:

```sh
python3 examples/random_bot.py --binary target/release/rift-api --seed 42
```

Replace `choose_action(view, suggestions, rng)` in that example with your policy. The function receives only its own filtered observation. Suggestions are useful starting candidates, **not an exhaustive list of legal actions**. A policy can construct and submit every action listed below, including different targets, optional costs, grouped movement, and custom combat assignments.

## Requests, responses, and revisions

The initial session is in `mulligan` phase, with decks `[0,1]`, seed `1`, Tactical AI, and player `0` going first. `info` reports protocol version `1`; `catalog` reports deck, card, domain, and battlefield IDs.

```json
{"id":"hello","command":"info"}
{"command":"reset","revision":0,"seed":42,"decks":[0,3],"difficulty":"tactical","first_player":0,"battlefields":[0,3]}
{"command":"observe","player":0}
{"command":"mulligan","revision":1,"player":0,"indices":[1,3]}
{"command":"mulligan","revision":2,"player":1,"indices":[]}
{"command":"start","revision":3}
{"command":"suggest","player":0}
```

These are consecutive requests, not a single JSON array. An optional `id` is echoed unchanged. Responses have one of these shapes:

```json
{"ok":true,"revision":4,"result":{"phase":"playing","turn":1,"active":0,"priority":0,"winner":null}}
{"ok":false,"revision":4,"error":{"code":"stale_revision","message":"Expected revision 4, received 3."}}
```

Every successful mutation increments the session revision by one. Reset also increments it; it never reuses an old revision. All mutation commands require the current revision. `observe`, `info`, `catalog`, `suggest`, and successful `validate` leave it unchanged. Rejected requests leave both game state and revision unchanged. The facade validates on a cloned game and commits only on success, including when the core engine rejects an action after beginning its internal work.

| Command | Fields besides `command` and optional `id` | Result |
| --- | --- | --- |
| `info` | none | Protocol, commands, current public summary, request size limit |
| `catalog` | none | Printed card data, published preset lists/rune splits, legend texts, battlefields |
| `reset` | `revision`; optional `seed`, `decks`, `difficulty`, `first_player`, `battlefields` | New match in mulligan phase |
| `observe` | `player` | Observation filtered for that seat |
| `mulligan` | `revision`, `player`, `indices` | Replace up to two distinct opening-hand positions, once per seat |
| `start` | `revision` | Begin match; an unused mulligan means keep that hand |
| `suggest` | `player` | `{exhaustive:false, actions:[{id, action}, ...]}`; empty when this seat cannot act |
| `validate` | `revision`, `player`, exactly one of `action` or `action_id` | `{legal:true}`, or an error; no mutation |
| `apply` | `revision`, `player`, exactly one of `action` or `action_id` | Apply one legal action and return public summary |
| `step_ai` | `revision`, `player` | Ask the built-in AI to take one action for the seat with priority |

Reset defaults match the initial settings. `decks` is a pair of preset IDs. `battlefields` is an optional pair of catalog battlefield IDs, one contributed by each player; omit it for preset defaults. A battlefield catalog ID is different from the in-match lane slot `0` or `1`. Invalid IDs are rejected. `difficulty` is `easy` or `tactical`. There is no automatic mulligan for either seat through this API.

Use observation `priority`, not `active`, to choose who acts. An opposing player can have priority while the active player owns the turn. Pending decisions can also transfer priority. `step_ai` works for either seat and processes one choice/action, not a whole turn.

Suggestion IDs such as `r4:p0:a2` are opaque and valid only for that seat and revision. Do not cache them across actions. Structured actions are not restricted to the suggestion list:

```json
{"command":"apply","revision":4,"player":0,"action":{"type":"play_card_targets","hand_index":0,"lane":null,"targets":[{"type":"unit","player":1,"unit_id":7}]}}
```

## Actions and targets

Each action is an object with a `type`. Unknown fields are errors, so misspellings cannot silently change a choice. Indices are zero-based and must be refreshed after a mutation. Unit, gear, hidden, chain, and trigger IDs identify live objects; they are not card catalog IDs. `lane:null` means base; `lane:0` or `lane:1` means a battlefield slot. Optional `lane` and `target` may be omitted. Optional boolean flags default to `false`, and omitted `targets`/`repeat_targets` default to empty arrays.

Targets have exactly one of these forms:

```json
{"type":"unit","player":1,"unit_id":7}
{"type":"gear","player":0,"gear_id":12}
{"type":"spell","chain_id":5}
```

| Action `type` | Fields | Purpose |
| --- | --- | --- |
| `play_card` | `hand_index`, optional `lane`, `target` | Play from hand with engine default targeting when target is omitted |
| `play_card_targets` | `hand_index`, optional `lane`, `targets` | Play with the exact target list, including an intentionally empty list |
| `play_card_options` | `hand_index`, optional `lane`, `targets`, `repeat_targets`, `repeat`, `accelerate`, `bonus` | Exact targets plus supported optional costs; `bonus` selects Rampage's extra payment |
| `play_champion` | optional `lane`, `target` | Play the chosen champion from its separate slot |
| `play_champion_options` | optional `lane`, `target`, `accelerate` | Champion play with Accelerate when supported |
| `move` | `unit_id`, optional `lane` | Move a ready unit to a legal destination |
| `move_group` | `unit_ids`, optional `lane` | Move an explicit group together |
| `attack` | `lane` | Open combat at a contested battlefield |
| `pass` | none | Pass priority/focus in an open response window |
| `end_turn` | none | End the active player's main phase when legal |
| `activate_rune` | `domain`, `recycle` | Exhaust a ready rune for stored energy, or recycle one for its domain power |
| `equip` | `gear_id`, `target` | Pay and attach gear to a friendly unit |
| `equip_with_trash` | `gear_id`, `target`, `trash_indices` | Equip Last Rites with an explicit pair of trash cards to recycle |
| `activate_gear` | `gear_id` | Activate Seal or Gold when legal |
| `activate_legend` | optional `target` | Activate the selected legend's supported ability |
| `empower_unit` | `unit_id`, optional `pay_power` | Activate Legion Marauder or Tail-Cloaked Matriarch's Empower ability |
| `hide` | `hand_index`, `lane` | Pay to hide a Hidden card at a controlled battlefield |
| `hide_champion` | `lane` | Hide a chosen champion with Hidden |
| `play_hidden` | `hidden_id`, optional `targets` | Play an eligible hidden card; an empty target list uses the engine's default targets |
| `choose` | `accept` | Accept/decline a binary decision; Predict accepts recycling the top card |
| `choose_target` | optional `target` | Select a pending unit/gear target; during Tornado Warrior's base choice, `null` chooses the legend |
| `select_cards` | `indices` | Select exact indices for discard, Top Three, Sabotage, or a granted Flow choice from trash |
| `play_from_trash` | `trash_index`, optional `lane`, `target` | Resolve a replay decision using default targeting |
| `play_from_trash_targets` | `trash_index`, optional `lane`, `targets` | Resolve a replay decision with exact targets |
| `play_from_trash_options` | `trash_index`, optional `lane`, `targets`, `repeat_targets`, `repeat`, `accelerate`, `bonus` | Play from trash with exact targets and supported optional costs, including Flow or a replay instruction |
| `assign_damage` | `assignments` | Ordered pairs `[[enemy_unit_id, damage], ...]`; obey lethal-before-next and total damage constraints |
| `choose_death_replacement` | `unit_id`, optional `gear_id` | Select a pending death replacement; `gear_id:null` selects Smite banishment |
| `order_trigger` | `trigger_id` | Finalize one of the current player's pending triggers; last finalized resolves first |
| `choose_resolution_group` | `index` | Select a listed legal target group when a resolving spell requires that choice |
| `set_optional_discount` | `power` | Set how many friendly Ezreal discounts reduce power; remaining discounts reduce energy |
| `place_token` | optional `lane` | Place the next Up from the Deep Tentacle at base or a battlefield you control |

The engine remains the authority on timing, costs, destinations, targeting, optionality, and available cards. A syntactically correct action may be illegal in the current state. Use `validate` to try a candidate without consuming a revision. `play_card_targets`, `play_card_options`, `play_from_trash_targets`, and `play_from_trash_options` preserve exact target arrays, including empty selections where legal. `play_hidden` retains an automatic-target convenience when its list is empty; provide the full two-target pair to choose a hidden Switcheroo's targets yourself. Permanent abilities that choose after entry must receive an empty play target list and then answer the subsequent `on_play_target` decision.

Optional costs also apply to trash plays. Fizz ignores a replayed spell's base energy cost, while a chosen Repeat cost must still be paid. Flow pays its alternative base cost plus any chosen additional cost. Last Rites can play Kai'Sa with Accelerate when the combined cost is affordable. Each repeated spell supplies its own `repeat_targets`; it may target the same objects when legal.

Resource actions may be available while a payment decision is pending. For example, generate power with a rune, Seal, Gold, or Kai'Sa, then complete a replay/payment choice using the new revision. The decision remains pending. Resource actions are not a way to bypass target, combat assignment, or card selection decisions.

`set_optional_discount` changes the player's `optional_power_discount` preference without spending resources or passing priority. The next optional-cost cast uses that preference. It still counts as a successful API mutation and advances the revision.

## Observations and decisions

Observations include phase, turn, active player, priority, winner, mulligan status, public resources, champion slots, units, gear, battlefield control/scoring, showdown state, and the public chain. The last chain entry is the top item, which resolves first after the required passes. Each player object includes hand/deck/rune-deck counts. Domain-indexed arrays use `Fury, Calm, Mind, Body, Chaos, Order`, matching `catalog.domains`. `energy` includes ready rune energy and stored energy; `energy_pool` is the stored portion. `runes` counts all runes on board, and `exhausted_runes` identifies its exhausted subset. `legend.spell_power` is restricted to spell costs.

Both seats expose the same public resource and equipment fields. A gear object's `attached_to` identifies its bearer unit, or is `null` when unattached. Its `lane` follows the bearer; `null` means base and does not by itself mean unattached. `card_id`, `name`, `token`, `ready`, and `temporary` remain visible while attached. Battlefield IDs refer to `catalog.battlefields`, which contains the field's official effect text; field abilities on the chain include their lane.

A unit's observed `might` is its current total, including equipment and currently applicable combat bonuses. Printed might remains in the catalog. Do not add an equipment bonus to the observed total again.

For token units and Gold, `token:true` distinguishes the generated object from a physical deck card. Its `card_id` is an engine source/reference identity, so use the observed `name` and `might` for the token instead of assuming that catalog entry's printed card kind or stats describe it.

The viewer sees their own hand as card IDs. The opposing hand is `null` except while an actual reveal effect permits it. Deck order and rune-deck order are omitted for both players. Hidden cards expose object ID, location, and hidden turn; `card_id` is `null` unless the viewer is allowed to see it. Granted Flow indices refer to the public trash and are visible to both viewers. Diagnostic game logs, prompts, random-generator state, and match seed are never exported in observations.

`decision` is `null` or an object with `type` and `player`. Other fields describe the current choice:

| Decision | Data and response |
| --- | --- |
| `predict` | Owner sees `card_id`; `choose` recycles it when true, keeps it on top when false |
| `counter_tax` | `chain_id`, energy `cost`; `choose`, with resource actions allowed before payment |
| `discard` | `count`, `draw_after`; `select_cards` indexes your current hand |
| `top_three` | Owner sees `cards`; `select_cards` indexes this list |
| `sabotage` | `opponent`, revealed `cards`; `select_cards` indexes that hand |
| `reveal_hand` | `opponent`, revealed `cards`; acknowledge with `choose` |
| `grant_flow` | `select_cards` indexes an eligible spell in your trash; an empty selection is legal only when no eligible spell exists |
| `optional_play` | `card_id`, optional `gear_id`; accept or decline Fizz/Last Rites before its optional play ability enters the chain |
| `replay_spell`, `last_rites`, `revive_unit` | `play_from_trash`, `play_from_trash_targets`, or `play_from_trash_options` for eligible optional costs; `last_rites` also includes `gear_id`; decline only when `can_decline_trash_play` allows it |
| `swap_unit` | `unit_id`; select another unit with `choose_target`, or decline |
| `empower_here` | `lane`; choose an eligible unit with `choose_target` |
| `on_play_target` | `card_id`, `source_id`, `lane`; select the unit/gear required by the finalized on-play ability |
| `legend` | `trigger` describes Irelia's ready/refresh choice; `choose` |
| `battlefield` | `lane`; `choose` whether to pay for the battlefield ability |
| `burn_for_might` | `unit_id`; `choose` |
| `combat_damage` | `lane`, `total`; `assign_damage` |
| `death_replacement` | `options:[{unit_id,gear_id}, ...]`; `choose_death_replacement` |
| `order_triggers` | `options:[{id,card_id,label}, ...]`; `order_trigger` |
| `resolution_targets` | `card_id`, `groups` of target objects, `repeated`; select the desired group by its index with `choose_resolution_group` |
| `token_placement` | `card_id`, `remaining`; use `place_token` separately for each remaining token |

For each decision, `suggest` supplies legal convenience responses, while structured actions allow alternate legal choices. Opposing observations omit private Predict and Top Three identities. Public replay/trigger/target decisions remain observable. Pending triggers are distinct from the public chain and cannot be targeted by counterspells until finalized.

A `revive_chosen` chain ability identifies the previously selected trash card. Its `trash_index` becomes `null` if that card has left the trash before resolution. An `empower_object` ability exposes its selected `player`, `object_id`, and `lane`; `object_id:null` means that player's legend at base.

Fizz and Last Rites ask whether to accept their optional play instruction before placing that ability on the chain. Once an accepted instruction resolves, its trash selection is required if a legal payable card exists. `required_trash_play` identifies that stage; `can_decline_trash_play` reports whether declining is currently legal. A bot should not assume that `choose:false` remains available after accepting the earlier optional play.

## Privacy boundary and reproducibility

This is a **trusted local coordinator**, not an adversarial multiplayer service. The process owner can request either player's observation, reset the match, and run either AI. To compete separate bots, keep this coordinator in a trusted process and send each bot only its own observation and suggestion list; do not forward the seed, the other seat's observation, or diagnostic stderr. Do not give an untrusted bot direct access to the server process or Rust `Game`. The API does not sandbox executable bots.

For hidden-information competition, choose a fresh seed known only to the coordinator. The documented default seed is intentionally reproducible; a bot that knows a seed and the build could reconstruct shuffled cards despite the filtered observation.

For reproducible tests, record the reset options and successful structured actions, mulligans, and AI-step commands. Replaying them with the same build produces the same result. Determinism is guaranteed within the same engine version, not across rule changes or future releases. Revision-bound suggestion IDs are useful during a match; store their structured actions for portable replay.

Normal responses never contain the engine log, which can include private card names. `Session`'s debug representation is also deliberately limited to a public summary. A legal reveal can expose hidden information; clients may remember information they were allowed to see even after the transient reveal ends.

## Errors and limits

Errors use `invalid_json`, `invalid_utf8`, `request_too_large`, `invalid_request`, `unknown_command`, `stale_revision`, `wrong_phase`, `wrong_player`, `unknown_action`, `illegal_action`, `engine_error`, or `revision_exhausted`. Read the accompanying message for details. Malformed, unknown, out-of-turn, and stale requests do not mutate the match. Requests are bounded to 1 MiB including their line ending; action arrays are bounded to 1024 entries. Oversized and invalid UTF-8 lines are drained, rejected, and followed by normal processing of the next line. EOF closes the process. There is no background progression: the state changes only after a successful mutation command.

## Rust facade

The library exposes `local_rift::api::{Session, ResetOptions, ApiError, ActionSuggestion}` and helpers `action_to_json`, `parse_action`, `target_to_json`, `parse_target`, and `catalog`. Use native `game::Action` values for programmatic decisions:

```rust
use local_rift::api::{ResetOptions, Session};
use local_rift::game::Action;

let mut session = Session::new(ResetOptions { seed: 42, ..Default::default() })?;
let opening = session.observe(0)?;
session.mulligan(0, session.revision(), &[])?;
session.mulligan(1, session.revision(), &[])?;
session.start(session.revision())?;
session.validate_action(0, session.revision(), Action::EndTurn)?;
session.apply_action(0, session.revision(), Action::EndTurn)?;
# Ok::<(), local_rift::api::ApiError>(())
```

`Session::observe` returns a JSON value intentionally matching the wire protocol. `Session::handle` accepts a parsed request; `process_line` parses one request. Neither lends out the authoritative game. `suggestions` and `apply_suggestion` implement the same revision-scoped convenience actions as JSONL. Direct `Game` access remains available in the engine library for trusted simulations and test fixtures, outside the filtered API boundary.

Validation tests cover all action round trips, private-state invariance, malformed and stale requests, explicit multi-target/group/combat choices, deterministic replay, and completed matches for all 16 preset pairings. Run `cargo test --lib api::tests` and `cargo test --bin rift-api`.
