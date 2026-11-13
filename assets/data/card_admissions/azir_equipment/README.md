# Conditional Calm equipment

This accepted card batch adds Brutalizer (SFD-042/221, runtime 444) and Hand Hammer
(VEN-027/166, runtime 445). The complete original faces were inspected again on
November 12, 2026. Both are Calm Gear/Equipment with 2 Energy, no play Power, a printed
+1 Might bonus and an Equip cost of 1 Calm Power. Neither has a reviewed erratum.
The exact raw gallery facts, separate effect panels, full-face and runtime image
hashes are retained in admissions.json. The preceding 444 main cards and seven
preset recipes remain unchanged.

Brutalizer adds 2 Might only while attached to a bearer it was actually attached to
this turn. Paying an Equip cost, queuing an ability, or attempting to attach an
already attached pair does not create that event. Core 435.1.f–g governs an actual
change of bearer and a no-op attachment. Its event history expires with the turn.

Hand Hammer adds 2 Might only while its bearer is at a battlefield with exactly one
other unit controlled by that bearer’s controller. Friendly tokens count; enemy
units and friendly units elsewhere do not. The base is not a battlefield. These
are live conditions, so losing either bonus during play can cause lethal cleanup.
Normal turn ending heals damage before this-turn effects expire (Core 317.2),
so Brutalizer's ordinary turn-end expiry does not kill a damaged bearer.

The cards occur in Riot’s [September 30 Los Angeles MICE Squirtle Azir registration](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/).
Their addition does not admit that complete registration or the Azir legend.
The previous Azir draft’s numeric IDs are obsolete and are not copied wholesale.
Its Sovereign location interpretation also needs correction before integration:
the [official Vendetta FAQ](https://playriftbound.com/en-us/news/rules-and-releases/vendetta-rules-faq-and-clarifications/) says its attack effect checks the source’s current
battlefield on resolution, rather than capturing an event battlefield.

This source inherits the unpublished Alpha Strike draft. Alpha’s Repeat/XP audit
and this equipment batch’s full engine, source, API, AI and native acceptance
remain outstanding. No public-release or complete Standard-parity claim is made.

Final native, Nix, installed UI and API evidence is recorded in
[the release validation](../../../../docs/validation/alpha_equipment.json).
