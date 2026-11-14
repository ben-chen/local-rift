# Implemented rules and limitations

**Local Rift is an unofficial, incomplete Riftbound client. This document describes its current behavior, not the complete tabletop game.** Full official card text appears in hover previews and pinned inspectors. Bundled playable cards have executable printed-effect mappings; the current supported count is listed in the README. An official description in the catalog and a passing simulation are not proof of complete tabletop equivalence.

For physical play, use Riot's [Rules Hub](https://playriftbound.com/en-us/rules-hub/), current errata, and printed card text. The core timing reference used here is the [October 9, 2026 Core Rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf). [SOURCES.md](SOURCES.md) records the card and deck sources.

## Setup

Choose a preset or a saved custom deck for each player. Presets contain 40 cards including a chosen champion; custom constructed lists may contain more. The champion begins in its separate playable zone; the remaining cards are shuffled and four cards are drawn. You may replace up to two once. Replacements are drawn before the originals return to the bottom of the deck. The AI chooses its own replacements.

Each player has a 12-card rune deck using the preset’s **published domain split**, preserved in the source manifest. Choose whether you or the AI starts. The second player channels an extra rune on their first turn.

Each player brings one battlefield. The client offers seventeen official choices; its battlefield panels display the selected names and effect text, including any field added during play. The default follows each player’s chosen deck. The battlefield picker shows artwork and full rules before selection. The deck editor can copy a preset, adjust supported cards, rune counts, chosen champion, and battlefield, and save a separate list for you or the AI. It validates the main deck, champion tag, full domain identity, three-copy and Signature limits, and exactly 12 runes. Native **Best of three** practice registers three fields and allows legal sideboarding between games using the complete tournament registrations in the deck library; explicit custom registrations are available through Rust and the bot API.

The desktop's **Practice format** chooses the global Standard date, defaulting to the local date at match acceptance. The date is fixed for that whole match, including later games; the rules engine itself does not consult a clock. The format panel identifies the source-review date and shows current rejection reasons and upcoming bans for both lists. Full registrations include every sideboard card and battlefield. Historical snapshot practice deliberately retains the older catalog and dated presets. Neither mode substitutes cards automatically, and an unsuccessful start preserves the existing game. See [format validation](validation/format_calendar.json) for its evidence and unfinished release checks.

## Turns and resources

The start of a turn advances through readying, beginning effects, hold scoring, channeling, and drawing. Trigger chains and choices pause these steps before the next one proceeds. Up to two runes are normally channeled, and you draw one card. The second player channels three runes on their first turn only. Both players draw on their first turn.

Runes supply energy by exhausting and power by recycling to the bottom of the rune deck. Each displayed rune is a physical game object. Click a specific rune to exhaust or recycle it before a play; both sides use the same ready/exhausted display. Exhausting stores energy; recycling stores colored power. A ready rune can supply energy before being recycled for power, so a 10-energy, 4-power cost does not require 14 distinct runes. If you play without preparing resources, automatic payment exhausts runes and recycles matching ones as needed, preserving unused energy from ready runes recycled for power.

The optional **Power: choose sources** mode lets you spend exact amounts of generated colored, flexible, and spell-only power, and choose rune domains to recycle. Its quote includes base costs, selected additional costs, and Deflect. It rejects unavailable, mismatched, insufficient, or excess sources without changing the match. The API exposes the same quote and payment action.

Printed costs and all printed domains are retained. For a card with multiple domains, its colored power can come from either printed domain. Generated colored, flexible, and spell-only power have separate pools; the latter only pays spell costs. Energy and power pools empty at the beginning of the main phase and at the end of the turn. The interface distinguishes stored energy from energy available by exhausting ready runes. Ezreal, Prodigy lets you allocate its reduction between energy and power for eligible optional additional costs; it does not discount Equip or Deflect.

## Battlefield choices

| Default preset | Battlefield | Implemented effect |
|---|---|---|
| Irelia | Sunken Temple | Conquering with a unit that has at least 5 might offers an optional 1-energy payment to draw a card. |
| Master Yi / Rengar | Seat of Power | On conquest, draw one card for each other battlefield you control. |
| Kennen | Minefield | On conquest, put the top two cards of your main deck into trash. |
| Kai'Sa | Void Gate | Each instance of damage a spell or ability deals to a unit here gains one bonus damage. Combat damage and damage dealt by units are unaffected. |
| — | The Academy | Holding creates a next-spell Repeat option equal to that spell's base cost, lasting this turn. |
| — | Firelights' Hideout | Conquering grants Accelerate to the next unit played this turn. The player may pay its additional cost to enter ready. |
| — | Ornn's Forge | While controlled, reduces the first friendly non-token gear play each turn by one Energy. |
| — | Hunters' Circle | For each player, reduces their first unit or gear played at this field on their own turn by one Energy. |
| — | Piltovan Forge | While controlled, reduces the first friendly gear activated ability each turn by one Energy. |
| — | Risen Altar | Reduces the controller's units' Empower costs here by a chosen one Energy or one Power. |
| — | Vaults of Helia | Holding creates a one-Energy surcharge on the controller's non-token unit plays for the rest of the turn. |
| — | Emperor's Dais | On conquest, optionally pay one Energy and return a controlled unit here to its owner's hand to play a 2-Might Sand Soldier here. |
| — | Star Spring | Each player's first non-token unit play here each turn offers a move of another friendly unit here to base. |
| — | Abandoned Hall | After a player plays a spell, that player may give a friendly unit here +1 Might this turn. |
| — | Targon's Peak | Conquering creates an end-of-turn ability to ready up to two runes. |
| — | Shadow Temple | Holding Burns three cards from the top of the controller's main deck. |
| — | Zaun Warrens | On conquest, discard one card, then draw one. |

Conquest triggers use the chain only when the battlefield scores for that player. Reconquering a field already scored that turn does not repeat those triggers. The final-point replacement draw still counts as scoring and does trigger them. Sunken Temple's optional payment finalizes its draw ability; drawing waits for responses. Seat of Power counts other controlled fields when its ability resolves. Void Gate uses Riot's corrected wording, so a spell targeting several locations only gains damage against units at Void Gate.

Hold and conquest grants are respondable abilities; their effects begin when those abilities resolve. First-play cost discounts use the relevant player's earlier finalized plays, including before that player acquired control. A zero-cost qualifying play still consumes the first position. Two copies of a Forge affect the same first play, while Hunters' Circle tracks each physical field separately. Piltovan also counts resource gear activations and Equip, but not a triggered Quick-Draw attachment. Its nested-payment behavior follows the current finalization and frozen-cost rules; this is a rule-derived interpretation rather than a located Piltovan-specific FAQ.

