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

The initial session is in `mulligan` phase, with decks `[0,1]`, seed `1`, Tactical AI, and player `0` going first. `info` reports protocol version `1`; `catalog` reports separate preset, legend, card, domain, and battlefield IDs.

The repository's `assets/data/full_catalog.json` is a separate inventory with schema version `2`; it is not the bot protocol or the playable `catalog` response. It preserves raw printing fields and adds reviewed `effective` facts, resolution references, and unresolved-field diagnostics. Deck validation uses those effective facts and rejects unresolved gameplay or variant fields, including conflicts on alternate printings. Source review alone never registers a playable card. Inventory-only identity `kind` preserves complete type sets as sorted `+`-joined strings, such as `gear+unit`; the corresponding printing’s `effective.types` retains both types. This does not add a playable bot-protocol card kind. The current expansion appends main-card IDs 349–361, while preserving existing IDs, presets, training decks, and chosen-champion eligibility. Additive Temporary status, token identity, evaluated passive values and target queries retain protocol version `1`. See [the inventory review contract](CATALOG_REVIEW.md) for evidence and remaining variant-review limits.

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

Every successful mutation increments the session revision by one. Reset also increments it; it never reuses an old revision. All mutation commands require the current revision. `observe`, `info`, `catalog`, `validate_deck`, `suggest`, `quote_payment`, `play_choices`, `cost_plan`, `target_candidates`, and successful `validate` leave it unchanged. Rejected requests leave both game state and revision unchanged. The facade validates on a cloned game and commits only on success, including when the core engine rejects an action after beginning its internal work.

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
| `quote_payment` | `revision`, `player`, exactly one of `action` or `action_id` | Suggested and available power sources, exact Power obligations; no mutation |
| `play_choices` | `revision`, `player`, exactly one of `action` or `action_id` | Available Repeat instances, Accelerate/bonus costs, original zone, and typed cost plan |
| `target_candidates` | `revision`, `player`, structured `action`, `execution` | Pure action-aware target view for the next unfilled slot; see below |
| `cost_plan` | `revision`, `player`, exactly one of `action` or `action_id` | Cost components, modifiers, applied discount order, final Energy, typed Power and XP obligations |
| `validate` | `revision`, `player`, exactly one of `action` or `action_id` | `{legal:true}`, or an error; no mutation |
| `apply` | `revision`, `player`, exactly one of `action` or `action_id` | Apply one legal action and return public summary |
| `step_ai` | `revision`, `player` | Ask the built-in AI to take one action for the seat with priority |

Reset defaults match the initial settings. `decks` is a pair of preset IDs. `battlefields` is an optional pair of catalog battlefield IDs, one contributed by each player; omit it for preset defaults. A battlefield catalog ID is different from the in-match lane slot `0` or `1`. Invalid IDs are rejected. `difficulty` is `easy`, `tactical`, or `strategic`. Strategic uses bounded public-information lookahead; it is not an expert-strength guarantee. There is no automatic mulligan for either seat through this API.

A custom list can replace either seat through `custom_decks:[deck_or_null,deck_or_null]`. Omission uses presets for both seats. Each deck object contains `name`, `legend`, `chosen_champion`, `cards` (pairs of card ID and count), `runes` (six counts in catalog domain order), and `battlefield`. `legend` accepts a stable numeric registry ID or its official printing ID, such as `"ogn-265-298"`; saved/output decks normalize it to the numeric ID. Existing numeric IDs0–3 retain their identities. The selected chosen-champion variant does not change the legend.

`catalog.legends` is independent of `catalog.presets`: ten legends are supported while the four sourced preset lists retain their original IDs. Every legend reports its official identity/text, domains, default chosen champion, `eligible_cards`, `eligible_champions`, and a complete editable `training_deck`. For Darius, Viktor, Lux, Ambessa, Rengar, and Ezreal those are authored practice lists, not tournament/meta recommendations; `training_note` describes the origin. Pass the returned training deck through `custom_decks` to practice with that legend. `CustomDeck::from_legend` provides the same starting list in Rust; `from_preset` still copies an exact sourced preset. `catalog.presets[].legend_id` links each preset to the registry. A new legend ID must not be used as a preset ID in `reset.decks`.

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
{"type":"battlefield","lane":1}
```

A battlefield target identifies physical slot `0` or `1`, independently of its controller or catalog definition. Two slots using the same battlefield card are distinct targets. It has no `player` or `battlefield_id` field, and cannot select a base. A spell's battlefield target belongs in `targets`; the action's outer `lane` keeps its existing deployment/movement meaning and is not an alias.

Each playable card's `play_targets` reports `{kind, maximum, required}`. `kind` is `unit`, `gear`, `unit_or_gear`, `spell`, `battlefield`, or `null`. This describes choices made while playing that card; delayed on-play triggers retain their own later decisions. It does not replace card-specific allegiance, location, timing, or payment validation. Firestorm requires exactly one battlefield target, including an empty field. Flurry of Blades and Avalanche have no targets; the units they damage are not individually chosen and do not add Deflect costs. Use `validate` for the complete action.

```json
{"type":"play_card_targets","hand_index":0,"targets":[{"type":"battlefield","lane":1}]}
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
| `activate_ability` | `source`, `ability_index`, optional `targets` | Use a registered unit or gear ability with its exact source, printed ability index, and chosen targets |
| `activate_ability_with_costs` | `source`, `ability_index`, optional `targets`, `trash_indices` | Activate with exact trash positions to recycle as costs; cost objects are separate from targets |
| `resolve_conversion` | `amount` | Pay and resolve the current energy/power conversion; integer from 0 to 65535 |
| `begin_payment` | `action` | Fix an unwrapped card or ability intent before using resource abilities during its payment |
| `complete_payment` | none | Pay the fixed obligations and finish the selected action; may use an exact power wrapper |
| `activate_legend` | optional `target` | Activate the selected legend's supported ability |
| `empower_unit` | `unit_id`, optional `pay_power` | Activate Legion Marauder or Tail-Cloaked Matriarch's Empower ability |
| `hide` | `hand_index`, `lane` | Pay to hide a Hidden card at a controlled battlefield |
| `hide_champion` | `lane` | Hide a chosen champion with Hidden |
| `play_hidden` | `hidden_id`, optional `targets` | Play an eligible hidden card; an empty target list uses the engine's default targets |
| `choose` | `accept` | Accept/decline a binary decision; Predict accepts recycling the top card |
| `resolve_predict` | `choice_id`, `recycle`, `top_order` | Complete a multi-card Predict; the two index arrays partition the looked-at occurrences, with `top_order[0]` drawn next |
| `resolve_draw_look` | `choice_id`, `index` | Reveal and draw one eligible inspected occurrence, or use explicit `index:null` to recycle all |
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
| `finalize_token_play` | optional `lane`, `accelerate` | Finalize a pending token, including an available Accelerate cost |
| `with_play_choices` | `action`, `choices` | Select independent Repeat executions, optional costs, discount order, and Power-reduction allocation |

