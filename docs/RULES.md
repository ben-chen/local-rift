# Implemented rules and limitations

**Local Rift is an unofficial, incomplete Riftbound client. This document describes its current behavior, not the complete tabletop game.** Full official card text appears in hover previews and pinned inspectors. All 379 bundled main-deck cards have executable printed-effect mappings. An official description in the catalog and a passing simulation are not proof of complete tabletop equivalence.

For physical play, use Riot's [Rules Hub](https://playriftbound.com/en-us/rules-hub/), current errata, and printed card text. The core timing reference used here is the [October 9, 2026 Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf). [SOURCES.md](SOURCES.md) records the card and deck sources.

## Setup

Choose a preset or a saved custom deck for each player. Presets contain 40 cards including a chosen champion; custom constructed lists may contain more. The champion begins in its separate playable zone; the remaining cards are shuffled and four cards are drawn. You may replace up to two once. Replacements are drawn before the originals return to the bottom of the deck. The AI chooses its own replacements.

Each player has a 12-card rune deck using the preset’s **published domain split**, preserved in the source manifest. Choose whether you or the AI starts. The second player channels an extra rune on their first turn.

Each player brings one battlefield. The client offers eleven official choices; its two battlefield panels display the selected names and effect text. The default follows each player’s chosen deck. The battlefield picker shows artwork and full rules before selection. The deck editor can copy a preset, adjust supported cards, rune counts, chosen champion, and battlefield, and save a separate list for you or the AI. It validates the main deck, champion tag, full domain identity, three-copy and Signature limits, and exactly 12 runes. There is no sideboarding.

## Turns and resources

The start of a turn advances through readying, beginning effects, hold scoring, channeling, and drawing. Trigger chains and choices pause these steps before the next one proceeds. Up to two runes are normally channeled, and you draw one card. The second player channels three runes on their first turn only. Both players draw on their first turn.

Runes supply energy by exhausting and power by recycling to the bottom of the rune deck. Use the rune controls to choose a domain to exhaust or recycle before a play. Exhausting stores energy; recycling stores colored power. A ready rune can supply energy before being recycled for power, so a 10-energy, 4-power cost does not require 14 distinct runes. If you play without preparing resources, automatic payment exhausts runes and recycles matching ones as needed, preserving unused energy from ready runes recycled for power.

The optional **Power: choose sources** mode lets you spend exact amounts of generated colored, flexible, and spell-only power, and choose rune domains to recycle. Its quote includes base costs, selected additional costs, and Deflect. It rejects unavailable, mismatched, insufficient, or excess sources without changing the match. The API exposes the same quote and payment action.

Printed costs and all printed domains are retained. For a card with multiple domains, its colored power can come from either printed domain. Generated colored, flexible, and spell-only power have separate pools; the latter only pays spell costs. Energy and power pools empty at the beginning of the main phase and at the end of the turn. The interface distinguishes stored energy from energy available by exhausting ready runes. Ezreal, Prodigy lets you allocate its reduction between energy and power for eligible optional additional costs; it does not discount Equip or Deflect.

## Battlefield choices

| Default preset | Battlefield | Implemented effect |
|---|---|---|
| Irelia | Sunken Temple | Conquering with a unit that has at least 5 might offers an optional 1-energy payment to draw a card. |
| Master Yi | Seat of Power | On conquest, draw one card for each other battlefield you control. |
| Kennen | Minefield | On conquest, put the top two cards of your main deck into trash. |
| Kai'Sa | Void Gate | Each instance of damage a spell or ability deals to a unit here gains one bonus damage. Combat damage and damage dealt by units are unaffected. |
| — | The Academy | Holding creates a next-spell Repeat option equal to that spell's base cost, lasting this turn. |
| — | Firelights' Hideout | Conquering grants Accelerate to the next unit played this turn. The player may pay its additional cost to enter ready. |
| — | Ornn's Forge | While controlled, reduces the first friendly non-token gear play each turn by one Energy. |
| — | Hunters' Circle | For each player, reduces their first unit or gear played at this field on their own turn by one Energy. |
| — | Piltovan Forge | While controlled, reduces the first friendly gear activated ability each turn by one Energy. |
| — | Risen Altar | Reduces the controller's units' Empower costs here by a chosen one Energy or one Power. |
| — | Vaults of Helia | Holding creates a one-Energy surcharge on the controller's non-token unit plays for the rest of the turn. |