Other tournament battlefields remain source-list references, including the historical banned Arena's Greatest; they are not selectable. The picker displays official field art and corrected readable text; the mat keeps each selected effect visible with a full-rules inspector.

## Legends

Each deck has its registered legend, independent of preset selection and chosen-champion variant. Eleven legends are available, with the original four preset identities preserved. Legends ready at the start of their controller's turn.

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

Irelia's optional costs are paid before its triggered ability becomes a finalized chain item. Declining leaves the original play intact. Kennen's Empower and Assault abilities also use the chain. Kai'Sa's resource ability resolves immediately, does not create a response window, and preserves priority. Supported resource abilities remain available during pending payment choices, such as a spell replay; their normal cost restrictions still apply. Generated resources expire when rune pools empty; they are not extra rune cards.

## Card support and targets

Select a hand card or the champion in its own zone to see available controls. Supported units enter exhausted at base or a battlefield you control, unless an implemented ability says otherwise. They normally ready next turn. The champion pays its normal cost; after defeat it goes to the appropriate zone like another unit, rather than returning automatically to the champion zone.

Hover over a card or unit for its art and complete official text; scroll inside a long preview, or right-click to pin the inspector. Generic “Implemented” labels have been removed. Specific practice notes appear only when there is a documented difference. All bundled cards are playable; unknown or unmapped cards are unavailable. No cards are swapped for stronger or simpler versions.

A targeted card requires legal targets when played. The engine can choose valid targets automatically for the play button; direct targeting selects an eligible unit, gear, chain spell, or battlefield according to the card. It validates allegiance, location, and card-specific restrictions. Several effects require multiple targets. Select those targets before confirming the play, or let the automatic-target control choose eligible targets. Each paid Repeat has its own target selection. Targeted on-play triggers ask for their targets after the permanent enters play. Fizz and Last Rites choose an exact public trash target before opponents respond. Once their parent ability resolves, the physical card enters Pending, then its child play chooses legal targets, destination, and additional costs. Matriarch follows the same pending-play timing. Trash replay choices retain target and destination controls. Explicit target lists preserve an intentional zero-target choice where the card allows it. If responses separate Bellows Breath's targets into different locations, its controller chooses which maximal legal group remains affected. Deflect is charged for each time a unit is chosen, including repeated choices.

Firestorm chooses one physical battlefield, even if it is empty. Click that field while editing the intended execution; a repeated Firestorm can choose a different field. It damages enemies present there when that execution resolves. Flurry of Blades damages all units at battlefields, and Avalanche all units in bases. These affected units are not individually chosen, so they do not add Deflect costs or trigger abilities that require choosing a unit. Each damage packet uses the source and location's applicable bonuses; lethal-damage cleanup waits until the physical spell finishes all its executions.

Vengeance, Soul Harvest, and Blast of Power issue an actual Kill instruction, including applicable death replacements. Soul Harvest checks the target's current Might limit both when played and when resolving. Blast of Power's target must still be at a battlefield. Supernova skips damage to an invalidated target but still draws two; countering the spell prevents both instructions. Rune Prison stuns without independently exhausting or preventing movement. Cleave grants temporary Assault 3, which stacks with other Assault and contributes Might only while attacking.

## Persistent gear

Gear remains on the board and normally enters ready at base; Iron Ballista and Honeyfruit explicitly enter exhausted. Deploy gear instead enters a battlefield. Equipment adds its printed might bonus and equipment text to its attached unit. The gear remains a separate object: if the unit leaves, its attachments detach at that location without changing readiness. Ordinary unattached gear returns to base during cleanup; Deploy gear stays at its battlefield.

Equip is an activated ability with its own power and additional costs. It enters the chain and can be answered. Quick-Draw permits the gear card's reaction timing and creates its free attachment trigger; it does not make an ordinary Equip activation a Reaction. Last Rites asks you to choose two trash cards to recycle for Equip, and its attached unit’s conquest or hold trigger can offer a unit play from trash with normal costs.

The gear implementation includes Boots of Swiftness's might/Ganking, Guardian Angel's heal-and-recall death replacement, the errata-corrected Zhonya's Hourglass replacement, Seal of Discord's generated Chaos power, and expendable Gold tokens that generate flexible power. Temporary gear is removed through its beginning-step ability. Attached equipment's printed rules text is inactive, but granted rules text remains active: attaching gear affected by Turn to Dust does not prevent its Temporary expiration. The inspector distinguishes the two origins.

When multiple death replacements apply, the affected unit's controller chooses which one to use, including Guardian Angel, Zhonya's Hourglass, and Smite's banishment. Remaining replacements are checked again if applicable. Last Rites uses a triggered-ability chain before its optional unit choice, and Hourglass can use Hidden. Check the inspector and [errata provenance](SOURCES.md).

## Gear at battlefields

Deploy gear may be played to any battlefield, including one occupied or controlled by the opponent, and cannot be played to base. Gear alone does not contest, control, conquer or hold a battlefield. When an opponent Holds its battlefield, each active Deploy gear creates its own respondable self-Kill ability. Enemy Conquer does not trigger it. Moving the source after that ability finalizes does not evade its Kill.

The board shows independent gear in a separate **GEAR** area for each player at each battlefield. Its count opens Loadout filtered to that location; each physical copy remains selectable and inspectable. Attached gear stays with its bearer's equipment strip. Dragging Deploy gear onto a field chooses its entry location, including when the pointer is over a unit; it does not Equip the gear. Gear has no ordinary move action.

Banner of Conquest grants Assault to friendly units here, Hunting Blind grants Shield 2, and Portable Portal grants Ganking. Their effects follow the source's live location and stop while its printed rules are inactive. Multiple Banners stack. Overloaded Traveler gains one Might while at a battlefield with any friendly Deploy gear; multiple gear do not multiply that bonus. Attached gear retains its Deploy characteristic even though its printed abilities are inactive.

Blossoming Dreamroot draws after an actual combat win here, before control and Conquer effects. It does not trigger for an uncontested Conquer or a combat with no result. Pillaged Armory Buffs a friendly unit anywhere on its play or when its controller Conquers its battlefield. Resource Extractor checks control of its battlefield at the end of its controller's turn and, if it triggers, plays exhausted Gold at base when it resolves. The Good Stuff discards one then draws three on play and returns its exact physical source to hand on a friendly Hold here.

Concert Poster's one-Energy payment and Portable Portal's one-Body payment are triggered **base costs**. Choose targets first where required, then accept or decline the cost before the ability becomes a finalized chain item. Deflect is added for an eligible enemy target, and spell-only power cannot pay an ability cost. Declining creates no finalized ability. A pending positive trigger payment may be cancelled even when funded; cancelling a staged plan restores its original choice, and declining the trigger also undoes resource Add actions used in that incomplete payment attempt. The source gear's earlier play remains completed. Once paid and finalized, countering the ability or losing its target does not refund the cost.

