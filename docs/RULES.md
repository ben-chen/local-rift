# Implemented rules and limitations

**Local Rift is an unofficial, incomplete Riftbound client. This document describes its current behavior, not the complete tabletop game.** Full official card text appears in hover previews and pinned inspectors. All 254 bundled cards have executable printed-effect mappings. An official description in the catalog and a passing simulation are not proof of complete tabletop equivalence.

For physical play, use Riot's [Rules Hub](https://playriftbound.com/en-us/rules-hub/), current errata, and printed card text. The core timing reference used here is the [October 9, 2026 Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf). [SOURCES.md](SOURCES.md) records the card and deck sources.

## Setup

Choose a preset or a saved custom deck for each player. Presets contain 40 cards including a chosen champion; custom constructed lists may contain more. The champion begins in its separate playable zone; the remaining cards are shuffled and four cards are drawn. You may replace up to two once. Replacements are drawn before the originals return to the bottom of the deck. The AI chooses its own replacements.

Each player has a 12-card rune deck using the preset’s **published domain split**, preserved in the source manifest. Choose whether you or the AI starts. The second player channels an extra rune on their first turn.

Each player brings one battlefield. The client offers four official choices drawn from the source presets; its two battlefield panels display the selected names and effect text. The default follows each player’s chosen deck. The deck editor can copy a preset, adjust supported cards, rune counts, chosen champion, and battlefield, and save a separate list for you or the AI. It validates the main deck, champion tag, full domain identity, three-copy and Signature limits, and exactly 12 runes. There is no sideboarding.

## Turns and resources

The start of a turn advances through readying, beginning effects, hold scoring, channeling, and drawing. Trigger chains and choices pause these steps before the next one proceeds. Up to two runes are normally channeled, and you draw one card. The second player channels three runes on their first turn only. Both players draw on their first turn.

Runes supply energy by exhausting and power by recycling to the bottom of the rune deck. Use the rune controls to choose a domain to exhaust or recycle before a play. Exhausting stores energy; recycling stores colored power. A ready rune can supply energy before being recycled for power, so a 10-energy, 4-power cost does not require 14 distinct runes. If you play without preparing resources, automatic payment exhausts runes and recycles matching ones as needed, preserving unused energy from ready runes recycled for power.

The optional **Power: choose sources** mode lets you spend exact amounts of generated colored, flexible, and spell-only power, and choose rune domains to recycle. Its quote includes base costs, selected additional costs, and Deflect. It rejects unavailable, mismatched, insufficient, or excess sources without changing the match. The API exposes the same quote and payment action.

Printed costs and all printed domains are retained. For a card with multiple domains, its colored power can come from either printed domain. Generated colored, flexible, and spell-only power have separate pools; the latter only pays spell costs. Energy and power pools empty at the beginning of the main phase and at the end of the turn. The interface distinguishes stored energy from energy available by exhausting ready runes. Ezreal lets you allocate its reduction between energy and power for eligible optional additional costs; it does not discount Equip or Deflect.

## Battlefield choices

| Default preset | Battlefield | Implemented effect |
|---|---|---|
| Irelia | Sunken Temple | Conquering with a unit that has at least 5 might offers an optional 1-energy payment to draw a card. |
| Master Yi | Seat of Power | On conquest, draw one card for each other battlefield you control. |
| Kennen | Minefield | On conquest, put the top two cards of your main deck into trash. |
| Kai'Sa | Void Gate | Each instance of damage a spell or ability deals to a unit here gains one bonus damage. Combat damage and damage dealt by units are unaffected. |

Conquest triggers use the chain only when the battlefield scores for that player. Reconquering a field already scored that turn does not repeat those triggers. The final-point replacement draw still counts as scoring and does trigger them. Sunken Temple's optional payment finalizes its draw ability; drawing waits for responses. Seat of Power counts other controlled fields when its ability resolves. Void Gate uses Riot's corrected wording, so a spell targeting several locations only gains damage against units at Void Gate.

