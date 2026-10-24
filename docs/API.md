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

Use `--custom-deck path/to/custom-you.json` or `--custom-opponent path/to/custom-ai.json` to use a saved list from the native editor. Invalid lists fail validation before play.

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

Every successful mutation increments the session revision by one. Reset also increments it; it never reuses an old revision. All mutation commands require the current revision. `observe`, `info`, `catalog`, `validate_deck`, `suggest`, `quote_payment`, and successful `validate` leave it unchanged. Rejected requests leave both game state and revision unchanged. The facade validates on a cloned game and commits only on success, including when the core engine rejects an action after beginning its internal work.

| Command | Fields besides `command` and optional `id` | Result |
| --- | --- | --- |
| `info` | none | Protocol, commands, current public summary, request size limit |
| `catalog` | none | Printed card data, published preset lists/rune splits, legend texts, battlefields |
| `validate_deck` | `deck` | Validate a constructed list without changing the session |
| `reset` | `revision`; optional `seed`, `decks`, `custom_decks`, `difficulty`, `first_player`, `battlefields` | New match in mulligan phase |
| `observe` | `player` | Observation filtered for that seat |
| `mulligan` | `revision`, `player`, `indices` | Replace up to two distinct opening-hand positions, once per seat |
| `start` | `revision` | Begin match; an unused mulligan means keep that hand |
| `suggest` | `player` | `{exhaustive:false, actions:[{id, action}, ...]}`; empty when this seat cannot act |
| `quote_payment` | `revision`, `player`, exactly one of `action` or `action_id` | Suggested and available power sources, exact cost obligations; no mutation |
| `validate` | `revision`, `player`, exactly one of `action` or `action_id` | `{legal:true}`, or an error; no mutation |
| `apply` | `revision`, `player`, exactly one of `action` or `action_id` | Apply one legal action and return public summary |
| `step_ai` | `revision`, `player` | Ask the built-in AI to take one action for the seat with priority |

Reset defaults match the initial settings. `decks` is a pair of preset IDs. `battlefields` is an optional pair of catalog battlefield IDs, one contributed by each player; omit it for preset defaults. A battlefield catalog ID is different from the in-match lane slot `0` or `1`. Invalid IDs are rejected. `difficulty` is `easy`, `tactical`, or `strategic`. Strategic uses bounded public-information lookahead; it is not an expert-strength guarantee. There is no automatic mulligan for either seat through this API.

A custom list can replace either seat through `custom_decks:[deck_or_null,deck_or_null]`. Omission uses presets for both seats. Each deck object contains `name`, `legend` (the supported legend ID), `chosen_champion`, `cards` (pairs of card ID and count), `runes` (six counts in catalog domain order), and `battlefield`. `catalog.cards` includes stable official printing IDs as well as current runtime IDs; each preset reports `eligible_cards` and `eligible_champions` for its legend. `local_rift::deck::CustomDeck::from_preset` creates an editable starting list, and its `to_json`/`from_json` use this same schema.

