# Registered best-of-three practice

The native app, shared Rust engine and JSON bot API support **untimed Standard
best-of-three matches** within the implemented card pool and in-game rules scope.
Timed tournament rounds, judge penalties and best-of-five are not implemented.

## Play in the app

1. From **Practice**, choose **Best of three**. Use **Next registered deck**
   on each seat to choose a complete list. The app
   displays the 40-card main deck count, ten-card sideboard count and all three
   registered battlefields. Click a field to read its rules.
2. Choose the AI difficulty and how to designate the opening chooser. **Coin
   toss** makes that choice randomly; the other options explicitly designate a
   seat for practice. Choose **Register & choose battlefield**.
3. Select an unused battlefield and confirm. The AI commits before your choice
   is revealed. Both fields then appear together with the chosen champions.
4. If designated, choose first or second; otherwise the AI chooses first.
   Opening hands are then drawn. Choose up to two replacements or keep your hand.
   The AI takes its own mulligan in the correct turn order.
5. Play to a game winner. The result screen shows match wins separately from
   point scores, plus the fields and result of each completed game.
6. Choose **Continue to sideboarding**. **Out >** moves one copy from main to
   sideboard, and **< In** moves one back. Click a card name for its full rules,
   or hover to preview it. The arrows page through the complete lists.
7. Keep exactly 40 main cards and an eligible chosen champion. **Choose** marks
   an eligible champion in the main deck. The confirmation button stays disabled
   while the list is illegal. **Undo this game's edits** restores the list from
   before this sideboarding window. Confirming locks the submitted choice.
8. Choose an unused field; the previous loser chooses play order. Repeat until
   one player wins two games. A new match restores the original registrations.

**Concede game** has a confirmation step and awards the opponent one game win.
**Leave match** discards the unfinished game after confirmation. Completed game
wins and losses remain in the practice record. Mote provides Beginner rules
reminders or Advanced planning advice in the sidebar throughout setup.

The native app offers four complete registrations: Xeno Irelia (preset 0),
Kennen (2), Rengar (4), and Arito Irelia (5). Use **Next registered deck** on
either seat in match setup. The LA lists include the exact published sideboards
and battlefields; their date is recorded as September 2026, not the article’s
September 30 publication date. Custom match registrations remain available through Rust and the bot API.
The Easy opponent keeps its main deck and chooses its next unused field.
Tactical and Strategic opponents remember revealed cards between games and
consider up to two or four one-copy swaps respectively, with the same limit on
total changes from the original registered main deck. They preserve Irelia’s
core targeting, readying, movement and draw package as well as the
registered unit, early-unit and gear counts and limit increases in Energy costs.
The current role evaluations cover gear answers, Vex, Decree of Focus, Gust,
Abandon, Decree of Strength against revealed Mind cards, and Lacerate against
revealed small units or units with Empower. Unknown sideboard roles are not
inferred from card text. Field choices use the current own deck's composition.
Sunken Temple's estimate includes Irelia's buffs and equipment rather than only
printed Might. All levels still choose first when designated; matchup-specific play/draw policy
and expert-strength sideboarding remain unfinished.

**Mote: review my sideboard** gives an optional suggestion for your current legal
list, using only cards your seat saw. It does not move cards or submit choices.
Editing a card or champion clears the suggestion so stale advice is not retained.
The review button is disabled while your draft is illegal. Native draw-agreement controls are not yet exposed; the API supports them.
The app does not save an unfinished match for later resumption.

## Run a complete match

```sh
cargo build --release --bins
python3 tools/series_bot.py --api target/release/rift-api --difficulty tactical
```

The example runs the sourced Arito Irelia list against itself and prints the
match score and game history. It keeps both sideboards unchanged by default;
add `--adaptive` to use the shared match planner for both seats. Replace
`choose_sideboard(view)` and `choose_field(view)` in the example to try another
policy; each receives only that player's observation. This demonstrates the
protocol, not advanced matchup strategy.

`--registrations file.json` accepts an array of two registrations. Each may be
`{"preset":5}` or an explicit object with:

- `deck`: the existing custom deck JSON, with exactly 40 main cards including
  the chosen champion, the registered legend and twelve runes;
- `sideboard`: `[card_id, count]` pairs totaling zero to ten cards;
- `battlefields`: three different supported legal battlefield IDs, including
  the deck's default battlefield.

Presets 0, 2, 4, and 5 carry complete runtime match registrations. Master Yi (1)
and the historical Kai'Sa list (3) remain available for ordinary games and can
supply an explicit legal custom registration; the API does not invent missing
registered sideboards or include banned battlefields.