Only these four battlefield choices are implemented. The other tournament battlefield cards remain source-list references, including the historical banned Arena's Greatest; they are not selectable. The scenery is artistic presentation, not the printed card artwork.

## Legends

Each deck has its registered legend, independent of preset selection and chosen-champion variant. Seven legends are available, with the original four preset identities preserved. Legends ready at the start of their controller's turn.

| Legend | Implemented power |
|---|---|
| Irelia | Choosing a friendly unit offers an optional trigger: exhaust the legend and pay one flexible power to ready that unit. Conquest offers a second optional trigger: pay one energy to ready the legend. |
| Master Yi | A friendly unit defending alone receives +2 might. This is a derived bonus and disappears when the unit attacks, leaves combat, or gains another friendly defender. |
| Kennen | Playing a card from outside the hand triggers Empower. In an open Action window, disempower and exhaust the legend to give a chosen unit Assault 2 for the turn. The bonus applies while that unit attacks. |
| Kai'Sa | Exhaust the legend to add one flexible power usable only for playing spells. It cannot pay unit, gear, or ability costs. |
| Darius | Exhaust for a Reaction resource ability that adds one energy if Legion is active. Legion checks an earlier finalized card this turn, even if it was countered. Before Legion, the legal activation exhausts the legend but adds nothing; see the rule-derived timing note in the audit. |
| Viktor | In an open own main phase, pay one energy and exhaust to queue a Recruit ability. Opponents may respond. After it resolves, choose base or a controlled battlefield for the pending 1 Might Recruit. |
| Lux | Completing a spell with printed Energy cost at least five triggers a separate draw-one ability. Discounts do not change that printed threshold; countered spells do not trigger it. |

Irelia's optional costs are paid before its triggered ability enters the chain. Declining leaves the original play intact. Kennen's Empower and Assault abilities also use the chain. Kai'Sa's resource ability resolves immediately, does not create a response window, and preserves priority. Supported resource abilities remain available during pending payment choices, such as a spell replay; their normal cost restrictions still apply. Generated resources expire when rune pools empty; they are not extra rune cards.

## Card support and targets

Select a hand card or the champion in its own zone to see available controls. Supported units enter exhausted at base or a battlefield you control, unless an implemented ability says otherwise. They normally ready next turn. The champion pays its normal cost; after defeat it goes to the appropriate zone like another unit, rather than returning automatically to the champion zone.

Hover over a card or unit for its art and complete official text; scroll inside a long preview, or right-click to pin the inspector. Generic “Implemented” labels have been removed. Specific practice notes appear only when there is a documented difference. All bundled cards are playable; unknown or unmapped cards are unavailable. No cards are swapped for stronger or simpler versions.

A targeted card requires legal targets when played. The engine can choose valid targets automatically for the play button; direct targeting selects an eligible unit or chain spell. It validates allegiance, location, and card-specific restrictions. Several effects require multiple targets. Select those units before confirming the play, or let the automatic-target control choose eligible targets. The Repeat target toggle selects a separate group for the repeated effect. Targeted on-play triggers ask for their targets after the permanent enters play. Fizz and Last Rites choose an exact public trash target before opponents respond. Once their parent ability resolves, the physical card enters Pending, then its child play chooses legal targets, destination, and additional costs. Matriarch follows the same pending-play timing. Trash replay choices retain target and destination controls. Explicit target lists preserve an intentional zero-target choice where the card allows it. If responses separate Bellows Breath's targets into different locations, its controller chooses which maximal legal group remains affected. Deflect is charged for each time a unit is chosen, including repeated choices.

## Persistent gear

Gear remains on the board and normally enters ready at base; Iron Ballista explicitly enters exhausted. Equipment adds its printed might bonus and equipment text to its attached unit. The gear remains a separate object: if the unit leaves, the equipment detaches and remains at base.

Equip is an activated ability with its own power and additional costs. It enters the chain and can be answered. Quick-Draw permits the gear card's reaction timing and creates its free attachment trigger; it does not make an ordinary Equip activation a Reaction. Last Rites asks you to choose two trash cards to recycle for Equip, and its attached unit’s conquest or hold trigger can offer a unit play from trash with normal costs.

