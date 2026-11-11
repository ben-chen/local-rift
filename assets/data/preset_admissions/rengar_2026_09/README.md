# Rengar / Pridestalker — Los Angeles winner

The fifth preset reproduces DSG Prismaticism's winning list from Riot's
[September 30, 2026 Los Angeles article](https://playriftbound.com/en-us/news/organizedplay/los-angeles-top-decks/).
It is a dated tournament example, published before Radiance. The main deck has
40 cards including the chosen Rengar, Trophy Hunter, with four Fury and eight
Body runes. Seat of Power is the default battlefield and belongs to the published
three-field pool.

`admission.json` pins the archived article, pilot, placing, all published sections
and the unchanged first four presets. `preset.json` is the exact new runtime
manifest row. The source test independently parses both occurrences of the
winner's list and requires them to agree before comparing each card quantity,
champion, legend, rune split and battlefield name. No unavailable card is replaced
by a similar card. The four previously missing unit identities depend on C28's
separate source admission and executable rules implementation.

The ten existing editable training recipes remain stable when a new tournament
preset is added. `predecessor_deck.rs` preserves the preceding source and
`predecessor_training_decks.json` records all ten exact JSON lists from the pinned
C27 release API. A Rust regression compares every current training recipe with
those actual preceding API outputs. The new preset keeps its own distinct list.

The article's full battlefield pool and sideboard are preserved as provenance.
This preset admission does not implement a registered three-battlefield match
or between-game sideboarding. Runtime acceptance must include the C28 card and
field rules, both-seat preset games, API selection and native UI controls before
this draft is published.