Conquest triggers use the chain only when the battlefield scores for that player. Reconquering a field already scored that turn does not repeat those triggers. The final-point replacement draw still counts as scoring and does trigger them. Sunken Temple's optional payment finalizes its draw ability; drawing waits for responses. Seat of Power counts other controlled fields when its ability resolves. Void Gate uses Riot's corrected wording, so a spell targeting several locations only gains damage against units at Void Gate.

Hold and conquest grants are respondable abilities; their effects begin when those abilities resolve. First-play cost discounts use the relevant player's earlier finalized plays, including before that player acquired control. A zero-cost qualifying play still consumes the first position. Two copies of a Forge affect the same first play, while Hunters' Circle tracks each physical field separately. Piltovan also counts resource gear activations and Equip, but not a triggered Quick-Draw attachment. Its nested-payment behavior follows the current finalization and frozen-cost rules; this is a rule-derived interpretation rather than a located Piltovan-specific FAQ.

Other tournament battlefields remain source-list references, including the historical banned Arena's Greatest; they are not selectable. The picker displays official field art and corrected readable text; the mat keeps each selected effect visible with a full-rules inspector.

## Legends

Each deck has its registered legend, independent of preset selection and chosen-champion variant. Ten legends are available, with the original four preset identities preserved. Legends ready at the start of their controller's turn.

| Legend | Implemented power |
|---|---|
| Irelia | Choosing a friendly unit offers an optional trigger: exhaust the legend and pay one flexible power to ready that unit. Conquest offers a second optional trigger: pay one energy to ready the legend. |
| Master Yi | A friendly unit defending alone receives +2 might. This is a derived bonus and disappears when the unit attacks, leaves combat, or gains another friendly defender. |
| Kennen | Playing a card from outside the hand triggers Empower. In an open Action window, disempower and exhaust the legend to give a chosen unit Assault 2 for the turn. The bonus applies while that unit attacks. |
| Kai'Sa | Exhaust the legend to add one flexible power usable only for playing spells. It cannot pay unit, gear, or ability costs. |
| Darius | Exhaust for a Reaction resource ability that adds one energy if Legion is active. Legion checks an earlier finalized card this turn, even if it was countered. Before Legion, the legal activation exhausts the legend but adds nothing; see the rule-derived timing note in the audit. |
| Viktor | In an open own main phase, pay one energy and exhaust to queue a Recruit ability. Opponents may respond. After it resolves, choose base or a controlled battlefield for the pending 1 Might Recruit. |
| Lux | Completing a spell with printed Energy cost at least five triggers a separate draw-one ability. Discounts do not change that printed threshold; countered spells do not trigger it. |
| Ambessa / Matriarch of War | Empowering another object triggers a separate ability to Empower the legend, even if the affected object belongs to the opponent. In the controller's open main phase, Disempower, pay one Any Power, and exhaust to Ready a chosen unit on resolution. Enemy Deflect adds its normal cost. |

Rengar and Ezreal add the following powers:

| Legend | Implemented power |
|---|---|
| Rengar / Pridestalker | Completing a unit play, including a token, triggers a required choice of one unit. The triggered ability enters the chain and gives that unit +1 Might for the current turn on resolution. Enemy targets pay Deflect and respect untargetability. |
| Ezreal / Prodigal Explorer | After choosing enemy units or gear at least twice this turn with spells or unit abilities, exhaust the legend as a Reaction to queue a draw-one ability. Choosing happens during finalization, so countering the spell does not erase it. Gear, legend and battlefield abilities do not count. Progress resets at each turn boundary for both seats. |