## Match sequence

1. The host registers both lists and supplies the designated player chosen by
   an agreed random method, or by playoff seeding. The seed reproduces shuffling;
   it does not replace the host's designated-player choice.
2. Each player commits a battlefield privately. Both choices reveal together.
3. The designated player chooses who goes first. Only then are opening hands
   created. Each player keeps or replaces cards in turn order, then play begins.
4. A normal game winner updates the match score. Before the next game, each
   player submits their new main list and chosen champion. The remaining cards
   form their sideboard. A player may simply submit the unchanged list.
5. After both lists are submitted, chosen champions are public. Players choose
   unused registered battlefields and the previous loser chooses play order.
   The match ends when a player has two game wins.

Submissions conserve the exact registered card pool. Deck sizes and combined
named-card limits are checked, while the legend, runes and battlefield registry
stay fixed. A champion change must leave an eligible champion in the main list.
Failed requests leave state and revision unchanged. Submitted choices lock;
construct and review the desired list before submitting.

Conceding a game or match is explicit. A drawn game requires both players to
agree after game setup; its replacement uses the same decks, fields and starting
player, with fresh shuffles and mulligans. It adds no win and opens no sideboard
window. Match draws also require both players. Successful game actions clear
pending draw offers. The protocol does not currently offer a game draw during
pre-game registration/field selection.

## Information and API boundaries

During setup, `observe` returns `phase: "series"` and a `series` object, with no
opening hand or previous game's private information. During a game, the existing
observation remains available and includes `series` metadata. Own registrations,
main lists and sideboards are visible; the opponent's card pool and committed
field stay private. Opponent sideboard counts, legends, revealed chosen champions
and completed game results are public. Completing or conceding a match does not
reveal an uncommitted opposing field.

The local coordinator remains trusted to route each bot only its own view. The
API is not an authenticated multiplayer server. Existing standalone `reset`,
`start` and gameplay commands keep their previous behavior; `reset` leaves the
series and starts a standalone practice game.

See [API.md](API.md#registered-match-commands) for commands,
[engine/API validation](validation/match_series.json) and
[native validation](validation/native_match_series.json) for tested scope.

## Rules audit

Reviewed November 11, 2026 against the current
[Riot Rules Hub](https://playriftbound.com/en-us/rules-hub/), its October 9
[Tournament Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/861752822d217edf37d9febaeaf77a7d924c1bb6.pdf),
and October 9 Core Rules. References: CR103, 110–118 and 488;
TR403, 404, 406, 407, 410 and 601.1. The source hash, reviewed PDF pages and
implementation boundaries are recorded with validation. Event-specific opening
rule trials do not replace the published Standard opening.

## Scouting and policy limits

Scouting stores distinct revealed identities, not inferred copy counts. It reads
public units, non-token gear, trash, banish, chosen champions, actual spells on
the chain, pending public card plays, and explicit hand/deck/facedown reveals.
Repeated observations do not manufacture additional evidence. Token source IDs
are not treated as cards in a deck. Hidden hands, future deck/rune order, preset
identity, opposing registrations and private sideboards are never planner inputs.
Facedown cards revealed at game end are remembered, as required by CR 422.4,
including games ended by concession or an agreed draw.
Some effects resolve between snapshots; this memory can miss information and is
not a reconstruction of the opponent's deck. It persists through games and draws
and resets when a new match starts.

The planner uses deterministic heuristics, not a trained matchup model. A revealed
card does not prove it remains in the opponent's list after sideboarding. Its
balance checks preserve counts, not every synergy. Suggestions are starting
points for practice, not established optimal tournament plans.

Run a paired comparison with the unchanged-deck baseline:

```sh
python3 tools/benchmark_match_ai.py --api target/release/rift-api --output comparison.json
```

This alternates policies with identical seeds, seats and in-game difficulty.
Non-Arito rivals have explicitly authored registrations with empty sideboards;
they are not presented as verified tournament match registrations. Simulated
wins measure these particular opponents and seeds, not human playing strength.

The November 12 evaluation used 32 fresh paired scenarios (64 matches, 151 games)
after one policy revision on a separate earlier cohort. The revised policy won
12/32 matches; keeping the original list won 9/32. Every paired first game had
the same result. These are limited simulated results on four included rivals,
not statistical proof of stronger play or validation against human experts.
See [the validation record](validation/match_ai.json) for the retained earlier
negative result, fresh-cohort setup, tests and visual checks.