Deck validation requires at least 40 main-deck cards including the chosen champion, no more than three copies by full name, at most three Signature cards in total, matching champion tags, complete domain compatibility, exactly 12 legal runes, and an implemented legal battlefield. A `Unique` card is limited to one copy. Only implemented Standard-legal cards and legends can be selected. Zero-count rows should be omitted. Invalid decks are rejected before resetting the live match. Optional reset `battlefields` overrides the decks' selected fields. The API leaves both opening hands untouched for each agent's mulligan.

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
| `with_power_payment` | `action`, `payment` | Pay for a single nested action using exact chosen power sources |
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
| `recycle_rune` | `domain`, required `exhausted` | Recycle an explicitly ready (`false`) or exhausted (`true`) rune; no implicit energy generation |
| `equip` | `gear_id`, `target` | Pay and attach gear to a friendly unit |
| `equip_with_trash` | `gear_id`, `target`, `trash_indices` | Equip Last Rites with an explicit pair of trash cards to recycle |
| `activate_gear` | `gear_id` | Activate a Seal, Energy Conduit, or Gold when legal |
| `activate_legend` | optional `target` | Activate the selected legend's supported ability |
| `empower_unit` | `unit_id`, optional `pay_power` | Activate Legion Marauder or Tail-Cloaked Matriarch's Empower ability |
| `hide` | `hand_index`, `lane` | Pay to hide a Hidden card at a controlled battlefield |
| `hide_champion` | `lane` | Hide a chosen champion with Hidden |
| `play_hidden` | `hidden_id`, optional `targets` | Play an eligible hidden card; an empty target list uses the engine's default targets |
| `choose` | `accept` | Accept/decline a binary decision; Predict accepts recycling the top card |
| `choose_target` | optional `target` | Select a pending unit/gear target; during Tornado Warrior's base choice, `null` chooses the legend |
| `choose_trigger_targets` | `targets` | Finalize an ordered target array for a `trigger_targets` decision; an empty array declines an optional trigger |
| `select_cards` | `indices` | Select exact indices for discard, Top Three, Sabotage, or a granted Flow choice from trash |
| `play_from_trash` | `trash_index`, optional `lane`, `target` | Resolve a replay decision using default targeting |
| `play_from_trash_targets` | `trash_index`, optional `lane`, `targets` | Resolve a replay decision with exact targets |
| `play_from_trash_options` | `trash_index`, optional `lane`, `targets`, `repeat_targets`, `repeat`, `accelerate`, `bonus`, `flow` | Play from trash with exact targets and supported optional costs, including Flow or a replay instruction |
| `finalize_pending_play` | optional `lane`, `targets`, `repeat_targets`, `repeat`, `accelerate`, `bonus` | Finalize the already chosen public pending card; its identity cannot be changed |
| `assign_damage` | `assignments` | Ordered pairs `[[enemy_unit_id, damage], ...]`; obey lethal-before-next and total damage constraints |
| `choose_death_replacement` | `unit_id`, optional `gear_id` | Select a pending death replacement; `gear_id:null` selects Smite banishment |
| `order_trigger` | `trigger_id` | Finalize one of the current player's pending triggers; last finalized resolves first |
| `choose_battlefield` | `lane` | Choose which eligible staged battlefield opens next |
| `choose_resolution_group` | `index` | Select a listed legal target group when a resolving spell requires that choice |
| `set_optional_discount` | `power` | Set how many friendly Ezreal discounts reduce power; remaining discounts reduce energy |
| `place_token` | optional `lane` | Finalize the next token whose effect allows a destination choice at base or a battlefield you control |

For `play_from_trash_options`, `flow:"printed"` or `flow:"granted"` selects that exact available Flow instance. Omission or `null` uses an available payable instance. Explicit selection never falls back to another cost. A Flow choice cannot override an instructed Fizz/Last Rites replay and does not change the spell’s normal timing. Observed `flow_costs` lists trash indices with each instance’s source, energy, power, and whether power is colored. Quote and exact-payment commands preserve the selected instance.

The engine remains the authority on timing, costs, destinations, targeting, optionality, and available cards. A syntactically correct action may be illegal in the current state. Use `validate` to try a candidate without consuming a revision. `play_card_targets`, `play_card_options`, `play_from_trash_targets`, `play_from_trash_options`, and `finalize_pending_play` preserve exact target arrays, including empty selections where legal. `play_hidden` retains an automatic-target convenience when its list is empty; provide the full two-target pair to choose a hidden Switcheroo's targets yourself. Permanent abilities that choose after entry must receive an empty play target list and then answer the subsequent `on_play_target` decision.

Optional costs also apply to trash plays. Fizz ignores a replayed spell's base energy cost, while a chosen Repeat cost must still be paid. Flow pays its alternative base cost plus any chosen additional cost. Last Rites can play Kai'Sa with Accelerate when the combined cost is affordable. Each repeated spell supplies its own `repeat_targets`; it may target the same objects when legal.

Resource actions may be available while a payment decision is pending. For example, generate power with a rune, Seal, Gold, or Kai'Sa, then complete a replay/payment choice using the new revision. The decision remains pending. Resource actions are not a way to bypass target, combat assignment, or card selection decisions. Energy discounts apply after an alternative base cost and any additional costs are combined; `flow_costs` describes each instance's base cost, while validation and payment quotes use the live discounted total.