Registered activations use a source object such as `{"type":"unit","player":0,"unit_id":12}` or `{"type":"gear","player":0,"gear_id":15}`. The source is distinct from the targets. `ability_index` is zero-based within that card's `catalog.cards[].activated_abilities`; for Renata Glasc, Mastermind, index `0` draws and index `1` scores. The catalog supplies each ability's label, timing, energy/power cost, power domain, exhaust cost, source battlefield requirement, target restrictions, whether it adds resources, recycling costs, source sacrifice, and conversion direction. Target restrictions distinguish units from gear and identify an “another” requirement. Use the catalog to discover registered sources; existing Equip, resource-gear, Empower, and legend actions retain their own interfaces.

```json
{"type":"activate_ability","source":{"type":"unit","player":0,"unit_id":12},"ability_index":1,"targets":[]}
{"type":"activate_ability","source":{"type":"gear","player":0,"gear_id":15},"ability_index":0,"targets":[{"type":"unit","player":1,"unit_id":7}]}
```

These action shapes require matching live objects and a legal timing window. Use `validate`, `quote_payment`, or `with_power_payment` as with card plays. Costs are paid before responses. Ordinary abilities enter the chain; Dragonsoul Sage's Reaction Add resolves immediately and can be used during a permitted payment choice. Its Reaction ability does not grant permission to play the unit as a Reaction.

`activate_ability_with_costs` requires distinct, in-range positions from the controller's current trash. Garbage Grabber requires three cards; Assembly Rig requires one unit. These are costs, not targets, and move before responses. Divining Shells kills and exhausts itself as costs; its target remains a separate unit. Hextech Formula targets another gear object, including opposing or attached gear.

XP abilities add `xp_cost`, decimal-string `xp_cost_exact`, nullable `minimum_xp`, and nullable `fixed_resource_output:{energy,power,domain}`. A null output means it is not a fixed producer; it does not mean the ability cannot add resources. Platewyrm Egg and converters retain their dynamic behavior. Output domains use the catalog's capitalized domain names; null Power domain means Any. Honeyfruit's index0 adds one Any Power, and index1 adds one Energy plus one Any Power while at Level6. Both exhaust that physical source, and both remain choices at6+. The observed `activated_abilities` omits a dependent ability whose Level requirement is absent; timing, payment and other legality still require validation.

Crowd Favorite and Enthralling Protector use ordinary `activate_ability` index0 with an empty target array. Their2XP cost is paid before the self-Buff response window; neither has an Exhaust cost. Self-Buff is not a chosen target. Already-buffed activation remains legal and pays its cost without adding a second Buff. `quote_payment` remains Power-only, but its disposable execution validates the whole action, including XP. Use `cost_plan` for the complete resource obligations. XP is never a client-selected Power source or a new action field.

`catalog.cards[].equipment` is null for non-equipment. Equipment reports `might`, `equip_energy`, `equip_power`, `equip_domain` (null when Power is zero), `equip_xp`, decimal-string `equip_xp_exact`, `recycle_trash`, and `quick_draw`. The descriptor covers those costs and the static Might bonus, not every printed effect. Shepherd's Heirloom uses ordinary `equip`, spends1XP with no Energy/Power, and attaches on resolution. Its separate completed-play trigger gains1XP before it is equipped; attaching is not another card play.

The catalog descriptors `only_if_not_empowered` and `disempower_source` separate use restrictions from costs. Questionable Tome, Hextech Disc, Platewyrm Egg, and Tools of Empire use ability index 0 to Empower their own exact source; submit an empty target array. Empower happens on resolution and can be responded to. Tome and Disc use index 1 to pay Disempower, Energy, and Exhaust before their draw/token effect enters the chain. Tools does not Exhaust for its index-0 Empower cost. Gear observations expose the current public `empowered` and `ready` states. Platewyrm Egg index 1 adds Energy immediately, including during permitted payments; its amount depends on its current Empowered state. Ordinary Empower abilities cannot be used inside a payment to improve another resource ability.

Ancient Henge and Hextech Anomaly exhaust when activated, then create `decision.type:"resource_conversion"`. The decision includes the exact source, card/ability identity, direction (`energy_to_power` or `power_to_energy`), and `available`, the maximum currently payable amount. Conversion payment occurs during resolution. `resolve_conversion` with zero completes without refunding Exhaust; `choose` with `accept:false` has the same zero-payment meaning here. Anomaly can be wrapped in `with_power_payment` to preserve chosen colors and runes. Spell-only power cannot fund it.

Other Add abilities and rune actions remain available during a conversion. A nested conversion must finish before its parent resumes. `resource_continuations` reports suspended tasks from outermost to innermost as `{player,task}` without exposing private decision payloads. These immediate abilities do not create a response window.

For `play_from_trash_options`, `flow:"printed"` or `flow:"granted"` selects that exact available Flow instance. Omission or `null` uses an available payable instance. Explicit selection never falls back to another cost. A Flow choice cannot override an instructed Fizz/Last Rites replay and does not change the spell’s normal timing. Observed `flow_costs` lists trash indices with each instance’s source, energy, power, and whether power is colored. Quote and exact-payment commands preserve the selected instance.

The engine remains the authority on timing, costs, destinations, targeting, optionality, and available cards. A syntactically correct action may be illegal in the current state. Use `validate` to try a candidate without consuming a revision. `play_card_targets`, `play_card_options`, `play_from_trash_targets`, `play_from_trash_options`, and `finalize_pending_play` preserve exact target arrays, including empty selections where legal. `play_hidden` retains an automatic-target convenience when its list is empty; provide the full two-target pair to choose a hidden Switcheroo's targets yourself. Permanent abilities that choose after entry must receive an empty play target list and then answer the subsequent `on_play_target` decision.

Optional costs also apply to trash plays. Fizz ignores a replayed spell's base energy cost, while a chosen Repeat cost must still be paid. Flow pays its alternative base cost plus any chosen additional cost. Last Rites can play Kai'Sa with Accelerate when the combined cost is affordable. The legacy `repeat`/`repeat_targets` fields select one printed Repeat. Use the following canonical choices for independently granted or multiple Repeat instances.