The gear implementation includes Boots of Swiftness's might/Ganking, Guardian Angel's heal-and-recall death replacement, the errata-corrected Zhonya's Hourglass replacement, Seal of Discord's generated Chaos power, and expendable Gold tokens that generate flexible power. Temporary gear is removed at the appropriate beginning; attached equipment's own rules text is inactive while attached.

When multiple death replacements apply, the affected unit's controller chooses which one to use, including Guardian Angel, Zhonya's Hourglass, and Smite's banishment. Remaining replacements are checked again if applicable. Last Rites uses a triggered-ability chain before its optional unit choice, and Hourglass can use Hidden. Check the inspector and [errata provenance](SOURCES.md).

## Unit and gear activations

A selected unit exposes **Abilities** alongside its movement controls. Choose the printed ability, select its targets, then confirm its costs. Gear uses the same controls for its printed activations. Each physical source is tracked separately, including two copies of the same card. Cancelling before confirmation spends nothing; committed costs are paid before opponents respond. Exact power-source choices apply to these costs too, including Deflect, and spell-only power cannot pay for them.

Heart of Dark Ice and Arena Kingpin grant temporary Might; Orb of Regret reduces Might with its printed minimum when the effect resolves. Arena Bar buffs an exhausted friendly unit. Iron Ballista targets a battlefield unit. Xerath and Caitlyn must themselves be at a battlefield to activate; this source restriction is checked when activating. Their targets must still satisfy the target restrictions when the ability resolves. Caitlyn deals damage equal to her current Might, and cannot supply a Might value after leaving play. Vanguard Armory queues three separate Recruit plays after its ability completes, allowing a legal destination for each.

Renata Glasc has two distinct abilities: pay one energy and one Mind power to draw, or pay four of each and exhaust her to score one point. Both require her at a battlefield. The draw ability does not exhaust her. Her point is not conquest and does not consume a battlefield's scoring opportunity.

Dragonsoul Sage's Reaction ability exhausts it to add one energy immediately, including during permitted payment choices. This does not allow the unit card itself to be played as a Reaction. The other activations in this group require the controller's open main phase. Non-resource abilities enter the chain and allow responses; gear-use triggers occur after their ability resolves, including when its chosen target has left, and do not fire for a countered ability.

Garbage Grabber recycles three selected trash cards and pays one energy before drawing on resolution. Assembly Rig instead recycles one unit, pays one energy and one Fury power, and plays a 3 Might Mech token at base after resolving. The trash choices are costs, not targets. Cards recycled together go to the bottom in random order. Divining Shells supplies Vision when played and kills and exhausts itself to give a unit temporary Might. Hextech Formula enters exhausted and can Empower another gear, including an opponent's or attached gear.

Ancient Henge converts energy to flexible power; Hextech Anomaly converts any-domain power to energy. Exhaust is their activation cost. The conversion amount is chosen and paid as the immediate ability resolves. You may use another Add ability during that payment, finish it, then resume the original conversion. A zero conversion completes without restoring the source's ready state. Exact power choices apply to Anomaly, and spell-only power cannot pay for it.

A committed payment keeps its targets, optional choices, and calculated discounts fixed while you generate resources. Target legality is checked again at completion. If the attempt cannot finish, rollback restores the complete position before the attempt, including resources spent or generated, and returns to any original mandatory choice. It never skips that mandatory action. A fully payable legal attempt must finish.

## Hidden, trash plays, and optional costs

A card with Hidden can be placed facedown for one flexible power at a battlefield you control during your open main phase. There is one facedown slot per battlefield. Hiding is not playing the card. On a later turn, the card can be played as a Reaction with its base cost waived; additional costs such as Deflect still apply. Losing control reveals the hidden card and puts it in trash.

Hidden plays normally restrict their targets to that battlefield, including when targets are rechecked on resolution. Explicit card exceptions, such as Tideturner's other-location swap, retain their printed permission. Opposing facedown identities remain concealed unless an effect grants access or the card is played. Scuttle Crab’s death trigger reveals the opponent’s hand and grants inspection of their facedown cards until the turn ends.

