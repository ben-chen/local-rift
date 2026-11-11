# Siren's Song — Irelia priority

The complete official RAD-098/167 face was visually inspected November 11, 2026.
It is a Chaos Reaction spell costing 2 Energy and 0 Power: Move an enemy unit that
moved this turn. Its dated legal identity has one printing and no erratum or
field correction. `admissions.json` pins the original face, bounded runtime art,
raw/effective facts, and preceding 427-card/five-preset hashes. Runtime ID 427
follows Battle Preparations. This is a practice-pool addition, not a new verified
tournament preset.

Core 448 defines Move; 457–458 distinguish corrective Recall from Move. The game
records actual movement of a physical unit, including forced movement, groups and
each successful half of a swap. Entry, zone changes, prevented moves and moves to
the same location do not create that history. Recall preserves existing history
without creating it. Changing controllers preserves the object's history; leaving
and replaying creates a new object. History expires at the end of the turn.

The enemy must meet the condition when chosen and when the effect resolves. A
movement restriction does not itself make an otherwise legal target invalid
(358.3.a): Hunker Down prevents the Move while leaving the earlier movement fact
intact. Ordinary protection, allegiance, Deflect and physical-target checks apply.

Core 355.4 and 820.2 require each repeated Move to choose its destination during
finalization. The original and repeated executions can choose different locations;
every destination must differ from the unit's location at finalization. Choosing
its original location for a later Repeat based on an anticipated earlier Move is
not legal. Destinations are frozen before responses and are not targets. The
native controls and JSON API support these independent choices for Siren's Song,
Charm and Ride the Wind.

The source is preserved at [Riot's official face](https://cmsassets.rgpub.io/sanity/images/dsfx7636/game_data_live/742967b3e85f51b242fc04b8db4c4e3cd7b88672-744x1039.png?accountingTag=RB).
Rules were checked against the [October 9 core rules](https://cmsassets.rgpub.io/sanity/files/dsfx7636/news_live/5c7946f7c3b8c6e0faff85ab6dadc6d2e721ea94.pdf).
