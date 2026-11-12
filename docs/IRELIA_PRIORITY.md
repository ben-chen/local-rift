# Irelia implementation priority

Updated November 11, 2026. Irelia cards take priority over the saved Azir draft.

The first checkpoint is the union of Xeno's second-place and TM2's fifth-place
Irelia lists in Riot's [Los Angeles results](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/).
Both complete main decks, Irelia's legend/chosen champion, and all three of their
battlefield options were already mapped by C28. Decree of Focus and Ravenbloom
Prefect, the remaining shared sideboard gaps, are now implemented and validated.
The dated sourced recipes remain unchanged.

The November 6–8 [Stuttgart list attributed to DonCapucino by Riftwatch](https://riftwatch.gg/decks/rc-stuttgart-2026-33/)
identified two further priority cards: **Find Your Center** (two main-deck copies) and **Hush**
(one sideboard copy). This is a secondary-source lead pending independent list
provenance review, not a newly admitted tournament preset. Both cards are now
implemented and validated for custom practice.

Hunker Down is now implemented for custom practice, including both players' movement
restrictions, independent draw, and Irelia's choose/ready triggers. Its admission
is a card-pool addition, not a claim that it is a staple of the sourced Los Angeles
or Stuttgart recipes.

Battle Preparations now has explicit Show Off, optional physical rune targets,
Repeat, companion advice and AI/API support. Siren's Song now tracks actual
physical-unit movement and supports separate destinations for repeated Moves.
The same destination correction also applies to Charm and Ride the Wind. Back Off
now distinguishes Hand, Hidden, Trash and Banish origins, including Repeat.
Hunters' Circle was already registered as battlefield 7. Its renewed
[interaction audit](validation/irelia_circle.json) now passes; this is not a new
card admission. The [Sacramento list attributed to Michael Mckain](https://riftwatch.gg/decks/sacramento-269/)
(October 24–25, 269th place) supplies a concrete secondary-source lead: three
Battle Preparations, two Siren's Song, two Back Off and that battlefield. It is
not a verified new preset or evidence of top-tier results. Reckless Innovator
remains a community lead behind cards tied to these specific lists. All-time
aggregate frequencies include older banned decks and must not be presented as
current Standard prevalence.

Back Off's independent draw requires an actual Hand origin; Hidden and instructed
replays do not draw. Its [validation record](validation/irelia_backoff.json) covers
these distinctions. Hunters' Circle already uses a per-player, per-field first-play
record and restricts its discount to that player's turn. The renewed audit covers
shared Unit/Deploy-gear usage, token and free plays, Hidden, instructed replays,
move-versus-play, payment refusal and turn reset. Its total-cost discount also
applies to Accelerate after Hidden ignores the base cost. Eight new regressions
pass alongside all eleven existing tests in the battlefield test target.

Next, review Reckless Innovator's printed Piltover tag and Exhaust-triggered
choices before implementing it. It remains a lower-confidence community lead,
not evidence of a current top-performing tournament list.

For each addition, exercise legal targeting, costs, responses, resolution after
targets change, Irelia's choose/ready triggers, bot-visible actions and UI hints.
Keep Beginner and Advanced advice tied to the resulting game state. Completing
these card pools does not by itself implement best-of-three sideboarding.

The unpublished Azir source, engine and preset drafts remain saved outside the
repository. Their tentative numeric IDs must be rebased after Irelia admissions;
they must not overwrite newly published identities.

Battle Preparations validation is recorded in [its acceptance receipt](validation/irelia_preparations.json).

Siren's Song validation is recorded in [its acceptance receipt](validation/irelia_siren.json).