The client supports Fizz's energy-waived spell replay, Last Rites unit plays from trash, Kennen's temporary Flow grant, and printed Flow conditions and costs. Fizz and Last Rites offer their optional play during trigger finalization; once accepted, a legal available play is required when the ability resolves. If no legal play remains, the choice can finish without playing a card. Trash plays enter the ordinary timing and response system. Matriarch and Kennen choose their trash cards before the revive or Flow-grant ability enters the chain; if that exact card leaves the trash before resolution, the ability cannot affect a replacement copy. Tornado Warrior similarly chooses its Empower target before opponents respond. Its delayed end-of-turn Disempower trigger uses the chain, with a response window before temporary effects expire. Kennen's legend triggers when an actual card is played from a zone other than hand; hiding a card alone does not empower it.

When a trash spell has both printed and temporarily granted Flow, you can select either available cost. Normal timing and additional costs still apply; an explicit unaffordable choice is rejected without substituting the other cost. Kennen’s grant expires at the appropriate turn end.

Supported play options include Repeat, Accelerate, Rampage's optional might payment, and eligible units' Empower abilities. Eligible trash plays also expose optional costs: Fizz can Repeat Hard Bargain, a Flow-granted Bellows Breath can Repeat, and Last Rites can Accelerate Kai'Sa. Matriarch chooses its eligible trash unit before responses, then offers Accelerate when that specific unit is actually played. Waiving a base cost does not waive additional costs. Optional costs must be selected and paid before the play is finalized. Up from the Deep and Recruit the Vanguard ask where to place each token separately, using your base or a battlefield you control. Vanguard Captain's tokens instead use its current location when the trigger resolves; a source that has left the board has no location. Empower persists on the object rather than expiring at turn end. Inspect the printed text and current unit state when choosing an option.

The expanded unit pool includes explicit Assault, Shield, Tank, Deflect, Ganking, Ambush, Accelerate, and Legion behavior. Shield adds its printed amount to Might while defending in combat; it does not change Tank/Backline damage priority. Equipment-granted keywords are included in the unit inspector and current Might. Ready-entry units enter ready, and Battering Ram’s energy discount counts cards played that turn while respecting its one-energy minimum. Legion play triggers capture whether another card was played that turn, then enter the ordinary response system. Discard-and-draw effects keep their resolving chain item in place until the discard choice and draws finish. Lunar Boon therefore cannot recycle itself into its own draw during burnout. Repeated draws also finish before the physical spell enters trash. Printed Flow can require a specific colored power: Iterative Design requires Mind, while a printed Any-power Flow cost accepts any domain. Wind Wall counters a spell of any cost, including its controller's spell, but cannot counter a triggered ability.

### Gear Empower and Disempower costs

Questionable Tome, Hextech Disc, Platewyrm Egg, and Tools of Empire have separate printed Empower activations. They are ordinary abilities, usable only when the exact source is not already Empowered. They do not choose or target their own source. Their printed costs are paid before responses; the source becomes Empowered only when the ability resolves. If that source leaves play, another copy does not inherit the effect. Tools pays two Energy without Exhaust, while the other three require Exhaust; Egg also costs one Energy.

Tome and Disc require an Empowered source for their second ability. Disempower, one Energy, and Exhaust are costs paid before responses. The later draw or Mech play still resolves if the source leaves, and countering the effect does not refund those costs. A fixed payment keeps the intended ability and resource obligation; source costs must still be legal when payment completes. Egg cannot fund its own Exhaust-cost Empower by first exhausting itself to add Energy. If that makes completion impossible, rollback restores the original position.

Egg enters exhausted. Its separate Reaction ability immediately adds one Energy, or two if it is currently Empowered, and can operate during permitted payments. Tools gives a chosen unit +2 Might this turn, or +4 if its source is Empowered when the instruction resolves. These are alternative amounts, not cumulative bonuses. A departed source cannot supply its former Empowered state for that condition.