Rengar's triggered choice can select the newly played unit. Moving the target preserves its identity; leaving play makes the bonus mistarget. A mandatory trigger with no finalizable legal target is removed under the same trigger rules as other abilities. Ezreal's draw uses the ordinary response chain and cannot interrupt a pending payment as though it were an Add ability. Its visible progress is capped at the required two choices, not a count of all choices made.

Irelia's optional costs are paid before its triggered ability enters the chain. Declining leaves the original play intact. Kennen's Empower and Assault abilities also use the chain. Kai'Sa's resource ability resolves immediately, does not create a response window, and preserves priority. Supported resource abilities remain available during pending payment choices, such as a spell replay; their normal cost restrictions still apply. Generated resources expire when rune pools empty; they are not extra rune cards.

## Card support and targets

Select a hand card or the champion in its own zone to see available controls. Supported units enter exhausted at base or a battlefield you control, unless an implemented ability says otherwise. They normally ready next turn. The champion pays its normal cost; after defeat it goes to the appropriate zone like another unit, rather than returning automatically to the champion zone.

Hover over a card or unit for its art and complete official text; scroll inside a long preview, or right-click to pin the inspector. Generic “Implemented” labels have been removed. Specific practice notes appear only when there is a documented difference. All bundled cards are playable; unknown or unmapped cards are unavailable. No cards are swapped for stronger or simpler versions.

A targeted card requires legal targets when played. The engine can choose valid targets automatically for the play button; direct targeting selects an eligible unit, gear, chain spell, or battlefield according to the card. It validates allegiance, location, and card-specific restrictions. Several effects require multiple targets. Select those targets before confirming the play, or let the automatic-target control choose eligible targets. Each paid Repeat has its own target selection. Targeted on-play triggers ask for their targets after the permanent enters play. Fizz and Last Rites choose an exact public trash target before opponents respond. Once their parent ability resolves, the physical card enters Pending, then its child play chooses legal targets, destination, and additional costs. Matriarch follows the same pending-play timing. Trash replay choices retain target and destination controls. Explicit target lists preserve an intentional zero-target choice where the card allows it. If responses separate Bellows Breath's targets into different locations, its controller chooses which maximal legal group remains affected. Deflect is charged for each time a unit is chosen, including repeated choices.

Firestorm chooses one physical battlefield, even if it is empty. Click that field while editing the intended execution; a repeated Firestorm can choose a different field. It damages enemies present there when that execution resolves. Flurry of Blades damages all units at battlefields, and Avalanche all units in bases. These affected units are not individually chosen, so they do not add Deflect costs or trigger abilities that require choosing a unit. Each damage packet uses the source and location's applicable bonuses; lethal-damage cleanup waits until the physical spell finishes all its executions.

Vengeance, Soul Harvest, and Blast of Power issue an actual Kill instruction, including applicable death replacements. Soul Harvest checks the target's current Might limit both when played and when resolving. Blast of Power's target must still be at a battlefield. Supernova skips damage to an invalidated target but still draws two; countering the spell prevents both instructions. Rune Prison stuns without independently exhausting or preventing movement. Cleave grants temporary Assault 3, which stacks with other Assault and contributes Might only while attacking.

## Persistent gear

Gear remains on the board and normally enters ready at base; Iron Ballista and Honeyfruit explicitly enter exhausted. Equipment adds its printed might bonus and equipment text to its attached unit. The gear remains a separate object: if the unit leaves, the equipment detaches and remains at base.

Equip is an activated ability with its own power and additional costs. It enters the chain and can be answered. Quick-Draw permits the gear card's reaction timing and creates its free attachment trigger; it does not make an ordinary Equip activation a Reaction. Last Rites asks you to choose two trash cards to recycle for Equip, and its attached unit’s conquest or hold trigger can offer a unit play from trash with normal costs.

The gear implementation includes Boots of Swiftness's might/Ganking, Guardian Angel's heal-and-recall death replacement, the errata-corrected Zhonya's Hourglass replacement, Seal of Discord's generated Chaos power, and expendable Gold tokens that generate flexible power. Temporary gear is removed through its beginning-step ability. Attached equipment's printed rules text is inactive, but granted rules text remains active: attaching gear affected by Turn to Dust does not prevent its Temporary expiration. The inspector distinguishes the two origins.

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

