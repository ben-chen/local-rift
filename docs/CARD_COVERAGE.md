# Official card inventory and implementation coverage

Snapshot checked **2026-10-24**. The official gallery contains **1,408 printing records** grouped into **1,103 distinct full-name identities**. After excluding **12 auxiliary token/tracker identities** and **12 banned identities**, **1,079 identities are legal in Standard/1v1 Constructed**. The separate 2v2 list leaves **1,078**.

**This client does not yet implement every legal card.** It currently maps **272 main-deck cards, 7 independently registered legends, 11 battlefields, and 6 rune resource types** (296 legal identities). **783 legal identities remain unimplemented and are not playable.** Importing this snapshot does not register cards with the engine. Full inventory coverage is not full rules support.

## Scope and authority

The [official Rules Hub](https://playriftbound.com/en-us/rules-hub/) says all currently produced sets are legal; its Constructed lists were updated **September 18, 2026**. Its Core and Tournament Rules links are dated October 9, 2026. The [Radiance overview](https://playriftbound.com/en-us/news/announcements/the-radiance-overview/) confirms its October 23, 2026 release. The reviewed release boundary is Origins, Proving Grounds, Spiritforged, Unleashed, Vendetta, and Radiance. Future gallery previews fail the importer until their release/legality is reviewed. This is the globally legal card pool; deck identity, signature, copy-limit, and other deckbuilding requirements still apply. The client supports 1v1, not the separately reported 2v2 format.

The inventory is extracted from the [official English card gallery](https://playriftbound.com/en-us/card-gallery/). A gameplay identity here means one full gallery name including subtitle; alternate art, signed, showcase, and cross-set reprints with the same name are grouped. All 305 additional gallery printings remain individually available. This proves completeness against this published gallery and all numbered base-set positions; it does not claim an inventory of every physical foil/promo/language variant.

| Set | Numbered base positions present | Gallery printings |
| --- | ---: | ---: |
| Origins (OGN) | 298/298 | 352 |
| Proving Grounds (OGS) | 24/24 | 24 |
| Spiritforged (SFD) | 221/221 | 288 |
| Unleashed (UNL) | 219/219 | 288 |
| Vendetta (VEN) | 166/166 | 237 |
| Radiance (RAD) | 167/167 | 219 |

Numbered positions include token slots where the publisher numbers them within a set (including Origins). Special, alternate, signed, rune, and token codes remain in the printing inventory even when they are outside these numbered ranges.

## Runtime gap by card kind

**The by-kind counts below are provisional:** 23 identities have contradictory official structured types and accessibility labels. The table retains structured types without adjudicating them; total identities and legality counts are unaffected. None of these unresolved identities is mapped to the runtime.

| Kind | Standard legal | Implemented mapping | Not implemented |
| --- | ---: | ---: | ---: |
| Battlefield | 69 | 11 | 58 |
| Gear | 127 | 54 | 73 |
| Legend | 58 | 7 | 51 |
| Rune | 6 | 6 | 0 |
| Spell | 269 | 69 | 200 |
| Unit | 550 | 149 | 401 |

Implemented mapping means the existing engine/preset/field/resource implementation, not a claim that every interaction has been formally proven. See `docs/RULES.md` for engine limits. Every unimplemented identity has `implementation.status: not_implemented` and `playable_in_client: false`. Printed keywords are recorded per card and counted under `unimplemented_by_observed_keyword` as discovery hints only; shared keywords do not imply shared effect semantics or completed implementation. Each unimplemented card still requires its specific effect, targets, timing, costs, continuous/replacement effects, and interaction tests to be reviewed and implemented.

## Ban lists

Standard/1v1 bans from the Rules Hub:

- Called Shot
- Ekko, Recurrent
- Draven, Vanquisher
- Fight or Flight
- Scrapheap
- Stealthy Pursuer
- Stacked Deck
- The Arena’s Greatest
- Aspirant’s Climb
- Dreaming Tree
- Obelisk of Power
- Reaver’s Row

2v2 additionally bans Master Yi, Wuju Bladesman (gallery identity **Wuju Bladesman, Starter**, `ogs-019-024`). The Rules Hub calls one banned field “Dreaming Tree”; the gallery calls it **The Dreaming Tree**. These two explicit name aliases are audited in the importer. Punctuation/case normalization handles the 2v2 list’s “Draven Vanquisher”. Bans apply to all printings of the identity.

## Text provenance and source issues

The snapshot preserves printed rules HTML, readable rules, equipment text, exact official IDs, costs/stats, domains, tags, and source image URLs for every printing. Missing structured values stay JSON `null`; they are not invented. **67 errata entries affecting 67 identities** are separately linked to their official article. Errata text preserves Riot’s `[C]`, `[A]`, `[M]`, and exhaustion notation. These patches are not automatically interpreted as executable effects or silently merged into printed text. Explicit errata aliases connect Dragon's Rage to its Lee Sin signature printing, Highlander to its Yi signature printing, Icathian Rain to its Kai'Sa signature printing, and Leblanc, Deceiver to the Deceiver legend.

- [Origins Errata](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-origins-card-errata/)
- [Spiritforged Errata](https://playriftbound.com/en-us/news/rules-and-releases/riftbound-spiritforged-errata/)
- [Unleashed Errata](https://playriftbound.com/en-us/news/rules-and-releases/unleashed-errata-updates/)
- [Vendetta Errata](https://playriftbound.com/en-us/news/announcements/vendetta-errata-updates/)
- [Radiance Errata](https://playriftbound.com/en-us/news/announcements/radiance-errata-updates/)

**13 Radiance records have an empty structured type field.** Their type is taken from the same record’s explicit official image accessibility label and marked `type_source: cardImage.accessibilityText`. The archived record preserves the missing field.

**23 records have contradictory structured and accessibility type labels.** `structured_types`, `accessibility_types`, and `type_labels_conflict` preserve both claims. Each affected identity lists `types` under `conflicting_fields`; the importer rejects any runtime mapping to such an identity, even when another printing is selected. No source label is silently preferred as the correct answer.

| Official ID | Card | Structured type | Accessibility type |
| --- | --- | --- | --- |
| `rad-007-167` | Rally for War | gear | spell |
| `rad-012-167` | Runeblade Anvil | unit | gear |
| `rad-016-167` | Coalesced Energy | spell | unit |
| `rad-025-167` | Consult the Heavens | unit | spell |
| `rad-033-167` | Command: Attack | gear | spell |
| `rad-035-167` | Invigorating Bloom | unit | gear |
| `rad-039-167` | Ephemeral Ward | unit | gear |
| `rad-044-167` | Hunting Blind | unit | gear |
| `rad-046-167` | Resplendent Rostrum | spell | gear |
| `rad-056-167` | Improvised Projectile | gear | spell |
| `rad-059-167` | Supernova | unit | spell |
| `rad-064-167` | Scrutinizing Scope | spell | gear |
| `rad-069-167` | Zero Point Inflection | spell | gear |
| `rad-072-167` | Overloaded Traveler | spell | unit |
| `rad-087-167` | Portable Portal | unit | gear |
| `rad-097-167` | Siren of the Shallows | spell | unit |
| `rad-101-167` | An Offer You Can't Refuse | gear | spell |
| `rad-108-167` | Bushwhack Trap | unit | gear |
| `rad-120-167` | Meticulous Restorationist | gear | unit |
| `rad-124-167` | Burn Away | gear | spell |
| `rad-130-167` | Battlefield Armorer | spell | unit |
| `rad-131-167` | Demacian Illuminators | spell | unit |
| `rad-136-167` | Dockside Lock-Up | unit | gear |

**Unresolved official stat conflict:** Kai'Sa, Rebel, `might`: `rad-063-167` = 5; `rad-sp4-005` = 4. Both values are retained. The numbered base card is the inventory representative, not a ruling that resolves this discrepancy.

Gallery text may contain editorial errors (for example Rousing Display ends with `instead.disa`); source text is preserved rather than silently repaired. The gallery’s auxiliary entries are also incomplete as a complete token catalog: official rules/card text mentions additional tokens such as Tentacle, Mech, and Bomb. Tokens cannot be included in legal constructed decks and are excluded from the 1,079 denominator; a future full engine still needs every rules-created token and marker.

## Reproduce and validate

No extra Python packages are needed:

```sh
python3 tools/import_catalog.py --check
python3 -m unittest discover -s tools -p 'test_*.py'
cargo test --test catalog_coverage
```

The four sourced tournament lists remain separate from the legend registry; new legend availability does not create a new tournament preset. The offline check verifies every archived source SHA-256, reparses gallery and legality, checks every official printing ID and all numbered base positions, rejects unknown sets/unresolved bans or errata, remaps the runtime manifests by official ID, verifies each independent legend’s source text/domains/artwork and matching chosen champion, and requires the generated snapshot and report to match byte for byte. It fails on truncated sources, missing records, stale implementation mappings, mapped identities with unresolved type conflicts, or unreviewed set changes.

```sh
# Compare live official source data without changing the committed snapshot.
python3 tools/import_catalog.py --check-live
# Refresh after reviewing newly released sets/legality; adjust RELEASED_SETS when needed.
python3 tools/import_catalog.py --refresh --as-of YYYY-MM-DD
# Regenerate after an intentional runtime mapping change, without a network request.
python3 tools/import_catalog.py --write
```

Source pages are archived as deterministic gzip files under `assets/data/catalog_sources/`; `assets/data/catalog_sources.json` records their official URLs, retrieval date, SHA-256 hashes, and reviewed release boundary. `--check-live` compares gameplay inventory, ban lists, and errata rather than unrelated website build markup. The snapshot is dated evidence; it cannot assert that a future ban list remains unchanged.

## Implemented main-card ID mapping

| Runtime ID | Card | Official printing ID |
| ---: | --- | --- |
| 0 | Irelia, Fervent | `sfd-057-221` |
| 1 | Abandon | `unl-131-219` |
| 2 | Akali, Silent | `ven-038-166` |
| 3 | Boots of Swiftness | `sfd-133-221` |
| 4 | Charm | `ogn-043-298` |
| 5 | Defiant Dance | `sfd-196-221` |
| 6 | Defy | `ogn-045-298` |
| 7 | Discipline | `ogn-058-298` |
| 8 | En Garde | `ogn-046-298` |
| 9 | Flash | `ogs-011-024` |
| 10 | Guardian Angel | `sfd-051-221` |
| 11 | Gust | `ogn-169-298` |
| 12 | Not So Fast | `sfd-045-221` |
| 13 | Pyke, Returned | `unl-145-219` |
| 14 | Ride the Wind | `ogn-173-298` |
| 15 | Scuttle Crab | `unl-053-219` |
| 16 | Star-Crossed | `unl-128-219` |
| 17 | Stellacorn Herder | `sfd-048-221` |
| 18 | Tideturner | `ogn-199-298` |
| 19 | Twilight Shroud | `ven-031-166` |
| 20 | Vex, Apathetic | `unl-150-219` |
| 21 | Zhonya's Hourglass | `ogn-077-298` |
| 22 | Master Yi, Tempered | `unl-113-219` |
| 23 | First Mate | `ogn-132-298` |
| 24 | Legion Marauder | `ven-074-166` |
| 25 | Lonely Poro | `sfd-036-221` |
| 26 | Onslaught | `ven-081-166` |
| 27 | Pit Rookie | `ogn-136-298` |
| 28 | Punch First | `sfd-097-221` |
| 29 | Rampage | `ven-083-166` |
| 30 | Rengar, Trophy Hunter | `unl-120-219` |
| 31 | Ruin Runner | `sfd-105-221` |
| 32 | Sabotage | `ogn-156-298` |
| 33 | Kennen, Storm of Shuriken | `ven-113-166` |
| 34 | Carrion Dredger | `unl-153-219` |
| 35 | Ezreal, Prodigy | `sfd-149-221` |
| 36 | Fizz, Trickster | `sfd-140-221` |
| 37 | Hard Bargain | `sfd-136-221` |
| 38 | Last Rites | `sfd-150-221` |
| 39 | Lightning Rush | `ven-156-166` |
| 40 | Rhasa the Sunderer | `ogn-195-298` |
| 41 | Salvage | `ogn-224-298` |
| 42 | Seal of Discord | `ogn-204-298` |
| 43 | Shadow Order Disciple | `ven-095-166` |
| 44 | Switcheroo | `sfd-145-221` |
| 45 | Tail-Cloaked Matriarch | `ven-104-166` |
| 46 | Tornado Warrior | `ven-099-166` |
| 47 | Traveling Merchant | `ogn-185-298` |
| 48 | Up from the Deep | `ven-100-166` |
| 49 | Kai'Sa, Survivor | `ogn-039-298` |
| 50 | Bellows Breath | `sfd-080-221` |
| 51 | Brynhir Thundersong | `ogn-026-298` |
| 52 | Eclipse | `unl-063-219` |
| 53 | Falling Star | `ogn-029-298` |
| 54 | Ferrous Forerunner | `sfd-021-221` |
| 55 | Hextech Ray | `ogn-009-298` |
| 56 | Inferna | `unl-002-219` |
| 57 | Lecturing Yordle | `ogn-087-298` |
| 58 | Long Sword | `sfd-022-221` |
| 59 | Noxus Hopeful | `ogn-012-298` |
| 60 | Ravenbloom Student | `ogn-103-298` |
| 61 | Singularity | `ogn-105-298` |
| 62 | Smite | `unl-007-219` |
| 63 | Stupefy | `ogn-095-298` |
| 64 | Thousand-Tailed Watcher | `ogn-116-298` |
| 65 | Time Warp | `ogn-122-298` |
| 66 | Turn to Dust | `unl-070-219` |
| 67 | Void Seeker | `ogn-024-298` |
| 68 | Watchful Sentry | `ogn-096-298` |
| 69 | Mega-Mech | `ogn-088-298` |
| 70 | Mountain Drake | `ogn-142-298` |
| 71 | Playful Phantom | `ogn-049-298` |
| 72 | Shipyard Skulker | `ogn-175-298` |
| 73 | Vanguard Sergeant | `ogn-219-298` |
| 74 | Incinerate | `ogs-003-024` |
| 75 | Falling Comet | `ogn-085-298` |
| 76 | Primal Strength | `ogn-154-298` |
| 77 | Progress Day | `ogn-114-298` |
| 78 | Premonition | `sfd-087-221` |
| 79 | Moonlight Affliction | `unl-066-219` |
| 80 | Rebuke | `ogn-172-298` |
| 81 | Smoke Screen | `ogn-093-298` |
| 82 | Challenge | `ogn-128-298` |
| 83 | Clash of Giants | `unl-110-219` |
| 84 | Lunar Boon | `unl-125-219` |
| 85 | Back to Back | `ogn-206-298` |
| 86 | Wind Wall | `ogn-064-298` |
| 87 | Recruit the Vanguard | `ogs-015-024` |
| 88 | Daring Poro | `ogn-210-298` |
| 89 | Laurent Duelist | `sfd-156-221` |
| 90 | Petty Officer | `ogn-215-298` |
| 91 | Horns of the Dragon | `ven-118-166` |
| 92 | Navori Scout | `sfd-037-221` |
| 93 | Pouty Poro | `ogn-013-298` |
| 94 | Soulspinner | `ven-123-166` |
| 95 | Blazing Scorcher | `ogn-001-298` |
| 96 | Legion Rearguard | `ogn-010-298` |
| 97 | Sharkling | `unl-006-219` |
| 98 | Rengar, Unseen | `unl-024-219` |
| 99 | Dangerous Duo | `ogn-016-298` |
| 100 | Scrapyard Champion | `ogn-020-298` |
| 101 | Trifarian Gloryseeker | `ogn-217-298` |
| 102 | Vanguard Captain | `ogn-218-298` |
| 103 | Eager Drakehound | `sfd-006-221` |
| 104 | Garen, Rugged | `ogs-007-024` |
| 105 | Laurent Bladekeeper | `sfd-096-221` |
| 106 | Master Yi, Honed | `ogs-009-024` |
| 107 | Mutated Mouser | `unl-036-219` |
| 108 | Pakaa Cub | `ogn-135-298` |
| 109 | Shen, Kinkou | `ogn-241-298` |
| 110 | Stalwart Poro | `ogn-052-298` |
| 111 | Staunch Survivor | `rad-088-167` |
| 112 | Sunlit Guardian | `ogn-054-298` |
| 113 | Towering Combatant | `unl-099-219` |
| 114 | Vanguard Attendant | `ogs-016-024` |
| 115 | Wrath of the Freljord | `rad-077-167` |
| 116 | Zephyr Sage | `ogs-005-024` |
| 117 | B.F. Sword | `sfd-161-221` |
| 118 | Doran's Blade | `sfd-095-221` |
| 119 | Sterak's Gage | `sfd-056-221` |
| 120 | Cloth Armor | `sfd-064-221` |
| 121 | Doran's Shield | `sfd-033-221` |
| 122 | Hexdrinker | `sfd-102-221` |
| 123 | Serrated Dirk | `sfd-009-221` |
| 124 | Nazuman Ntofo | `rad-071-167` |
| 125 | Seal of Focus | `ogn-081-298` |
| 126 | Seal of Insight | `ogn-120-298` |
| 127 | Seal of Rage | `ogn-040-298` |
| 128 | Seal of Strength | `ogn-163-298` |
| 129 | Seal of Unity | `ogn-245-298` |
| 130 | Energy Conduit | `ogn-098-298` |
| 131 | Battering Ram | `sfd-012-221` |
| 132 | Consult the Past | `ogn-083-298` |
| 133 | Downstage Dramatics | `unl-061-219` |
| 134 | Dredge Up | `ven-049-166` |
| 135 | Feral Strength | `sfd-034-221` |
| 136 | Frigid Touch | `sfd-066-221` |
| 137 | Bonds of Strength | `sfd-151-221` |
| 138 | Iterative Design | `ven-051-166` |
| 139 | Cloud Drake | `ven-048-166` |
| 140 | Evershade Stalker | `unl-123-219` |
| 141 | Field Musicians | `ven-026-166` |
| 142 | Whiteflame Protector | `ogn-082-298` |
| 143 | War Scout | `rad-008-167` |
| 144 | Riptide Rex | `ogn-092-298` |
| 145 | Riptide Mage | `rad-058-167` |
| 146 | Blastcone Fae | `ogn-097-298` |
| 147 | Faithful Manufactor | `ogn-211-298` |
| 148 | Frisky Hunter | `unl-033-219` |
| 149 | Honest Broker | `sfd-155-221` |
| 150 | Hunting Dog | `rad-119-167` |
| 151 | Trove Golem | `sfd-174-221` |
| 152 | Demacian Diplomat | `unl-092-219` |
| 153 | Voracious Gromp | `unl-100-219` |
| 154 | Herald of Spring | `unl-034-219` |
| 155 | Scrutinizing Sergeant | `unl-157-219` |
| 156 | Pit Crew | `ogn-091-298` |
| 157 | Fretful Feline | `ven-071-166` |
| 158 | Simian Ancestor | `sfd-047-221` |
| 159 | Vicious Snapjaws | `unl-129-219` |
| 160 | Eminent Benefactor | `sfd-152-221` |
| 161 | Plundering Poro | `sfd-069-221` |
| 162 | Treasure Hunter | `sfd-130-221` |
| 163 | Black Market Broker | `sfd-121-221` |
| 164 | Ember Monk | `ogn-167-298` |
| 165 | Bubble Bot | `sfd-062-221` |
| 166 | Twilight Reveler | `ven-020-166` |
| 167 | Crackshot Corsair | `ogn-130-298` |
| 168 | Ahri, Inquisitive | `ogn-119-298` |
| 169 | Harpoon Squad | `sfd-137-221` |
| 170 | Reluctant Leader | `ven-121-166` |
| 171 | Undercover Agent | `ogn-178-298` |
| 172 | Chemtech Enforcer | `ogn-003-298` |
| 173 | Diana, No Longer Human | `unl-149-219` |
| 174 | Cithria of Cloudfield | `ogn-139-298` |
| 175 | Viktor, Innovator | `ogn-117-298` |
| 176 | Viktor, Leader | `ogn-246-298` |
| 177 | Machine Evangel | `ogn-239-298` |
| 178 | Royal Guard | `sfd-157-221` |
| 179 | Noxian Drummer | `ogn-222-298` |
| 180 | Corinna Veraza | `sfd-179-221` |
| 181 | Yordle Explorer | `sfd-100-221` |
| 182 | Lux, Illuminated | `ogs-006-024` |
| 183 | Darius, Trifarian | `ogn-027-298` |
| 184 | Sanctum Conservator | `rad-030-167` |
| 185 | Teemo, Scout | `ogn-197-298` |
| 186 | Suave Negotiator | `rad-099-167` |
| 187 | Walking Roost | `unl-130-219` |
| 188 | Windsinger | `sfd-138-221` |
| 189 | Zaunite Bouncer | `ogn-188-298` |
| 190 | Beast Below | `sfd-132-221` |
| 191 | Carnivorous Snapvine | `ogn-149-298` |
| 192 | Solari Shieldbearer | `ogn-051-298` |
| 193 | Mystic Poro | `ogn-171-298` |
| 194 | Strigian Oracle | `rad-053-167` |
| 195 | Frigid Jewel | `unl-074-219` |
| 196 | Pirate's Haven | `ogn-143-298` |
| 197 | Mask of Foresight | `ogn-060-298` |
| 198 | Vanguard Helm | `ogn-228-298` |
| 199 | Sacred Shears | `sfd-172-221` |
| 200 | Doran's Ring | `sfd-124-221` |
| 201 | Cull | `sfd-134-221` |
| 202 | World Atlas | `sfd-086-221` |
| 203 | Eye of the Herald | `sfd-153-221` |
| 204 | Warmog's Armor | `sfd-108-221` |
| 205 | Pendulum Blade | `ven-011-166` |
| 206 | Recurve Bow | `sfd-016-221` |
| 207 | Trinity Force | `sfd-115-221` |
| 208 | Ahri, Alluring | `ogn-066-298` |
| 209 | Shen, Leader of the Kinkou Order | `ven-138-166` |
| 210 | Shen, Scourge of Shadows | `ven-042-166` |
| 211 | Enthusiastic Promoter | `unl-043-219` |
| 212 | Spectral Centaur | `unl-068-219` |
| 213 | Storm Raptor | `rad-036-167` |
| 214 | Wraith of Echoes | `ogn-118-298` |
| 215 | Prize of Progress | `sfd-075-221` |
| 216 | Upstage Comedy | `unl-009-219` |
| 217 | Marching Orders | `sfd-114-221` |
| 218 | Gentlemen's Duel | `ogs-008-024` |
| 219 | Wages of Pain | `sfd-070-221` |
| 220 | Desert's Call | `sfd-031-221` |
| 221 | Guttural Roar | `ven-072-166` |
| 222 | Monster Harpoon | `unl-014-219` |
| 223 | Sudden Storm | `sfd-017-221` |
| 224 | Right of Conquest | `unl-015-219` |
| 225 | Show of Strength | `sfd-106-221` |
| 226 | Sky Splitter | `ogn-014-298` |
| 227 | Shock Blast | `ven-059-166` |
| 228 | Against the Odds | `sfd-001-221` |
| 229 | High Note | `rad-048-167` |
| 230 | Heart of Dark Ice | `sfd-052-221` |
| 231 | Orb of Regret | `ogn-090-298` |
| 232 | Iron Ballista | `ogn-017-298` |
| 233 | Arena Kingpin | `unl-001-219` |
| 234 | Arena Bar | `ogn-124-298` |
| 235 | Vanguard Armory | `sfd-168-221` |
| 236 | Dragonsoul Sage | `unl-093-219` |
| 237 | Xerath, Freed | `unl-026-219` |
| 238 | Caitlyn, Patrolling | `ogn-068-298` |
| 239 | Renata Glasc, Mastermind | `sfd-088-221` |
| 240 | Garbage Grabber | `ogn-099-298` |
| 241 | Assembly Rig | `sfd-019-221` |
| 242 | Divining Shells | `unl-161-219` |
| 243 | Hextech Formula | `ven-062-166` |
| 244 | Ancient Henge | `sfd-117-221` |
| 245 | Hextech Anomaly | `sfd-083-221` |
| 246 | Poro Snax | `sfd-046-221` |
| 247 | The Syren | `ogn-184-298` |
| 248 | Iridescent Avian | `rad-027-167` |
| 249 | Ultrasoft Poro | `unl-160-219` |
| 250 | Questionable Tome | `ven-054-166` |
| 251 | Hextech Disc | `ven-087-166` |
| 252 | Platewyrm Egg | `ven-075-166` |
| 253 | Tools of Empire | `ven-077-166` |
| 254 | Sun Disc | `ogn-021-298` |
| 255 | Ravenborn Tome | `ogn-032-298` |
| 256 | Temporal Portal | `sfd-078-221` |
| 257 | Raging Firebrand | `ogn-031-298` |
| 258 | Astral Heron | `ven-044-166` |
| 259 | Annie, Fiery | `ogs-001-024` |
| 260 | Consuming Curse | `ven-010-166` |
| 261 | Nami, Headstrong | `unl-052-219` |
| 262 | Bushwhack | `sfd-004-221` |
| 263 | Confront | `ogn-129-298` |
| 264 | Magma Wurm | `ogn-011-298` |
| 265 | Rek'Sai, Breacher | `sfd-029-221` |
| 266 | Syndra, Transcendent | `unl-146-219` |
| 267 | Applied Researchers | `ven-055-166` |
| 268 | Helm of Suppression | `ven-045-166` |
| 269 | Stargazer | `ven-098-166` |
| 270 | Vex, Cheerless | `sfd-146-221` |
| 271 | Irelia, Graceful | `sfd-141-221` |

## Implemented legend ID mapping

Registry IDs are independent of the four sourced tournament presets. The original four retain their existing implementation status for compatibility; additional legends use `implemented_legend`. These mappings identify reviewed runtime identities, not a proof of all interactions.

| Legend ID | Legend | Official printing ID |
| ---: | --- | --- |
| 0 | Blade Dancer | `sfd-195-221` |
| 1 | Wuju Bladesman, Starter | `ogs-019-024` |
| 2 | Heart of the Tempest | `ven-155-166` |
| 3 | Daughter of the Void | `ogn-247-298` |
| 4 | Hand of Noxus | `ogn-253-298` |
| 5 | Herald of the Arcane | `ogn-265-298` |
| 6 | Lady of Luminosity, Starter | `ogs-021-024` |

Card names, rules, and images are Riot Games intellectual property. This noncommercial, unofficial project includes source references for verification and does not claim Riot affiliation.
