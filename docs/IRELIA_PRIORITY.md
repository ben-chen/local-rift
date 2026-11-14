# Irelia implementation priority

Updated November 14, 2026. Irelia cards take priority over the saved Azir draft.

Consult the Heavens is available as an additional Calm card for custom Irelia
practice. It grants Shield 2 for the turn and independently offers Predict.
Local validation passed: 1,063 library tests, eight focused rules/API scenarios,
both companion levels, six best-of-three series (12 games), and native controls
at both sizes. The small-window hover shows the temporary grant separately from
total Shield. Installed acceptance passed 1,989 Rust tests, 326 source checks,
1,176 simulated matches, 19 interface cases (114 captures), and eight exact
API comparisons (51 full records). Four installed screens were visually reviewed.
The seven sourced recipes are unchanged. Its [admission](../assets/data/card_admissions/irelia_heavens/admissions.json)
records the printed face and existing gallery type correction;
[validation](validation/irelia_heavens.json) retains the checks and limitations.

A separate [defensive decision benchmark](validation/irelia_defense.json) now
creates actual combat from movement and compares Pass with an explicit Consult
cast. Across both seats and five attacker strengths, Strategic makes all ten
preferred combat decisions. Relaxed and Tactical each make six of ten, spending
the spell unnecessarily against the weakest and strongest threats. These are
measured decision outcomes, separate from complete-match acceptance; no AI
scoring change or tournament-strength claim follows from this narrow audit.

November 14: Crumbling Sands is available for custom sideboard practice.
Its shared spell-finalization history distinguishes a spell being played from
successfully resolving, including countered spells and actual replays. The
official Singapore Irelia list is historical and contains now-banned Stacked
Deck; the October 8 community list is a relevance lead, not a new tournament
preset. Local engine/API tests, six best-of-three series (13 games), source checks,
companion advice and native controls at both sizes passed. Its immutable installed
release passed 1,972 Rust tests, 322 source checks, 1,176 simulated matches,
15 interface cases (92 captures), and six exact native/package API comparisons.
Four installed captures were visually reviewed. See [validation](validation/irelia_sands.json).

November 14: Royal Entourage passed local rules, native, complete-match API and
immutable installed acceptance checks. It can ready Irelia or exhaust an opposing legend, with
target selection before responses and the ready/exhaust choice on resolution.
It is an additional custom-practice card; the seven sourced recipes remain
unchanged. The source and timing record is in its
[admission](../assets/data/card_admissions/irelia_entourage/admissions.json), with
[validation evidence](validation/irelia_entourage.json).

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
exhaustion choice, resolving draw cost, exhausted Gold, and repeated
Irelia-ready interactions use the same rules through the client and bot API.
The original [validation record](validation/irelia_innovator.json) separates complete
games, focused interactions, source review and native interface checks. Its
pre-response payment interpretation is superseded by the November 12 timing
correction in [the rules audit](RULES_AUDIT.md).

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
controller. Tactical and Strategic match AI now evaluates the six Arito sideboard
roles using revealed-card scouting, with optional Mote review of the human draft.
Next: measure and improve matchup decisions, expose additional complete
registrations, and continue the end-to-end rules audit. Card-library
completeness remains a separate active goal.

Match practice now includes the exact Xeno LA registration and complete
Kennen and Rengar opponents from Riot’s same September article.
Decree of Strength and Lacerate fill their final sideboard gaps. The main-deck
recipes remain unchanged. Unyielding Spirit adds global spell/ability damage
prevention, while Repulse adds linked unit and chain targeting, including Repeat
choices and physical-copy tracking. Both include bot API support, native controls
and Beginner/Advanced companion advice. Their acceptance evidence is recorded in
[reaction-card validation](validation/master_yi_reactions.json).
Master Yi’s complete registration has passed native and installed acceptance;
see [its validation](validation/master_yi_registration.json). The historical Kai’Sa registration contains a now-banned
battlefield and is not admitted as a complete current Standard registration.