Resource actions may be available while a payment decision is pending. For example, generate power with a rune, Seal, Gold, or Kai'Sa, then complete a replay/payment choice using the new revision. The decision remains pending. Resource actions are not a way to bypass target, combat assignment, or card selection decisions. Energy discounts apply after an alternative base cost and any additional costs are combined; `flow_costs` describes each instance's base cost, while validation and payment quotes use the live discounted total.

`set_optional_discount` changes the player's `optional_power_discount` preference without spending resources or passing priority. The next optional-cost cast uses that preference. It still counts as a successful API mutation and advances the revision.

## Empowered state, conditional effects, and Predict

`catalog.cards[].passive_rules` describes the supported live passives with `rule_index`, `condition` (`always`, `empowered`, `not_empowered`, `attacking_with_another`, `combat_alone`, `buffed`, `other_friendly_here`, `controller_runes_at_least`, `controller_xp_at_least`), `condition_threshold` (a number for rune/XP thresholds; null otherwise), `recipients` (`self_unit`, `friendly_units`, `friendly_empowered_units`, `other_friendly_units_here`, `friendly_token_units`, `attached_bearer`), and a typed `effect`. An effect is either `{type:"might",amount}` or `{type:"keyword",keyword,amount?}`. Ganking has no numeric amount. Observed units retain authoritative `might`, `keywords`, and `empowered`; their additional `passive_contributions` list reports each physical source, card/rule identity, active flag, and effect. Inactive conditional benefits are explanations, not current keywords or Buff counters. Two copies have separate source IDs and can contribute separately.

Combat conditions require an actual attacker or defender, rather than merely occupying an open showdown. Alone/with-another checks include every other friendly unit at that location, including tokens, stunned or exhausted units, and units with zero Might. Local recipients share the source's controller and current location, exclude the source itself, and distinguish the two players' bases. Moving or removing a source changes its live contributions; an Assault or Shield grant still applies its Might bonus only in its proper combat role.

`buffed` tests an actual Buff counter, not positive token base Might or a temporary modifier. `other_friendly_here` does not require combat and distinguishes the two bases. Rune thresholds count controlled exhausted runes as well as ready runes; XP thresholds use current controller XP. `friendly_token_units` excludes gear tokens. `attached_bearer` applies only while that exact equipment is attached, and separate copies contribute separately.

Activated abilities retain nominal `energy` and `power`. Their `cost_modifiers` describe `energy_per_controlled_rune` with `amount`, or `energy_if_runes_at_most` with `maximum` and `amount`. Use `cost_plan` for the evaluated obligation, including Risen Altar and discount ordering. Controlled rune counts include exhausted runes but exclude the rune deck. Once `begin_payment` fixes a cost, recycling a rune does not reprice that pending activation.

`catalog.legends[].activation` is null for legends without an activated ability. Otherwise it reports timing, Energy, Power, optional Power domain (null means Any), Exhaust/Disempower costs, target policy, and effect. Matriarch of War uses `activate_legend` with a unit target, including an enemy unit; it pays normal enemy Deflect. The resulting chain abilities are `matriarch_empower` and `matriarch_ready`, with the latter retaining its chosen target. Empowering is an effect; its triggered observers wait for a real transition and follow the actor, independently of the target's controller.

A `predict_cards` decision reports `player`, `choice_id`, and actual available `count` to both viewers. Only its controller receives `cards`, ordered from next draw downward. Those cards remain in the main deck while the choice is pending. Submit exactly one occurrence index for every looked-at card across these two arrays:

```json
{"type":"resolve_predict","choice_id":12,"recycle":[1],"top_order":[0]}
```

`top_order[0]` will be drawn next. The recycled subset goes to the bottom in randomized order; `recycle` does not select a bottom order. Identical card IDs still have distinct occurrence indices. Duplicate, missing, out-of-range, and stale choice IDs are rejected atomically. No opponent view contains private identities or chosen order. An empty deck creates no choice and causes no burnout. Legacy single-card `predict` and its Boolean `choose` action are unchanged.

Move-based trigger `context.movement` records `origin` and `destination`; a null endpoint means base. A null movement object means this was not a Move event. These event facts survive source departure. Akali's target battlefield comes from the recorded event, while its Empowered damage and Ambessa's comparative Might use the source's live state. Validate the full proposed target action, including its slot and other selected targets.

## Repeat instances and cost choices

Ask `play_choices` for the intended action, with its initial targets already selected. It returns the card's `origin` (`hand`, `champion`, `trash`, `banish`, `hidden` with `lane`, or `token`), a `repeats` list, optional `accelerate` and `bonus` descriptors, and `costs`. Each Repeat option has an opaque `id`, source, label, and its own Energy, Power, and domain requirements. Temporal Portal, Syndra, and printed Repeat can supply separate options. Discover IDs from the current position; do not construct or persist them across matches. Repeat-option, cost-component, and cost-modifier IDs are decimal strings on the wire, preserving all 64 bits in JavaScript clients. Requests also accept exact unsigned JSON integers for these IDs. Card, unit, lane, and grant IDs retain their existing numeric representation.

Wrap the action in `with_play_choices`. Its `choices` accepts:

- `repeats`: an ordered array of `{option_id, choices:{targets, mode}}`. Each option can be paid at most once. Targets are chosen separately for each execution; currently `mode` must be absent or `null`.
- `accelerate` and `bonus`: optional booleans, defaulting to false. The descriptor's label identifies the relevant printed additional cost, including Nami's optional payment.
- `cost_order`: an ordered array of `{modifier_id, use_power, power_allocations}`. `use_power` selects Power for a modifier that explicitly allows Energy or Power; otherwise it must be false. `power_allocations` contains `{component_id, amount}` entries to distribute that modifier's usable Power reduction across eligible components.

Missing arrays are empty. Declining Repeat still consumes a next-spell grant when the spell's play begins. Do not combine canonical Repeat selections with the legacy `repeat:true` adapter. `begin_payment` can contain a `with_play_choices` action so the selected costs and grants remain fixed while resources are generated. `with_power_payment` around the chosen action selects the actual payment sources; this is separate from choosing which cost a discount reduces.