Portable Portal can choose an already-ready unit here, including an enemy; Ready then does nothing. Hunting Blind can choose an enemy unit at another location and uses the source's live battlefield when it resolves. A departed source supplies no last-known destination.

Command: Attack requires an explicit choice between moving one friendly Deploy gear to a battlefield and privately inspecting the top five cards of the Main Deck. For the move mode, choose the exact gear and then its destination. Attached Deploy gear remains a legal target, but cannot move separately from its bearer. The client requires a different destination; applying the Move definition to non-Unit gear is a documented interpretation in [the rules audit](RULES_AUDIT.md). For the draw mode, optionally reveal and draw one inspected Gear, then recycle the remainder. Each Repeat has its own mode and targets. The spell retains its ordinary Main timing.

## Unit and gear activations

A selected unit exposes **Abilities** alongside its movement controls. Choose the printed ability, select its targets, then confirm its costs. Gear uses the same controls for its printed activations. Each physical source is tracked separately, including two copies of the same card. Cancelling before confirmation spends nothing; committed costs are paid before opponents respond. Exact power-source choices apply to these costs too, including Deflect, and spell-only power cannot pay for them.

Heart of Dark Ice and Arena Kingpin grant temporary Might; Orb of Regret reduces Might with its printed minimum when the effect resolves. Arena Bar buffs an exhausted friendly unit. Iron Ballista targets a battlefield unit. Xerath and Caitlyn must themselves be at a battlefield to activate; this source restriction is checked when activating. Their targets must still satisfy the target restrictions when the ability resolves. Caitlyn deals damage equal to her current Might, and cannot supply a Might value after leaving play. Vanguard Armory queues three separate Recruit plays after its ability completes, allowing a legal destination for each.

Renata Glasc has two distinct abilities: pay one energy and one Mind power to draw, or pay four of each and exhaust her to score one point. Both require her at a battlefield. The draw ability does not exhaust her. Her point is not conquest and does not consume a battlefield's scoring opportunity.

Dragonsoul Sage's Reaction ability exhausts it to add one energy immediately, including during permitted payment choices. This does not allow the unit card itself to be played as a Reaction. The other activations in this group require the controller's open main phase. Non-resource abilities enter the chain and allow responses; gear-use triggers occur after their ability resolves, including when its chosen target has left, and do not fire for a countered ability.

Garbage Grabber recycles three selected trash cards and pays one energy before drawing on resolution. Assembly Rig instead recycles one unit, pays one energy and one Fury power, and plays a 3 Might Mech token at base after resolving. The trash choices are costs, not targets. Cards recycled together go to the bottom in random order. Divining Shells supplies Vision when played and kills and exhausts itself to give a unit temporary Might. Hextech Formula enters exhausted and can Empower another gear, including an opponent's or attached gear.

Ancient Henge converts energy to flexible power; Hextech Anomaly converts any-domain power to energy. Exhaust is their activation cost. The conversion amount is chosen and paid as the immediate ability resolves. You may use another Add ability during that payment, finish it, then resume the original conversion. A zero conversion completes without restoring the source's ready state. Exact power choices apply to Anomaly, and spell-only power cannot pay for it.

A payment keeps its targets, optional choices, and calculated discounts fixed while you generate resources. Target legality is checked again at completion. **Undo payment** restores the position at the start of that payment plan, including resource actions inside it and the original pending choice. A positive Energy or Power finalization cost can be refused even when fully funded; merely opening a payment plan does not pay it. For an ordinary play or activation, Undo also ends that attempt.

An instructed card or token and a pending triggered ability offer a separate **Decline** outcome. This ends only the selected pending occurrence, restores the resource actions used to fund that attempt, and returns reserved next-play grants. An instructed card returns to its original Trash or Banishment occurrence; an unplayed token has no card to return. The completed parent stays resolved, and later pending work continues. Undo instead returns to the same pending choice so you can try again. A valid zero-resource-cost finalization cannot be declined through payment refusal. Sole-XP costs, Hide, and payments inside a resolving instruction such as Hard Bargain are outside this refusal control. Finish nested resource instructions before using either control.

## Hidden, trash plays, and optional costs

A card with Hidden can be placed facedown for one flexible power at a battlefield you control during your open main phase. There is one facedown slot per battlefield. Hiding is not playing the card. On a later turn, the card can be played as a Reaction with its base cost waived; additional costs such as Deflect still apply. Losing control reveals the hidden card and puts it in trash.

Hidden plays normally restrict their targets to that battlefield, including when targets are rechecked on resolution. Explicit card exceptions, such as Tideturner's other-location swap, retain their printed permission. Opposing facedown identities remain concealed unless an effect grants access or the card is played. Scuttle Crab’s death trigger reveals the opponent’s hand and grants inspection of their facedown cards until the turn ends.

The client supports Fizz's energy-waived spell replay, Last Rites unit plays from trash, Kennen's temporary Flow grant, and printed Flow conditions and costs. Fizz and Last Rites offer their optional play during trigger finalization. Once accepted, perform the instructed play if legally possible; a positive resource cost in that child finalization may still be refused as described above. If no legal play remains, the choice can finish without playing a card. Trash plays enter the ordinary timing and response system. Matriarch and Kennen choose their trash cards before the revive or Flow-grant ability enters the chain; if that exact card leaves the trash before resolution, the ability cannot affect a replacement copy. Tornado Warrior similarly chooses its Empower target before opponents respond. Its delayed end-of-turn Disempower trigger uses the chain, with a response window before temporary effects expire. Kennen's legend triggers when an actual card is played from a zone other than hand; hiding a card alone does not empower it.

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

Apprentice Mage's actual Empower transition triggers Predict 2. Its controller looks at up to two available cards, chooses which to recycle, and orders the rest on top. Cards stay in the deck while the private choice is pending; confirming commits the whole choice. Predict neither draws nor causes burnout with a short deck. The other seat and its companion cannot see the looked-at cards.

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

The newest chain item resolves first after consecutive priority passes. Priority is reassessed between resolutions. A pass is not an irrevocable promise to stop responding: a new play or a resolved item can create another decision. When a chain completes during a showdown, focus passes after a player-initiated spell or ordinary activated ability. A chain opened by a triggered ability does not pass focus; Add abilities also preserve it.

