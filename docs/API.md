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

The repository's `assets/data/full_catalog.json` is a separate inventory with schema version `2`; it is not the bot protocol or the playable `catalog` response. It preserves raw printing fields and adds reviewed `effective` facts, resolution references, and unresolved-field diagnostics. Deck validation uses those effective facts and rejects unresolved gameplay or variant fields, including conflicts on alternate printings. Source review alone never registers a playable card. Inventory-only identity `kind` preserves complete type sets as sorted `+`-joined strings, such as `gear+unit`; the corresponding printing’s `effective.types` retains both types. This does not add a playable bot-protocol card kind. Main-card IDs are append-only; the latest additions are 435–440. Existing IDs and sourced preset lists retain their identities, while eligible-card and chosen-champion lists expand with supported cards. Additive Temporary status, token identity, evaluated passive values and target queries retain protocol version `1`. See [the inventory review contract](CATALOG_REVIEW.md) for evidence and remaining variant-review limits.

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
| `start` | `revision` | Begin play; standalone games implicitly keep unused mulligans, registered matches require both players to submit first |
| `suggest` | `player` | `{exhaustive:false, actions:[{id, action}, ...]}`; empty when this seat cannot act |
| `quote_payment` | `revision`, `player`, exactly one of `action` or `action_id` | Suggested and available power sources, exact Power obligations; no mutation |
| `play_choices` | `revision`, `player`, exactly one of `action` or `action_id` | Available Repeat instances, Accelerate/bonus costs, original zone, and typed cost plan |
| `target_candidates` | `revision`, `player`, structured `action`, `execution` | Pure action-aware target view for the next unfilled slot; see below |
| `trigger_target_candidates` | `revision`, `player`; optional ordered `targets` prefix | Pure next-slot choices for pending trigger/legend targets; includes `can_confirm` |
| `cost_plan` | `revision`, `player`, exactly one of `action` or `action_id` | Cost components, modifiers, applied discount order, final Energy, typed Power and XP obligations |
| `validate` | `revision`, `player`, exactly one of `action` or `action_id` | `{legal:true}`, or an error; no mutation |
| `apply` | `revision`, `player`, exactly one of `action` or `action_id` | Apply one legal action and return public summary |
| `step_ai` | `revision`, `player` | Ask the built-in AI to take one action for the seat with priority |

Reset defaults match the initial settings. `decks` is a pair of preset IDs. `battlefields` is an optional pair of catalog battlefield IDs, one contributed by each player; omit it for preset defaults. A battlefield catalog ID is different from its in-match lane index. Starting fields occupy slots `0` and `1`; card effects can append more slots during play. Invalid IDs are rejected. `difficulty` is `easy`, `tactical`, or `strategic`. Strategic uses bounded public-information lookahead; it is not an expert-strength guarantee. There is no automatic mulligan for either seat through this API.

A custom list can replace either seat through `custom_decks:[deck_or_null,deck_or_null]`. Omission uses presets for both seats. Each deck object contains `name`, `legend`, `chosen_champion`, `cards` (pairs of card ID and count), `runes` (six counts in catalog domain order), and `battlefield`. `legend` accepts a stable numeric registry ID or its official printing ID, such as `"ogn-265-298"`; saved/output decks normalize it to the numeric ID. Existing numeric IDs0–3 retain their identities. The selected chosen-champion variant does not change the legend.

`catalog.legends` is independent of `catalog.presets`: eleven legends are supported and the seven sourced preset lists have separate stable IDs. Every legend reports its official identity/text, domains, default chosen champion, `eligible_cards`, `eligible_champions`, and a complete editable `training_deck`. For Darius, Viktor, Lux, Ambessa, Rengar, Ezreal, and Riven those are authored practice lists, not exact tournament registrations; `training_note` describes the origin. Pass the returned training deck through `custom_decks` to practice with that legend. `CustomDeck::from_legend` provides the same starting list in Rust; `from_preset` still copies an exact sourced preset. `catalog.presets[].legend_id` links each preset to the registry. A new legend ID must not be used as a preset ID in `reset.decks`.

Deck validation requires at least 40 main-deck cards including the chosen champion, no more than three copies by full name, at most three Signature cards in total, matching champion tags, complete domain compatibility, exactly 12 legal runes, and an implemented legal battlefield. A `Unique` card is limited to one copy. Only implemented Standard-legal cards and legends can be selected. Zero-count rows should be omitted. Invalid decks are rejected before resetting the live match. Optional reset `battlefields` overrides the decks' selected fields. The API leaves both opening hands untouched for each agent's mulligan.

`validate_deck` and `validate_registration` accept an optional `format_date` in
`YYYY-MM-DD` form. This adds explicit date-specific checks against the accepted
catalog and announced **global Standard/1v1** bans. Dates before the catalog's
2026-10-24 boundary are rejected. For example:

```json
{"command":"validate_registration","registration":{"preset":5},"format_date":"2026-11-22"}
{"command":"validate_registration","registration":{"preset":5},"format_date":"2026-11-23"}
```