Field-targeting spells use the same per-execution target arrays. For example, Firestorm's base play can target slot `0`, one paid Repeat slot `1`, and another slot `0`. Every execution derives its affected enemy units from the current board. Payment, Flow, and chain observations preserve those distinct field choices. An explicitly empty array does not choose a field automatically; the legacy `play_card` default-target action remains available when automatic selection is desired.

`cost_plan` accepts selected card plays and supported Equip, Empower, or registered ability actions. It returns nominal `components`, applicable `modifiers`, the actual `applied_order`, and final `energy`, `power`, `xp`, and decimal-string `xp_exact`. Every component and optional cost also includes `xp` and `xp_exact`, including zero. XP is a fixed obligation determined by the source, and Energy/Power discounts do not reduce it. Each Power obligation retains its allowed domains and `spell_eligible` flag; totals alone cannot represent different colored costs. Modifier scope is either `total` or `component` with a `component_id`. Cost increases apply before discounts. Component discounts precede total discounts; explicit cross-stage reversal is rejected. Within a stage, the player may order reductions, including reductions with a local minimum. A minimum applies only to that discount, not as a permanent floor on the resulting cost. An omitted order uses a deterministic legal order.

Explicit Power allocations must name distinct eligible components, fit their remaining cost, and allocate the entire usable reduction. A selected optional cost remains paid when discounts reduce it to zero. Invalid option IDs, duplicates, unsupported modes, and inconsistent allocations reject without changing the match. These queries require the acting seat and current revision; a preview is not a reservation of the position.

One physical spell remains on the chain through every execution and any intervening discard, look, target-group, or other mandatory choice. `chain[].executions` is the canonical list; legacy fields describe only the initial and first repeated execution. `resolution_progress` reports `{chain_id, execution, executions}`, with a zero-based execution index while resolving. There is no response window between executions. The spell produces one completed-play event and moves to its final zone only after all its instructions finish. Private choices remain filtered for their owner throughout.

Public `play_grants` identify each delayed effect's controller, source, expiry turn, and typed effect. `play_history` exposes finalized non-token gear plays, gear activations, and per-player/per-lane unit-or-gear plays. These are shared public facts, identical in both observations. They are distinct from completed-play counters and from a live cost quote.

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

This example spends one generated flexible power to hide a card while preserving colored power and runes. The battlefield and card must satisfy the usual hiding rules. Use `validate` to check any proposed allocation. Nested payment wrappers are rejected. Generated power counts (`colored`, `any`, and `spell`) must be integers from 0 to 65535; `recycle` rune counts remain integers from 0 to 255. Both arrays must contain six counts. Every rejected payment preserves game state and revision. Quoted sources are only valid for the current match revision.

## Using abilities during a fixed payment

Use `begin_payment` when the intended action needs resources from Add abilities, especially when consuming a source would change a discount. Select targets and optional costs first. The engine fixes the resulting obligations; subsequent resource actions do not recalculate that cost. For example, using Empowered Gold while paying for Shock Blast preserves the discount established before the Gold leaves.

```json
{"type":"begin_payment","action":{"type":"play_card_targets","hand_index":0,"targets":[{"type":"unit","player":1,"unit_id":7}]}}
{"type":"activate_gear","gear_id":15}
{"type":"complete_payment"}
```

Each action is a separate `apply` request with the current revision and acting player. `begin_payment` cannot wrap another payment, an exact-power wrapper, or `complete_payment`. Quote `complete_payment` after generating the resources, then wrap that completion in `with_power_payment` if exact source selection is wanted.

The top-level `payment_plan` remains available during nested conversions. It contains `player`, fixed `energy`, ordered `power` obligations, a public `pending_card` ID when it moved a card from its source zone, and the normalized `action` for the acting player only (`null` for the opponent). Count `pending_card` once for physical-card conservation; its source zone already excludes it. An instructed card already listed in `pending_card_plays` has no second payment-plan card. The action retains its originally selected source index; it is an intent description, not an index into the temporarily shortened zone. `decision.type:"payment"` also reports `can_cancel`. Targets and choices remain fixed, but the engine checks their legality again when completing. If the attempt cannot complete, `choose` with `accept:false` restores the entire pre-attempt state, including resource abilities used during it and any original mandatory decision. It cannot skip that decision, and a fully payable legal attempt cannot be cancelled.

## Observations and decisions

Observations include phase, turn, active player, priority, winner, mulligan status, public resources, champion slots, units, gear, battlefield control/scoring, showdown state, and the public chain. The last chain entry is the top item, which resolves first after the required passes. Each player object includes hand/deck/rune-deck counts. Domain-indexed arrays use `Fury, Calm, Mind, Body, Chaos, Order`, matching `catalog.domains`. `energy` includes ready rune energy and stored energy; `energy_pool` is the stored portion. `runes` counts all runes on board, and `exhausted_runes` identifies its exhausted subset. `legend.spell_power` is restricted to spell costs.

A pending `payment_plan` exposes `xp` and `xp_exact` to both seats, alongside its frozen Energy/Power obligations. Beginning a payment may stage a cost that is not yet affordable; completion checks the actual balance. Resource Add actions cannot substitute Power for XP. Failed completion leaves the pending state intact; the existing rollback choice restores its original XP balance and positive-gain history along with resources, readiness and choices. Successful completion spends the recorded XP once.

Both seats expose numeric `xp`, decimal-string `xp_exact`, and Boolean `xp_gained_this_turn`. Use `xp_exact` in clients whose numbers cannot represent every 64-bit integer exactly. A positive completed XP gain sets the history flag; zero does not. The flag survives later balance changes and clears for both players at the start of each turn, before Awaken. It is not inferred from the current balance. These are observations, not client-settable fields. An XP sum beyond the host's unsigned 64-bit capacity rejects the entire attempted action with error code `engine_capacity`; state and revision remain unchanged. This is a representation limit, not a Riftbound XP maximum.

Dynamic Might and damage retain their numeric JSON fields and add decimal-string companions. Units expose `might_exact`, `signed_might`/`signed_might_exact`, `damage_exact`, `buff_exact`, and `temporary_buff_exact`. Referenced `might` floors a negative signed value at zero; it has no 255 cap. Computed Assault and Shield include `assault_exact` and `shield_exact`, and observed Might/Assault/Shield passive contributions include `effect.amount_exact`. Printed catalog descriptors remain unchanged. Chain items add `bonus_damage_exact`; combat decisions add `total_exact`. Cost-plan modifiers include `energy_exact`, preserving even a large Might-derived reduction while the applied discount is limited by the actual remaining cost. Submitted combat allocations retain numeric pairs and add `submitted_combat_damage_exact`, with string amounts in the same order (or null before submission).

