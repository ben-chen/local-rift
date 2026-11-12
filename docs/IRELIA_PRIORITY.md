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
not a verified new preset or evidence of top-tier results. Reckless Innovator's
Irelia deck relevance remains a community lead; its availability does not verify
a new tournament preset. All-time
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

Reckless Innovator is now implemented and validated. Its printed Piltover tag
is preserved as a reviewed correction of the gallery's empty tag list. Its
exhaustion choice, pre-response draw cost, exhausted Gold, and repeated
Irelia-ready interactions use the same rules through the client and bot API.
The [validation record](validation/irelia_innovator.json) separates complete
games, focused interactions, source review and native interface checks.

Orianna, Exquisite is now implemented and validated for custom practice. Her
combat contribution follows the physical location of friendly gear, including
gear on another friendly unit, without reducing her Might or Shield. Her
[validation record](validation/irelia_orianna.json) includes both gear conditions,
movement and removal, native inspection, API views, and all three AI levels.

Gust Monk is now implemented and validated for custom practice. Its optional
play Energy, unit target and separate pre-response banish cost are explicit in
the client and API. The [validation record](validation/irelia_gust.json) covers
both trash owners, Irelia's choose/ready triggers, counters, Deflect, physical-copy
tracking during payment, temporary Assault and Beginner/Advanced advice.

The primary [Convergence IV standings](https://topdeck.gg/bracket/convergence-4)
and [Arito deck list](https://topdeck.gg/deck/convergence-4/3Mb0hLsUTwgBYrpAauGOaRvDhnq1)
were independently read on November 11. They confirm second place, an 8–2–0
record and a 272-player event on October 31. The previous note incorrectly said
Gust Monk completed its main card pool: **two Edge of Night were still missing**.
Edge of Night is now implemented with the Spiritforged erratum, Hidden-origin
targeting, ordinary Equip and explicit attachment timing. The exact list is the
sixth preset, **Irelia / Arito Convergence IV**: 40 main cards including the chosen
champion, ten sideboard cards, twelve runes and three registered battlefields.
The original five recipes and all ten legend training recipes remain unchanged.

The deck library separates main deck, sideboard and battlefield inspection.
The native Best of three mode now uses this complete registration for legal
sideboarding. The original card and preset admission is recorded in the
[combined validation record](validation/irelia_arito.json) for the precise tested
scope and remaining limits.

Continue the end-to-end Irelia rules audit, including tournament match flow and
sideboarding; card-pool coverage alone does not make that match experience complete.

For each addition, exercise legal targeting, costs, responses, resolution after
targets change, Irelia's choose/ready triggers, bot-visible actions and UI hints.
Keep Beginner and Advanced advice tied to the resulting game state. Completing
these card pools does not by itself implement best-of-three sideboarding.

The unpublished Azir source, engine and preset drafts remain saved outside the
repository. Their tentative numeric IDs must be rebased after Irelia admissions;
they must not overwrite newly published identities.

Battle Preparations validation is recorded in [its acceptance receipt](validation/irelia_preparations.json).

Siren's Song validation is recorded in [its acceptance receipt](validation/irelia_siren.json).

The native app, shared match controller and bot API now support untimed
best-of-three practice with the complete Arito registration. Native setup,
sideboarding, field reveal, play order, results and match coaching use the shared
controller. Next: improve matchup and sideboard policies, expose additional
complete registrations, and continue the end-to-end rules audit. Card-library
completeness remains a separate active goal.
