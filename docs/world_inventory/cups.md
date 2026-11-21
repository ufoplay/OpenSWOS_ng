# World inventory: cups, continental competitions, super cups, winners

Part of the [world inventory](../WORLD_INVENTORY.md) (docs/PLAN.md M4 #4). Upstream OpenSWOS at
`df643c2`; line numbers are at that commit. The `rules/` files and components proposed
here are merged into one design in the inventory, whose §12 maps each name used here to its final one.

Upstream pin df643c2. Paths are relative to the upstream checkout.

Notes on how the world is chosen in these files:
- `World_Current()` is 0 for WORLD_9697, so `!World_Current()` means "96/97" (include/world.h:20).
- Every `World_*` table accessor goes through `WORLD_CALL` (src/world_baked.c:57-65): 2627 gets
  `WorldData2627_*` (src/world_data.c), 8485 gets `WorldData8485_*` (src/world_data_8485.c), 96/97 gets
  the fallback (0 / count 0). Two exceptions: `World_UefaSeeds` and `World_LeaguePhaseCups` fall back to
  a mod's league phases in 96/97 (`LeaguePhase_ModSeeds` / `LeaguePhase_ModCups`, src/league_phase.c:836-851).
- `SWOS_WORLDS` is a build switch (include/swos_types.h:105, default 1), not a world. Its guards are
  counted as NOT_A_BRANCH unless they change something.
- The club-to-country mapping calls (`World_FileCountry`, `World_Slot`, `World_SlotTeam`,
  `World_SyncSecondFiles`) are identity in 96/97 and 8485, and map Italy's second team file in 2627
  (include/world.h:66-86). They belong to the teams area; one row per file notes them.

## src/world_cups.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 30-50, 90-95 | cup_of, cup_under_way, cup_index | Whether a cup record (or the DIY cup under way, by id) is a "world cup". This turns the whole module on, cup by cup. | none (`World_Cups` count 0): original code | 101 cups, `kWorld2627Cups` (src/world_data.c:7315) | 32 cups, `kWorld8485Cups` (src/world_data_8485.c:3099) | DATA | NEW `rules/national_cups.json` `cups[]`, keyed by the cup's record id in `competitions/cups.json`. 96/97: empty list. |
| 108-151 | build | A later season's clubs of a world cup: from the divisions by quota (`sr_cup_entrants`, src/rules), with last season's places when kept. Returns 0 (real list again) in the first season, when the country has no groups/cup clubs, or when a quota has a cup-only club (255, l.115-117). | not run | only cups of the 4 countries with league groups (`kWorld2627Groups`) are rebuilt; the others keep the real list (`World_CupClubs` is empty) | countries without groups use `kWorld8485CupClubs` (src/world_data_8485.c:1529) | COMPONENT | `cup_entrants`: impl `team_file` (original: career_teams.c fills the cup from the team file, career club first) vs `fixed_list_then_quotas` (world_cups.c build/entrants + rules lib `sr_cup_entrants`, src/rules/sr_cups.c). Quotas are DATA: `rules/national_cups.json` `cups[].quota` [[division, clubs, byes]]. |
| 113-118, 155-159, 207-219 | build, cup_groups, club_level | Which table gives a country's clubs and their division for the cups: league groups, else the world's cup-club list. | none | `LeagueGroups_ForCountry` only (4 countries) | `World_CupClubs` (`kWorld8485CupClubs`, 32 countries) | DATA | NEW `rules/national_cups.json` `cup_clubs{country: [{slot, base_division}]}` (8485 from src/world_data_8485.c:1529); league groups belong to the leagues area. |
| 162-204, 228-270 | cup_levels, cup_open_to, entrants | Career club not in a cup's list: does it play it, and whose place does it take (round 1's club from the lowest division). Rule: a level between the cup's own levels, or the level just below a whole-country cup, not a top-flight-only league cup (l.191), not a level that has its own lower cup (l.195-202). Career games only (`_gameType != 4`, l.238). | original: career club always in slots 1 and 2 | level rule | level rule | COMPONENT | `cup_entrants` sub-rule `career_entry`: impl `always` (original) vs `level_rule` (cup_open_to + entrants, "Playa 2026-11-17"). Levels come from the quota data. |
| 272-283 | WorldCups_Plays | Does the career club play this cup (a world cup is a fixed list). Called 6x from career_init.c. | always 1 | list membership | list membership | COMPONENT | `cup_entrants` (fixed list: a club can be outside its own country's cup). |
| 285-302 | WorldCups_KeepPlace | Keep each club's final place and level at the season end, only for countries that have a world cup (called from league_playoffs.c). | nothing kept | kept | kept | COMPONENT | `cup_entrants`/`fixed_list_then_quotas` input; the kept places are SAVE (`g_worldCups.place`). |
| 304-356 | own_table, WorldCups_Install | Each world cup goes into its country's competition table at slot 1 (national cup) or 2 (league cup); a country gets its own table copy. Called from frontend.c. | 96/97 tables unchanged (league cups in slot 2: England, Scotland, Japan) | cups added/replaced per `kWorld2627Cups` (slot 1 and 2) | national cups per `kWorld8485Cups` (slot 1 only) | DATA | `competitions/country_competitions.json` (per country: national cup, league cup ids) + records in `competitions/cups.json`. 2627/8485 values from the WorldCup `country`/`slot`/`record` fields. `World_SyncSecondFiles` (l.355): teams area. |
| 358-374 | WorldCups_Entrants | The loader reads this season's clubs (built list or real list) into the record; the count is the cup's listed clubs. Called from career_teams.c. | count unchanged | world list | world list | COMPONENT | `cup_entrants`. Count is DATA `cups[].clubs`. |
| 376-410 | WorldCups_EntryReport | SWOS_TEST_CUP_ENTRY report. | - | - | - | OUT_OF_SCOPE | debugging aid. |
| 412-448 | WorldCups_Padded, WorldCups_CupMade | Bracket wider than the cup's clubs: spare places hold the first club; round 1 with byes (each waiting club's tie with itself, not played); a later season draws only round 1's clubs. | engine draw only | per cup `ties`, `n` | per cup `ties`, `n` | COMPONENT | `cup_draw`: impl `engine_draw` (original DIY draw, `DIY_ShuffleTeamGridPairs`) vs `real_bracket_with_byes` (WorldCups_Padded, WorldCups_CupMade, WorldCups_ArrangeNextRound). DATA `cups[].round1_ties`. |
| 80-83, 450-479 | first_season, WorldCups_ArrangeNextRound | First season of the world: after the engine's draw, put back the real bracket (winners of ties 2k, 2k+1; round 1 by `order1`; home/away by `flips`). Called from diy_competition.c. | engine draw | real bracket in season 1 | real bracket in season 1 | COMPONENT | `cup_draw` impl `real_bracket_with_byes`. DATA `cups[].real_rounds`, `order1[]`, `flips[]` (2627/8485 per-cup arrays above each table). Season counter is SAVE (`g_worldCups.seasons`). |
| 490-519 | WorldCups_TieDecided, league_cup, WorldCups_LeagueCupWinner | A world league cup's (slot 2) final winner is kept for the season; read by league_playoffs.c / world_playoff.c (Coppa Italia Serie C winner's play-off place). | 0 (no world league cup) | any of its 14 slot-2 cups (the Coppa Italia Serie C's winner is the one a play-off reads) | none (no slot 2) | DATA | `rules/national_cups.json` `cups[].slot` = 2; the play-off that takes the winner is in the play-offs area. Winner is SAVE (`g_worldCups.leagueCupWinner`). |
| 521-533 | WorldCups_PlayLeagueCupEarly | Play a background league cup early because a play-off needs its winner (world_playoff.c). | not called | yes | not used | DATA | Triggered by the play-off spec (play-offs area) + `cups[].slot`. |
| 535-567 | WorldCups_RoundStringIndex | Round name: tries play-off, Coppa D, Super Cup, UEFA qualifying, conti tie, then a world cup's numbered rounds (FIRST..SIXTH, ROUND OF 16). | -1 (engine names) | names | names | VIEW | `cups[].numbered_from` (`firstRound`, e.g. EFL Cup 4). |
| 569-573 | WorldCups_LeagueCupEurope | Does a cup's winner take the country's league-cup European place (career_season.c:843). | 1 for every cup (96/97's England League Cup) | 0 for every listed cup, the EFL Cup included | 0 for listed cups; 96/97's England League Cup, if still in slot 2, gives 1 (unsure) | DATA | `rules/national_cups.json` `cups[].europe`; default true for a cup not listed (96/97). |
| 86-88, 306-309 | (constants) | Capacities: 128 world cups, 64 clubs per list, 96 countries with own tables. | 0 cups | 101 cups | 32 cups | LIMIT | `rules/limits.json` `national_cups_max`, `cup_list_clubs`, `cup_table_countries`. |
| 141-146, 212, 238, 292, 300, 355 | build, club_level, entrants, KeepPlace, Install | Club to country/slot mapping (second team file). | identity | Italy's 2nd file (197) | identity | DATA | teams area (`teams/<COUNTRY>.json`). |

## include/world_cups.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 30-47 | WorldCup, World_Cups | Schema of a world cup (country, slot, record, n, ties, europe, rounds, flips, groups+quota, order1, numbered, firstRound) and its accessor. | none | kWorld2627Cups | kWorld8485Cups | DATA | schema of `rules/national_cups.json`. |
| 50-59 | WorldCupsState | Seasons ended, league cup winner, last places (512 slots). | not written | saved | saved | SAVE | save item "world_cups", written only when world != 9697 (src/save_format_json.c:7446). |

## src/world_cupd.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 320-321, 333-334, 458-461, 497-500, 580, 589, 630, 642, 664 | CupD_KeepPlace, CupD_NoteArrival, CupD_ArrangeNextRound, step, CupD_SeasonStart, CupD_TieDecided, CupD_RestoreSlot, CupD_RoundStringIndex, CupD_Member | `World_CupD()` present: the Coppa Italia Serie D (a 162-club cup outside the engine) exists. | none: module inert | `kWorld2627CupD` (src/world_data.c:10140) | none | DATA | NEW `rules/cup_d.json` (absent = no such cup). Selects component `table_cup`. |
| 57-68, 89-149, 186-231, 364-380, 495-576 | below, cd_below, cd_gen, cd_n, cd_first, cd_side, cd_sim, host_of, begin_round, step | How the cup runs: rounds stepped at each calendar advance; ties decided by real result (season 1) or by strength sim (the Mod's statesim) with the Mod's LCG seeded per round; drawn semi-finals; listed/drawn/alternate hosts. | not run | Mod gplay.c cd_* | not run | COMPONENT | `table_cup`: impl `none` vs `mod_cupd` (step, cd_sim, host_of, begin_round, CupD_Step; uses the Mod LCG 1103515245/12345). Round/tie bytes are DATA `rules/cup_d.json` `rounds[]`, `ties[]`. |
| 78-81 | gp_seas | Seed's season byte = 126 + seasons: hardcodes 2026 as the first year. | - | 126 (2026-1900) | - | DATA | derive from the world's first year (src/world.c:17 `kFirstYear`); proposed `WORLD.ini` `first_year`. |
| 170-182 | cd_str | A club's strength: per-slot table, pool clubs' table (by team pool identity), min 5, squared. | - | `strength[]`, `poolStrength[]` of kWorld2627CupD | - | DATA | `rules/cup_d.json` `strength[]`, `pool_strength[]` (src/world_data.c kWorld2627CupDStrength / kWorld2627CupDPoolStrength). |
| 239-304, 384-454 | make_record, build_tie, make_cup64, build_cup64, ours64 | Career club's tie as a one-tie contest in slot 4 (pens, no ET; 2 legs semis/final); from the round of 64 the rest as the game's 64-club cup in slot 2 (ids 201/202). | - | yes | - | COMPONENT | `table_cup` impl `mod_cupd`. |
| 318-361 | CupD_KeepPlace, CupD_NoteArrival, cd_make | From season 2, rounds 0-1 generated from last season's Serie D tables and pool arrivals (rules lib `sr_cupd_*`, src/rules/sr_cupd.c). Called from league_playoffs.c. | - | yes | - | COMPONENT | `table_cup` impl `mod_cupd`. DATA `rules/cup_d.json` `first`, `groups` (Serie D level), `base[512]`. |
| 560 | step | First season: the real result of a tie (`tie[t][4]`). | - | from data | - | DATA | `rules/cup_d.json` `ties[].winner`. |
| 615-618 | CupD_TieDecided | SWOS_TEST_CUPD_WIN forces wins. | - | - | - | OUT_OF_SCOPE | debugging aid. |
| 640-660 | CupD_RoundStringIndex | Round names by ties in round. | -1 | names | -1 | VIEW | - |
| 662-671 | CupD_Member | Is a club a Serie D club this season. No caller found (unsure: dead code). | 0 | yes | 0 | DATA | `rules/cup_d.json` `base[]`, `first`, `groups`. |
| 159, 164, 321, 327, 665-666 | cd_slot, cd_word, KeepPlace, Member | Club to slot mapping. | identity | Italy's 2nd file | identity | DATA | teams area. |

## include/world_cupd.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 41-56 | WorldCupD, World_CupD | Schema of the table cup and its accessor (0: none). | 0 | kWorld2627CupD | 0 | DATA | schema of `rules/cup_d.json`. |
| 58-60 | (constants) | 168 ties, 41 preliminary ties, 64 first-round ties. | - | used | - | LIMIT | `rules/limits.json` `cupd_ties`, `cupd_pre_ties`, `cupd_r1_ties`. |
| 63-80 | WorldCupDState | The cup's season state. | - | saved | - | SAVE | save item (save_format_json.c:7464). |

## src/world_supercup.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 71-80 | table | `!World_Current()` returns none, else `World_SuperCups`. Turns the module on. The 96/97 check is redundant (the accessor already returns 0). | none | 10 entries, `kWorld2627SuperCups` (src/world_data.c:10161) | none | DATA | NEW `rules/super_cups.json` `cups[]` (absent/empty = none). |
| 64 | gp_seas | Seed's season byte = 126 + seasons (hardcoded 2026). | - | 126 | - | DATA | derive from first year (src/world.c:17); proposed `WORLD.ini` `first_year`. |
| 83-131 | entry_of, SuperCup_TableEnded, SuperCup_CupEnded, SuperCup_HoldersEnded | Season end keeps a country's final table and cup final only if it has a Super Cup; European holders only if the UEFA Super Cup (country 0xFF) exists. Called from career_season.c. | nothing kept | kept | nothing | COMPONENT | `super_cups`: impl `none` vs `mod_supercups`. Kept data is SAVE ("world_supercup"). |
| 136-189 | last_season, place, rank_of, rule_club | Entrants by rule codes (champion, 2nd, 3rd, 4th, cup winner, finalist, Mexico's stage titles from `g_leagueWinners` + `title[]`, UEFA CL/EL/ECL holders), with a fallback code in the high nibble. | - | per cup `rule[4]`, `title[2]` | - | DATA | `rules/super_cups.json` `cups[].rule[4]` (code \| fallback<<4), `title[2]`. |
| 192-197, 379-385, 427-436 | decide, step, SuperCup_TieDecided | A semi-final or final nobody plays: the better placed wins 3 in 5, Mod LCG seeded `gp_seas*7919 + seed (+1)`. | - | yes | - | COMPONENT | `super_cups` impl `mod_supercups` (decide). DATA `cups[].seed`. |
| 201-227 | make_record | Tie rule: pens after 90' or ET then pens. | - | `level` | - | DATA | `rules/super_cups.json` `cups[].level`. |
| 280-317 | uefa_step | UEFA Super Cup: CL v EL holder (ECL if one club holds both), not in the world's first season when `fromSeason` 2; only if the career club is in it. | - | yes | - | COMPONENT | `super_cups` impl `mod_supercups`. DATA `from_season`, `rule[]`, `week`. |
| 302-305, 410-416 | uefa_step, SuperCup_TieDecided | SWOS_TEST_USC_FORCE / SWOS_TEST_SUPERCUP_WIN. | - | - | - | OUT_OF_SCOPE | debugging aid. |
| 319-387 | step | National Super Cup: career only (`_gameType == 4`, not national selector); built at `week` (or season start if `early`); first season uses the real entrants `first[4]`; 2 = one match, 4 = final four. | - | yes | - | DATA | `rules/super_cups.json` `cups[]`: `country`, `early`, `week`, `final_week`, `n`, `first[4]`, `from_season`. |
| 489-494 | SuperCup_RoundStringIndex | Round name (final / semi-final). | -1 | names | -1 | VIEW | - |
| 496-543 | SuperCup_RecordLine, SuperCup_RecordKind, SuperCup_WinnersLine | Season-record line and SEASON WINNERS line of a Super Cup won (statistics.c, world_playoff.c, world_winners.c). | 0 | `record`, `name[2]` | 0 | VIEW | `rules/super_cups.json` `cups[].name[2]`, `record` (unsure: the line is also stored in the season record). |
| 331, 535 | step, SuperCup_WinnersLine | Career club's country (second file). | identity | Italy 2nd file | identity | DATA | teams area. |

## include/world_supercup.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 34-38, 44-60 | SC_* codes, WorldSuperCup, World_SuperCups | Rule codes and schema of a Super Cup; 0xFF = UEFA Super Cup. | none | kWorld2627SuperCups | none | DATA | schema of `rules/super_cups.json`. |
| 39-40 | (constants) | 16 Super Cups, 24 table places kept. | - | 10 used | - | LIMIT | `rules/limits.json` `super_cups_max`, `super_cup_table`. |
| 62-94 | WorldSuperCupKept, WorldSuperCupState | Season-end inputs and this season's state. | - | saved | - | SAVE | save item "world_supercup" (save_format_json.c:7488). |

## src/world_europe.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 20-37 | WorldEurope_Install | The three European cup records (and their template sizes) replace `_careerContests[0..2]`; VIEW WORLD's Europe row points at them. Called from `WorldData8485_InstallBaked` (src/world_data_8485.c:625). | 96/97's `_euroCup`, `_cupWinnersCup`, `_uefaCup` (competition_data.c) | not called (league phases via `LeaguePhase_InstallCups`, src/world_data.c:1869) | `kWorld8485_worldEuropeCc/Cwc/Uefa` (src/world_data_8485.c:3171, 3216, 3261): 32/32/64-club knockouts | DATA | `competitions/career.json` entries 0..2 pointing at records in `competitions/cups.json` / `tournaments.json`; template size is SAVE (see career_comp_copies.c). The `_europeTable` patch is VIEW. |
| 39-46 | WorldEurope_Active | The world plays its European cups as knockouts with per-country places. Read by career_eurocups.c:356/767, career_season.c:1837, career_teams.c:824. | 0 | 0 (`EuropeKnockouts` none) | 1 | COMPONENT | `european_access`: impl `original` (career_eurocups.c eligibility lists, seed buckets, holder in a random place) vs `knockout_places` (world_europe.c + those callers) vs `uefa_access_list` (uefa_coef.c). Selected by `rules/europe.json` `format`. |
| 48-60 | WorldEurope_Places | A country's places in the CC / CWC / UEFA Cup. | none (96/97's places live in career_eurocups.c's lists, unsure) | none here (2627's `kWorld2627EuropePlaces`, src/world_data.c:7572, feed the league phases instead) | `kWorld8485EuropePlaces` (src/world_data_8485.c:3337) | DATA | NEW `rules/europe.json` `places{country: [cc, cwc, uefa]}`. |
| 62-131 | same_ties, fix_pairs, euro_knockout, WorldEurope_DrawMade, WorldEurope_RoundPlayed | No two clubs of one country in a tie of rounds 1 and 2 (swap away clubs, 4 passes). Only career, cups 0..2, knockout world. Called from diy_competition.c. | not applied | not applied | applied | COMPONENT | `knockout_pairing`: impl `free` (original draw) vs `no_same_country_early` (fix_pairs, the Mod 3.0 careerworld w_fixpairs). Rounds count could be DATA `rules/europe.json` `no_same_country_rounds` = 2. |
| 66 | same_ties | Club's country (second file). | identity | - | identity | DATA | teams area. |

## include/world_europe.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 20-27 | WorldEuropeKO, World_EuropeKnockouts | Schema: three cup records, their template sizes, per-country places. | 0 | 0 | kWorld8485EuropeKO (src/world_data_8485.c:3372) | DATA | `competitions/career.json` + `rules/europe.json`. |

## src/world_conti.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 104-139, 797-806, 819-820, 855, 996, 1042, 1055, 1092, 1114, 1129, 1157 | parse, ct_contest_id, ct_round_spec, ct_country, ct_go, ct_finish, ct_result, ct_gamein, ct_new1, ct_resume, ct_gplace, ct_gend | `CT_SPEC` present: the other confederations' club cups exist. The spec holds the cups' rounds (24 bytes each), access list, holder places, association->team file, club ranking, guest clubs, regions, the world's team-file bitmap, first holders, contest ids. | no spec: inert | `kWorld2627ContiSpec[5242]` (src/world_data.c:8105, made by scripts/world_conti.py from the Mod's internal/conti) | no spec | DATA | NEW `rules/continental_cups.json` `conti.spec` fields: `cups[].rounds[]`, `access[]`, `holders[]`, `assoc_file[]`, `ranking[]`, `guests[]`, `assoc_region[]`, `world_files`, `first_holders[]`, `contest_ids[]`. |
| 142-214, 336-611, 619-761, 959-993 | below, sq, pts, goals, match, tie, draw_ko, draw_groups, group, ct_round, run | How an edition is played: Mod LCG; binomial goals by squared strengths; tie levels (pens / ET / away goals CAF / away+ET CONCACAF); draw kinds D_*; association rules; holder in A1; regions; rounds in calendar order from August. | - | Mod conti.c | - | COMPONENT | `continental_club_cups` impl `conti_spec` (world_conti.c ct_* + conti_season.c). Round bytes, draws, levels are DATA in the spec. |
| 149-154, 177, 545, 914, 1032, 1098 | pts, match, draw_groups, snapshot, ct_season, ct_new1 | Edition flags in `CT_ED.pad` (ED_R ranking draws, ED_G2 goals model, S1): an older save's edition replays with the rules it began with. | - | flags | - | SAVE | save item "conti_edition" `flags`. |
| 245-333 | access | Entrants: guests, holders' own places, access codes (champion, cup winner, k-th, next, Apertura/Clausura), a vacant holder place to the best ranked club. | - | from spec | - | DATA | `rules/continental_cups.json` `conti.spec.access[]`, `holders[]`, `guests[]`. |
| 259 | access | Holder places for even/odd editions use `2028 + CT_ST.season`: hardcodes World 2026-27's first edition year. | - | 2028 | - | DATA | derive from the world's first year (src/world.c:17) or `rules/continental_cups.json` `conti.first_edition_year`. |
| 262-264 | access | The UEFA CL holder (game's) plays the Intercontinental only if its team file is a world file. | - | `wfile` bitmap | - | DATA | `conti.spec.world_files`. |
| 86, 163-168, 283-303, 1059-1061, 1144, 1167-1172 | access, ct_gamein, ct_gplace, ct_gend | Test builds: CONTI_FORCE / FORCE2 / GPLACE / G1 / NATIVE (forced places stored in the spec header and `gid[ncup..]`). | - | - | - | OUT_OF_SCOPE | debugging aid. |
| 781-874 | ct_table, ct_table_words, ct_begin, ct_country, ct_go | Season-end input: each country's final table, top 9 records, cup winner; the career club; a seed; the UEFA holder. | - | yes | - | COMPONENT | `continental_club_cups` impl `conti_spec`. |
| 994-1028, 1078-1117 | ct_finish, s1list, ct_new1, ct_load1 | First season end: holders from the spec's `s1h` (the build's 2027 winners). The season-1 block `CT_S1` is always NULL upstream. | - | from spec | - | DATA | `conti.spec.first_holders[]`. |
| 1126-1202 | ct_gplace, ct_gend | A group stage's end in the game's contest: the career club's place; Libertadores-style R16 draw (8 groups, D_SEED12); league stage by region (D_PLACE). | - | from spec | - | COMPONENT | `continental_club_cups` impl `conti_spec`. |
| 1204-1209 | CT_LAYOUT | Struct layout table for the native test. | - | - | - | OUT_OF_SCOPE | test aid. |

## include/world_conti.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 22-30 | (constants) | 128 associations, 32 league clubs, 1024 clubs, 16 cups, 8 rounds, 64 entrants a round, 24-byte rounds. | - | used | - | LIMIT | `rules/limits.json` `conti_*`. |
| 32-38 | (enums) | Spec codes: round kinds, draws, association rules, access codes. | - | - | - | DATA | schema of `rules/continental_cups.json`. |
| 49-53, 65-78, 95-101 | ct_state, ct_ed, CT_ED_* | Holders, the edition's entrants and the career club's results; CT_NED 440 entries. | - | saved | - | SAVE | save items "conti_holders", "conti_edition" (save_format_json.c:7860-7884). |

## src/conti_season.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 88-104, 139-151 | Conti_Country, spec, ours | `World_ContiSpec()` present turns the conti season end on (called per country from career_eurocups.c:346). | none | spec | none | DATA | presence of `rules/continental_cups.json` `conti`. |
| 107-113 | lists | Which cups have a game contest of their own (`CT_LP`). | - | `kWorld2627ContiCups[].contest` (src/world_data.c:8436) | - | DATA | `rules/continental_cups.json` `conti.cups[].contest`. |
| 115-135 | Conti_SeasonEnd | Next editions: seed from the first grid record, career club, UEFA CL holder (`_euroCupTitleHolderTeamId`) for the Intercontinental. Called from career_season.c. | - | yes | - | COMPONENT | `continental_club_cups` impl `conti_spec`. |
| 165-210 | make_tie, round_byte, month | Career club's tie in slot 3 (index 7): cup name/short name, FINAL by round key 8, rank, FIFA conf, away goals from level, subs 0x35. | - | from `kWorld2627ContiCups` | - | DATA | `conti.cups[]`: `name`, `short_name`, `rounds[8]`, `rank`, `conf`. Names are VIEW. |
| 216-365 | main_round, cal_last, make_main, make_game | Career club's main round as the game's contest (index 8), copied from 96/97's `_euroCup` (groups + 4 KO) or `_cupWinnersCup` (KO); late build rules. | - | yes | - | COMPONENT | `continental_club_cups` impl `conti_spec`. Uses 96/97 records as templates. |
| 368-543 | Conti_SeasonTie, Conti_RestoreTie, into_slot3, Conti_TieDecided, Conti_GroupsEnd, next_game, Conti_Daily | Career club's conti games in slot 3, results back to the edition. Called from career_init.c, diy_competition.c, career_teams.c. | stubs | yes | inert | COMPONENT | `continental_club_cups` impl `conti_spec`. |
| 546-553 | Conti_RoundStringIndex | Round name by key. | -1 | names | -1 | VIEW | `conti.cups[].rounds[]`. |
| 556-589 | Conti_TieIsFinal, Conti_TrophyTier, Conti_TrophyBit | Trophy tier (feeds the trophy bonus, career_status.c:2129-2139, 3328-3336) and trophy bit (statistics.c) from the cup's rank and conf. | - | rank/conf | - | DATA | `conti.cups[].rank`, `conf`; the bit is VIEW. |
| 592 | Conti_ReplacesCopa | The world's own cups replace the 96/97 Copa Libertadores (read by career_copa.c, teams.c). | 0 | 1 | 0 | COMPONENT | selector of `continental_club_cups` (`copa_top2` vs `conti_spec`). |

## include/conti_season.h
No branch point: declarations only (slot indices 7/8, `Conti_ReplacesCopa`); see src/conti_season.c.


## src/conti_view.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 28, 1531-1547 | (file) | Views compiled for SDL with worlds; stubs otherwise. | stubs/empty | views | empty | VIEW | - |
| 62-117 | snapshot, cups | Edition replay for the views; cup list from `World_ContiCups` capped by `spec[0]`. | none | yes | none | VIEW | - |
| 143-144, 203-219 | cal_slot, mweek, k_confName, k_confContinent, conf_of_team | August-first order; confederation names/continents; career club's confederation from `World_ContinentCountries`. | - | yes | - | VIEW | - |
| 1394-1506 | ContiView_Available, ShowConfs, CupsOfContinent, CupName, ShowCup, StatSummaryAvailable, ShowStatSummary | Menu availability and cup views (menus.c, teams.c, world_winners.c); European record from `CupRecord_Euro`. | unavailable | yes | unavailable | VIEW | - |
| 1509-1530 | ContiView_SelfTest | --conti-view-selftest. | - | - | - | OUT_OF_SCOPE | debugging aid. |

## include/conti_view.h
No branch point: view API and menu sizes only (VIEW).


## src/uefa_coef.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 46-52 | UefaCoef_On | Coefficients and the UEFA access list run only with seeds AND league-phase cups. Read by league_phase.c, world_finance.c. | off (on only with a mod's league phases + seeds, `LeaguePhase_ModSeeds`) | on | off (`WorldData8485_UefaSeeds` 0) | COMPONENT | selector of `uefa_coefficients` (`none` / `mod_uefacoef`) and of `european_access` impl `uefa_access_list`. The mod case is OUT_OF_SCOPE (mod system). |
| 78-94, 213-258, 410-411 | init, rank_into, rank, UefaCoef_AssociationRank, UefaQual_Fill | Seeds: associations' last 5 seasons (pack order breaks ties), clubs' units, the cup-only association. Rank is also read by league_zones.c. | none | `kWorld2627UefaSeeds`: 54 countries (src/world_data.c:7629), 376 clubs (7686), cupOnly 90 (8065) | none | DATA | NEW `rules/uefa_coefficients.json` `associations[{file, seasons[5]}]`, `clubs[{country, ordinal, units}]`, `cup_only`. |
| 97-209, 262-299 | ok, add, UefaCoef_StageStart, UefaCoef_Result, UefaCoef_PhaseEnd, UefaCoef_Draw, UefaCoef_Club, UefaCoef_PotKey, close_season | Coefficient points (eighths): CL participation 48, win 16, draw 8, place bonus, reach 12/8/4, tie 24/8, final 16, qualifying via `uq_coef`; club floor = association/625; 5-season ring; CL holder first in pots. | - | yes | - | COMPONENT | `uefa_coefficients` impl `mod_uefacoef` (Mod uefacoef.py). The point values could be DATA in `rules/uefa_coefficients.json` `points` (optional). |
| 340-400 | UefaQual_Country | Each country's top 9 + cup winner into the access input (ranks 1..55 only); a country's 'europe' play-off gives its last UECL place (`World_LeaguePlayoffs` spec). Called from career_eurocups.c. | - | yes | - | COMPONENT | `european_access` impl `uefa_access_list` (uefa_coef.c + uefa_access.c `uq_season`). Play-off spec is DATA in the play-offs area. |
| 403-482 | UefaQual_Fill | Holders (96/97's three holder globals read as CL/EL/UECL), 2 European Performance Spots, seed, `uq_season`, lists into the cups (holders join after). Called from career_season.c. | - | yes | - | COMPONENT | `european_access` impl `uefa_access_list`. |
| 412, 418-421, 445, 450 | UefaQual_Fill | 550 club records + 3 holders = 553 input clubs. | - | used | - | LIMIT | `rules/limits.json` `uefa_access_records` 550. |
| 454-460 | UefaQual_Fill | SWOS_UEFAQ_FORCE. | - | - | - | OUT_OF_SCOPE | debugging aid. |
| 493-544 | kTieName, tie_week, make_tie | Career club's qualifying tie record: weeks Q1 0x31, Q2 0x35, Q3 0x39, PO 0x3C; two legs, ET, pens, no away goals (0x94). | - | yes | - | COMPONENT | `uefa_access_list` constants (2024-27 calendar). Names are VIEW. |
| 548-651 | UefaQual_Post, UefaQual_SeasonTie, UefaQual_RestoreTie, slot3, UefaQual_TieDecided, UefaQual_Daily | Career club's ties in slot 3 (indices 4..6), then the league phase reached. Called from career_init.c, career_teams.c, diy_competition.c, career_season.c. | inert (no ties) | yes | inert | COMPONENT | `european_access` impl `uefa_access_list`. |
| 655-661 | UefaQual_RoundStringIndex | Round name by tie week. | -1 | names | -1 | VIEW | - |
| 42 | assoc | Club's association (second file). | identity | Italy 2nd file | identity | DATA | teams area. |

## include/uefa_coef.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 24-43 | WorldUefaSeeds, World_UefaSeeds | Seeds schema and accessor. | mod's or 0 | kWorld2627UefaSeeds | 0 | DATA | schema of `rules/uefa_coefficients.json`. |
| 45 | UEFA_COEF_CLUBS | 1024 clubs. | - | used | - | LIMIT | `rules/limits.json` `uefa_coef_clubs`. |
| 46-64 | UefaCoefState, g_uqState | Coefficient ring and the season's bracket. | not written | saved | not written | SAVE | save items (save_format_json.c:7647, 7765). |

## include/uefa_access.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 22-24 | (enum) | The 2024-27 access list's rounds (CL/EL/UECL LS, PO, Q3, Q2, Q1, champions/league paths). | - | yes | - | COMPONENT | `european_access` impl `uefa_access_list` (src/uefa_access.c, not in this inventory). |
| 25-28, 71 | NA, NL, NC, NE, NBR | 55 ranks, 10 clubs a country, 600 clubs, 400 entries, 320 bracket ties. | - | used | - | LIMIT | `rules/limits.json` `uefa_*`. |
| 31-48 | uq_in | Inputs incl. `lcup` (a league cup winner's last UECL place, England's EFL Cup; "no pack of World 2026-27 has", uefa_coef.c:393) and `eplay`. | - | `eplay` only | - | DATA | `rules/national_cups.json` `cups[].europe` (unsure) and play-offs area. |
| 72-85 | uq_state | The season's qualifying bracket. | - | saved | - | SAVE | save item (save_format_json.c:7765). |
| 106-108 | UQ_CUPONLY_FILE | Association with a cup winner and no league (Liechtenstein). | - | 90 | - | DATA | `rules/uefa_coefficients.json` `cup_only`. |

## src/career_copa.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 44-46, 187-198, 239-245 | s_copaEligibleMaster, copaTryConsume, Career_ProcessCompetitionForCopaLibertadoresQualification | Copa field: champion + runner-up of 10 CONMEBOL leagues (43, 45, 46, 48, 49, 50, 64, 65, 71, 77). | used (upstream addition, not original SWOS) | not used | not used | DATA | NEW `rules/continental_cups.json` `copa.leagues[]`, `copa.places_per_league` = 2 (from career_copa.c:44-46, 243-244). |
| 98-107 | copa_in_world | `World_SeasonCountries` must contain Brazil (46) for the Copa to exist. | no list: yes | list has 46 (src/world_data.c:1462) | no 46 (src/world_data_8485.c:447): no Copa | DATA | presence of `copa` in `rules/continental_cups.json` (or of the Copa record in `competitions/tournaments.json`). |
| 109-119, 263-266, 287-305 | Career_CopaWorkingCopy, Career_CopaLibertadoresExpectedTeamCount, Career_UpdateCopaLibertadoresCompetitionSlot | Copa off when the world has its own cups (`Conti_ReplacesCopa`) or no South America; expected count 20/0 feeds the season-transition gate (0x50 + Copa). | on (20) | off (0) | off (0) | COMPONENT | `continental_club_cups`: impl `copa_top2` (career_copa.c) vs `conti_spec` vs `none`. |

## src/career_comp_copies.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 57-66 | _euroCupCopy, _cupWinnersCupCopy, _uefaCupCopy | Buffer sizes of the three European cup copies: big enough for league phases (120) and a 64-club knockout (160); a save writes the current world's template size. | 112 / 92 / 92 (96/97 records) | 120 each (league phase) | CompetitionCup / same / +64 bytes (src/world_data_8485.c:3374) | SAVE | template size from `competitions/career.json` entries 0..2; capacity in `rules/limits.json` `euro_copy_bytes` (unsure). |

## src/cup_record.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 41-61 | CupRecord_LeaguePhaseEnd | `!World_Current()`: 96/97 keeps no record of the career club's league phase (even with a mod's league phases). Called from league_phase.c. | nothing | recorded | never called (no league phases) | SAVE | save item "cup_record" (save_format_json.c:7527). |
| 90-129 | CupRecord_Match | `!World_Current()`: 96/97 records no cup match; worlds record European knockout games and conti games (ids contiTie/contiMain). Called from diy_competition.c. | nothing | recorded | European cups ids 0..2 not league phases: only conti (none) -> nothing (unsure) | SAVE | save item "cup_record". |
| 131-140 | CupRecord_Euro, CupRecord_Conti | `World_Current() &&`: the record is shown only in a world. | NULL | record | record/NULL | VIEW | used by conti_view.c. |

## include/cup_record.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 21-23 | (constants) | 36 table rows, 96 European games, 160 conti games. | - | used | - | LIMIT | `rules/limits.json` `cup_record_*`. |
| 41-53 | CupRecordState | The record kept with the career. | not written | saved | saved if any | SAVE | save item "cup_record". |

## src/world_winners.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 68-114 | country_rules, viewed_titles | `!World_Current() \|\| _gameType != 4`: no titles; else the division's title lines from `World_LeaguePlayoffs(...).winners`. | none | titles | titles | VIEW | title lines belong to the play-offs area data. |
| 285-298 | supercup_line | `!World_Current()` or not career: no Super Cup line. | none | line | none (no Super Cups) | VIEW | - |
| 314-325 | season_title | Header year from `World_SeasonYear`. | 1996.. | 2026.. | 1984.. | VIEW | first year (src/world.c:17). |
| 533-571 | WorldWinners_Available, CompetitionStatsInit | WINNERS / SUMMARY menu entries shown. | hidden | shown when titles | shown when titles | VIEW | - |
| 581-587 | WorldWinners_SeasonDecided | Stamps last season's titles (`g_leagueWinners.season`), which world_supercup.c's `rule_club` (Mexico's stage titles) reads. Called from career_season.c. Inert when no titles were written. | inert | yes | yes | SAVE | save item (save_format_json.c:7630); also an input of `super_cups`. |
| 100, 113, 295, 363 | viewed_titles, supercup_line, build | Club's country (second file). | identity | Italy 2nd file | identity | VIEW | - |

## include/world_winners.h
No branch point: WINNERS page API only (VIEW).

## Summary

**Rows per class** (116 rows): DATA 43, COMPONENT 30, VIEW 16, SAVE 12, LIMIT 8, OUT_OF_SCOPE 7.
DATA includes 5 rows that only note the second-team-file mapping calls (teams area).
**NOT_A_BRANCH: 12** `#if SWOS_WORLDS` build guards with no world effect: world_europe.c 16, 22, 41, 50, 62, 112,
124; world_conti.c 35; conti_season.c 32; cup_record.c 28; uefa_coef.c 301, 327. Two other guards do have an effect
and have rows: conti_view.c 28 (VIEW) and career_comp_copies.c 57 (SAVE, copy sizes).

**Proposed rules/ files**
- `rules/national_cups.json`: `cups[]` {cup id, country, slot 1/2, clubs, round1_ties, europe, real_rounds, flips[],
  quota[[division, clubs, byes]], order1[], numbered_from}; `cup_clubs{country: [{slot, base_division}]}`.
  2627 from `kWorld2627Cups` (src/world_data.c:7315), 8485 from `kWorld8485Cups` / `kWorld8485CupClubs`
  (src/world_data_8485.c:3099, 1529), 96/97 empty (`europe` defaults to true for unlisted cups).
- `rules/cup_d.json`: the table cup (Coppa Italia Serie D): country, rounds[], ties[], first, groups, base[512],
  strength[], pool_strength[], name[2]. 2627 only (`kWorld2627CupD`, src/world_data.c:10140).
- `rules/super_cups.json`: cups[] {country or UEFA, seed, from_season, early, week, final_week, level, n, rule[4],
  first[4], title[2], name[2], record}. 2627 only (`kWorld2627SuperCups`, src/world_data.c:10161).
- `rules/europe.json`: `format` (original / knockout / league_phase), `places{country: [cc, cwc, uefa]}`,
  `no_same_country_rounds`. 8485 from `kWorld8485EuropePlaces` (src/world_data_8485.c:3337). The cup records go in
  `competitions/career.json` entries 0..2 + `competitions/cups.json`.
- `rules/uefa_coefficients.json`: associations[{file, seasons[5]}], clubs[{country, ordinal, units}], cup_only,
  optional `points`. 2627 only (`kWorld2627UefaSeeds`, src/world_data.c:7629/7686/8065).
- `rules/continental_cups.json`: `copa` {leagues[], places_per_league} (96/97, career_copa.c:44-46) or `conti`
  {spec: cups' rounds, access, holders, assoc_file, ranking, guests, regions, world_files, first_holders,
  contest_ids; cups[]: name, short_name, rounds[8], rank, contest, conf; first_edition_year}. 2627 from
  `kWorld2627ContiSpec` (src/world_data.c:8105) + `kWorld2627ContiCups` (8436).
- `rules/limits.json`: national cups 128 / list 64 / tables 96; cupd 168/41/64; super cups 16 / table 24; conti
  128/32/1024/16/8/64/440; uefa coef clubs 1024; uefa access 55/10/600/400/320, records 550; cup record 36/96/160;
  euro copy bytes.
- `WORLD.ini` new field `first_year` (src/world.c:17 `kFirstYear` 1996/2026/1984), to replace the hardcoded 126 and 2028.

**Proposed components**
- `cup_entrants`: `team_file` (original career_teams.c, career club first) | `fixed_list_then_quotas` (world_cups.c
  build, entrants, cup_open_to, WorldCups_Entrants/Plays/KeepPlace; `sr_cup_entrants` in src/rules/sr_cups.c).
- `cup_draw`: `engine_draw` (DIY draw) | `real_bracket_with_byes` (WorldCups_Padded/CupMade/ArrangeNextRound).
- `table_cup`: `none` | `mod_cupd` (world_cupd.c step, cd_sim, host_of, build_tie, build_cup64, CupD_*; `sr_cupd_*`).
- `super_cups`: `none` | `mod_supercups` (world_supercup.c step, uefa_step, rule_club, decide, SuperCup_*).
- `european_access`: `original` (career_eurocups.c) | `knockout_places` (world_europe.c + career_eurocups.c:356/767,
  career_teams.c:824, career_season.c:1837) | `uefa_access_list` (uefa_coef.c UefaQual_* + uefa_access.c uq_*).
- `uefa_coefficients`: `none` | `mod_uefacoef` (uefa_coef.c UefaCoef_*, close_season, rank_into).
- `knockout_pairing`: `free` | `no_same_country_early` (world_europe.c fix_pairs, WorldEurope_DrawMade/RoundPlayed).
- `continental_club_cups`: `none` | `copa_top2` (career_copa.c) | `conti_spec` (world_conti.c ct_*, conti_season.c Conti_*).
- The Mod's LCG (`s = s*1103515245 + 12345`) is used inside `mod_cupd`, `mod_supercups` and `conti_spec`, not the game's RNG.

**96/97 facts a world overrides**
- career_copa.c:44-46, 104, 243-244, 265: Copa = top 2 of 10 CONMEBOL leagues, 20 clubs; Brazil (46) as the
  "South America present" probe.
- world_cups.c:569-573: an unlisted cup (96/97's England League Cup) always takes the league-cup European place.
- world_cups.c:304-356 / world_cups.h:22: 96/97 country tables (national cup slot 1; a league cup in slot 2 for England, Scotland and Japan, per `country_competitions.json`).
- world_europe.c:23-27, 104-107: European cups = `_careerContests[0..2]`, 96/97's `_euroCup/_cupWinnersCup/_uefaCup`, ids 0..2.
- career_comp_copies.c:64-66: 96/97 copy sizes 112/92/92.
- uefa_coef.c:268, 413 and conti_season.c:123: 96/97's three holder globals reused as CL/EL/UECL holders.
- conti_season.c:292, 328: 96/97's `_euroCup` / `_cupWinnersCup` records used as templates for conti contests.

**Surprising or uncertain**
- Hardcoded World 2026-27 years: `gp_seas()` = 126 + seasons (world_cupd.c:80, world_supercup.c:64), and
  `2028 + CT_ST.season` (world_conti.c:259). They are wrong for any other world.
- world_supercup.c:74 `!World_Current()` is redundant (the accessor already returns 0 in 96/97).
- In 2627, `World_CupClubs` is empty, so only cups of the 4 countries with league groups are rebuilt from quotas; the
  other cups replay their real list every season (unsure if intended).
- In 96/97, `UefaCoef_On` can be true with a mod's league phases (mod system), but cup_record records nothing then.
- `CupD_Member` has no caller (dead code, unsure).
- No 2627 or 8485 world cup has `europe` = 1, so in 2627 no league cup (EFL Cup included, 14 slot-2 cups) gives a
  European place. In 8485, 96/97's England League Cup may stay in slot 2 and still answer 1 (unsure).
- The Copa Libertadores is an OpenSWOS addition, not original SWOS 96/97, yet runs in the 96/97 world.
- Conti test hooks (CONTI_FORCE, GPLACE) are bytes inside the spec (`fcup/frnd`, `gid[ncup..]`).
- `WorldCups_RoundStringIndex` dispatches round names for play-offs, Coppa D, Super Cups, UEFA ties and conti ties.
- `CT_S1` (season-1 block) is always NULL upstream: season 1 is played at the first season end.