`assign_damage` keeps its ordered `[unit_id, amount]` pairs. An amount may be an unsigned JSON integer or a canonical unsigned decimal string through 18446744073709551615. Strings contain digits only, with no leading zero except `"0"`; signs, whitespace, fractions, exponents and out-of-range values are rejected. Unit IDs remain numeric. Suggestions use numeric amounts through 9007199254740991 and decimal strings above that threshold, so copying a suggestion through JavaScript preserves it exactly. Allocation legality still requires the actual total and the normal lethal, Tank and Backline ordering rules; the representable maximum is not permission to over-assign damage.

The Rust API uses `numeric::{SignedMight=i64,Might=u64,DamageAmount=u64}`. A unit's signed Might must fit i64; damage and combat pools must fit u64. `try_unit_stats`, `try_lane_might`, and the existing numeric query adapters return `Result`, as do action/candidate queries that depend on them. Wide intermediate calculations may exceed the storage type if their final result fits, including a signed Swap. A true representation failure returns `engine_capacity`; the entire attempted command, pending resolution/payment state, RNG and API revision remain unchanged. Queries report the error without inventing a capped value. These are finite host limits, not game-rule maxima or a claim of arbitrary-precision arithmetic.

Card metadata `entry_ready` is `null`, `{"condition":"always"}`, or `{"condition":"controller_xp_at_least","threshold":3}`. The condition is evaluated at actual entry; entering ready does not perform a Ready action. `xp_energy_discounts` lists `{minimum_xp,energy}` tiers. Concentrate's tiers are 6→2 and 11→4: only the highest eligible tier applies, and the result uses the ordinary cost plan and remains fixed during payment. A passive condition of `controller_gained_xp_this_turn` refers to the public history flag and has no `condition_threshold`.

Both seats expose the same public resource and equipment fields. A gear object's `attached_to` identifies its bearer unit, or is `null` when unattached. Its `lane` follows the bearer; `null` means base and does not by itself mean unattached. `card_id`, `name`, `token`, `ready`, and `temporary` remain visible while attached. `temporary_status:{intrinsic,granted,active}` separates the origin from its current activity. Printed Temporary becomes inactive while attached; Temporary granted by Turn to Dust remains active. The existing `temporary` flag describes its presence, including an inactive printed instance. Battlefield IDs refer to `catalog.battlefields`, which contains the field's official effect text; field abilities on the chain include their lane.

Each player's `activated_abilities` lists the registered abilities of their public live units and gear as `{source, card_id, ability_index}`. This describes printed options, including exhausted sources and abilities that are currently unusable; it is not a list of legal actions. No private hand or deck abilities are exposed. An activated chain item has `ability.type:"activated"` with the same live source identity, immutable `card_id`, and selected `ability_index`, even after that source leaves play. Its outer `chain.source` remains the card-definition reference. The two fields distinguish which physical object used which printed ability.

Player observations expose the actual `legend_id` independently of `preset` (which is `null` for custom lists). The `legend` object also includes its `id`, `official_id`, `name`, `champion`, and `domains`, alongside its existing status fields. These identities and domains are public; an opponent's custom list remains private.

Ezreal's legend object additionally reports `targeting_progress:{"completed":0,"required":2}`. `completed` ranges from zero to two and measures progress toward the prerequisite, not the total number of choices. It is public to both viewers and resets for both players on each new turn. The ordinary `activate_legend` action becomes legal once the prerequisite, Reaction timing and ready state permit it. The draw waits for resolution; it does not expose the card to the opponent. Other legends retain their prior observation fields. Rengar uses the pending `legend_targets` decision and shared `choose_trigger_targets` action; an empty selection is rejected.

Every chain item has an additive canonical `source`: `{"type":"card","card_id":...}`, `{"type":"legend","player":...,"legend_id":...}`, `{"type":"battlefield","lane":...,"battlefield_id":...}`, or `{"type":"unit","player":...,"unit_id":...}`. The unit form identifies the physical bearer that generated Hunt; the other forms identify rules definitions. The older top-level `card_id` remains a main-card art/reference field for compatibility; on a legend ability it is not the legend's identity or its rules definition. Legend chain abilities include `viktor_recruit`, `lux_draw`, `rengar_might` (with its chosen unit target), and `ezreal_draw`. Viktor's ordinary activation uses `activate_legend` without a target; its Recruit destination is selected only after the ability resolves. Darius's Reaction resource ability uses the same action and resolves immediately. Lux is passive and has no activation action.

Attachment abilities identify their origin: `ability.type:"equip"` is the activated Equip ability, while `ability.type:"quick_draw_attach"` is Quick-Draw's on-play triggered attachment. Both carry `gear_id` and `target`. An activated gear ability is observed as used when it resolves, even if its attachment target has become invalid; Quick-Draw is not an activated ability. Resource gear resolves immediately and preserves priority, while any resulting triggered ability still waits on the chain.

A unit's observed `might` is its current total, including equipment and currently applicable combat bonuses. Printed might remains in the catalog. Do not add an equipment bonus to the observed total again.

Each unit also exposes `keywords`: numeric `assault`, `shield`, `deflect`, and `hunt`, Boolean `tank`, `backline`, `ganking`, and `ambush`, plus optional printed `accelerate:{energy,power,domain}`. These values account for token identity, current equipment, level requirements, and granted abilities. Assault and Shield are their keyword values even outside combat; apply no extra bonus to observed `might`. Accelerate describes a play option, not an action that readies a unit already on the board. `damage_priority` reports the automatic assignment preference: `0` for Tank (including Tank plus Backline), `1` for ordinary units, and `2` for Backline alone. A unit with both Tank and Backline may legally occupy either priority group; this field does not restrict the controller to the automatic choice.

For token units and Gold, `token:true` distinguishes the generated object from a physical deck card. Its `card_id` is an engine source/reference identity, so use the observed `name` and `might` for the token instead of assuming that catalog entry's printed card kind or stats describe it.

A player object identifies its `legend_id`, `custom` flag, and `preset` (`null` for custom decks). `custom_deck` contains the saved list only for its owner; opposing observations always omit that list even when a hand reveal is active. The declared deck name and public legend/champion remain visible.

The viewer sees their own hand as card IDs. The opposing hand is `null` except while an actual reveal effect permits it. Deck order and rune-deck order are omitted for both players. Hidden cards expose object ID, location, and hidden turn; `card_id` is `null` unless the viewer is allowed to see it. Granted Flow indices refer to the public trash and are visible to both viewers. Diagnostic game logs, prompts, random-generator state, and match seed are never exported in observations.