Temporal Portal, The Academy, and Syndra can grant independent Repeat instances. Each has its own cost and target choices; choose their execution order before paying. Syndra's Chaos Power requirement stays distinct from another Repeat's domain requirement. The physical spell stays on the chain through every repeated effect and intervening mandatory choice, then completes one card play. A repeated look or discard-and-draw effect asks for each choice separately. Countering the spell prevents all its executions.

Next-play grants attach when the applicable play begins, including when that card or token is queued as Pending. Declining an optional Repeat or Accelerate does not save that next-play grant for another card. A fixed payment retains the selected grant and discount through resource generation, and rollback restores them. Raging Firebrand grants a next-spell Energy discount. Astral Heron observes the first completed card play each turn while at a battlefield, then grants a next-card Energy and Power discount; an earlier countered card does not count as a completed play. **Turn effects** shows active grants, their source, and expiry.

Sun Disc, Bushwhack, Confront, and Magma Wurm can make units enter ready. Entering ready is different from performing a Ready action on a unit already in play. Magma Wurm affects other units and does not ready its own first copy. Rek'Sai grants Accelerate to units played from outside the hand, including pending unit tokens. Nami's optional Calm payment creates a targeted Stun trigger after entry; her Hold effect instead creates a delayed unit-play trigger that actually Readies and Buffs that unit, with the corresponding observers and response window.

Ravenborn Tome attaches Bonus Damage to the next spell even if that spell deals no damage. Annie supplies a live bonus to her controller's spells and abilities. Consuming Curse counts same-name cards currently in that player's trash for each damage packet; its own resolving physical card is still on the chain. These bonuses add to applicable Void Gate damage, but never turn a unit's combat or Fight damage into spell damage, and never create damage from a zero-damage instruction.

Cost modifiers preserve their scope. Ezreal's discounts apply separately to optional-cost components before whole-play discounts. Applied Researchers, Stargazer, Vex, Irelia, and battlefield modifiers then follow their conditions, permitted order, and individual minima. Helm's surcharge is applied before discounts. A Power reduction can be allocated among eligible colored cost components; an Energy-or-Power discount requires one choice. A discounted optional cost still counts as paid at zero. **Review costs** shows these choices before resource payment.

The expanded unit pool includes explicit Assault, Shield, Tank, Deflect, Ganking, Ambush, Accelerate, and Legion behavior. Shield adds its printed amount to Might while defending in combat; it does not change Tank/Backline damage priority. Equipment-granted keywords are included in the unit inspector and current Might. Ready-entry units enter ready, and Battering Ram’s energy discount counts cards played that turn while respecting its one-energy minimum. Legion play triggers capture whether another card was played that turn, then enter the ordinary response system. Discard-and-draw effects keep their resolving chain item in place until the discard choice and draws finish. Lunar Boon therefore cannot recycle itself into its own draw during burnout. Repeated draws also finish before the physical spell enters trash. Printed Flow can require a specific colored power: Iterative Design requires Mind, while a printed Any-power Flow cost accepts any domain. Wind Wall counters a spell of any cost, including its controller's spell, but cannot counter a triggered ability.

### Live Empowered abilities

Empowered is a status of the physical object. Ordinary Empower activations pay their printed costs before opponents may respond, and apply the status when they resolve. An already-Empowered object does not become Empowered again. Returning a unit to hand or otherwise removing its physical object ends its status; another copy does not inherit it.

The supported conditional Might, Assault, Shield, Deflect, and Ganking abilities follow the live status. The unit inspector explains active and inactive benefits and identifies each contributing source. These benefits do not add Buff counters. Rage Amplifier grants +1 Might, or +2 instead while Empowered. Each Empowered Aurok General grants +2 to friendly Empowered units, including itself and eligible tokens. Removing an aura can make wounded units die; cleanup completes all resulting deaths before finalizing their triggered abilities.