**Pass priority** and `Space` pass within a window. They only end your turn when no chain, showdown, or pending choice remains. A pending choice must be answered first. Implemented counters include countering, returning a spell to hand, and asking the spell's controller to pay additional energy. Predict offers its controller a keep-on-top or recycle-to-bottom choice; it does not itself draw a card, and looking at an empty deck does not cause burnout.

Repulse chooses a friendly battlefield unit, then an enemy spell or ability that
chooses it and no other friendly unit. All Repeat choices belong to the same
spell. A second chosen unit moving to base still counts; leaving the board ends
that physical targeting relationship. Not So Fast uses the same live-object
tracking. Dependent target controls and API previews preserve this order.

Unyielding Spirit prevents all spell and ability damage for both players for the
rest of the turn, including damage bonuses and later entrants. The effect remains
through ending triggers. Combat and damage explicitly dealt by units still work;
existing marked damage remains, and independent instructions such as drawing
cards still resolve. **Turn effects** shows the active protection and its expiry.

Triggered abilities and turn-start steps pause for choices and response windows. Pending event groups finalize in chronological order. Within one simultaneous trigger group, the active player finalizes their triggers first, followed by the other player. Each controller chooses the order of their own triggers; the last placed on the chain resolves first. On-play targeting and optional trigger payments happen during finalization, before the resulting ability can resolve.

The expanded card pool has explicit observers for playing cards and tokens, readying, exhausting, buffing, moving, combat designations, drawing, death, holding, conquest, and gear activation. Chosen trigger targets are selected before the ability becomes a finalized chain item, with Deflect paid then; implicit references such as “me” or “it” do not create an extra target choice. Optional Windsinger play triggers can be declined. “Here” and applicable Hidden restrictions are checked again on resolution. Attached equipment abilities use the bearer as their source. Surviving death observers are distinct from Deathknell abilities captured before their source leaves play; a first-time observer does not multiply across simultaneous deaths. These are explicit card mappings, not a parser that treats arbitrary printed text as implemented.

The additional spells include readying and fights with Repeat, ordered damage and Gold creation, Sand Soldier creation, and modifiers or draws based on current public board state. Their condition checks and optional costs follow their printed instructions. Dynamic costs are determined before payment and remain fixed through resource generation; temporary Might modifiers expire at turn end. As with the older pool, these tests cover selected rule cases rather than proving all interactions.

## Movement and combat

A standard move exhausts a ready unit. It can move from base to a battlefield, or from a battlefield back to base. Direct battlefield-to-battlefield standard movement is unavailable unless a specifically implemented ability permits movement. A readying effect may enable another move during the same turn.

Use **Group all … ready here** to move several ready units from the selected unit’s location in a single standard move. Shift-click individual units to choose a different group. Official rules also permit units with different origins to move together to one destination; the engine accepts that, while this grouping button selects one origin. Choose the group before entering a showdown: ordinary movement cannot add extra attackers after the response window begins. Cards with legal timing may still change the fight.

After the showdown closes, each player assigns their captured combat damage among the opposing units. Exhausted units normally contribute their current Might; Stunned and printed restrictions can prevent a unit from contributing damage without removing its Might. Assign all damage, respecting Tank first and Backline last, and assign lethal damage before moving to another unit. If a unit has both restrictions, such as Caitlyn wearing Doran’s Shield, the assigning player may place it first or last, independently for each such unit. The UI offers a suggested allocation, target reordering, and an **Exact amounts** editor with per-unit controls. It shows assigned and remaining damage, lethal requirements, and any invalid choice before confirmation. A rejected allocation preserves the draft and game state. Both sides finish assigning before damage is dealt simultaneously. If lethal damage creates a replacement choice, the affected units stay visible until their controller makes that choice and cleanup resumes.

A unit with positive damage at least equal to its current might is defeated. A zero-might unit is not defeated merely for having zero might and can conquer or hold an undefended battlefield. Combat cleanup clears damage from **all surviving units**, including those elsewhere on the board. If defenders remain at that cleanup, surviving attackers return to base, preserving their ready or exhausted state. This heal and recall happen once. Subsequent reactions can deal fresh damage that is not healed again by the same combat. End-of-turn cleanup also clears surviving damage and temporary might changes.

Abilities caused by combat damage and cleanup, including Deathknell, resolve before the combat result is determined. Result effects finish before control is established. Conquer effects finish before combat ends and Assault, Shield and other combat designations expire. These stages still allow the responses and choices belonging to their chains. They do not permit ordinary movement or ending the turn. For example, Mournful Spirit can defeat the last surviving attacker before that attacker scores.

If cleanup recalled attackers, the combat has no result even if only defenders remain. If later effects leave both players with units at the battlefield, another combat is staged and waits until the current combat ends. If no units remain, the field becomes uncontrolled at the control step. A unit that leaves and later re-enters the same combat regains its designation, but its Attack or Defend trigger conditions are checked only on its first designation in that combat. A condition that was false on first entry is not checked again after re-entry (383.4.e–f). A newly entering physical unit still receives its own first check.

Crimson Pigeons gains its bonus while attacking with another friendly unit at that location. Wielder of Water gains its bonus while attacking or defending alone. These conditions require actual combat and count friendly tokens, stunned or exhausted units, and units with zero Might. Captain Farron and Taric continuously grant their printed keyword to other friendly units at their current location; they do not grant it to themselves. Moving or removing the source updates the bonus. The inspector identifies each contributing source separately.

### Rengar, Pouncing and Ezreal, Dashing

Rengar, Pouncing can be played as a Reaction to a battlefield where you are the attacking player. That permission remains during the combat's response chains even if your last unit there leaves. It grants an additional placement destination; it still requires legal timing, full payment and any destination imposed by the instruction playing him. It does not let the defending player use an unrelated battlefield or create a response window during a closed combat step.

Ezreal, Dashing keeps his current Might but contributes **zero combat damage**. He can still receive lethal combat damage, and his teammates contribute normally. This printed restriction is separate from Stunned. His Attack/Defend ability deals his current Might to a chosen enemy unit at his current location when the ability resolves. Moving either unit, removing Ezreal, or changing his Might can therefore change the outcome. The ability uses the chain, including target choice, Deflect and responses.

Dashing's separate activation costs one Mind Power and moves that physical unit to your base. It does not exhaust him and is an Action, so it cannot answer an unresolved spell or ability. The Action marker on this activation does not give the Unit card Action timing. Effects that make units deal their Might to one another outside the Combat Damage Step still use Dashing's full current Might.


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

The desktop calculates one opponent action in the background while inspection and navigation remain available. Its result can update only the same unchanged match. Opening a menu or inspector pauses applying the result; a new match discards an old result. A calculation error leaves the live position unchanged and offers an explicit retry. Restarting does not interrupt the old calculation itself, so a fresh opponent action can wait for that work to finish.