`decision` is `null` or an object with `type` and `player`. Other fields describe the current choice:

| Decision | Data and response |
| --- | --- |
| `predict` | Owner sees `card_id`; `choose` recycles it when true, keeps it on top when false |
| `draw_look` | Public source, kind, choice ID and count; only the owner sees top-first `cards` and eligible occurrence indices. Use `resolve_draw_look`. |
| `revealed_card` | Public source, card ID and completed `outcome` (`draw`, `recycle`, or `trash`); acknowledge with `choose` and `accept:true`. This record is not a card zone. |
| `predict_cards` | Public `choice_id` and actual `count`; only owner sees `cards`. Submit the exact index partition and kept order with `resolve_predict`. |
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
| `legend_targets` | Rengar’s mandatory unit choice; `source` identifies the legend, `required` and `maximum` are 1, and `candidates` contains legal targets; submit `choose_trigger_targets` |
| `battlefield_order` | `lanes`, `combat`; the turn player chooses one listed lane using `choose_battlefield`; non-combat showdowns precede staged combats |
| `battlefield` | `lane`; `choose` whether to pay for the battlefield ability |
| `burn_for_might` | `unit_id`; `choose` |
| `combat_damage` | `lane`, `total`, decimal-string `total_exact`; `assign_damage` |
| `death_replacement` | `options:[{unit_id,gear_id}, ...]`; `choose_death_replacement` |
| `order_triggers` | `options:[{id,card_id,label}, ...]`; `order_trigger` |
| `resolution_targets` | `card_id`, `groups` of target objects, zero-based `execution`, legacy `repeated`; select the desired group by its index with `choose_resolution_group` |
| `token_placement` | `card_id`, `remaining`; use `place_token` separately for each remaining token |
| `token_play` | `card_id`, `remaining`, `destination`; use `finalize_token_play`, optionally wrapped in `with_play_choices` and/or staged payment |

For each decision, `suggest` supplies legal convenience responses, while structured actions allow alternate legal choices. Opposing observations omit private Predict and Top Three identities. Public replay/trigger/target decisions remain observable. Pending triggers are distinct from the public chain and cannot be targeted by counterspells until finalized.

Generic card triggers retain a public `context` with `source:{player,unit_id,gear_id}` and an optional event `subject` target. Equipment granted abilities identify their bearer as the unit source. A finalized chain ability has `type:"card_trigger"`, its metadata `rule_index`, and the same context; chosen targets live in the chain item's ordered `targets` array. Target choices finish before response priority opens.

The catalog and observed units expose `tags`, including token tags. Gear observations include their `empowered` status. Player observations distinguish `cards_played_this_turn` (finalized cards, including later-countered spells), `cards_completed_this_turn` (completed card plays), and `cards_drawn_this_turn`. Token plays do not increment either card-play counter. `cards_drawn_this_turn` remains a JSON integer; the always-present `cards_drawn_this_turn_exact` is its canonical unsigned decimal string for clients whose number type cannot represent every 64-bit integer. A successful ordinary or specified-card Draw increments this history. Looking, recycling, burning, and unsuccessful terminal draws do not. A Draw beyond the 64-bit host capacity returns `engine_capacity` and restores the entire attempted command, including earlier instructions, choices and RNG; this limit is not a Riftbound rule.

A unit’s `keywords.hunt` is its current total printed and granted Hunt amount. A Conquer/Hold event generates one ability for that bearer with `ability:{type:"hunt",source:{type:"unit",player,unit_id},amount,source_name}`. Its outer `source` is the same physical unit. The captured amount and name remain valid after equipment or the bearer leaves; a token’s name remains its actual token name. The older outer `card_id` is only a compatible art/reference field.

Token plays created by a resolving effect wait until that effect finishes. `pending_token_plays` exposes their public pending IDs, controller, source `card_id`, Might, `gear`, and `entry_ready` flags in finalization order. `entry_ready:true` means this instructed token play enters ready; it is independent of optional Accelerate and does not perform a Ready action. Existing ordinary creators use false. This is effect-owned metadata, not a client-selectable field on `place_token` or `finalize_token_play`. `destination:{mode:"choose"}` permits the normal base/controlled-field choice; `{mode:"fixed",lane:null}` fixes base and a numeric lane fixes a battlefield. Pending IDs identify chain objects, not selectable board units. Placement choices finalize one token at a time; earlier tokens can be on the board while later placements remain pending. These choices do not grant a response window between token plays.

When a pending unit token can pay Accelerate, `token_play` exposes that additional choice before entry. Inspect `play_choices` for a `finalize_token_play` action to obtain its cost. Declining the optional cost still plays the required token. Gear tokens do not consume next-unit grants and do not acquire unit-only Accelerate.

A `play_chosen` or `revive_chosen` chain ability identifies the previously selected trash card. Its `trash_index` becomes `null` if that card has left the trash before resolution. An `empower_object` ability exposes its selected `player`, `object_id`, and `lane`; `object_id:null` means that player's legend at base.

Fizz and Last Rites ask whether to accept their optional play instruction before placing that ability on the chain. Once an accepted instruction resolves, its trash selection is required if a legal payable card exists. `required_trash_play` identifies that stage; `can_decline_trash_play` reports whether declining is currently legal. A bot should not assume that `choose:false` remains available after accepting the earlier optional play.

## Predict instruction continuations

Clairvoyance (396) and Scryer's Bloom (397) use the existing `predict_cards` decision and `resolve_predict` action. The top-level `predict_continuation` is `null` outside an unfinished Predict sequence. While the choice is pending it is a public object with `chain_id`, `player`, `source_card_id`, `remaining_draw`, `remaining_xp`, and `remaining_xp_exact`. These are printed remaining instructions, not private looked-at cards. Both viewers receive the same object. Only the choosing player receives the decision's card identities.

Clairvoyance first completes Predict 5, then Draw 2. Bloom completes Predict 2, then Draw 1, then Gain 1 XP. `resolution_progress` identifies their retained chain item; Bloom reports execution 0 of 1. Bloom's gear has already entered Trash as its paid Kill cost, but its exact ability source remains on the chain. Its completed gear-use event occurs after the final instruction. The existing `play_history.gear_activations` counts finalized gear abilities, so it has already advanced while Predict is pending; it must not be interpreted as a count of completed uses. A repeated Clairvoyance completes its entire current sequence before the next execution inspects a fresh deck. The physical spell does not enter Trash between executions.