The first succeeds and includes `result.format` with the review date, scope and
upcoming restrictions affecting the list. The second rejects the registered
Stellacorn Herder copies under the [announced November 23 bans](https://playriftbound.com/en-us/news/announcements/radiance-ban-list-updates-effective-november-23-2026/).
Registration checks include the entire sideboard and all three battlefields.
Both requests are read-only and preserve the session revision and game state.

`reset` and `series_begin` also accept `format_date`. They reject a banned list
before changing the session; single-game validation includes the actual
`battlefields` overrides. The selected date is retained in response summaries
and each player's observation as `format: {scope, as_of}` throughout the match.
No card is substituted. A rejected reset preserves the previous match and date.

The engine retains that date in each game, including later games in a registered
match and rematches. Strategic AI samples unknown cards from the corresponding
public legal pool, without consulting private registrations. Rust callers can
use `Game::choose_format_date(Some("2026-11-23"))` before play and read
`Game::format_date()`. This checks both actual decks and selected fields; later
battlefield choices must remain legal. Changing the date after play starts is
rejected. `None` explicitly selects the accepted snapshot before play.

Omitting the date selects the accepted catalog snapshot and preserves the older
response shape. Native date selection and automatic current-format selection
remain unfinished. The schedule describes known announcements, not a guarantee
about later unannounced changes. Regional release exceptions are recorded in the
source manifest and are not assigned the global effective date.

Use observation `priority`, not `active`, to choose who acts. An opposing player can have priority while the active player owns the turn. Pending decisions can also transfer priority. `step_ai` works for either seat and processes one choice/action, not a whole turn.

Repulse (card442) has ordered, dependent targets. Its `play_targets.ordered_slots`
metadata describes a friendly battlefield unit followed by an enemy spell or
ability choosing it and no other friendly unit. Query `target_candidates` with
`execution:0` and the current target prefix: the next slot changes from `unit`
to `spell`. Supply both targets in that order to confirm. Candidate queries and
payment previews do not spend resources. References from all Repeat executions
count; movement between board locations preserves a targeting relationship,
while departure to a non-board zone breaks it.

Suggestion IDs such as `r4:p0:a2` are opaque and valid only for that seat and revision. Do not cache them across actions. Structured actions are not restricted to the suggestion list:

```json
{"command":"apply","revision":4,"player":0,"action":{"type":"play_card_targets","hand_index":0,"lane":null,"targets":[{"type":"unit","player":1,"unit_id":7}]}}
```

## Actions and targets

Each action is an object with a `type`. Unknown fields are errors, so misspellings cannot silently change a choice. Indices are zero-based and must be refreshed after a mutation. Unit, gear, hidden, chain, and trigger IDs identify live objects; they are not card catalog IDs. `lane:null` means base; a nonnegative `lane` indexes the current observation's `battlefields` array. Optional `lane` and `target` may be omitted. Optional boolean flags default to `false`, and omitted `targets`/`repeat_targets` default to empty arrays.

Targets have exactly one of these forms:

```json
{"type":"unit","player":1,"unit_id":7}
{"type":"gear","player":0,"gear_id":12}
{"type":"spell","chain_id":5}
{"type":"battlefield","lane":1}
```

A battlefield target identifies a zero-based physical slot in the current observation's `battlefields` array, independently of its controller or catalog definition. Games start with slots `0` and `1`; an effect adding a battlefield appends another slot. Two slots using the same battlefield card are distinct targets. It has no `player` or `battlefield_id` field, and cannot select a base. A spell's battlefield target belongs in `targets`; the action's outer `lane` keeps its existing deployment/movement meaning and is not an alias. A well-formed index for a nonexistent slot is an `illegal_action`; malformed indices are an `invalid_request`. Rejected actions preserve the state and revision.

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
| `refuse_payment` | none | Decline a supported finalization cost, undo its payment actions, and finish that refusal |
| `activate_legend` | optional `target` | Activate the selected legend's supported ability |
| `empower_unit` | `unit_id`, optional `pay_power` | Activate Legion Marauder or Tail-Cloaked Matriarch's Empower ability |
| `hide` | `hand_index`, `lane` | Pay to hide a Hidden card at a controlled battlefield |
| `hide_champion` | `lane` | Hide a chosen champion with Hidden |
| `play_hidden` | `hidden_id`, optional `targets` | Play an eligible hidden card; an empty target list uses the engine's default targets |
| `choose` | `accept` | Accept/decline a binary decision; Predict accepts recycling the top card |
| `resolve_predict` | `choice_id`, `recycle`, `top_order` | Complete a multi-card Predict; the two index arrays partition the looked-at occurrences, with `top_order[0]` drawn next |
| `resolve_draw_look` | `choice_id`, `index` | Reveal and draw one eligible inspected occurrence, or use explicit `index:null` to recycle all |
| `resolve_trigger_payment` | `choice_id`, `pay` | Accept or decline the trigger's cost at the decision's explicit `timing`: `finalization` or `resolution` |
| `choose_target` | optional `target` | Select a pending unit/gear target; during Tornado Warrior's base choice, `null` chooses the legend |
| `choose_trigger_targets` | `targets` | Finalize an ordered target array for a `trigger_targets` decision; an empty array declines an optional trigger |
| `choose_trigger_mode` | `choice_id`, `mode` | Choose the zero-based mode of a pending `trigger_mode` occurrence before targets and responses; later instruction costs wait for resolution |
| `equip_exhaust_unit` | `gear_id`, unit `target`, `exhaust_unit_id` | For equipment offering this cost, exhaust a ready friendly unit instead of paying its printed Equip Power; only the bearer is targeted |
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

Rengar, Pouncing (card411) uses the existing card-play actions. Its Reaction permission and destination permission are separate: it may enter the battlefield where its controller is the designated attacking player in combat, including after that player's last unit has left. An unrelated field or a merely staged showdown does not qualify. Validate each complete action with its intended `lane`; cost and choice metadata alone does not establish legal timing or placement. Suggestions need not enumerate every legal destination.

Ezreal, Dashing (card412) uses `activate_ability` index0 with an empty target array to pay one Mind Power and Move its physical source to Base. The Action ability enters the chain and does not Exhaust or Ready the unit. Its Action timing does not change the unit card's ordinary play timing. Attack/Defend triggers use `choose_trigger_targets`; the chosen enemy must still be Here and the source must still exist when the trigger resolves. Damage uses the source's live Might at resolution, so responses can change it. These interactions introduce no new action type.

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

`catalog.cards[].passive_rules` describes the supported live passives with `rule_index`, `condition` (`always`, `empowered`, `not_empowered`, `attacking_with_another`, `combat_alone`, `buffed`, `other_friendly_here`, `controller_runes_at_least`, `controller_xp_at_least`), `condition_threshold` (a number for rune/XP thresholds; null otherwise), `recipients` (`self_unit`, `friendly_units`, `friendly_empowered_units`, `other_friendly_units_here`, `other_friendly_units`, `friendly_units_with_tag`, `friendly_token_units`, `attached_bearer`), and a typed `effect`. An effect is either `{type:"might",amount}` or `{type:"keyword",keyword,amount?}`. Ganking and Vision have no numeric amount. A `friendly_units_with_tag` rule additionally includes `recipient_tag`, matched against the actual unit or token tags. Observed units retain authoritative `might`, `keywords`, and `empowered`; their additional `passive_contributions` list reports each physical source, card/rule identity, active flag, and effect. Inactive conditional benefits are explanations, not current keywords or Buff counters. Two copies have separate source IDs and can contribute separately.

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

- `repeats`: an ordered array of `{option_id, choices:{targets, mode, destination}}`. Each option can be paid at most once. Targets, supported modes and Move destinations are chosen separately for each execution. Omit `mode` or use `null` for non-modal spells; discover supported modes in `play_choices`.
- `accelerate` and `bonus`: optional booleans, defaulting to false. The descriptor's label identifies the relevant printed additional cost, including Nami's optional payment.
- `exhaust_unit`: optional physical unit ID. Meditation requires it when `bonus:true`; Life from Stone always requires it. The unit must be ready and controlled by the acting player. It pays a cost and is not included in `targets`.
- `cost_order`: an ordered array of `{modifier_id, use_power, power_allocations}`. `use_power` selects Power for a modifier that explicitly allows Energy or Power; otherwise it must be false. `power_allocations` contains `{component_id, amount}` entries to distribute that modifier's usable Power reduction across eligible components.

Missing arrays are empty. Declining Repeat still consumes a next-spell grant when the spell's play begins. Do not combine canonical Repeat selections with the legacy `repeat:true` adapter. `begin_payment` can contain a `with_play_choices` action so the selected costs and grants remain fixed while resources are generated. `with_power_payment` around the chosen action selects the actual payment sources; this is separate from choosing which cost a discount reduces.

Cards with an exhaustion cost add `unit_cost:{type:"exhaust_unit",optional,candidates,is_target:false}` to `play_choices`. Candidates are physical IDs of ready friendly units, including unit tokens. This field is absent for other cards. Incomplete previews may omit the unit; finalization cannot. A selected unit appears in `costs.object_costs` and exhausts when payment completes, before opponents respond. It does not trigger abilities that observe spell targets. Repeat repeats the draw/channel instructions without exhausting another unit. Countering the spell leaves its costs paid. Clockwork Keeper's optional Calm Power cost uses `bonus`; its errata creates a separate draw trigger after the unit enters, including when a discount reduces that optional cost to zero.

Field-targeting spells use the same per-execution target arrays. For example, Firestorm's base play can target slot `0`, one paid Repeat slot `1`, and another slot `0`. Every execution derives its affected enemy units from the current board. Payment, Flow, and chain observations preserve those distinct field choices. An explicitly empty array does not choose a field automatically; the legacy `play_card` default-target action remains available when automatic selection is desired.

Charm, Ride the Wind and Siren's Song choose a unit Move destination separately
from their unit target. The original execution uses the action's `lane` (`null`
means the unit's base). Each Repeat can provide
`destination:{"type":"base"}` or
`destination:{"type":"battlefield","lane":0}` (any existing battlefield slot). Omission inherits
the original destination for older callers; explicit `null` is invalid. Other
cards reject this field. Every destination must be legal at finalization, before
any execution resolves. `target_candidates` returns the selected execution's
`destination` and filters targets against it; finalized `chain[].executions`
contains an explicit destination for every Move execution. Payment wrappers
preserve these choices.

Units expose public `moved_this_turn` history. Siren's Song (427) advertises the
target condition `moved_this_turn`, which is rechecked on resolution. Forced
movement counts; entry, recall and prevented movement do not create history.
A moved unit with Hunker Down remains eligible, but the new Move is prevented.

Back Off (428) uses the existing play and Hidden actions. The public chain's
`play_origin` determines its independent draw: `hand` draws one per execution;
`hidden`, `trash` and `banish` do not. This remains true for Repeat and for
instructed plays that waive costs. Losing the target prevents its Stun but does
not cancel a hand-origin draw. Hidden keeps its usual field targeting and later
Reaction permission; it waives the base cost, including neither Deflect nor paid
Repeat costs. Before revelation, opponent observations keep its hidden `card_id`
null. The standard unit `stunned` field reflects resolution without changing
`ready` or Might.

`cost_plan` accepts selected card plays and supported Equip, Empower, or registered ability actions. It returns nominal `components`, applicable `modifiers`, the actual `applied_order`, and final `energy`, `power`, `xp`, and decimal-string `xp_exact`. Every component and optional cost also includes `xp` and `xp_exact`, including zero. XP is a fixed obligation determined by the source, and Energy/Power discounts do not reduce it. Each Power obligation retains its allowed domains and `spell_eligible` flag; totals alone cannot represent different colored costs. Modifier scope is either `total` or `component` with a `component_id`. Cost increases apply before discounts. Component discounts precede total discounts; explicit cross-stage reversal is rejected. Within a stage, the player may order reductions, including reductions with a local minimum. A minimum applies only to that discount, not as a permanent floor on the resulting cost. An omitted order uses a deterministic legal order.

Explicit Power allocations must name distinct eligible components, fit their remaining cost, and allocate the entire usable reduction. A selected optional cost remains paid when discounts reduce it to zero. Invalid option IDs, duplicates, unsupported modes, and inconsistent allocations reject without changing the match. These queries require the acting seat and current revision; a preview is not a reservation of the position.

One physical spell remains on the chain through every execution and any intervening discard, look, target-group, or other mandatory choice. `chain[].executions` is the canonical list; legacy fields describe only the initial and first repeated execution. `resolution_progress` reports `{chain_id, execution, executions}`, with a zero-based execution index while resolving. There is no response window between executions. The spell produces one completed-play event and moves to its final zone only after all its instructions finish. Private choices remain filtered for their owner throughout.

Public `play_grants` identify each delayed effect's controller, source, expiry turn, and typed effect. `play_history` exposes finalized non-token gear plays, gear activations, and per-player/per-lane unit-or-gear plays. These are shared public facts, identical in both observations. They are distinct from completed-play counters and from a live cost quote.

`play_history.finalized_spells` contains two arrays of physical chain IDs, one
for each player who finalized the spells this turn. Resolved or countered spells
remain recorded. Repeat adds no new entry, while an actual replay does. Both
arrays reset at the next turn. Queries and staged payments do not add entries.
Crumbling Sands (456, `ven-039-166`) can choose any spell, including a friendly
one. On resolution it counters only if an opponent finalized another spell
this turn, excluding the chosen spell's chain ID. That condition does not remove
the play from legal actions. Tactical and Strategic policies can decline an
unproductive cast without changing its legality.

Public `turn_effects` lists global temporary rules. Unyielding Spirit (card 441)
adds `{type: "prevent_spell_and_ability_damage", source_card_id: 441,
scope: "both_players", amount: "all", expires: "end_of_turn", expires_turn,
prevents_unit_damage: false}` only after it resolves. Both viewers receive the
same entry, including when later units enter. The array becomes empty when the
effect expires after ending triggers. Multiple resolutions do not stack separate
finite shields. This does not counter spells, erase marked damage, stop combat,
or prevent damage explicitly dealt by units such as Challenge. Other instructions
still resolve. Playing this Reaction requires no targets and uses the existing
`play_card_targets` action with an empty `targets` array.

Gear observations include public `attached_this_turn`: true only when that
physical equipment actually attached to its current bearer during this turn.
Paying Equip or queuing it does not set the flag; losing the bearer clears its
meaning, and the history expires at turn end. Brutalizer (444) exposes the
`bearer_attachment_this_turn` passive condition. Hand Hammer (445) exposes
`bearer_exactly_one_other_friendly_unit_here`. Both contribute an additional
2 Might when active, separate from their printed 1 Might bonus. Live unit
`passive_contributions` identify the exact gear copy, current condition and
`computed_might_exact`, including inactive contributions. Friendly tokens count
for Hand Hammer; base units and enemies do not. These identities remain in the
unpublished development batch until its acceptance work is complete.

### Alpha Strike choices (development)

Alpha Strike (443) chooses its friendly source first, followed by distinct enemy
units at battlefields. Query `target_candidates` after each choice; the maximum
depends on the source's current Might. Each selected Repeat has its own targets
in `with_play_choices`, but the spell remains one physical chain item.

During resolution, both observers see `decision.type: "split_damage"`, its
acting `player`, original legal `targets`, `total_exact`, `required_recipients`,
and `lethal_first: false`. Submit `assign_damage` with `[unit_id, amount]` pairs.
Use canonical decimal strings for amounts beyond safe JSON integer precision.
Every retained recipient requires a positive amount, and the sum must equal
the total. Reduced Might can require choosing a smaller subset of the original
recipients. No combat damage ordering applies.

A second split decision can follow the first without a response window or
intermediate lethal cleanup. Use the new revision for every submission;
replayed, wrong-player or invalid allocations leave the session unchanged.
Only after the whole parent finishes do actual deaths create separate public
chain abilities `{type: "xp", amount: 1}`. `order_triggers` can precede their
finalization. Each ability has a distinct chain ID and its own response window;
the player's XP changes when that ability resolves. The
[development audit](ALPHA_STRIKE_AUDIT.md) records the Repeat interpretation,
tests and remaining acceptance work.

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

Each action is a separate `apply` request with the current revision and acting player. `begin_payment` cannot wrap another payment, an exact-power wrapper, `complete_payment`, or `refuse_payment`. Quote `complete_payment` after generating the resources, then wrap that completion in `with_power_payment` if exact source selection is wanted.

The top-level `payment_plan` remains available during nested conversions. It contains `player`, fixed `energy`, ordered `power` obligations, a public `pending_card` ID when it moved a card from its source zone, and the normalized `action` for the acting player only (`null` for the opponent). Count `pending_card` once for physical-card conservation; its source zone already excludes it. An instructed card already listed in `pending_card_plays` has no second payment-plan card. The action retains its originally selected source index; it is an intent description, not an index into the temporarily shortened zone. Targets and choices remain fixed, but the engine checks their legality again when completing.

The plan also exposes `refusal_kind` and `can_refuse` to both seats. The kind is `"ordinary"`, `"pending_card"`, `"pending_token"`, `"trigger"`, `"instruction"`, or `null`; it describes the fixed cost scope, not immediate availability. Emperor’s Dais also has its nonstandard return-unit base cost, described below. It remains populated during nested resource instructions, while `can_refuse:false` prevents interrupting those instructions. `decision.type:"payment"` reports both `can_cancel` and `can_refuse`. These fields expose no rollback snapshots or private historical cards.

During a payment decision, `choose` with `accept:false` undoes the payment attempt and restores the original decision, including Add actions performed inside the plan. This is permitted for a positive Energy/Power finalization cost even when fully funded, and retains the existing undo path when completion is impossible. For an ordinary card or activation, undo and refusal have the same outcome. For a pending card, token, or trigger, undo returns to its original choice without consuming it.

`refuse_payment` instead ends the selected finalization after rolling back the attempt. A pending card leaves the chain and returns to its exact recorded origin occurrence; a pending token disappears without creating an off-chain card; a trigger is consumed without creating a finalized ability. Already completed parent effects and unrelated pending work remain intact. Reserved grants and direct Add actions in the pending finalization's rollback boundary are restored. The ordinary resource-cost rule requires positive frozen Energy/Power: zero costs and sole-XP obligations cannot be refused under that rule. Emperor’s Dais separately retains an optional nonstandard return-unit base cost, so its trigger can still be declined if its Energy component is reduced to zero. For `refusal_kind:"instruction"`, refusal skips the dependent effect of the resolving trigger and preserves completed Add actions; Undo still restores the staged attempt. CounterTax retains its separate choice and does not gain finalization refusal. Missing plans, wrong seats, stale revisions, malformed fields and premature refusal during nested Add reject atomically.

## Observations and decisions

`combat_resolution` is `null` outside the completion of an assigned combat. While combat effects finish, it reports public `{lane, attacker, step, attackers_recalled, result}` identically to both seats. `step` names the **next** outstanding task: `cleanup`, `determine_result`, `establish_control`, or `end`. Damage and cleanup effects finish before `determine_result`; control and Conquer effects finish before `end`. `result` is initially `null`, then `{type:"winner", player}` or `{type:"no_result"}` once captured. Later responses do not rewrite that captured result. Combat ends automatically after its remaining effects finish, or the game may end earlier through victory.

`showdown` can remain present during damage assignment and combat completion to preserve attacker/defender characteristics, including Assault and Shield. Its additive `focus_open` flag is true only when a new showdown focus action is available; it is false during chains, choices, damage assignment, and combat completion. Use `decision`, `chain`, `priority`, and validated actions to respond to the current effect. A retained `showdown` is not permission to pass focus or take Main actions. Combat completion requires no new API action.

Observations include phase, turn, active player, priority, winner, mulligan status, public resources, champion slots, units, gear, battlefield control/scoring, showdown state, and the public chain. The last chain entry is the top item, which resolves first after the required passes. Each player object includes hand/deck/rune-deck counts. Domain-indexed arrays use `Fury, Calm, Mind, Body, Chaos, Order`, matching `catalog.domains`. `energy` includes ready rune energy and stored energy; `energy_pool` is the stored portion. `runes` counts all runes on board, and `exhausted_runes` identifies its exhausted subset. `legend.spell_power` is restricted to spell costs.

A pending `payment_plan` exposes `xp` and `xp_exact` to both seats, alongside its frozen Energy/Power obligations. Beginning a payment may stage a cost that is not yet affordable; completion checks the actual balance. Resource Add actions cannot substitute Power for XP. Failed completion leaves the pending state intact; the existing rollback choice restores its original XP balance and positive-gain history along with resources, readiness and choices. Successful completion spends the recorded XP once.

Both seats expose numeric `xp`, decimal-string `xp_exact`, and Boolean `xp_gained_this_turn`. Use `xp_exact` in clients whose numbers cannot represent every 64-bit integer exactly. A positive completed XP gain sets the history flag; zero does not. The flag survives later balance changes and clears for both players at the start of each turn, before Awaken. It is not inferred from the current balance. These are observations, not client-settable fields. An XP sum beyond the host's unsigned 64-bit capacity rejects the entire attempted action with error code `engine_capacity`; state and revision remain unchanged. This is a representation limit, not a Riftbound XP maximum.

Dynamic Might and damage retain their numeric JSON fields and add decimal-string companions. Units expose `might_exact`, `signed_might`/`signed_might_exact`, `damage_exact`, `buff_exact`, and `temporary_buff_exact`. Referenced `might` floors a negative signed value at zero; it has no 255 cap. Computed Assault and Shield include `assault_exact` and `shield_exact`, and observed Might/Assault/Shield passive contributions include `effect.amount_exact`. Printed catalog descriptors remain unchanged. Chain items add `bonus_damage_exact`; combat decisions add `total_exact`. Cost-plan modifiers include `energy_exact`, preserving even a large Might-derived reduction while the applied discount is limited by the actual remaining cost. Submitted combat allocations retain numeric pairs and add `submitted_combat_damage_exact`, with string amounts in the same order (or null before submission).

Each public unit also exposes `combat_contribution:{might,might_exact,damage,damage_exact,stunned,prevents_combat_damage}` from the engine's combat calculation. Here `might` is its full current Might, while `damage` is what it would contribute to its controller's combat damage pool. Real Stun and a printed combat-damage restriction are independent booleans; either makes that contribution zero. Ezreal, Dashing can therefore have three Might, zero combat contribution, and `stunned:false`; its triggered damage and the Might needed to defeat it remain separate. The outer unit `damage` field still means damage marked on that unit.

Abilities that refer to a source's Assault or Shield use the current keyword totals in `keywords.assault` / `keywords.shield` (and their exact decimal-string companions). Those are distinct from Might and combat contribution. Lucian's Attack damage follows current Assault, including grants; Tusker's Move damage follows current Shield even when it is not defending. Removing a granting source in response changes the value at resolution. The unit's printed cost and Might remain available in the catalog.

Graves's Attack/Defend damage affects all current enemy units at his current location without a target-choice request or Deflect payment. His combat contribution is separately zero. Tusker uses the existing optional `trigger_targets` decision (`required:0`, `maximum:1`): choose a unit at either battlefield or submit an empty target array to decline. Both players' base units are invalid choices. As with other trigger decisions, `candidates` is a coarse pool of objects matching the target kind; it can include objects excluded by location, allegiance, previous choices or payment constraints. Apply the reported target rule and use `validate` for an intended choice; membership in that pool alone does not establish legality. These interactions use the existing protocol commands and revision/actor checks.

This object reports potential contribution even for a unit at Base; it does not assert that the unit is participating in combat. Use location, showdown and combat decisions to identify participants and actual pooled damage. Decimal-string companions preserve exact values for clients using binary64 numbers. The object is additive under protocol version1, is identical for both viewers, and is read-only. It is not a per-unit combat-damage assignment or kill-attribution ledger.

`assign_damage` keeps its ordered `[unit_id, amount]` pairs. An amount may be an unsigned JSON integer or a canonical unsigned decimal string through 18446744073709551615. Strings contain digits only, with no leading zero except `"0"`; signs, whitespace, fractions, exponents and out-of-range values are rejected. Unit IDs remain numeric. Suggestions use numeric amounts through 9007199254740991 and decimal strings above that threshold, so copying a suggestion through JavaScript preserves it exactly. Allocation legality still requires the actual total and the normal lethal, Tank and Backline ordering rules; the representable maximum is not permission to over-assign damage.

The Rust API uses `numeric::{SignedMight=i64,Might=u64,DamageAmount=u64}`. A unit's signed Might must fit i64; damage and combat pools must fit u64. `try_unit_stats`, `try_lane_might`, and the existing numeric query adapters return `Result`, as do action/candidate queries that depend on them. Wide intermediate calculations may exceed the storage type if their final result fits, including a signed Swap. A true representation failure returns `engine_capacity`; the entire attempted command, pending resolution/payment state, RNG and API revision remain unchanged. Queries report the error without inventing a capped value. These are finite host limits, not game-rule maxima or a claim of arbitrary-precision arithmetic.

Card metadata `entry_ready` is `null`, `{"condition":"always"}`, or `{"condition":"controller_xp_at_least","threshold":3}`. The condition is evaluated at actual entry; entering ready does not perform a Ready action. `xp_energy_discounts` lists `{minimum_xp,energy}` tiers. Concentrate's tiers are 6→2 and 11→4: only the highest eligible tier applies, and the result uses the ordinary cost plan and remains fixed during payment. A passive condition of `controller_gained_xp_this_turn` refers to the public history flag and has no `condition_threshold`.

Both seats expose the same public resource and equipment fields. A gear object's `attached_to` identifies its bearer unit, or is `null` when unattached. Its `lane` is its actual location: an attached gear follows the bearer, and an unattached gear can independently occupy a battlefield. `null` means base and does not by itself mean unattached. `card_id`, `name`, `token`, `ready`, and `temporary` remain visible while attached. `temporary_status:{intrinsic,granted,active}` separates the origin from its current activity. Printed Temporary becomes inactive while attached; Temporary granted by Turn to Dust remains active. The existing `temporary` flag describes its presence, including an inactive printed instance. Battlefield IDs refer to `catalog.battlefields`, which contains the field's official effect text; field abilities on the chain include their lane.

Each player's `activated_abilities` lists the registered abilities of their public live units and gear as `{source, card_id, ability_index}`. This describes printed options, including exhausted sources and abilities that are currently unusable; it is not a list of legal actions. No private hand or deck abilities are exposed. An activated chain item has `ability.type:"activated"` with the same live source identity, immutable `card_id`, and selected `ability_index`, even after that source leaves play. Its outer `chain.source` remains the card-definition reference. The two fields distinguish which physical object used which printed ability.

Player observations expose the actual `legend_id` independently of `preset` (which is `null` for custom lists). The `legend` object also includes its `id`, `official_id`, `name`, `champion`, and `domains`, alongside its existing status fields. These identities and domains are public; an opponent's custom list remains private.

Ezreal's legend object additionally reports `targeting_progress:{"completed":0,"required":2}`. `completed` ranges from zero to two and measures progress toward the prerequisite, not the total number of choices. It is public to both viewers and resets for both players on each new turn. The ordinary `activate_legend` action becomes legal once the prerequisite, Reaction timing and ready state permit it. The draw waits for resolution; it does not expose the card to the opponent. Other legends retain their prior observation fields. Rengar uses the pending `legend_targets` decision and shared `choose_trigger_targets` action; an empty selection is rejected.

Every chain item has an additive canonical `source`: `{"type":"card","card_id":...}`, `{"type":"legend","player":...,"legend_id":...}`, `{"type":"battlefield","lane":...,"battlefield_id":...}`, or `{"type":"unit","player":...,"unit_id":...}`. The unit form identifies the physical bearer that generated Hunt; the other forms identify rules definitions. The older top-level `card_id` remains a main-card art/reference field for compatibility; on a legend ability it is not the legend's identity or its rules definition. Legend chain abilities include `viktor_recruit`, `lux_draw`, `rengar_might` (with its chosen unit target), and `ezreal_draw`. Viktor's ordinary activation uses `activate_legend` without a target; its Recruit destination is selected only after the ability resolves. Darius's Reaction resource ability uses the same action and resolves immediately. Lux is passive and has no activation action.

Attachment abilities identify their origin: `ability.type:"equip"` is the activated Equip ability, while `ability.type:"quick_draw_attach"` is Quick-Draw's on-play triggered attachment. Both carry `gear_id` and `target`. An activated gear ability is observed as used when it resolves, even if its attachment target has become invalid; Quick-Draw is not an activated ability. Resource gear resolves immediately and preserves priority, while any resulting triggered ability still waits on the chain.

A unit's observed `might` is its current total, including equipment and currently applicable combat bonuses. Printed might remains in the catalog. Do not add an equipment bonus to the observed total again.

Each unit also exposes `keywords`: numeric `assault`, `shield`, `deflect`, `hunt`, and `vision`, Boolean `tank`, `backline`, `ganking`, and `ambush`, plus optional printed `accelerate:{energy,power,domain}`. These values account for token identity, current equipment, level requirements, and granted abilities. Assault and Shield are their keyword values even outside combat; apply no extra bonus to observed `might`. Accelerate describes a play option, not an action that readies a unit already on the board. `damage_priority` reports the automatic assignment preference: `0` for Tank (including Tank plus Backline), `1` for ordinary units, and `2` for Backline alone. A unit with both Tank and Backline may legally occupy either priority group; this field does not restrict the controller to the automatic choice.

For token units and Gold, `token:true` distinguishes the generated object from a physical deck card. Its `card_id` is an engine source/reference identity, so use the observed `name` and `might` for the token instead of assuming that catalog entry's printed card kind or stats describe it.

A player object identifies its `legend_id`, `custom` flag, and `preset` (`null` for custom decks). `custom_deck` contains the saved list only for its owner; opposing observations always omit that list even when a hand reveal is active. The declared deck name and public legend/champion remain visible.

The viewer sees their own hand as card IDs. The opposing hand is `null` except while an actual reveal effect permits it. Deck order and rune-deck order are omitted for both players. Hidden cards expose object ID, location, and hidden turn; `card_id` is `null` unless the viewer is allowed to see it. Granted Flow indices refer to the public trash and are visible to both viewers. Diagnostic game logs, prompts, random-generator state, and match seed are never exported in observations.

`decision` is `null` or an object with `type` and `player`. Other fields describe the current choice:

| Decision | Data and response |
| --- | --- |
| `predict` | Owner sees `card_id`; `choose` recycles it when true, keeps it on top when false |
| `draw_look` | Public source, kind, choice ID and count; only the owner sees top-first `cards` and eligible occurrence indices. Use `resolve_draw_look`. |
| `trigger_payment` | Public choice ID, card/rule, physical source context, targets, `cost`, `timing`, nullable `resolving_chain_id`, nullable `base_cost`, and separate Deflect. Use `resolve_trigger_payment` at the indicated step. |
| `trigger_mode` | Public `choice_id`, card/rule and physical source `context`, with `options` containing `index`, `label`, nullable `base_cost`, and nullable `resolution_cost`. Use `choose_trigger_mode`; this mandatory choice has no decline action. |
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
| `trigger_return` | Public `card_id`, retained `chain_id`, and exact unit `target`, with `timing:"resolution"` and `optional:true`. Send `choose` with `accept:true` to return it, or `false` to leave it. No response action is allowed while this instruction resolves. |
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

`trigger_target_candidates` previews a pending `trigger_targets` or `legend_targets` choice. Omit `targets` or send `[]` to start; send the ordered prefix to query the next slot. It returns `targets`, `maximum`, `required`, `next_slot:{index,kind}` (null when full), filtered `candidates`, and `can_confirm`. Unlike the coarse candidate pool in an observation, this query checks allegiance, location, protection, previous slots and available Deflect resources. A valid prefix can still have no legal continuation; `can_confirm` checks the complete action on a clone. No costs are spent and no targets are committed. Invalid prefixes, stale revisions and the wrong actor are rejected without changing state. As with other queries, re-query after state changes.

```json
{"command":"trigger_target_candidates","revision":4,"player":0,"targets":[{"type":"battlefield","lane":0}]}
```

Blade of the Exile (card 438) uses the existing `equip` action for its 1 Any Power option. Its ordinary card-play cost remains 3 Energy and 1 Calm-or-Chaos Power. The equipment catalog reports `equip_domain:"Any"` and `alternate_cost:{type:"exhaust_friendly_unit",ready_required:true,may_be_bearer:true,targets_unit:false,action:"equip_exhaust_unit"}`. For that alternative, the selected `exhaust_unit_id` belongs to the acting player and can identify the bearer or a different ready friendly unit at any location. It is a cost, not an additional target. The `cost_plan` response adds `object_costs:[{type:"exhaust_unit",player,unit_id}]`; its normal resource fields still include any applicable modifiers. The alternative does not spend the printed Equip Power. Piltovan Forge's Energy reduction does not remove either the Power cost or the exhaustion cost.

```json
{"command":"apply","revision":4,"player":0,"action":{"type":"equip_exhaust_unit","gear_id":20,"target":{"type":"unit","player":0,"unit_id":7},"exhaust_unit_id":9}}
```

Queries and payment previews do not exhaust the unit or queue its triggers. At finalization the unit exhausts and triggers such as Reckless Innovator are processed; the pending Equip ability then attaches on resolution. Its chain targets contain only the bearer. A failed or countered Equip does not refund the exhaustion, and the cost unit leaving afterward does not invalidate the bearer. Attached equipment cannot activate Equip again. Both alternatives use the same public gear attachment, +3 Might, and Ganking observations. Invalid identities, wrong ownership, already-exhausted cost units, unsupported equipment and surplus exact Power payment are rejected atomically. Staged payment preserves readiness until completion; once a fully payable ability is finalized, it cannot be cancelled merely to avoid its cost.

Bedeviling Stranger (card 437) reports `target_rule.battlefield_first:true`: choose a battlefield, then one enemy unit there. Both choices are mandatory before responses even when that field currently has fewer than four units. When the trigger resolves, it rechecks the targets and counts all units at the chosen field. With at least four units, `trigger_return` offers the later optional return and `resolution_progress.chain_id` identifies the retained trigger. Declining does not refund Deflect paid during finalization. A countered trigger or invalid target has no return choice. A play from Hidden restricts both targets to the entry battlefield, and the effect mistargets if its physical source leaves before resolution (Core811.1.d.3.a). The same source-departure restriction does not apply to an ordinary hand play. The Sonecon Stuttgart preset includes this card in its complete Riven registration.

`trigger_targets` decisions now include an additive `target_rule` object: `kind` (`unit`, `gear`, or `unit_or_gear`), `allegiance`, nullable `maximum_printed_energy`, and `energy_at_most_live_source_might`. Use the returned physical `candidates` and submit `choose_trigger_targets` with ordered targets. A gear target is `{"type":"gear","player":1,"gear_id":700}`; its ID is distinct from its card definition and bearer. An optional trigger reports `required: 0`, allowing an empty list to decline. The normal revision checks and atomic validation still apply.

Detonate/Brittle Steel play queries report `gear`; Decree of Unity reports `unit_or_gear` and restricts candidates to enemy Chaos objects. Gold and unit tokens use their own domainless, zero-Energy identity. `target_candidates`, `quote_payment`, and `apply` share those rules. Gear-trigger observations expose printed Energy restrictions separately from the live candidate list; bots must re-query after state changes.

### Return-to-hand and captured death locations

Factory Recall uses an ordinary `gear` play target, including attached gear. Downwell and Angler Beast do not choose targets. Returning a physical card moves it to hand; a token ceases to exist. These transitions do not trigger Deathknell. Unit returns detach equipment, while Downwell returns the original set of both units and gear exactly once.

Trigger `context` now has additive nullable `departure_location`, captured before a dying source leaves. Locations are `{"type":"base","player":0}` or `{"type":"battlefield","lane":1}`. `target_rule` adds nullable `here_basis` (`live_source` or `source_departure`) and `location`. Mournful Spirit uses `source_departure`; ordinary Here abilities keep following their live source. Null does not mean either base.

The existing trigger `candidates` list enumerates physical objects of the required type. It does not assert allegiance, location, Energy, slot legality or affordable Deflect. Use the metadata to present choices and `validate` on the complete `choose_trigger_targets` action before submitting it. Mournful Spirit allows an empty list to decline. A chosen object that leaves the remembered location before resolution is no longer a legal target. Opponents receive the same public location metadata, without private-card information.

Retreat’s target query accepts friendly units in base or at a battlefield. A legal return supplies its owner to the linked exhausted Channel, including when the unit is a token; a missing target supplies no owner and does not Channel. Wind and Ghosts accepts units at battlefields without a Might cap. Its Banish/return branch uses Might when resolving. Direct Banish is visible in both players’ public `banish` arrays and does not produce a death-replacement decision. Trigger target metadata now also exposes the boolean `target_rule.battlefield` restriction. Apothecary’s on-play decision has optional friendly battlefield targeting and permits its own newly entered unit.


### Banish-origin instructed plays

Portal Rescue, Temporal Breach, Thrill of the Hunt and Arcane Shift use the existing `pending_play` decision and `finalize_pending_play` action. Their `kind` values are `portal_rescue`, `temporal_breach`, `thrill_of_the_hunt` and `arcane_shift`. The decision and its `pending_card_plays` row additionally expose `origin:{type:"banish"}` and `instruction:{base_cost_waiver:"all",destination:...}`. Destination is `{mode:"base"}`, `{mode:"fixed",lane:null|N}`, `{mode:"any_battlefield"}` or `{mode:"normal"}`, respectively; `N` is an existing battlefield slot. These additions apply only to the new Banish-origin cases; the three older Trash kinds retain their wire shape.

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

## Vision instances and source identity

Units expose their current Vision count in `keywords.vision` and its exact decimal companion `keywords.vision_exact`. Units and gear also expose `vision_instances`, an array of `{source, origin, active}`. `source` is the physical bearer (`{type:"unit",player,unit_id}` or `{type:"gear",player,gear_id}`). An origin is either `{type:"printed"}` or `{type:"granted",source,card_id,rule_index}`, where that nested source identifies the granting permanent. Inactive conditional grants do not supply Vision. Printed gear Vision remains a characteristic while attached, with `active:false`; it does not trigger from inactive Rules text.

Each active instance captured on entry becomes a separate chain ability: `{type:"vision",source,source_name,origin}`. The outer chain `source` also identifies the bearer. A token keeps its actual token name and physical ID; the older outer `card_id` remains an art/reference field. The captured source and origin remain available after either permanent leaves play, while live observations immediately reflect the grants that remain.

Vision uses the existing private `predict` decision and `choose` action. Each instance independently inspects one current top card on resolution; multiple instances are not one multi-card Predict. `accept:true` recycles that card, while `false` keeps it on top. The other viewer receives no card identity. Predict does not increment Draw history, and an empty deck causes no Burn Out. Ordering options and source metadata contain public facts only. No new action or private-choice shape is introduced.

Mystic Poro (193) and Divining Shells (242) now use this explicit Vision chain representation rather than generic `card_trigger` metadata. Their one-instance gameplay is retained. Clients should use the physical `source` for ability identity and treat names and card IDs as presentation/reference information.

## Battlefield gear and modal plays

Live gear adds public `deploy_status:{characteristic,printed_abilities_active,opponent_hold_trigger_active}`. The Deploy characteristic remains referenceable when an attached gear's printed rules are inactive. This is a live typed rule fact; a card merely mentioning Deploy in its text does not acquire that characteristic. The field is identical in both seat observations. Use `gear.lane` for its current physical location and `gear.id` to distinguish separate copies with the same `card_id`.

Play Deploy gear with an explicit battlefield `lane` through the existing `play_card`, `play_card_targets`, or instructed `finalize_pending_play` action. All current battlefield indices are available regardless of control or occupants; Base (`lane:null`) is not a legal Deploy destination. Gear entry itself does not establish control, contest a field, or begin combat. Its opponent-Hold destruction is a separate respondable trigger; an opponent merely conquering the field does not trigger it. There is no generic gear `move` action.

`play_choices` adds `modes`, an ordered array of `{index,label,target_kinds}`. Select a card's base execution mode in `with_play_choices.choices.mode`; each Repeat retains its independent `repeats[].choices.mode`. Non-modal cards reject a supplied mode. For backward-compatible non-modal serialization, an absent base mode is omitted from output; the parser also accepts `null` as absent. A real Command mode is required to finalize Command: Attack. Its stable mode indices are:

- `0`: move a friendly Deploy gear to a battlefield. Targets are ordered as the exact gear object, then the destination battlefield.
- `1`: inspect the top five cards, optionally reveal and draw one Gear occurrence, then recycle the rest. This mode has no targets.

```json
{"type":"with_play_choices","action":{"type":"play_card_targets","hand_index":0,"targets":[{"type":"gear","player":0,"gear_id":12},{"type":"battlefield","lane":1}]},"choices":{"mode":0}}
```

`target_candidates` adds `next_slot.kind` so an ordered mixed-target execution reports its next actual kind. In Command's movement mode, the first slot is `gear`; after selecting that exact object, the next slot is `battlefield`. Use the returned candidates for ownership, Deploy, and same-location restrictions. Attached Deploy gear remains targetable; its separate Move is negated when the effect resolves, because attached cards cannot move separately from their bearer. A destination is not inferred from another target or a previous Repeat execution. These queries remain read-only, and invalid mode/target/order/destination/payment submissions preserve both observations and the revision.

`chain[].executions` retains each mode and ordered targets while the spell resolves, including across a private choice. A Repeat can select a different mode from the base execution. The spell remains one physical card, with one completed-play event, and no response window is introduced between its executions.

Command's inspection uses the existing `draw_look` decision with `kind:"command_attack"`. Both viewers see the choice ID, source card, and actual count; only the controller sees `cards` and `eligible` occurrence indices. Duplicate card IDs remain distinct occurrences. `resolve_draw_look` with an eligible index selects that occurrence; explicit `index:null` declines and recycles all. The selected card is publicly revealed through the existing `revealed_card` acknowledgement. Only an actual successful Draw advances Draw history.

Passive metadata adds `condition:"friendly_deploy_gear_here"` for the existential condition at a unit's battlefield, and `recipients:"friendly_units_here"` for friendly units at a source's live battlefield, including a gear source. Overloaded Traveler gets one bonus from the existence of a qualifying gear; separate Banner sources still contribute independently. Base is not a battlefield.

## Paying an optional trigger's base cost

Concert Poster and Portable Portal begin their triggered effects with an optional cost. That cost is paid during trigger finalization under Core 204.1.c and 403–407, before the trigger becomes a respondable chain item. It is not a later payment choice during resolution. Portal first uses the existing optional `trigger_targets` decision; an empty list declines the optional ability, while selecting its target advances to payment. Poster has no targets and advances directly to payment.

The public `trigger_payment` decision includes `player`, `choice_id`, `card_id`, `rule_index`, `context`, ordered `targets`, `cost`, `timing`, `resolving_chain_id`, `base_cost`, `paid_before_responses`, and separate generic `deflect`. For a base cost, `timing` is `"finalization"`, `resolving_chain_id` is null, and `base_cost` equals `cost`. `context.source` identifies the exact physical source with `{player,unit_id,gear_id}`. The cost uses `{energy,power,domains,xp,xp_exact}` and is the trigger's full frozen base price, not an incremental balance or the gear's earlier play price. Deflect is separate so clients can explain the target's added obligation without double-counting it. Use `cost_plan` or `quote_payment` for the accepting action's total resource requirements and exact valid sources; spell-only power is ineligible for ability costs.

```json
{"type":"resolve_trigger_payment","choice_id":123,"pay":true}
{"type":"resolve_trigger_payment","choice_id":123,"pay":false}
```

`pay` must be an explicit Boolean. Declining spends nothing and removes the pending trigger without an effect or a finalized chain item. Accepting pays the frozen cost, rechecks the pinned targets, and finalizes the ordinary trigger with its normal response window. The token or Ready effect happens only when that finalized trigger resolves. Losing the target or being countered later does not refund the paid cost. There is no automatic replacement target.

Reckless Innovator (card429) creates a `trigger_mode` choice on each actual Ready-to-Exhausted transition. Mode 0 plays a Gold token exhausted; mode 1 finalizes without payment and opens responses. Only when that ability resolves does `trigger_payment` request one Chaos Power. This decision has `timing:"resolution"`, `paid_before_responses:false`, `base_cost:null`, and the retained parent ID in `resolving_chain_id`; `cost` describes the payment. `resolution_progress` identifies the same parent, including during nested Add or payment controls. The response window does not reopen between payment and draw. `context.mode` is included only after a modal choice and remains present on the finalized chain ability, even if the physical source leaves play. Declining payment skips the draw without switching modes and preserves completed Add actions. A staged `refuse_payment` reports `refusal_kind:"instruction"` with the same outcome; Undo restores only the staged attempt. Choice IDs reject stale submissions; the chosen mode, cost, source, and resulting public Gold are visible to both seats. Drawn card identities remain private. The Gold reminder text does not grant Reaction timing to the unit card.

The nullable top-level `trigger_payment_context` preserves that same public decision through a nested payment plan or Add continuation. It is identical in both seat observations and contains no hidden instructions. Distinct copies of the same card remain distinguishable by their source object IDs and originally selected targets. The opponent still sees `payment_plan.action:null`; exposing the already-public trigger context does not expose a private hand action.

Wrap the accepting action in `with_power_payment` to choose exact power sources, or `begin_payment` to use permitted Add/resource actions before `complete_payment`. Beginning a plan preserves the original choice ID and fixed source, targets, and cost. For a positive resource cost, `choose` with `accept:false` cancels this plan even when fully funded, once any nested resource instruction has finished. It restores the original pending trigger and the state from before the plan, including rollback of its Add actions. The controller can then explicitly decline the trigger cost. A zero resource cost cannot be declined. For a finalization cost, declining the pending trigger also undoes Add actions performed directly during that trigger's payment before a plan began; it preserves the already completed source play and pinned target choices. Successful payment retains its Add effects and clears this rollback boundary. Successful acceptance or decline consumes the choice ID. Missing, stale, wrong-seat, malformed, insufficient, invalid-source, or invalid-target attempts reject without changing the revision or either observation.

## Physical runes and battlefield choices

Each player's public `rune_objects` lists the board runes as `{id,domain,ready}`. Both seats receive the same list. IDs identify physical runes within that player; the complete identity is `(player,id)`. The existing `runes` and `exhausted_runes` arrays remain aggregate counts. Recycling removes that physical ID. A later channeled rune has a fresh ID, even in the same domain. Neither Rune Deck order nor the private rune-ID allocator is exposed.

Use `{"type":"activate_rune_object","rune_id":12,"recycle":false}` to exhaust the current player's exact ready rune for Energy, or `recycle:true` to recycle that rune for its domain Power. Existing `activate_rune` and `recycle_rune` actions retain their deterministic domain selection behavior. An exhausted rune cannot be exhausted again, and a recycled/missing ID rejects atomically. Payment Undo/refusal restores physical rune IDs and their state along with the other resources from the applicable rollback boundary. The Rust `set_rune_setup` helper exists only for explicitly controlled test positions; it is not an API request or an observation-time repair.

A rune target is `{"type":"rune","player":0,"rune_id":12}`. A rune is not an activated Unit/Gear ability source. As with other targets, public parse/validate/apply checks and the revision/acting-seat rules still apply.

Battlefield sources use `{type:"battlefield",lane,battlefield_id}`. Two copies of the same battlefield at different lanes remain different sources. `battlefield_targets` decisions expose `player`, `choice_id`, `source`, `kind`, `required`, `maximum` and full public `candidates`. Kind is `{type:"star_spring",played_unit_id}`, `{type:"abandoned_hall"}`, or `{type:"targon_peak_ready"}`. Submit `{"type":"resolve_battlefield_targets","choice_id":123,"targets":[...]}`. Star Spring and Abandoned Hall choose zero or one Unit; Targon's Peak chooses zero to two distinct board runes from either player. These choices finalize a respondable chain ability. They do not immediately perform its effect.

`star_spring_used` is a public two-by-two Boolean array indexed `[player][lane]`. It records whether that player's first successful non-token unit play at that Star Spring has occurred this turn, including an opportunity declined or left without a legal other unit. It resets at the actual turn boundary. Moving into the battlefield, playing Gear, and playing tokens do not consume it.

`delayed_battlefield_triggers` exposes `{id,player,source,turn}` for scheduled Peak occurrences. The Conquer ability first resolves to schedule the ending trigger. Runes are chosen when the delayed trigger finalizes at ending; the initial Conquer ability has no rune targets. Response changes can invalidate an exact chosen rune, but cannot replace it with another same-domain object. These records and the board rune list contain no private Rune Deck information.

Emperor's Dais uses a separate cost protocol. `battlefield_cost` exposes `choice_id`, `source`, `optional:true`, and `cost_unit_ids`. Send `{"type":"select_battlefield_cost","choice_id":123,"unit_id":456}` to pin the physical friendly unit to return as a cost, or `unit_id:null` to decline. This is cost selection, not targeting, and does not create a chosen-target event or Deflect cost. Its following `battlefield_payment` exposes the same `choice_id`, `source`, `cost_unit_id`, and frozen `cost`.

Use `{"type":"resolve_battlefield_payment","choice_id":123,"pay":true}` to complete the composite cost, or `pay:false` to decline. The accepting action supports ordinary exact `with_power_payment` and staged `begin_payment` wrappers. Successful payment spends its resources and returns the selected unit before opponents respond to the token-producing chain ability. Countering that finalized ability refunds neither cost. `battlefield_payment_context` remains available through a suspended Payment/Add choice, containing only this public decision. It does not expose a payment snapshot or private hand action. Undo preserves the original choice and selected object; successful acceptance/decline consumes that occurrence. Its return-unit base cost remains an optional obligation even if the Energy component is reduced to zero; staged `refuse_payment` reports kind `trigger` and consumes the declined occurrence. Declining rolls back Add actions from that payment, including direct Add before a staged plan, while preserving the completed Conquer and its score.

`pending_token_plays[].source` is nullable. When a battlefield ability instructs a token play, it names that actual battlefield source separately from the token factory's existing `card_id` and `token_identity`. The child still goes through ordinary pending-token choices and any selected additional costs.

### Decree of Focus targeting

Card 421 (`ven-040-166`) adds an optional `play_targets.condition` descriptor:
`{"type":"in_combat_with_unit_or_chosen_by_spell","controller":"enemy","domain":"Fury","rechecked_on_resolution":true}`.
Existing card descriptors omit this field. The regular target preview filters the
friendly units using the live combat and spell chain, including Repeat choices.
The same predicate guards finalization and resolution. Abilities and unchosen
area-damage recipients do not satisfy the spell condition. No new action type is
needed; use `play_card_targets` and the existing payment/response commands.

### Find Your Center and Hush

Card 422 (`ogn-047-298`) uses the ordinary zero-target spell and payment actions.
Cost previews and final payment reduce its Energy by two when the opponent's
score is at least five. Resolution draws one, then channels one exhausted rune;
an empty Rune Deck does not produce an extra draw.

Card 423 (`rad-041-167`) uses ordinary spell targets and adds
`play_targets.condition: {"type":"spell_printed_power_at_least","minimum":2}`.
Eligible targets are spells with at least two printed or copied Power. Cost
modifiers and additional payments do not affect that characteristic. Abilities
are ineligible. Target previews, validation and application share this rule.

### Ravenbloom Prefect: optional object cost

Card 424 (`ven-102-166`) uses the ordinary unit-play action. An opponent's completed
gear play creates a public `trigger_object_cost` decision with `card_id`,
`rule_index`, `context`, `optional:true`, `can_accept` and `paid_before_responses:true`.
Its `cost` is `{"type":"banish_source","source":{"type":"unit","player":0,"unit_id":123}}`.
`context.subject` names the exact played gear, including Gold; this is not a
chosen target. Both observers see these public fields.

Use `{"type":"choose","accept":true}` to pay or `accept:false` to decline.
Acceptance banishes the source immediately, then finalizes the ability for
responses. If the source is unavailable, `can_accept` is false and acceptance is
rejected without changing the revision or state. Decline remains available.
Removing the gear after acceptance does not refund the source. Gear activations
and friendly gear plays do not trigger this card. No resource payment wrapper is
needed for this object cost.


## Hunker Down movement restriction

Unit observations retain `movement_locked`, meaning the controller cannot move
that physical unit this turn. Hunker Down also blocks movement by the opponent;
these units additionally expose
`"movement_restriction":{"scope":"any_player","expires":"end_of_turn"}` to
both viewers. Other units omit this optional field. Ready and movement legality
are separate. Movement spells can still choose restricted units; their Move
instruction does nothing, while other independent instructions still resolve.
Basic and grouped moves reject the locked unit without changing state or revision.
The restriction expires with the turn and does not follow a card that leaves play
and is played again as a new object.


## Show Off and Battle Preparations

Battle Preparations (426) accepts `choices.show_off` on `with_play_choices`.
Omitting the array or supplying `[]` declines the optional Show Off. Instance 0
accepts exactly one own-hand gear or friendly board gear:

```json
{
  "type": "with_play_choices",
  "action": {
    "type": "play_card_targets",
    "hand_index": 0,
    "targets": [
      {"type": "unit", "player": 0, "unit_id": 500},
      {"type": "rune", "player": 0, "rune_id": 12}
    ]
  },
  "choices": {
    "show_off": [{"instance": 0, "object": {"type": "hand", "hand_index": 1}}]
  }
}
```

Board selections use `{"type":"gear","gear_id":700}`. Hand indices refer to
the same pre-play hand as the wrapped action, and cannot select the played card.
`play_choices` returns optional `show_off` descriptors and owner-only candidates.
Only the acting player may query them. No selected object is automatically revealed
by a read-only query. Other cards omit this descriptor and reject Show Off choices.

Use `target_candidates` for each execution: its first slot is a required unit;
after Show Off, its next slots are optional `rune` targets with physical IDs.
Choose zero to two different runes; either player's objects and already-ready
runes are legal. Static card metadata also includes `conditional_slots`. Rune
choices belong to the base/Repeat execution's `targets`; the Show Off array is
shared because Repeat does not repeat finalization instructions.

Public chain items and staged `payment_plan` observations include `show_off` only
when used. Each captured entry has `instance`, `source`, `card_id` and `token`.
A hand source is `{"type":"hand"}` without a private hand index; a board source
includes its gear ID. Both viewers see the selected card, never the rest of the
hand. The snapshot remains valid if shown Gold is spent during payment or a shown
object leaves play afterward. Costs, targets and Show Off cannot be changed once
payment is staged. Automatic Power payment preserves chosen rune targets; manually
recycling a target before finalization requires Undo/refusal or a different play.
Invalid input or payment completion preserves the current state and revision.

Orianna, Exquisite's `combat_contribution` adds `requirement: {"type":"friendly_gear_here","met":false}`. `met` is true when the requirement is satisfied. The object is absent on units without that printed requirement. The value uses public physical gear locations, including attachments on another friendly unit, and changes immediately as gear moves or leaves. `prevents_combat_damage` remains independent of `stunned`; `might` retains the full reference value either way. The optional requirement is explanatory and read-only.


Gust Monk uses the existing `play_choices.bonus` flag for its optional extra
1 Energy. After the unit enters, `choose_trigger_targets` chooses exactly one
unit. The resulting `trigger_payment` decision includes an `object_cost` with
`type: "banish_from_any_trash"`, `is_target: false`, `paid_before_responses: true`
and public `candidates` containing `player`, `trash_index`, and `card_id`.
Only the unit appears in the ability's `targets`.

Submit `{"type":"resolve_trigger_trash_payment","choice_id":ID,"trash_player":0,"trash_index":0}`
to banish the exact selected trash card and pay any Deflect before finalizing.
Both players' trash cards are eligible. `resolve_trigger_payment` with `pay:false`
declines the incurred cost; `pay:true` cannot bypass its explicit trash selection.
The new action supports validation, exact Power payment, payment quotes, and
`begin_payment`. During a staged payment, preceding trash removals update the
pinned copy's index. If it leaves or the trash is recycled by burnout, payment
cannot substitute a different copy; Undo/refusal restore the payment boundary.
Existing resource-only trigger observations retain their prior object shape.

## Preset registrations and Edge of Night

`catalog.presets[].registration` is populated for presets 0 (Xeno Irelia),
1 (Master Yi), 2 (Kennen), 4 (Rengar), 5 (Arito Irelia), and 6 (Sonecon Riven).
It remains `null` for historical Kai'Sa (3). The object contains `pilot`, `event`, `date`, `placing`, `battlefields`
(array of battlefield IDs), and `sideboard` (array of `[card_id, count]` pairs).
The top-level `catalog.as_of` dates the card-data snapshot; `registration.date`
is the sourced event date or month; the LA registrations use `September 2026`.
Preset 5 still uses legend ID 0. Its main `cards` include the chosen champion
once, for 40 total cards; sideboard entries are separate metadata. The existing
reset battlefield choices select among the registered options 13, 0 and 14.
Begin a registered match explicitly with `series_begin`; ordinary `reset`
continues to create a single game. Master Yi's exact registration has sideboard
`[[443,1],[421,1],[433,2],[366,3],[442,1],[31,1],[441,1]]` and battlefields
`[11,1,12]`. Its published main list and rune split remain unchanged.

Edge of Night is card 432. Its ordinary play takes no attachment target; later
Equip uses the existing equipment action with one Chaos Power. Playing it from
Hidden first puts the gear in play, then emits a `trigger_targets` decision.
Use `choose_trigger_targets` with one friendly unit at the original Hidden
battlefield. A target supplied prematurely with the play is rejected atomically.
The gear may already be at base while this decision or its chain item waits;
that does not redirect targeting to base. The attachment and +2 Might are
visible only on resolution, through the existing equipment and unit views.
Both viewers see the public decision; the facedown identity remains private
before reveal. Validation is read-only and uses the same restrictions.

## Registered match commands

See [MATCH_SERIES.md](MATCH_SERIES.md) for the full lifecycle and restrictions.
These commands add untimed best-of-three coordination without changing standalone
sessions. All mutations require the current `revision`; IDs and actors use the
same conventions as game commands. Registration validation is read-only.

| Command | Additional request fields | Result |
|---|---|---|
| `validate_registration` | `registration` | Validated registration; no mutation |
| `series_begin` | `registrations` (two objects), `designated_player`, optional `seed`, `difficulty` | New registered match, before field selection |
| `series_suggest` | `player`, optional `policy` (`adaptive` or `keep`) | Read-only own plan, reasons and scouting history; no revision change |
| `series_step_ai` | `player`, optional `policy` | Revision-checked submission of one legal sideboard, field or play-order choice |
| `series_sideboard` | `player`, `cards` pairs, `chosen_champion` | Lock that player's legal main list; sideboard is the remaining registered pool |
| `series_battlefield` | `player`, `battlefield` | Commit an unused field privately; reveal when both have committed |
| `series_play_order` | `player`, `first_player` | The designated player chooses play order; creates the opening hands |
| `series_concede` | `player`, `scope` (`game` or `match`) | Explicit concession |
| `series_draw` | `player`, `scope`, boolean `accept` | Offer/withdraw agreement; both seats must agree |

A registration is either `{"preset":N}` for a complete preset (0, 1, 2, 4, 5, 6) or
`{"deck":CUSTOM_DECK,"sideboard":[[424,2],[369,2],[20,1],[421,2],[11,2],[1,1]],"battlefields":[13,0,14]}`.
The main `deck` must contain exactly 40 cards; the shown sideboard is valid with
Arito's main list. Do not apply that example sideboard to an arbitrary deck
without checking combined copy limits and the legend's identity.

`observe.series.phase` progresses through `battlefields`, `play_order`, `game`,
`sideboarding`, and `complete`. `wins`, `game_number`, `designated_player`,
`first_player`, readiness flags, draw offers and `history` describe progress.
`series.players` contains public seat information. The separate `registration`,
`deck`, `sideboard`, and `available_battlefields` fields belong only to the viewer.
Before a game exists, ordinary game action methods refuse requests and
`suggest` is empty. No previous-game private view is returned during setup.

During `game`, use existing `mulligan`, `start`, `observe`, `apply`, `validate`,
`suggest`, payment and AI methods. Series mulligans must be explicitly submitted
in turn order, including an empty `indices` array to keep. `start` requires both
mulligans. A completed game automatically records its actual winner and scores.
Concessions have no fabricated final point score. Game draw agreement is exposed
after setup, preserves configuration and starting player, and creates a fresh
mulligan without sideboarding. Draws and match concessions do not fabricate game
wins. A new series always starts from the original registrations.

`series_suggest` returns `plan` (main `cards`, `chosen_champion`, derived
`sideboard`, available `battlefield`, preferred `first_player`, one-copy `swaps`
and `reasons`) plus this seat's `scouting`. It may be inspected during a game,
but only `series_step_ai` during the appropriate setup phase can submit choices.
The designated-player and already-submitted guards are the same as manual
commands. Suggestions remain private to the requested seat; mutation responses
do not publish sideboard plans. Both policies choose first, while `keep` also
keeps the deck and takes its next unused field. Easy difficulty uses that baseline
even with `adaptive`. A trusted coordinator must route these responses only to
the requesting seat, as with `observe`.

### LA sideboard choices

Decree of Strength (433) has normal spell timing and no unit target. In this
two-player client its chosen opponent is the other seat. Resolution exposes
`decision.type = "recycle_revealed_hand"`, `opponent`, `domain = "Mind"`, the
whole revealed `cards` list, and `eligible_indices`. Submit `select_cards` with
exactly one eligible index, or an empty list when none exist. The opponent
recycles that physical card to the bottom of their deck. Both player views show
the revealed hand during this decision and hide it from the opponent again
afterward. Repeat creates a new choice over the remaining hand.

Lacerate (434) uses one ordinary unit target of either allegiance, in base or at
a battlefield. Neither Empowered nor starting Might restricts target legality.
Resolution removes Empowered, then checks live Might and kills at 3 or less.
Responses, Deflect, death replacements, and physical-target revalidation apply.
Its printed Flow costs 4 Energy and 2 Order Power and banishes the played card,
including when countered. Flow does not grant Action or Reaction timing.


### Riven, Broken Blade (card 439)

The exhaust trigger uses the existing `trigger_mode` decision and stable mode indices:
`0` Buff the source, `1` spend its Buff to Stun one enemy, and `2` spend its Buff to Ready the source.
Its options additionally expose `available`, `once_each_turn`, and `used_this_turn`.
An unavailable mode can be previously used or have no payable legal target; clients must retain its
original index. The source unit's public `trigger_modes_chosen_this_turn` lists its used indices,
even between trigger decisions. Copies track their choices independently; changing control of the
same object preserves that history, replaying creates a new object, and the next turn resets it.

Modes 1 and 2 expose `resolution_object_cost` with `type:"spend_source_buff"`, its `player` and
`unit_id`, and `targets_unit:false`. This cost is not part of initial payment or targeting. Stun
chooses its enemy and pays any Deflect before responses. A Buff is not required yet when choosing
the mode. On resolution the live controlled source must have an actual Buff counter for the linked
effect to happen; temporary Might and Empowered do not count. An absent source cannot spend a
replacement copy's Buff. If Stun's only target has become invalid, that entire linked instruction,
including its cost, is ignored. A counter leaves the Buff intact but does not restore the chosen mode.
Neither Buff nor Ready implicitly targets the source. No unused legal mode removes the pending
trigger without offering a decline action. A missing Buff at resolution does not change the selected
mode or offer another choice.

This implements the Radiance FAQ's resolution-time Buff cost and exhaust-before-move trigger order.
The linked instruction's mistargeting behavior is derived from Core Rules 359.3.e.7 and 740.4.a.
The complete Sonecon Stuttgart registration uses this champion with The Exile.

### The Exile (legend 10)

The legend catalog includes `rad-149-167`, with Riven, Broken Blade as its default
chosen champion and an authored editable training deck. Its passive needs no activation.
A `play_choices` result for a qualifying unit play includes `legend_entry_ready` with
`{"type":"legend","player":N,"legend_id":10}`. The property is omitted when this
legend does not grant ready entry. Unit tokens qualify; gear and spells do not.
This read-only preview checks the selected play location. A missing property does not
rule out other ready-entry effects, such as a printed rule or paid Accelerate.

The engine retains The Exile's replacement from finalization, including suspended
resource payments. Base plays receive no benefit. It does not grant permission to play
at uncontrolled battlefields or ready units already on the board. A Baron play retains
the ready-entry replacement from its originally chosen controlled battlefield even
when its own entry replacement sends it to a newly added Baron Pit.


### Baron Nashor and token battlefields

The ordinary `catalog.battlefields` list remains the legal starting-field registry.
`catalog.token_battlefields` contains auxiliary game pieces; Baron Pit is ID1000
(`unl-t01`) and has `can_start_game:false`. Reset and registered-match setup reject
it as a starting choice. Its creation appends a physical slot to the observation's
`battlefields` array and every corresponding ownership/scoring array.

For a Baron play that creates the Pit, `play_choices.entry_replacement` reports
`{source_card_id:440,battlefield_id:1000,lane:2,adds_battlefield:true}` on a normal
two-field board. The slot is the destination of the entry replacement, while the
action's `lane` remains the chosen play location for cost and ready-entry facts.
This preview does not mutate the game. During `begin_payment`, the Pit already
exists publicly and Baron is still pending. Completing payment preserves this
play's replacement without adding another Pit; Undo or refusing the finalization
restores the previous board. When a Pit already existed before this play, the
replacement property is absent and Baron enters at the chosen play location.

Baron Pit permits Standard Move into its physical slot from any other location,
without granting Ganking for moves out. Clients should use legal actions rather
than assuming two locations or inferring permission from Ganking alone. Rematches
restore the original two starting battlefields. The Sonecon Stuttgart preset provides
the complete Riven registration for single-game and best-of-three practice.


### Damage attribution

`chain[].source` now identifies physical sources for activated unit/gear
abilities and metadata-driven unit triggers, alongside the existing legend and
battlefield identities. `chain[].card_id` remains the catalog/art reference.
Consumers must handle the documented source union rather than assume that
every ordinary ability has a `type:"card"` source.

Observations include `damage_attribution.pending` and `damage_attribution.last`.
The pending record follows a resolving physical parent through repeated
instructions and death-replacement choices. The last record is the most recently
completed parent and cleanup, not a cumulative match history. Collect observations
after each action if your bot needs a history; a later parent replaces `last`.

Each record contains `parent`, `damage` and `kills`. A damage packet supplies its
responsible `player`, public `target`, numeric `amount`, decimal `amount_exact`,
and `sources`. A spell or ability source carries `chain_id`, `player` and
`card_id`; physical unit/gear sources use the normal object shape. An ability can
have both its own chain identity and its physical creator. Unit-dealt effects
contain the unit source without adding the causing spell as a damage dealer.

A kill supplies its target, `kind` (`instruction`, `cleanup` or `combat_cleanup`)
and zero or more `credits`, each with a responsible player and source list.
Cleanup credit can differ from the Deal source; for example, Challenge's damage
is dealt by its chosen units while its ensuing cleanup kills refer to the
physical spell. Combat refers to the opposing physical group, without fabricated
per-unit damage amounts. Prevented damage and replaced unit deaths are omitted.

These fields expose public events equally to both seats. They currently cover
resolved damage, primary unit/gear Kill instructions and their cleanup. They are
not a complete death feed: kill costs, replacement-caused gear deaths and some
legacy effect ancestors still require causal tracking. A null origin or empty
credit list must not be treated as evidence that the game rules forbid a source.
See the rules audit's attribution follow-up before using this data to implement
additional source-sensitive cards.

## Spirit Wheel

Spirit Wheel (`454`, `sfd-144-221`) observes friendly-unit choices by spells and
abilities. Its optional `trigger_payment` decision has `timing:"finalization"`
and `object_cost:{type:"exhaust_gear",player,gear_id,paid_before_responses:true,is_target:false}`.
The `cost_plan` includes that same physical source in `object_costs`, separately
from the 1 Energy obligation. `resolve_trigger_payment` with `pay:true` pays both
costs before creating the respondable draw ability. A declined trigger spends
neither; an already paid trigger does not refund costs if countered.

The choosing spell remains on the chain while the Wheel's separate draw resolves.
Opponent observations expose hand counts, not drawn identities. Payment queries
do not mutate state, and stale/wrong-seat requests or an unavailable source reject
atomically. The [registered match checker](../tools/check_spirit_wheel_api.py)
retains per-level/seat payment, draw and zero-use evidence.

## Royal Entourage and legend targets

Royal Entourage (455, `sfd-039-221`) finalizes one mandatory legend target before
responses. A target is `{"type":"legend","player":0,"legend_id":0}`: the seat
and current catalog legend identity must both match. `trigger_target_candidates`
and the `trigger_targets` decision expose both players' available legends.

At resolution, `legend_readiness` retains the trigger on the chain and reports
`choice_id`, `card_id`, the fixed `target`, `currently_ready`, two `options`,
`timing:"resolution"` and `optional:false`. Send
`{"type":"resolve_readiness","choice_id":123,"ready":true}` to ready it,
or `ready:false` to exhaust it. Use the returned choice ID, not this example.
Either choice is legal even if it leaves readiness unchanged. Retargeting,
passing, and other responses are unavailable during this resolving instruction.
The opponent may use legal Reaction activations in the preceding response window;
exhausting that legend afterward does not undo an already paid activation.

The target and decision are public. Wrong-seat, stale-revision, stale-choice and
malformed requests reject atomically. Current supported registrations have one
Champion Legend per player; future additional legends require physical identities
and are not covered by this representation. The
[complete-match checker](../tools/check_entourage_api.py) records observed targeting
and resolution separately at all three AI difficulties and both seats.