Frostcoat Mother and Grumpy Rockbear's nominal 12-Energy Empower costs lose one Energy per controlled rune, including exhausted runes. Baccai Sandspinner's nominal five becomes two with four or fewer controlled runes. The ability controls and API quote the payable cost; the printed cost remains available in details. A payment fixes that cost before resource generation or rune recycling.

Apprentice Mage's actual Empower transition triggers Predict2. Its controller looks at up to two available cards, chooses which to recycle, and orders the rest on top. Cards stay in the deck while the private choice is pending; confirming commits the whole choice. Predict neither draws nor causes burnout with a short deck. The other seat and its companion cannot see the looked-at cards.

Covert Informant's Move, Nasus's conquest, Emissary and Witherclaw's Deathknell, and Ambessa's attack require Empowered status when the event occurs. Once created, the trigger persists independently. Akali's Move records the battlefields she moved to or from, so those locations remain valid if she leaves; its damage is two only while that same source remains Empowered, otherwise one. Ambessa's kill rechecks an enemy's current location and strictly lower Might on resolution. Witherclaw channels up to two available runes exhausted, without drawing or burnout.

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

Triggered abilities and turn-start steps pause for choices and response windows. Pending event groups finalize in chronological order. Within one simultaneous trigger group, the active player finalizes their triggers first, followed by the other player. Each controller chooses the order of their own triggers; the last placed on the chain resolves first. On-play targeting and optional trigger payments happen during finalization, before the resulting ability can resolve.

The expanded card pool has explicit observers for playing cards and tokens, readying, exhausting, buffing, moving, combat designations, drawing, death, holding, conquest, and gear activation. Chosen trigger targets are selected before the ability enters the chain, with Deflect paid then; implicit references such as “me” or “it” do not create an extra target choice. Optional Windsinger play triggers can be declined. “Here” and applicable Hidden restrictions are checked again on resolution. Attached equipment abilities use the bearer as their source. Surviving death observers are distinct from Deathknell abilities captured before their source leaves play; a first-time observer does not multiply across simultaneous deaths. These are explicit card mappings, not a parser that treats arbitrary printed text as implemented.

The additional spells include readying and fights with Repeat, ordered damage and Gold creation, Sand Soldier creation, and modifiers or draws based on current public board state. Their condition checks and optional costs follow their printed instructions. Dynamic costs are determined before payment and remain fixed through resource generation; temporary Might modifiers expire at turn end. As with the older pool, these tests cover selected rule cases rather than proving all interactions.

## Movement and combat

A standard move exhausts a ready unit. It can move from base to a battlefield, or from a battlefield back to base. Direct battlefield-to-battlefield standard movement is unavailable unless a specifically implemented ability permits movement. A readying effect may enable another move during the same turn.

Use **Group all … ready here** to move several ready units from the selected unit’s location in a single standard move. Shift-click individual units to choose a different group. Official rules also permit units with different origins to move together to one destination; the engine accepts that, while this grouping button selects one origin. Choose the group before entering a showdown: ordinary movement cannot add extra attackers after the response window begins. Cards with legal timing may still change the fight.

After the showdown closes, each player assigns their captured combat might among the opposing units. Exhausted units normally contribute might; stun and combat modifiers may change that total. Assign all damage, respecting Tank first and Backline last, and assign lethal damage before moving to another unit. If a unit has both restrictions, such as Caitlyn wearing Doran’s Shield, the assigning player may place it first or last, independently for each such unit. The UI offers a suggested allocation, target reordering, and an **Exact amounts** editor with per-unit controls. It shows assigned and remaining damage, lethal requirements, and any invalid choice before confirmation. A rejected allocation preserves the draft and game state. Both sides finish assigning before damage is dealt simultaneously. If lethal damage creates a replacement choice, the affected units stay visible until their controller makes that choice and cleanup resumes.

A unit with positive damage at least equal to its current might is defeated. A zero-might unit is not defeated merely for having zero might and can conquer or hold an undefended battlefield. Combat cleanup clears damage from **all surviving units**, including those elsewhere on the board. If both sides retain units after combat, surviving attackers return to base, preserving their ready or exhausted state. An empty battlefield becomes unclaimed. End-of-turn cleanup also clears surviving damage and temporary might changes.