An empty Predict creates no private decision and proceeds to the later instructions, so the later Draw can Burn Out. Rejected partitions, stale choice IDs and deferred numeric-capacity failures preserve both observations and the revision. There is no response window inside this sequence. Ornn, Blacksmith (395) instead uses `draw_look` with `kind:"ornn"`, top-four occurrence indices, Gear-only eligibility and the existing public `revealed_card` Draw acknowledgement.

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

## Ordered targets, Temporary and Sprite identity

`target_candidates` accepts an incomplete play draft and zero-based `execution`: 0 is the original, 1 onward follows the draft's selected Repeat order. It accepts structured `action`, not `action_id`. Cost/payment/play-choice wrappers use the same parser as `apply`. During payment, `action:{"type":"complete_payment"}` queries the stored intent, including its detached physical card. The current actor and revision are required.

```json
{"command":"target_candidates","revision":4,"player":0,"action":{"type":"play_hidden","hidden_id":9,"targets":[{"type":"unit","player":0,"unit_id":7}]},"execution":0}
```

The result contains `card_id`, `origin`, `execution`, the original ordered `targets`, `spec:{kind,maximum,required}`, `next_slot`, and `candidates`. An incomplete slot reports `{index,hidden,different_location_from}`. `hidden` is null, `{type:"at_battlefield",lane}`, or `{type:"outside_battlefield",lane}`; `different_location_from` is a preceding slot index or null. A valid fully filled draft has `next_slot:null` and no candidates. Invalid prefixes or execution indices return an error without changing state. To replace an earlier choice, query a draft truncated before that slot, rather than compacting later targets into it. Final `validate`/`apply` still check timing, complete choices and payment.

Fading Memories uses `unit_or_gear`: units must be at a battlefield, while gear may be anywhere. Shadow's Call chooses a friendly unit without Temporary. Smoke and Mirrors chooses friendly units at different locations; from Hidden, slot 0 must be at the original field and slot 1 uses the different-location exception. Temporary is a condition for Smoke's movement, not its target selection. Each original slot retains its role if another target later leaves.

Units and gear expose `temporary_status:{intrinsic,granted,active}`. The characteristic exists when intrinsic or granted is true. Attached gear's intrinsic Rules-panel expiry is inactive; a granted expiry remains active. A queued expiry has `ability:{type:"temporary",source,source_name}` and a physical `chain.source` of `unit` or `gear`, with its controller/object ID. The captured name survives departure. This replaces the old generated `triggered/DestroyGear` expiry representation; Rules-panel gear triggers also expose their actual `gear` source instead of a catalog-only `card` source; their existing context, payload and numeric exact fields retain their meanings.

Each catalog card has `created_token:null` or `{name,might,kind,domains,tags,deflect,temporary,count}`. Individual `pending_token_plays[].token_identity` uses the same identity without creator `count`. Tokens are domainless; they do not inherit their creator's name, domains, tags or abilities. Sprite is a 3-Might Fae unit with Temporary. Entry readiness and destination remain on the pending instruction. `token_play` and `token_placement` decisions include the exact `token_id`; use it to key client drafts through payment and between identical children. Token fields cannot be overridden by action input. Suggested plays omit redundant ignored unit targets for targetless Hidden cards such as Zhonya’s Hourglass; the targetless play remains available. Suggestion IDs are revision-local, so clients must use fresh results rather than persist an old index.

Passive contributions add `computed_might_exact:string|null`: the exact evaluated Might contribution, zero for an inactive Might expression, null for a keyword. Petal Pixie's descriptor remains `{type:"might_per_friendly_temporary_here",amount:1}` while the evaluated count updates with the board. Existing fixed effects keep their prior descriptors. The legacy unit `buff` numeric field also stores token base Might; use live stats rather than treating that field as a Buff-counter count.


### Gear interaction targets

`trigger_targets` decisions now include an additive `target_rule` object: `kind` (`unit`, `gear`, or `unit_or_gear`), `allegiance`, nullable `maximum_printed_energy`, and `energy_at_most_live_source_might`. Use the returned physical `candidates` and submit `choose_trigger_targets` with ordered targets. A gear target is `{"type":"gear","player":1,"gear_id":700}`; its ID is distinct from its card definition and bearer. An optional trigger reports `required: 0`, allowing an empty list to decline. The normal revision checks and atomic validation still apply.

Detonate/Brittle Steel play queries report `gear`; Decree of Unity reports `unit_or_gear` and restricts candidates to enemy Chaos objects. Gold and unit tokens use their own domainless, zero-Energy identity. `target_candidates`, `quote_payment`, and `apply` share those rules. Gear-trigger observations expose printed Energy restrictions separately from the live candidate list; bots must re-query after state changes.

### Return-to-hand and captured death locations

Factory Recall uses an ordinary `gear` play target, including attached gear. Downwell and Angler Beast do not choose targets. Returning a physical card moves it to hand; a token ceases to exist. These transitions do not trigger Deathknell. Unit returns detach equipment, while Downwell returns the original set of both units and gear exactly once.

Trigger `context` now has additive nullable `departure_location`, captured before a dying source leaves. Locations are `{"type":"base","player":0}` or `{"type":"battlefield","lane":1}`. `target_rule` adds nullable `here_basis` (`live_source` or `source_departure`) and `location`. Mournful Spirit uses `source_departure`; ordinary Here abilities keep following their live source. Null does not mean either base.

The existing trigger `candidates` list enumerates physical objects of the required type. It does not assert allegiance, location, Energy, slot legality or affordable Deflect. Use the metadata to present choices and `validate` on the complete `choose_trigger_targets` action before submitting it. Mournful Spirit allows an empty list to decline. A chosen object that leaves the remembered location before resolution is no longer a legal target. Opponents receive the same public location metadata, without private-card information.

Retreat’s target query accepts friendly units in base or at a battlefield. A legal return supplies its owner to the linked exhausted Channel, including when the unit is a token; a missing target supplies no owner and does not Channel. Wind and Ghosts accepts units at battlefields without a Might cap. Its Banish/return branch uses Might when resolving. Direct Banish is visible in both players’ public `banish` arrays and does not produce a death-replacement decision. Trigger target metadata now also exposes the boolean `target_rule.battlefield` restriction. Apothecary’s on-play decision has optional friendly battlefield targeting and permits its own newly entered unit.


### Banish-origin instructed plays