`set_optional_discount` changes the player's `optional_power_discount` preference without spending resources or passing priority. The next optional-cost cast uses that preference. It still counts as a successful API mutation and advances the revision.

## Choosing power sources

Automatic payment remains the default. Request `quote_payment` with the intended action, including its targets and optional costs. The result contains `suggested`, `available`, and `costs`. A quote validates on a disposable match copy and neither advances play nor changes the revision.

Power amounts have this form; both arrays use **Fury, Calm, Mind, Body, Chaos, Order** order:

```json
{"colored":[0,0,0,0,0,0],"any":1,"spell":0,"recycle":[0,0,0,0,0,0]}
```

`colored`, `any`, and `spell` spend generated power. `recycle` activates that many channelled runes of each domain. Spell-only power cannot pay for units, gear, abilities, or hiding. A recycled ready rune can exhaust for energy first, preserving that energy in the pool. Select exactly the power needed: unused selected sources are an error. Existing unselected resources remain untouched.

The payment wrapper recycles exhausted runes before ready ones and exhausts ready runes for energy before recycling. To choose a different physical rune state, prepare power with `recycle_rune` first: `exhausted:false` recycles a ready rune without generating energy, even if an exhausted rune of that domain is also available. The original `activate_rune` recycling action retains its exhausted-first convenience behavior.

Submit the original action inside a single payment wrapper:

```json
{"command":"apply","revision":4,"player":0,"action":{"type":"with_power_payment","action":{"type":"hide","hand_index":0,"lane":0},"payment":{"colored":[0,0,0,0,0,0],"any":1,"spell":0,"recycle":[0,0,0,0,0,0]}}}
```

This example spends one generated flexible power to hide a card while preserving colored power and runes. The battlefield and card must satisfy the usual hiding rules. Use `validate` to check any proposed allocation. Nested payment wrappers are rejected, every count must be an integer from 0 to 255, and both arrays must contain six counts. Every rejected payment preserves game state and revision. Quoted sources are only valid for the current match revision.

## Observations and decisions

Observations include phase, turn, active player, priority, winner, mulligan status, public resources, champion slots, units, gear, battlefield control/scoring, showdown state, and the public chain. The last chain entry is the top item, which resolves first after the required passes. Each player object includes hand/deck/rune-deck counts. Domain-indexed arrays use `Fury, Calm, Mind, Body, Chaos, Order`, matching `catalog.domains`. `energy` includes ready rune energy and stored energy; `energy_pool` is the stored portion. `runes` counts all runes on board, and `exhausted_runes` identifies its exhausted subset. `legend.spell_power` is restricted to spell costs.

Both seats expose the same public resource and equipment fields. A gear object's `attached_to` identifies its bearer unit, or is `null` when unattached. Its `lane` follows the bearer; `null` means base and does not by itself mean unattached. `card_id`, `name`, `token`, `ready`, and `temporary` remain visible while attached. Battlefield IDs refer to `catalog.battlefields`, which contains the field's official effect text; field abilities on the chain include their lane.

A unit's observed `might` is its current total, including equipment and currently applicable combat bonuses. Printed might remains in the catalog. Do not add an equipment bonus to the observed total again.

Each unit also exposes `keywords`: numeric `assault`, `shield`, and `deflect`, Boolean `tank`, `backline`, `ganking`, and `ambush`, plus optional printed `accelerate:{energy,power,domain}`. These values account for token identity, current equipment, level requirements, and granted abilities. Assault and Shield are their keyword values even outside combat; apply no extra bonus to observed `might`. Accelerate describes a play option, not an action that readies a unit already on the board. `damage_priority` is `0` for Tank, `1` for ordinary units, and `2` for Backline, matching the engine's combat-assignment ordering.

For token units and Gold, `token:true` distinguishes the generated object from a physical deck card. Its `card_id` is an engine source/reference identity, so use the observed `name` and `might` for the token instead of assuming that catalog entry's printed card kind or stats describe it.