Crimson Pigeons gains its bonus while attacking with another friendly unit at that location. Wielder of Water gains its bonus while attacking or defending alone. These conditions require actual combat and count friendly tokens, stunned or exhausted units, and units with zero Might. Captain Farron and Taric continuously grant their printed keyword to other friendly units at their current location; they do not grant it to themselves. Moving or removing the source updates the bonus. The inspector identifies each contributing source separately.

## Global instructions and live equipment effects

The Ruination kills all units at both bases and battlefields as one explicit instruction. Death replacements apply to the original set of units; choosing one replacement does not recompute that set. A repeated execution performs a new Kill instruction, so a unit saved by the first execution can die in the next. The physical spell remains on the chain while replacement choices and all executions finish. New lethal damage caused by a lost aura is handled by subsequent cleanup.

Mobilize and Catalyst of Aeons channel available runes exhausted. Mobilize draws only if it channels none; Catalyst draws once if it channels fewer than two, including when one rune was available and successfully channeled. Exhausted runes contribute to controlled-rune thresholds without supplying ready Energy. Raucous Anthem readies friendly units and gives them temporary Might; an already-ready unit still receives the bonus without producing a Ready event. Unchecked Power exhausts friendly units at bases and fields, then damages every unit at a battlefield.

The live inspector distinguishes actual Buff counters from temporary Might and Empower. Wizened Elder and Bilgewater Bully require a Buff counter. Trusty Ramhound checks for another friendly unit at its current location without requiring combat. Soul Shepherd benefits friendly unit tokens, including tokens played later. Master Yi, Meditative counts all controlled runes, while Targonian Visionary and Soul Sword use current XP. An attached Soul Sword grants its ordinary +1 Might plus another +1 at Level3; separate copies stack, and detaching removes their benefits.

Hunter’s Machete grants Hunt to its bearer. Printed and granted Hunt values add together into one ability each time that unit Conquers or Holds. The ability captures its amount and identifies that physical bearer; removing equipment or the bearer after finalization does not alter the captured XP. Boneshiver’s separate channel ability occurs on Conquer only. Protector’s Shield shows its attached Might and Tank effects, while Petricite Monument grants friendly units Deflect and expires through its respondable Temporary ability before Hold.

Antagonistic Apparatus creates a ready Mech; Industrious Mechsmith creates an ordinary exhausted Mech. This readiness belongs to that particular play instruction, not to all Mech tokens, and entering ready is not a Ready action. Each creator’s “here” uses its location when its ability resolves; an absent creator supplies no location. Once a token play is pending, its captured destination stays fixed. Optional Accelerate remains a separate play choice.


XP has no game-rule maximum. The engine records positive gains separately from the current balance and clears that history for both players at the start of each turn. Wily Newtfish's Might and Ganking depend on gaining XP this turn, including a gain before it entered. Live Level benefits track current XP; crossing a threshold is not permanent. Gustwalker gains Ganking at Level3, Mosstomper gains Deflect at Level3, and Gemhand Hunter gains Might at Level6.

Bandle Soldier and Scorchclaw check Level3 when they actually enter; entering ready does not trigger abilities that observe a Ready action. Wuju Apprentice checks Level6 when its completed play occurs. Once its draw trigger is generated, later XP changes do not remove it. Combat Experience instead checks current XP when each execution resolves, giving +3 Might instead of +1 at Level6. Concentrate uses only the highest eligible Energy reduction: two at Level6, four at Level11. The determined cost remains fixed during payment.

Mister Root gains XP from a real Move to a battlefield; Nilah, Joyful Ascetic observes any real Move, including a return to base. Merely entering a location or recalling a unit does not create a Move event. Their XP abilities use the ordinary response chain. The separate XP history indicator and live inspector explain which benefits currently apply.