Portal Rescue, Temporal Breach, Thrill of the Hunt and Arcane Shift use the existing `pending_play` decision and `finalize_pending_play` action. Their `kind` values are `portal_rescue`, `temporal_breach`, `thrill_of_the_hunt` and `arcane_shift`. The decision and its `pending_card_plays` row additionally expose `origin:{type:"banish"}` and `instruction:{base_cost_waiver:"all",destination:...}`. Destination is `{mode:"base"}`, `{mode:"fixed",lane:null|0|1}`, `{mode:"any_battlefield"}` or `{mode:"normal"}`, respectively. These additions apply only to the new Banish-origin cases; the three older Trash kinds retain their wire shape.

The child is already absent from Banishment while pending. Count it exactly once in `pending_card_plays`, including while paying additional costs; quotes report Banish origin and a zero printed base cost. Optional/additional costs remain payable. The unit's owner receives this mandatory decision, including an opposing Temporal Breach target. `choose:false` is legal only when finalization is impossible and restores the child to Banishment, leaving completed parent effects intact. Tokens disappear and do not create a pending card. Prescribed battlefield locations permit enemy-held fields; Arcane Shift uses ordinary placement restrictions.

Arcane Shift finishes its separate enemy-field damage and self-Banish before finalization of the child. A missing friendly target does not suppress the damage, and a missing enemy target does not suppress the replay. Its departure from the chain stops later Repeat executions. Qualifying Ezreal target choices still count when finalized, even if a later Repeat never executes.

### Clocktower Guardian’s optional Banish play

`banish_play` is an optional decision with public `player`, `card_id`, `banish_index`, `kind:"clocktower_guardian"`, `origin:{type:"banish"}`, `optional:true` and `base_cost_waiver:"energy"`. The card is already in that player’s public Banish array. Submit `choose` with `accept:false` to leave it there, or `accept:true` to select that exact physical card for play. A stale origin position rejects the attempted acceptance atomically. The opponent cannot answer the controller’s decision.

Acceptance produces the existing `pending_play` decision, now with kind `clocktower_guardian`, Banish origin, and `instruction:{base_cost_waiver:"energy",destination:{mode:"normal"}}`. Use `finalize_pending_play` and the existing targeting, `play_choices`, payment quotation and resource-payment commands. Base Energy is waived for any main-deck card kind; Power and additional costs are preserved. A failed finalization follows the existing mandatory-play cancellation proof and returns the child to Banish. This instruction does not recycle resolved spells. The earlier Fizz/Last Rites/Matriarch wire shapes remain unchanged.

Guardian’s play and Hold effects use the existing private `predict_cards` / `resolve_predict` protocol, with up to three actual cards. Opponents receive only the count and choice ID; no card identities are exposed before the Deathknell banishes a card publicly.

### Deck selection, public reveals, and linked Empower

`deck_look_play` is an optional owner-private choice. Both observers receive `player`, `choice_id`, `source_card_id`, `kind`, `count`, and Hook's public `killed_might` / `killed_might_exact` reference. Only the owner receives `cards` (top first) and `eligible` occurrence indices. Submit `resolve_deck_look` with that `choice_id` and an eligible `index`, or explicit `index:null` to recycle all. Duplicate card IDs remain separate occurrences. Wrong/stale choices reject without changing the state or revision. These viewed cards are still in the deck; do not add them again when counting physical cards.

Selection banishes the exact occurrence, moves a permitted child play to Pending, and recycles the other looked cards in random order. Parent instructions, including later Repeat executions, finish before any child finalizes. Hook and Aurora waive base Energy and Power, while extra costs remain payable. Reinforce and Wild Claw instead expose `instruction.total_energy_discount:5`, alongside `base_cost_waiver:"none"`; this is a total-cost modifier and can reduce additional Energy costs. The regular `play_choices`, `quote_payment`, exact payment, and staged payment commands apply. Old instruction shapes omit this field when no such discount applies.

Aurora emits `revealed_deck`, with public `source_card_id` and `cards`. Its first revealed unit has already moved to Banish/Pending, and the other cards have recycled; the array is an inspection record, not an additional zone. `choose` with `accept:true` continues. An empty deck causes no draw, burnout, or reveal acknowledgement.

Wild Claw emits `empower_played` after the child finalizes. Both observers receive the source, pending link, and fixed `target` (Unit/Gear, or null if no child entered). `choose` accepts or declines the separate reflexive ability. Acceptance creates `empower_linked` on the chain, allowing normal responses. It does not choose a new target; its physical object link survives movement but cannot follow a card that leaves the board. Empower is permanent unless another effect removes it.

## Inspected draws and Move reveals

Double Trouble, Fate Weaver and Trifarian Captain produce `draw_look`, with `kind` respectively `double_trouble`, `fate_weaver` or `trifarian_captain`. Both seats receive `player`, `choice_id`, `source_card_id`, actual `count` and `optional:true`; only the owner receives `cards` and `eligible`. Cards are still in the deck while inspected. Duplicate IDs have separate zero-based occurrence indices. Send `{"type":"resolve_draw_look","choice_id":123,"index":1}` to select that eligible occurrence, or `index:null` to decline and recycle all. Omitted, invalid, ineligible, stale or wrong-player selections reject atomically. Fate Weaver and Captain have mandatory on-play triggers; the draw choice is optional only when those triggers resolve.

Selecting an occurrence publicly reveals it, draws that exact card, and recycles the other inspected cards together in random order. It then produces `revealed_card`, exposing only the selected identity. The card is already in hand; count it there once, not again in the decision. Acknowledgement finishes this execution before a paid Repeat begins a fresh look. Declining never reveals the private remainder or creates a Draw event. A short or empty look does not burn out.

Apprentice Smith, Raving Witness and Pakaa Protector reveal the current top card when their Move trigger resolves. Their `revealed_card` record has already completed its reported movement. Smith draws Gear and otherwise recycles; Witness draws Units with printed Might at least five and otherwise recycles; Pakaa draws Units and otherwise trashes the card and gives its surviving source +2 Might this turn. An empty deck creates no reveal record, Draw, burnout or Pakaa bonus. These effects follow actual Move events, not entry, Ready or Recall.

`choose` with `accept:true` acknowledges a public `revealed_card`; false is invalid. No response window is added by this inspection step. The desktop pauses either seat's public reveal for the human to inspect, while private opponent choices remain hidden. Lightning Rush retains its existing `top_three`/`select_cards` protocol and does not reveal its chosen card; its selected Draw now notifies ordinary Draw observers before the other inspected cards are trashed.