Poro Snax draws through its exact-object on-play trigger; its later draw activation pays Energy, Calm power, Exhaust, and Kill-self first. The Syren uses its corrected text to Move a friendly battlefield unit to its base, preserving its ready/exhausted state. Iridescent Avian either Buffs its chosen unit or, if that unit is already buffed when the effect resolves, gives +2 Might this turn instead. Ultrasoft Poro can activate only at a battlefield and queues two separate 1-Might Bird tokens with Deflect after its ability finishes. Each token has its own ordinary legal placement choice; the source's later departure does not cancel them.

## Priority, chains, and showdowns

The turn owner and the player allowed to act are separate. The interface identifies **your priority** or **AI priority**. You can receive priority during the opponent's turn.

During an ordinary open turn, the turn owner can play legal cards and make standard moves. Moving into an uncontrolled battlefield opens a showdown, including when the battlefield is empty. Control and scoring wait for that window to finish.

In an **open showdown**, the player with focus may play a legal Action or Reaction, or pass. Two consecutive passes close the showdown and advance to combat or control. Playing a spell creates a chain; the game is then in a **closed state**, where only legal Reactions can be added.

If multiple battlefields become staged during a closed chain, they wait for the chain and pending finalizations to finish. Non-combat showdowns open before staged combats, and the turn player chooses among fields of the same class. The player who applied Contested is the contender and receives initial focus; the player who began the spell chain does not determine the attacker. Entering an ongoing non-combat showdown during a closed chain does not activate combat designations until that chain ends.

The newest chain item resolves first after consecutive priority passes. Priority is reassessed between resolutions. A pass is not an irrevocable promise to stop responding: a new play or a resolved item can create another decision. When a chain completes during a showdown, focus returns according to that window's state.

**Pass priority** and `Space` pass within a window. They only end your turn when no chain, showdown, or pending choice remains. A pending choice must be answered first. Implemented counters include countering, returning a spell to hand, and asking the spell's controller to pay additional energy. Predict offers its controller a keep-on-top or recycle-to-bottom choice; it does not itself draw a card, and looking at an empty deck does not cause burnout.

Triggered abilities and turn-start steps pause for choices and response windows. When several triggers are pending, the active player finalizes their triggers first, followed by the other player. Each controller chooses the order of their own triggers; the last placed on the chain resolves first. On-play targeting and optional trigger payments happen during finalization, before the resulting ability can resolve.

The expanded card pool has explicit observers for playing cards and tokens, readying, exhausting, buffing, moving, combat designations, drawing, death, holding, conquest, and gear activation. Chosen trigger targets are selected before the ability enters the chain, with Deflect paid then; implicit references such as “me” or “it” do not create an extra target choice. Optional Windsinger play triggers can be declined. “Here” and applicable Hidden restrictions are checked again on resolution. Attached equipment abilities use the bearer as their source. Surviving death observers are distinct from Deathknell abilities captured before their source leaves play; a first-time observer does not multiply across simultaneous deaths. These are explicit card mappings, not a parser that treats arbitrary printed text as implemented.

The additional spells include readying and fights with Repeat, ordered damage and Gold creation, Sand Soldier creation, and modifiers or draws based on current public board state. Their condition checks and optional costs follow their printed instructions. Dynamic discounts are evaluated when paying; temporary Might modifiers expire at turn end. As with the older pool, these tests cover selected rule cases rather than proving all interactions.

## Movement and combat

A standard move exhausts a ready unit. It can move from base to a battlefield, or from a battlefield back to base. Direct battlefield-to-battlefield standard movement is unavailable unless a specifically implemented ability permits movement. A readying effect may enable another move during the same turn.

Use **Group all … ready here** to move several ready units from the selected unit’s location in a single standard move. Shift-click individual units to choose a different group. Official rules also permit units with different origins to move together to one destination; the engine accepts that, while this grouping button selects one origin. Choose the group before entering a showdown: ordinary movement cannot add extra attackers after the response window begins. Cards with legal timing may still change the fight.