The [registered-opponent and timing checkpoint](validation/registered_opponents.json) records final Nix acceptance, installed UI checks and matching native/packaged API games for these registrations and the corrected Innovator sequence.

Alpha Strike is available with resolution-time allocation, public bot choices,
native controls and companion advice. Its physical-parent Repeat interpretation
remains explicit in [the rules audit](ALPHA_STRIKE_AUDIT.md). Brutalizer and Hand
Hammer have visible live bonuses and attachment history. The [release validation](validation/alpha_equipment.json)
records completed native and installed checks. Master Yi now supplies another
registered opponent for Irelia match practice. No Azir preset is added, and both
Irelia recipes remain unchanged.

Jae Medarda is an additional Calm/Chaos-compatible card for spell-choice
practice. The shipped tournament recipes remain unchanged; the validation
campaign uses an explicitly authored Irelia list. His separate draw trigger,
card inspection and Beginner/Advanced advice passed native and installed acceptance described in
[the Jae validation record](validation/irelia_choices.json). This admission does
not establish that he is a current tournament staple.

Spirit Wheel adds another Calm/Chaos-compatible practice option. A friendly
unit chosen by a spell or ability offers a separate draw trigger; its Energy
and source-exhaustion cost is paid before responses. The seven sourced recipes
remain unchanged. Its [validation](validation/irelia_wheel.json) distinguishes
rules scenarios, authored matches and unproven strategic use. Its frozen installed
release passed the full regression, source and simulation gates, both window
sizes, best-of-three flow and four exact API replay comparisons.

The [Strategic setup investigation](validation/irelia_ai_search.json) distinguishes
an unused card from a demonstrated bad decision. Authored positions cover useful
Wheel/Discipline sequencing, resource restraint, winning conquest and hidden
information independence. Equivalent encodings of ordinary hand plays now share
a search slot; optional costs and target choices stay distinct. This improves
search diversity without changing card-specific scoring weights. Local and
installed release acceptance passed: 1,951 Rust tests, 318 source checks,
1,176 simulated matches and the recorded client/API checks. This is regression
evidence, not proof of expert opponent strength.

Clockwork Keeper, Meditation and Life from Stone extend the Calm card pool for
Irelia-compatible custom practice. Keeper uses its official erratum: the optional
Power is paid on play, and a separate trigger draws after responses. The spells
share explicit unit-exhaustion costs, distinct from choosing spell targets.
The [resource-card validation](validation/irelia_resources.json) records both-seat
rules scenarios, native controls, companion advice and six complete API matches.
The campaign uses an authored coverage list; the seven sourced presets remain
unchanged. These additions do not claim new tournament-list provenance.

Royal Entourage adds explicit legend targeting followed by a ready-or-exhaust
choice after responses. Its [validation record](validation/irelia_entourage.json)
includes 1,961 passing Rust tests, 320 source checks, 1,176 simulated matches,
78 installed-client captures and five exact native/package API comparisons.
Four installed captures were visually reviewed, including the small-window
choice and both companion levels. The seven sourced recipes remain unchanged;
authored coverage matches do not establish tournament provenance or expert AI.

Rockfall Path adds an Irelia battlefield option identified in the secondary Murcia list. Its shared play restriction covers ordinary and instructed unit plays, Hidden and tokens, while movement remains available. The [validation record](validation/irelia_rockfall.json) records 1,055 passing library tests, both-seat rules/API scenarios, both companion levels, ten native captures at two sizes and 14 complete games across all three AI levels. Four captures were visually reviewed. The six authored best-of-three series observed 22 distinct unit arrivals by movement; they do not establish optimal play. Installed acceptance passed 1,980 Rust tests, 324 source checks, 1,176 simulated matches, 17 interface cases (102 captures) and seven complete native/package API comparisons. Four installed captures were independently viewed. The seven sourced recipes remain unchanged.