For an optional counter tax, the AI can decline immediately when available Energy is insufficient and no physical resource source can add more. This policy conservatively keeps the existing exact search if it finds an Energy producer, unknown output, conversion or nested payment. It does not change which payments are legal, establish a negative mandatory-payment proof, or grant a new refusal option.

Mote's **Beginner** advice explains resources, deployment, scoring, and the current timing window. **Advanced** advice discusses future holds, useful trades, interaction costs, and broad archetype goals. It sees your hand and information available to you, including cards an effect explicitly reveals or lets you inspect. It does not examine the opponent’s private hand, facedown identities, or Predict result. Its small panel is optional and never blocks input.

The deck library includes the real legends and authored strategy notes. All ten registered legend powers execute in matches, while the notes describe strategic goals beyond what the simple AI can teach. Mote and the simulator cannot establish competitive matchup rankings.

## Temporary and Sprite interactions

Temporary units and gear create a kill ability at their controller's Beginning event, before hold scoring. They remain in play while players respond. Countering that ability does not remove Temporary; Recall and Move retain it. Actual departure clears a granted instance. Attached gear keeps its intrinsic characteristic, but only a separately granted expiry remains active while attached.

Beginning observers and existing Temporary objects are captured together once. A Sprite created by Sprite Queen or a dying Sprite Fountain during that event survives until a later Beginning event. Fountain's Deathknell is a real gear-death trigger; it does not count as another gear play. Its printed Rules panel is inactive while attached. Mother and Trevor need a live source for “here”; Queen and Fountain use their fixed base destination. Hidden Sprite Call retains its original field. Each token is a separate pending play with its own identity and optional costs.

Last Stand takes a fresh current-Might snapshot on each execution, including current combat bonuses, and adds that amount for the turn before granting Temporary. Negative references contribute zero. Losing a combat bonus later does not change the previously fixed increase. Petal Pixie counts friendly Temporary units at its battlefield, including itself if Temporary; base is not a battlefield.

Shadow's Call requires a friendly non-Temporary target at selection, but its Draw 2 is independent if that target becomes invalid. Smoke and Mirrors does not require Temporary to select its ordered pair. At resolution, it checks the live pair and each movement restriction; one locked mover does not stop the other legal move. A departed target supplies no invented destination, and Draw 1 remains independent. From Hidden, its first target is here and its second is at a different location. Tasty Faefolk channels as many of the two exhausted runes as remain, then draws independently.

These interactions have focused runtime/API tests and complete legal-home AI games; they do not establish full Standard parity.

## Battlefield costs, delayed choices, and physical runes

Emperor's Dais returns the selected unit as a cost before opponents respond. Selecting that unit is not targeting: it does not charge Deflect or trigger abilities that observe choosing a target. The payment panel supports resource generation, Undo, and declining before finalization. Once finalized, countering the ability does not refund its Energy or return the unit to play. The Sand Soldier has the battlefield as its source; it is a token, not an Azir card.

Star Spring tracks the first successful non-token unit play separately for each player and each physical battlefield each turn. Declining, or having no other eligible unit, still uses that opportunity. Moving a unit here and playing a token do not use it. Abandoned Hall belongs to the player who played the spell, regardless of who controls the field. It triggers after the spell successfully resolves and its play completes, not at finalization; a countered spell does not trigger it. Both battlefield abilities choose their optional targets before responses and recheck them when resolving.

Targon's Peak first establishes a delayed trigger. Its rune choices happen at the end of that turn, not when it is conquered. You may choose zero, one, or two distinct runes from either player, including already-ready runes. An exhausted rune keeps its identity when readied; a recycled rune leaves play. If a chosen rune leaves and another of the same domain is channeled, the new rune does not inherit the old target. Each delayed trigger identifies its original physical battlefield.

Shadow Temple uses the normal Burn and burnout rules. Zaun Warrens completes the discard choice before drawing; with an empty hand, it still draws. These effects pause the surrounding turn or combat sequence until their choices and response windows finish.

## Live combat values and the Rengar preset

Yasuo, Remorseful's Attack ability uses his current Might when it resolves. Graves, Blasting Through does the same on Attack or Defend, damaging all enemy units at his current location without choosing them individually. Graves contributes no combat damage. Lucian, Gunslinger uses his current combined Assault value for his Attack ability; Armored Tusker uses its current combined Shield value for its optional Move ability. Assault, Shield, Might, and combat damage contribution are distinct values. The inspector displays their sources, and the companion explains which value a pending effect uses.

Irresistible Faefolk remembers the battlefield it moved to. Its optional trigger can move an enemy there even if Faefolk later leaves; the destination belongs to that movement event. The enemy still must be a legal target when the effect resolves. The AI evaluates public combat values and control when choosing a pull. Its advice and choices do not guarantee an outcome against future responses.

Kinkou Initiate checks the combined current Might of your other units when its Play ability resolves. Nidalee, Cat Form draws through a combat-win trigger; her printed Ambush permission allows a Reaction play at a battlefield where you have units. Pyke, Dockside Butcher's optional Fury Power is an additional play cost. Paying it creates a respondable Play ability that readies that physical Pyke and gives it +2 Might for the turn. Hidden waives the base cost, not the chosen additional cost. A cost discounted to zero still counts as paid when selected and finalized.

The fifth preset is the sourced Los Angeles Rengar winning list. Copying that preset into the editor preserves its published 40 cards and 4 Fury / 8 Body runes, with Seat of Power as the default field. The separate Rengar training recipe remains an authored practice list; it is not labeled as the tournament deck. Published sideboards and three-field registrations are preserved as source evidence. The native app and bot API also support Rengar’s complete registration for best-of-three practice.

## Remaining fidelity limits

Eleven supported legends, seventeen battlefield choices, and 449 main-deck cards are mapped through seven presets and the custom deck editor. Alpha Strike, Brutalizer and Hand Hammer have completed their documented acceptance checks. Both Irelia preset recipes remain unchanged. Native match registrations cover Xeno Irelia, Arito Irelia, Master Yi, Kennen, Rengar and Sonecon Riven. The native app, shared engine and bot API support untimed best-of-three play and legal sideboarding. Timed tournament rounds, multiplayer and full card-set coverage remain unfinished. The Master Yi registration has completed native and packaged acceptance; see [its validation](validation/master_yi_registration.json). The presets are dated tournament lists, not a ranking of the current metagame.

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

Mournful Spirit records where it actually died, then offers an optional unit-or-gear target at that location. Either base remains distinct from the other base. Its ordinary detached gear recalls to base during cleanup, while Deploy gear remains at the field; this can change which physical gear is eligible. The target must still be at the recorded location when the ability resolves. Unit protection, Deflect and death replacements remain applicable; gear is a separate target. The Board and Loadout tabs retain the choice, and the companion explains the location rule.