Crowd Favorite and Enthralling Protector spend2XP as an ordinary activation cost to Buff themselves. They do not Exhaust or choose a target. XP is paid before opponents may respond; removing the source later prevents its Buff without refunding the cost. An already-buffed unit can pay again but gains no additional Buff. Spending can remove current Level benefits, while a positive-gain history flag remains set for the turn.

Honeyfruit enters exhausted. Its first Reaction ability exhausts it to add one Any Power; its separate Level6 ability exhausts it to add one Energy and one Any Power. Both remain available at6XP, and neither gains XP. Add resolves immediately, including during permitted payments, with no intervening response window. The UI shows the exact output and the dependent ability's requirement.

Shepherd's Heirloom gains1XP through its own completed-play trigger while unattached. Its Equip ability spends1XP, has no Energy/Power cost, and gives its bearer+2 Might when attachment resolves. Gear Rules Text and attached Equipment Effect Text have separate source identities. Attaching does not play the gear again, and a trigger already created remains independent of later attachment or removal.

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

The deck library includes the real legends and authored strategy notes. All ten registered legend powers execute in matches, while the notes describe strategic goals beyond what the simple AI can teach. Mote and the simulator cannot establish competitive matchup rankings.

## Temporary and Sprite interactions

Temporary units and gear create a kill ability at their controller's Beginning event, before hold scoring. They remain in play while players respond. Countering that ability does not remove Temporary; Recall and Move retain it. Actual departure clears a granted instance. Attached gear keeps its intrinsic characteristic, but only a separately granted expiry remains active while attached.

Beginning observers and existing Temporary objects are captured together once. A Sprite created by Sprite Queen or a dying Sprite Fountain during that event survives until a later Beginning event. Fountain's Deathknell is a real gear-death trigger; it does not count as another gear play. Its printed Rules panel is inactive while attached. Mother and Trevor need a live source for “here”; Queen and Fountain use their fixed base destination. Hidden Sprite Call retains its original field. Each token is a separate pending play with its own identity and optional costs.

Last Stand takes a fresh current-Might snapshot on each execution, including current combat bonuses, and adds that amount for the turn before granting Temporary. Negative references contribute zero. Losing a combat bonus later does not change the previously fixed increase. Petal Pixie counts friendly Temporary units at its battlefield, including itself if Temporary; base is not a battlefield.

Shadow's Call requires a friendly non-Temporary target at selection, but its Draw 2 is independent if that target becomes invalid. Smoke and Mirrors does not require Temporary to select its ordered pair. At resolution, it checks the live pair and each movement restriction; one locked mover does not stop the other legal move. A departed target supplies no invented destination, and Draw 1 remains independent. From Hidden, its first target is here and its second is at a different location. Tasty Faefolk channels as many of the two exhausted runes as remain, then draws independently.

These interactions have focused runtime/API tests and complete legal-home AI games; they do not establish full Standard parity.

## Remaining fidelity limits

Ten supported legends, eleven battlefield choices, and 379 main-deck cards are available through the four supplied presets and custom deck editor. The additional cards preserve all preset and training recipes; add them through the custom deck editor where eligible. There is no sideboarding, tournament match structure, multiplayer mode, or full card-set coverage. The presets are dated tournament lists, not a ranking of the current metagame.

The desktop and bot API expose exact damage allocations and power-source payments. The default automatic payment remains available; energy can be prepared by exhausting chosen rune domains before a play. These controls share engine validation, but broader card and interaction coverage is still required.

Dynamic Might and damage no longer stop at the previous eight-/sixteen-bit limits. Signed Might and temporary modifiers use signed 64-bit storage; damage and combat pools use unsigned 64-bit storage. References treat negative Might as zero, while calculations retain the signed value. A computation beyond the host's representation returns a capacity error and preserves the attempted action's state. These are finite implementation bounds, not official game-rule maxima. The API supplies exact decimal strings, and the combat editor accepts exact decimal allocations.

Further sequencing and interaction audits are still needed. Tests cover particular official examples and whole-match consistency; they do not establish that every interaction is identical to standard Riftbound. Consult the official reference when preparing for an interaction outside those checked cases.