A player object identifies its `legend_id`, `custom` flag, and `preset` (`null` for custom decks). `custom_deck` contains the saved list only for its owner; opposing observations always omit that list even when a hand reveal is active. The declared deck name and public legend/champion remain visible.

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
| `trash_target` | `source_card_id`, optional `gear_id`; choose exactly one eligible trash index with `select_cards`, before the ability becomes respondable |
| `revive_unit` | Choose Matriarch’s eligible trash unit with `play_from_trash` and no additional choices; its ability then enters the chain |
| `pending_play` | `pending_id`, `card_id`, `kind` (`fizz`, `last_rites`, or `matriarch`); use `finalize_pending_play` for this fixed card. Fizz waives base energy, Matriarch waives the base cost and fixes base as destination, and Last Rites pays the full cost. Additional costs still apply. `choose:false` returns it to trash only if `can_decline_trash_play` confirms it cannot be played. |
| `swap_unit` | `unit_id`; select another unit with `choose_target`, or decline |
| `empower_here` | `lane`, optional `source_id`; choose an eligible unit with `choose_target`. Hidden play abilities retain their source for location/liveness checks. |
| `on_play_target` | `card_id`, `source_id`, `lane`; select the unit/gear required by the finalized on-play ability |
| `trigger_targets` | `card_id`, `rule_index`, `context`, `maximum`, `required`, `candidates`; send an ordered array using `choose_trigger_targets`. Candidate membership alone does not establish that a combination or slot is legal; validate the complete action. Zero targets declines an optional trigger. |
| `legend` | `trigger` describes Irelia's ready/refresh choice; `choose` |
| `battlefield_order` | `lanes`, `combat`; the turn player chooses one listed lane using `choose_battlefield`; non-combat showdowns precede staged combats |
| `battlefield` | `lane`; `choose` whether to pay for the battlefield ability |
| `burn_for_might` | `unit_id`; `choose` |
| `combat_damage` | `lane`, `total`; `assign_damage` |
| `death_replacement` | `options:[{unit_id,gear_id}, ...]`; `choose_death_replacement` |
| `order_triggers` | `options:[{id,card_id,label}, ...]`; `order_trigger` |
| `resolution_targets` | `card_id`, `groups` of target objects, `repeated`; select the desired group by its index with `choose_resolution_group` |
| `token_placement` | `card_id`, `remaining`; use `place_token` separately for each remaining token |

For each decision, `suggest` supplies legal convenience responses, while structured actions allow alternate legal choices. Opposing observations omit private Predict and Top Three identities. Public replay/trigger/target decisions remain observable. Pending triggers are distinct from the public chain and cannot be targeted by counterspells until finalized.

Generic card triggers retain a public `context` with `source:{player,unit_id,gear_id}` and an optional event `subject` target. Equipment granted abilities identify their bearer as the unit source. A finalized chain ability has `type:"card_trigger"`, its metadata `rule_index`, and the same context; chosen targets live in the chain item's ordered `targets` array. Target choices finish before response priority opens.

The catalog and observed units expose `tags`, including token tags. Gear observations include their `empowered` status. Player observations distinguish `cards_played_this_turn` (finalized cards, including later-countered spells), `cards_completed_this_turn` (completed card plays), and `cards_drawn_this_turn`. Token plays do not increment either card-play counter.

Token plays created by a resolving effect wait until that effect finishes. `pending_token_plays` exposes their public pending IDs, controller, source `card_id`, Might, and `gear` flag in finalization order. `destination:{mode:"choose"}` permits the normal base/controlled-field choice; `{mode:"fixed",lane:null}` fixes base and a numeric lane fixes a battlefield. Pending IDs identify chain objects, not selectable board units. Placement choices finalize one token at a time; earlier tokens can be on the board while later placements remain pending. These choices do not grant a response window between token plays.

A `play_chosen` or `revive_chosen` chain ability identifies the previously selected trash card. Its `trash_index` becomes `null` if that card has left the trash before resolution. An `empower_object` ability exposes its selected `player`, `object_id`, and `lane`; `object_id:null` means that player's legend at base.

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

Instructed real-card plays also wait in Pending until their parent effect finishes. `pending_card_plays` exposes `{id,player,card_id,kind}` in finalization order. The physical card is then absent from trash and counted in this queue until its actual play finalizes. Fizz and Last Rites choose their public trash target before responses; opponents may remove that exact target before resolution. A card that left trash and returned is a new object and does not satisfy the old target. Finalization chooses the child card’s legal targets, destination, and optional costs, without an intervening response window.