Retreat returns a friendly unit and then its owner channels one rune exhausted. A token disappears but still supplies its owner; an unavailable target does not cause Channel. Grim Apothecary can Ambush and optionally return a friendly unit at any battlefield, including itself. Wind and Ghosts chooses a unit at a battlefield, then checks its current Might: at most three is banished, otherwise returned to hand. Banish is separate from Kill and bypasses death prevention and Deathknell. Use Trash → Banish to inspect either player’s banished cards.


## Banish and instructed replay

Burn Away chooses a unit with referenced Might at most three, or a gear. The chosen object is banished without Kill, death prevention or Deathknell. Its separate Burn 2 instruction executes even if that target becomes illegal. Tokens disappear when banished.

Portal Rescue follows its official erratum: the friendly unit’s owner plays it to their base. Temporal Breach allows either player’s unit and captures its current location at resolution; a Hidden play also rechecks its original battlefield restriction. Thrill of the Hunt requires either battlefield, including one held by an opponent. Arcane Shift uses normal legal deployment locations, separately deals three to the chosen enemy battlefield unit, then banishes itself. That departure stops any remaining Repeat executions (Core 359.3.e.15).

The parent finishes before child choices, optional/additional payments or entry (Core 354.3, 420.3). The child’s printed base cost is waived. An impossible child play restores that card to Banishment without undoing the parent (Core 358.5). A returning unit is a new object: prior damage, buffs, stun, protection and empowerment do not follow it, and equipment detaches without changing readiness. Completed unit play triggers Rengar normally. Ezreal counts qualifying finalized target choices, including granted Repeat choices, whether or not they later resolve. The admitted pool has no control-transfer effects; stored player ownership must be separated before such cards are admitted.

### Clocktower Guardian

Clocktower Guardian is a Mind/Shurima unit. Play and Hold each generate a Predict 3 trigger: inspect the available top cards privately, recycle any selection, then order the rest. A Hold trigger resolves before the subsequent Channel and draw tasks. Its Deathknell is mandatory and can be responded to before it resolves. It banishes the current top card, then offers an optional play; an empty deck does neither and does not cause burnout.

Accepting puts that exact banished card into Pending. The parent finishes before choosing its own targets, location and additional costs. Unit, Gear and Spell cards are eligible. Only base Energy is waived; Power, Repeat, Accelerate and other extra costs remain payable. Declining the initial offer leaves the card in Banish. If accepted but impossible to finalize, the failed play restores only that card to Banish. Spells go to Trash normally after resolving; the Energy waiver does not supply Fizz’s recycle instruction. A pending child preserves battlefield control until its finalization, so a replacement unit can enter the battlefield vacated by Guardian.

### Deck-to-Banish plays

Baited Hook, Dazzling Aurora, Reinforce and Wild Claw use their reviewed effective text, preserving older printed faces and errata separately. Hook/Aurora ignore base costs; Reinforce/Wild Claw apply a five-Energy discount to the total, after component discounts. Power remains payable for the latter pair, while total Energy reductions may also cover optional additions (Core356.4).

Hook kills its chosen friendly unit as an effect. All death replacements finish before the private look. Only an actually killed unit supplies its referenced Might, read immediately before death with equipment and continuous effects included. Prevention or a missing target leaves no usable reference, so the top five cards are still inspected and recycled but none may be selected. A killed unit with zero referenced Might is distinct from a missing reference. Cleanup waits until the parent has finished its selection and recycle instructions.

Private look choices preserve physical top-card occurrences. The selected card goes through Banish to Pending; the rest recycle randomly. Each Repeat execution finishes before the next begins, and all parent executions finish before children finalize. An impossible child returns only itself to Banish. A prohibition on playing negates the Play instruction, but permits the preceding optional Banish and the independent recycling.

Aurora triggers at its controller's Ending. It publicly reveals through the first unit, banishes/plays that unit, and recycles the rest. No-unit and empty-deck cases never draw or burn out. Ending cleanup waits for the resulting chain and pending plays.

Wild Claw's final optional Empower is a separate reflexive ability. Its child unit or gear enters before that ability finalizes; normal responses then apply. The ability follows that exact board object if it moves, but not a replacement copy or an object in a non-board zone. Its Empower state is permanent. Choosing no card or an impossible child leaves no object to Empower. These timing distinctions follow Core337,354,358 and388–389; they are not automatic on-entry Empower.

## Inspected draws and Move reveals

Double Trouble privately looks at the top three cards and may reveal and draw one Unit, recycling the rest. Its Repeat costs two Energy; each execution completes its own draw and recycling before looking again. Fate Weaver's mandatory on-play trigger privately looks at four and may reveal and draw a Spell with printed Energy cost at least four. Trifarian Captain's mandatory on-play trigger privately looks at four and may reveal and draw a Unit or Gear with printed Energy cost at most three and printed Power cost at most one. Declining recycles every inspected card. Duplicate cards remain separate physical occurrences. These optional resolution choices do not make the on-play triggers optional (Core383.3.a.3).

Apprentice Smith, Raving Witness and Pakaa Protector each trigger on an actual Move. They reveal the current top card when that trigger resolves, including after their source leaves the board. Smith draws Gear and otherwise recycles. Witness draws a Mighty Unit and otherwise recycles; Might in the deck is its printed Might, unaffected by board auras (Core711). Pakaa draws a Unit and otherwise trashes the card and gives its original surviving source +2 Might this turn. A replacement object cannot receive that bonus. Entering, readying and recalling do not count as Move events.

Look and Reveal do not move cards themselves. Only the selected identity becomes public for a private look; unselected identities remain private. A public acknowledgement lets the player inspect the revealed card after the prescribed movement, without creating a new response window or an extra card zone. Short looks use only available cards. If a revealed top card does not exist, further instructions on it do nothing, including Pakaa's otherwise branch; no Draw or Burn Out occurs (Core432.1.c.1).

Drawing an explicitly selected card still emits the ordinary Draw event (Core414.1.a). Lightning Rush now uses this same path before trashing its remaining inspected cards, so effects such as Frigid Jewel see the successful draw. Drawing the last selected card does not attempt a second top-card draw. Repeat, cleanup and pending observers wait for the parent instruction to finish; the native and API choice controls cannot skip those obligations.


### Predict followed by drawing

Clairvoyance is a Reaction: Predict 5, then Draw 2. Choose which inspected cards to recycle and arrange the kept cards in the order you want to draw them. Nothing is drawn until you confirm. If the spell repeats, finish the first Predict and its two draws before looking at a fresh group for the next execution.