The [independent audit](RULES_AUDIT.md) tracks confirmed fixes and open cases, including separate events within one resolving item. The [full card inventory](CARD_COVERAGE.md) measures unimplemented cards separately from implemented mappings.


## Gear interaction expansion

Detonate and Brittle Steel choose one physical gear, including attached gear and Gold. The gear uses its own characteristics; it does not inherit its bearer's targeting protection or its token creator's domains and cost. Detonate draws for the legal target's controller. An invalidated target also invalidates that linked draw. Thermo Beam fixes all gear recipients, captures their Deathknells, and removes them before later cleanup or children can enter.

Decree of Unity targets an enemy Chaos unit or gear, including a unit in base. Ordinary unit death replacements still apply. Disarming Rake and Stalwart Goatherd offer optional gear choices on play; Goatherd uses printed Energy up to four. Noxian Demolitionist compares printed Energy to its live Might, including revalidation on resolution. A departed source has no Might; a negative referenced Might is zero. Adaptatron's Conquer ability buffs its original source only after an actual gear kill.

Dropboarder checks control of two gear at its play event and emits a real Ready event when its ability resolves. Production Surge uses the actual Mech tag for its two-Energy reduction; its token cannot reduce its own parent. The parent draws before the fixed-base 3-Might Mech child is finalized. Each Repeat makes a separate child. Brittle Steel can use its printed four-Energy, one-Fury Flow cost.


## Return to hand and remembered death locations

Factory Recall returns one gear, including attached equipment, to hand. Returning a unit detaches its gear and preserves that gear’s ready state. Downwell returns the original set of all units and gear once. Tokens disappear when returned. These effects do not kill and do not trigger Deathknell; Recall instead moves a unit to base and keeps its attachments.

Angler Beast returns all units with current Might of two or less when its play ability resolves. It checks the complete set before any returned aura can change another unit’s Might. Targeting protection and Deflect do not apply because this effect does not choose targets.

Mournful Spirit records where it actually died, then offers an optional unit-or-gear target at that location. Either base remains distinct from the other base. Its own detached gear is now in base, which can change its eligibility. The target must still be at the recorded location when the ability resolves. Unit protection, Deflect and death replacements remain applicable; gear is a separate target. The Board and Loadout tabs retain the choice, and the companion explains the location rule.

Retreat returns a friendly unit and then its owner channels one rune exhausted. A token disappears but still supplies its owner; an unavailable target does not cause Channel. Grim Apothecary can Ambush and optionally return a friendly unit at any battlefield, including itself. Wind and Ghosts chooses a unit at a battlefield, then checks its current Might: at most three is banished, otherwise returned to hand. Banish is separate from Kill and bypasses death prevention and Deathknell. Use Trash → Banish to inspect either player’s banished cards.


## Banish and instructed replay

Burn Away chooses a unit with referenced Might at most three, or a gear. The chosen object is banished without Kill, death prevention or Deathknell. Its separate Burn 2 instruction executes even if that target becomes illegal. Tokens disappear when banished.

Portal Rescue follows its official erratum: the friendly unit’s owner plays it to their base. Temporal Breach allows either player’s unit and captures its current location at resolution; a Hidden play also rechecks its original battlefield restriction. Thrill of the Hunt requires either battlefield, including one held by an opponent. Arcane Shift uses normal legal deployment locations, separately deals three to the chosen enemy battlefield unit, then banishes itself. That departure stops any remaining Repeat executions (Core 359.3.e.15).

The parent finishes before child choices, optional/additional payments or entry (Core 354.3, 420.3). The child’s printed base cost is waived. An impossible child play restores that card to Banishment without undoing the parent (Core 358.5). A returning unit is a new object: prior damage, buffs, stun, protection and empowerment do not follow it, and equipment detaches without changing readiness. Completed unit play triggers Rengar normally. Ezreal counts qualifying finalized target choices, including granted Repeat choices, whether or not they later resolve. The admitted pool has no control-transfer effects; stored player ownership must be separated before such cards are admitted.