After the showdown closes, each player assigns their captured combat might among the opposing units. Exhausted units normally contribute might; stun and combat modifiers may change that total. Assign all damage, respecting Tank first and Backline last, and assign lethal damage before moving to another unit. If a unit has both restrictions, such as Caitlyn wearing Doran’s Shield, the assigning player may place it first or last, independently for each such unit. The UI offers a suggested allocation, target reordering, and an **Exact amounts** editor with per-unit controls. It shows assigned and remaining damage, lethal requirements, and any invalid choice before confirmation. A rejected allocation preserves the draft and game state. Both sides finish assigning before damage is dealt simultaneously. If lethal damage creates a replacement choice, the affected units stay visible until their controller makes that choice and cleanup resumes.

A unit with positive damage at least equal to its current might is defeated. A zero-might unit is not defeated merely for having zero might and can conquer or hold an undefended battlefield. Combat cleanup clears damage from **all surviving units**, including those elsewhere on the board. If both sides retain units after combat, surviving attackers return to base, preserving their ready or exhausted state. An empty battlefield becomes unclaimed. End-of-turn cleanup also clears surviving damage and temporary might changes.

## Scoring and victory

A player wins with **at least eight points and more points than the opponent**, checked by game cleanup.

- **Conquer:** gain control of a battlefield with your units there and no enemy unit remaining, after its response window resolves.
- **Hold:** begin your turn controlling an uncontested battlefield with your units still there.
- Each battlefield can score once for each player during a turn. Switching control repeatedly does not let the same player score that battlefield again that turn.

Holding can earn the eighth point directly. A conquest earns the eighth point only after the other battlefield has already scored for you that turn. Otherwise it draws a card instead and still uses that battlefield's scoring opportunity. Scoring the other battlefield later that turn can then win.

When drawing or burning requires a card from an empty main deck, the trash is shuffled into a new deck and the opponent scores one point. That burnout point can end the match. If no drawable card becomes available, burnout repeats until someone wins. Predict is not a draw. There is no match timer.

## AI and Mote

**Relaxed** makes simple choices and may accept a poor attack. **Tactical** considers public battlefield strength, useful interaction, resources, and scoring pressure. **Strategic** compares candidate outcomes and a bounded second own action, with group movement and explicit spell targets. It resolves hypothetical chains assuming further discretionary responses are passed, and samples unknown draws from cards legal for the public legend identity. All three levels are deterministic. They do not examine the opponent’s hidden hand, private list, facedown identities, or future deck order; Strategic also replaces its own unknown future deck order before simulation. Strategic adds planning depth; its playing strength has not been calibrated against human experts. The AI handles priority passes and its own pending choices, with a bounded action loop to keep play moving.

Mote's **Beginner** advice explains resources, deployment, scoring, and the current timing window. **Advanced** advice discusses future holds, useful trades, interaction costs, and broad archetype goals. It sees your hand and information available to you, including cards an effect explicitly reveals or lets you inspect. It does not examine the opponent’s private hand, facedown identities, or Predict result. Its small panel is optional and never blocks input.

The deck library includes the real legends and authored strategy notes. All seven registered legend powers execute in matches, while the notes describe strategic goals beyond what the simple AI can teach. Mote and the simulator cannot establish competitive matchup rankings.

## Remaining fidelity limits

Seven supported legends, four battlefield choices, and 254 main-deck cards are available through the four supplied presets and custom deck editor. There is no sideboarding, tournament match structure, multiplayer mode, or full card-set coverage. The presets are dated tournament lists, not a ranking of the current metagame.

The desktop and bot API expose exact damage allocations and power-source payments. The default automatic payment remains available; energy can be prepared by exhausting chosen rune domains before a play. These controls share engine validation, but broader card and interaction coverage is still required.

Further sequencing and interaction audits are still needed. Tests cover particular official examples and whole-match consistency; they do not establish that every interaction is identical to standard Riftbound. Consult the official reference when preparing for an interaction outside those checked cases.

The [independent audit](RULES_AUDIT.md) tracks confirmed fixes and open cases, including separate events within one resolving item. The [full card inventory](CARD_COVERAGE.md) measures unimplemented cards separately from implemented mappings.