Scryer's Bloom enters exhausted. Its ability costs Kill this, one Energy, and Exhaust. Once those costs are paid, the gear is in your Trash while the ability waits to resolve. Predict 2, then draw one, then gain one XP. The panel shows the source and remaining instructions while you choose. An empty Predict does nothing, but the separate Draw still follows Burn Out rules.

Ornn, Blacksmith triggers when played and when it Holds. Look at the top four available cards, optionally reveal and draw one Gear, then recycle the rest. The trigger itself is mandatory; choosing a Gear is optional. The opponent sees only the Gear you reveal. Ornn can be in an eligible Calm main deck, but it cannot be the chosen champion of a legend with a different champion tag.

### Vision from friendly units

Vision means that playing the permanent triggers a private look at the top card of your Main Deck. Keep that card on top or recycle it to the bottom. This is not a Draw, and an empty deck does not cause Burn Out. Each Vision instance triggers separately: keeping the first card can mean seeing that same card again on the next instance. You can respond between separate abilities, after the current choice and effect finish.

Gemcraft Seer has its own Vision and grants it to every other friendly unit, including tokens, wherever they are on your board. Forecaster grants Vision to your actual Mechs, including itself. It does not have a second printed instance. Jeweled Colossus has printed Vision and Shield 1; Shield increases its Might only while defending. Multiple grants stack as separate Vision abilities.

The inspector lists the current instances and their sources. The chain records the instance that triggered when the bearer entered. Removing a granting unit stops its future grants, but does not cancel already triggered abilities. Removing the bearer likewise does not undo its pending Vision. Moving a unit or giving an existing unit Vision does not count as playing it again. Token abilities belong to the actual token, not the card that created it.


## Live values for triggered damage

Yasuo, Remorseful chooses one enemy at his current location for his Attack ability, then deals damage equal to his current Might when the ability resolves. Responses can change his Might or move either unit. Graves, Blasting Through reads current Might for both Attack and Defend, damaging every enemy at his current location together. Graves chooses no targets, so this instruction does not require Deflect payments. His printed restriction prevents combat damage, while his Might still determines ability damage and the damage needed to kill him.

Lucian, Gunslinger reads his total Assault for his Attack ability. Armored Tusker reads its total Shield for its optional Move ability. These keyword values exist outside combat and include current grants from gear and other units. They are distinct from full Might. Tusker may choose a friendly or enemy unit at either battlefield, including itself if it is there; a unit in Base is not a legal target. Moving Tusker to Base can still generate the ability. Declining ends that optional choice.

These are ordinary triggered abilities with response windows. A chosen target must remain legal when the ability resolves. Removing the physical source before resolution leaves no current value to read; another copy cannot supply it. A zero value deals no damage and does not become positive through Bonus Damage. Graves determines all affected units and damage packets before applying the simultaneous marks and performing cleanup. An undamaged unit does not die merely because its Might becomes zero.

Hovering or pinning one of these units shows the current reference value before damage bonuses or prevention. This is a preview of the current board, not a promise about the result after responses. The beginner companion explains the mechanic; advanced commentary points out relevant response and positioning choices without interrupting play.

## Decree of Focus

Decree of Focus is a 1-Energy Calm Reaction that gives a friendly unit +4 Might
this turn. That unit must be fighting an enemy Fury unit or chosen by an enemy
Fury spell still on the chain. The condition is checked again on resolution:
removing the threat or moving the unit out of combat can invalidate the target.
Repeat choices count; area damage without unit targets and activated or triggered
abilities do not. Irelia's choose triggers still resolve independently of Decree.

## Find Your Center and Hush

Find Your Center draws a card, then channels one exhausted rune. If the opponent
has five or more points, its Energy cost drops from three to one. An empty Rune
Deck does not add a fallback draw; a victory during Draw ends play immediately.

Hush is a 1-Energy, 2-Calm-Power Reaction. It counters a spell whose printed or
copied Power cost is at least two. Deflect, Repeat payments and discounts do not
change that printed characteristic (Core 131.3, 204.1.a and 356.1.c). Hush does not
counter activated or triggered abilities.

## Ravenbloom Prefect

An opponent's gear play offers a choice: banish Prefect now, then banish that
exact gear after responses. Gold token plays count; activating gear does not.
The source is a cost paid before the ability enters the chain. The gear is an
event subject, not a chosen target. If it leaves play, a later copy is unaffected
and the paid source stays banished. Banish does not trigger Deathknell or a death
replacement. Declining keeps your unit. Mote explains the timing and helps compare
the gear with the unit's combat and scoring value.


## Hunker Down

Hunker Down readies a friendly unit, prevents that unit from moving this turn, and
draws one card. It can choose an already-ready unit. Irelia gains Might separately
for being chosen and for actually becoming ready. If the target leaves play in
response, the independent draw still resolves (Core 359.3.e.5). A counter stops
the entire spell.

The restriction stops both players' Move effects as well as basic/grouped movement.
Movement spells remain legal to cast on it (Core 358.3.a), and independent Ready
or Draw instructions still happen. Tideturner and Smoke and Mirrors move each
unrestricted unit to the other's original location. Vex's narrower restriction
continues to stop only movement performed by the unit's controller. Return-to-hand,
Recall, and banish are separate actions; a newly played copy has no inherited lock.
The board label, hover explanation, companion tips and bot observation describe
this restriction, which expires at the end of the turn.


## Battle Preparations and Show Off

Battle Preparations chooses one unit for +2 Might this turn. During finalization,
you may show off a gear from your hand or board. A gear in hand is revealed;
a board gear is picked without targeting it, including attached gear or Gold.
If you showed off, choose zero, one or two different rune objects to ready. The
unit and runes can belong to either player. Runes may already be ready when chosen.

The Show Off fact is captured before costs. Shown Gold can be spent to help pay
for the spell. Runes chosen to ready can be exhausted for Energy; automatic Power
payment preserves those rune objects. Recycling a chosen rune during staged
payment fails the final legality check, allowing Undo or refusal. After the spell
is finalized, losing a rune affects only that rune's readying. Losing the unit
does not stop the independent rune instructions. A counter stops the spell.

Repeat uses the same Show Off fact with independently selected units and runes
for each execution. Show Off itself does not trigger Irelia's Choose ability.
Only the chosen card is revealed; the rest of the hand remains private. The
client's play choices panel offers Show Off and Repeat separately, and the two
rune zones highlight legal physical targets. Mote explains the timing and the
tradeoff between revealing a hand card and picking public gear.


## Reckless Innovator and modal triggers

Reckless Innovator may pay its Accelerate cost of 1 Energy and 1 Chaos Power to
enter ready. Entering exhausted does not trigger its ability. Each later change
from ready to exhausted creates its own choice, including a Standard Move cost
or an effect's Exhaust instruction. Readying it with Irelia can enable another
exhaustion in the same turn. There is no once-per-turn restriction.

Choose one effect before costs and responses: play a Gold token exhausted at
Base, or pay 1 Chaos Power to draw one. The selected mode stays fixed through
payment and resolution, even if the original unit leaves play. An effect that
exhausts and then kills Innovator still produces its pending trigger. Already
exhausted copies do not trigger again merely because an effect says Exhaust.

Gold cannot be spent while exhausted. The draw mode costs Power before the
ability can be answered; it cannot use spell-only Power. Declining that positive
cost skips this occurrence without producing Gold. Undoing a staged payment
returns to the same selected mode and restores that payment attempt. The native
controls and bot API expose the same public choices. The Gold reminder's
Reaction keyword does not grant Reaction timing to the unit card.


## Orianna, Exquisite and nearby gear

Orianna retains her 4 printed Might and Shield even when she cannot deal combat
damage. Her combat contribution is zero while no friendly gear shares her
location. When defending, Shield supplies +1 Might regardless of that condition;
lethal damage and noncombat Fight effects still use her full current Might.

A gear attached to Orianna or any other friendly unit at the same location counts.
Unattached gear there, including Gold, also counts. The gear may be exhausted,
and its own printed abilities may be inactive while attached. Enemy gear, gear
elsewhere, and facedown cards do not count. Moving an equipped ally or removing
gear before combat damage immediately changes the condition. Stun independently
prevents combat damage even when gear is present.

The board retains the zero-combat badge, and the live inspector explains whether
the nearby gear requirement is met. The bot API exposes that same public condition
alongside Might and combat damage. Mote explains the interaction at both levels.


## Gust Monk and a banish cost

Gust Monk costs 2 Energy. Its optional extra 1 Energy enables its play trigger;
it does not itself grant Assault. Choose one unit, then banish one card from
either trash to finalize the ability. The trash card is a cost, not a target.
Enemy targets may require Deflect, paid with power eligible for abilities.
The controller can decline this incurred cost; the completed unit play remains.

The banish happens before responses. Assault 2 is gained only on resolution,
adds 2 Might while attacking, and expires at turn cleanup. The chosen unit may
be Gust Monk, a friendly unit, or an enemy. Source departure does not end the
ability. A counter or lost target does not refund a paid banish cost.

The client shows separate play choices, unit targets and a tray containing both
trashes. Each cost card can be inspected before payment. Staged payment tracks
the chosen physical copy through preceding removals; a departed copy cannot be
replaced by another at the same index. Undo and refusal restore payment-resource
actions while keeping the completed unit play. The bot API uses the same choices.

## Edge of Night and the registered Irelia list

Edge of Night costs 3 Energy and gives its bearer +2 Might. Playing it normally
leaves it unattached at base; its separate Equip ability costs one Chaos Power.
Hiding costs one Power and is not a play. On a later turn, playing it from Hidden
ignores its base cost and triggers an attachment to one friendly unit at the
battlefield where it was hidden.

Unattached non-Deploy gear returns to base during cleanup. This does not change
the Hidden ability’s original battlefield restriction: the Spiritforged erratum
makes “(here)” reminder text. Choose the attachment target after the gear enters;
the attachment happens only after responses. A missing gear source or a target
that leaves the original battlefield cannot be attached by that ability.
Equipment then follows its bearer; when the bearer leaves, it detaches. Nearby
friendly attached gear satisfies Orianna’s combat requirement. Mote explains
these distinctions at Beginner and Advanced levels without blocking input.

Arito’s sixth preset preserves the exact 40-card main list, with the chosen
Irelia included in that count. Its ten sideboard cards are displayed separately
and are never shuffled into the main deck. Its three registered battlefield
choices can be inspected and selected for either preset seat before a game.
The native **Best of three** mode uses this complete registration for sideboard
swaps between games. The deck-library tabs remain an inspectable reference.

## Registered match practice

The native match screens, shared controller and bot API implement the sequence described in
[MATCH_SERIES.md](MATCH_SERIES.md): fixed registration, exact 40-card main decks,
one-for-one sideboarding, private simultaneous battlefield selection, the losing
player's next play-order choice, ordered mulligans and first-to-two game wins.
Sideboard card identities stay private; their counts remain visible. Drawn games
retain both deck configurations, fields and starting player without sideboarding.
Native match play begins from **Practice → Best of three**. Tactical and Strategic opponents may make legal sideboard swaps using revealed-card scouting; Relaxed keeps its list. Native draw agreements and custom registration
editing are not exposed. Timed rounds and judge procedures remain outside this version.


## Baron Nashor and Baron Pit

Baron is a unit costing 10 Energy and 3 Chaos Power, with 12 Might. During play choices, he adds the
Baron Pit token battlefield if none exists (Core355.1). Only the play that creates
it receives the delayed entry replacement (Core369.3); subsequent Barons enter at
their selected legal locations. The chosen location still determines Hunters'
Circle's cost reduction and The Exile's ready-entry replacement, as explicitly
confirmed in the [Radiance FAQ](https://playriftbound.com/en-us/news/organizedplay/radiance-rules-faq-and-clarifications/).

The added battlefield is public while payment is pending. Undo, refusal or an
illegal play rolls back the attempted play's new field and associated resources.
It remains after a successfully played Baron dies or leaves. Units may Standard
Move into the Pit from another field without Ganking; ordinary readiness,
exhaustion, timing and movement restrictions still apply. Moving from the Pit to
another field still requires permission such as Ganking. The Pit participates in
conquest, holding, final-point requirements and all location-based choices.

Enemy spells and abilities cannot choose Baron, including during combat. Friendly
choices and untargeted effects still apply. Each Baron grants 2 Might to every other
friendly unit at every location, including tokens and other Barons. These live
bonuses stack and cease when their source leaves the board; Recall retains the
source at base and therefore retains its aura. The Pit is an auxiliary token,
excluded from starting selections and the constructed legal-card denominator.

Controlled tests cover both seats, direct and suspended payment, rollback, plays
from trash and banish, repeated copies, targeting, auras, movement, scoring and
rematches. The exact Sonecon list has completed 84 single games and 12 registered engine matches (29 games), plus 12 bot API matches (29 games). The [release acceptance](validation/riven_stuttgart.json) records 1,743 Rust tests, 278 source checks, 1,176 Nix simulations and installed client/API validation. These cases are not a claim of complete Standard parity.
