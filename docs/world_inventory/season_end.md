# World inventory: the season end and league formats

Part of the [world inventory](../WORLD_INVENTORY.md) (docs/PLAN.md M4 #4). Upstream OpenSWOS at
`df643c2`; line numbers are at that commit. The `rules/` files and components proposed
here are merged into one design in the inventory, whose §12 maps each name used here to its final one.

Sources of the world tables (read for this inventory):
- World 2026-27: `src/world_data.c` — `kWorld2627Playoffs` (35 countries, l.4229-4262; accessor l.4265), `kWorld2627Groups` (Italy 20, Argentina 43, Australia 44, El Salvador 51; l.2348-2353; accessor l.2356), `kWorld2627Splits` (38 tables; accessor l.9888), `kWorld2627States46` (Brazil; accessor l.8813), `kWorld2627Promedio43` (Argentina, 30 rows; accessor l.8828), `kWorld2627LeaguePhases` + `kWorld2627EuropePlaces` (54; accessor l.8070), `WorldData2627_CupClubs` empty (l.2377). `FixtureRounds_Install()` and `LeaguePhase_InstallCups()` in its InstallBaked (l.1869-1870).
- Europe 1984-85: `src/world_data_8485.c` — `kWorld8485Playoffs` count 0 (l.1828-1845), `kWorld8485Groups` count 0 (l.643), `kWorld8485Splits` count 0 (l.3492), States / Promedio / LeaguePhaseCups / UefaSeeds none (l.3435, 3447, 3155, 3473), `kWorld8485CupClubs` has data (l.1565). InstallBaked calls `WorldEurope_Install` and `FixtureRounds_Install()` (l.625-626).
- 96/97: no tables. `src/world_baked.c` `WORLD_CALL` (l.57-67) returns the `otherwise` branch: NULL / count 0, except `World_LeaguePhaseCups` -> `LeaguePhase_ModCups` and `World_UefaSeeds` -> `LeaguePhase_ModSeeds` (l.153-156, 188-191).

"Spec" below = the Mod's compiled knockout bytes (gplay.c format, read by `src/rules/sr_gplay.c`): per play-off `kind, entrants, rounds, ...`, entrants 4 bytes (type, division, place, ...), rounds `ties, legs, host, level, pairing, gap` + member codes.

## src/world_playoff.c

The whole module is inert unless `World_LeaguePlayoffs()` has the career country (96/97: NULL; 8485: count 0). So for every row below 9697 = 8485 = "skipped (no table)", unless stated.

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 1584-1598, 1600-1621 | WorldPlayoff_CareerLeagueEnded, league_ended | Does the career club play its own play-off as a contest when its league ends (instead of the original's slot-4 rebuild, `career_teams.c:2052/2176`)? Gate: `World_LeaguePlayoffs` has the club's country (`World_FileCountry`), league type 0, 4..32 clubs, slot 4 free. | returns 0: original slot-4 English play-off rebuild (`DIY_RebuildCompetitionAndTeamSelectionCaches`) | built from `kWorld2627Playoffs` | returns 0 (empty table) | COMPONENT | `CareerPlayoffContest`: `original_slot4_playoffs` (96/97 `Playoffs` records, `competitions/playoffs.json`; `DIY_RebuildCompetitionAndTeamSelectionCaches`), `fixed_tier_contest` (kinds B_PLAYOUT/B_PLAYOFF/C_PLAYOFF), `spec_contest` (kinds GROUP_TIE/PLACEMENT/SPEC/GROUPS). Selected by `rules/league_playoffs.json` presence and its `fixed_tiers` field. |
| 1638-1646, 1663-1690 | league_ended | Fixed Italian brackets. `lp->italy`; Serie B = `lp->serieB` with >= 17 clubs; Serie C groups = `groupFirst .. groupFirst+groups`. Serie B play-off: places 3rd-8th, list `kPlayoffPlaces = {4,7,5,6,2,2,3,3}` (l.1640); play-out 16th v 17th (l.1681-1688); held per `LeaguePlayoffs_PlayoffHeld/OutHeld`. | skipped | Italy only (`italy=1, serieB=1, groupFirst=2, groups=3`, world_data.c l.4229) | skipped | COMPONENT (impl `fixed_tier_contest`) + DATA | `rules/league_playoffs.json` per country: `fixed_tiers: {second_division, group_first, groups}` from `kWorld2627Playoffs[Italy]` fields 5-8. The bracket shape (places, byes) stays code in the component, as in `sr_dplay.c`. |
| 1647-1662 | league_ended | Serie C career club: 2nd-10th, or the Coppa Italia Serie C winner placed 11th+ (via `WorldCups_LeagueCupWinner`, cups area) -> national play-off; else its group play-out. | skipped | Italy only | skipped | COMPONENT (impl `fixed_tier_contest`) | see c_playoff row. Needs the cups area's league-cup winner. |
| 1663-1668 | league_ended | Any other league of a country with a spec: group play-out tie, group placement, spec play-off, in that order. | skipped | spec from `kWorld2627_<country>Spec` | skipped | COMPONENT (impl `spec_contest`) | `rules/league_playoffs.json` `spec` (per country, play-offs in order). |
| 1692-1700 | league_ended | A spec contest's first week when the league ends after 96/97's play-off dates: compares with `_div1Playoffs.comp.beginMonth`. | skipped | uses 96/97's Division One play-off date as the reference | skipped | DATA | Reference date = 96/97's `div1Playoffs.begin_month` in `competitions/playoffs.json`. 96/97 fact used by a world. |
| 92-160 | make_record | The contest's cup record: header copied from 96/97's `_div1Playoffs` (dates, subs), compId 200, no away goals, round rule bytes per kind (Serie B 0x10/0x80/0x80; Serie C 0x00,0x00,0x80,0x80,0x94,0x94; spec: from spec round bytes), names. | skipped | as described | skipped | COMPONENT (part of `CareerPlayoffContest`) | Italian round bytes hardcoded (l.111-122); spec kinds read `spec` round legs/level (l.131-135). Template = 96/97 `div1Playoffs` record. |
| 49-53, 2030-2042 | (statics), WorldPlayoff_RoundName | Names "PLAY-OFF SERIE B", "PLAY-OUT SERIE B", "PLAY-OFF SERIE C", "PRELIMINARY ROUND", "" for a single round. | skipped | Italian names hardcoded | skipped | VIEW | Could move to `rules/league_playoffs.json` `names`. |
| 260-312 | group_tie | Group play-out the club is in: spec kind 4, first round; `gap` points (too far apart: no tie), host (better placed), level (LV_*: better / et+pens / et / pens) -> rule byte. Name from the level's name (`LeagueGroups_ForCountry`). | skipped | Italy's Serie C/D play-outs (spec) | skipped | DATA (+ impl `spec_contest`) | `rules/league_playoffs.json` `spec`; level names from `rules/league_groups.json` `levels[].name`. |
| 316-482, 484-553 | read_table, simulate_early, c_rng/c_below, sbetter, ssort, sshuffle, lay_out, c_next_round, cup_out, c_playoff | Serie C national play-off: needs exactly 3 groups (`ng != 3`), >= 11 clubs each; other 2 groups simulated now (`Career_AutoAdvanceInternationalCompetitionFixtures`); 28 entrants (group rounds 5v10,6v9,7v8 = `r1`, l.487); cup place via `WorldCups_PlayLeagueCupEarly/LeagueCupWinner` (l.513-515); play-out skip at 8+ points (`cup_out`, l.476-478); national rounds seeded draw with own LCG seeded by a hash of entrants (l.357-361, 424-425). | skipped | Italy only | skipped | COMPONENT (impl `fixed_tier_contest`) | Own LCG `1103515245/12345` (not Rand2). Hardcoded: 3 groups, 28 clubs, seeds 21-23 / 25-27, cup slot 24, 8-point gap. Mirror of `src/rules/sr_dplay.c`. |
| 342-354, 1534-1582, 1426-1438 | simulate_early, spec_playoff, groups_playoff | Other divisions the contest's entrants come from (<= 2, a group contest <= 9) are simulated at the career league's end, tables kept for the season end. | skipped | yes (France/Germany barrages, Serie C, group contests) | skipped | COMPONENT (part of `CareerPlayoffContest`) | Uses the game's match simulation earlier than 96/97 does (the season end simulates again then restores, `career_season.c:1720-1722`). RNG call order differs per world (unsure of exact effect). |
| 575-623 | group_placement | Group placement play-off (spec kind 3, 4 entrants, 2 rounds). | skipped | Serie D (spec) | skipped | DATA (+ impl `spec_contest`) | `rules/league_playoffs.json` `spec`. |
| 640-944 | k_spec, k_round, k_member, k_flags, k_games, k_members, k_build, k_waiting, k_lay_out, k_names, k_make_ex | Spec contest: members from codes, pairing best v worst, host, `gap`, best-of-three series, byes, bracket <= 32 places and <= 6 engine rounds (else in parts, `gend`), presim of round 1 via `LeaguePlayoffs_ContestSeed/ContestTie` (rules library RNG). Names from `names[pidx]`. | skipped | `kWorld2627_*Spec`, `*Names` | skipped | DATA (+ impl `spec_contest`) | `rules/league_playoffs.json` `spec`, `names` (long, short). Engine caps: 32 places, 6 rounds, `K_GAMES` 12. |
| 946-980 | k_done | Stage title result kept (`g_worldStageTitle`); `then[pidx]` builds the next play-off (Mexico's Campeon de Campeones). | skipped | `kWorld2627_*Then` | skipped | DATA | `rules/league_playoffs.json` `then` (per play-off, index+1 or 0). |
| 1299-1371 | WorldPlayoff_StageEnded, WorldPlayoff_Advance | A split stage's end builds a stage-title knockout mid-season (deferred to the calendar's advance, `career_teams.c:1877`); a contest in parts continues. | skipped | splits with `rec[22]` (league_split.c:426-430) | skipped | DATA | Trigger in `rules/league_splits.json` stage `after_stage_playoff`; spec kind 8 (stage) in `rules/league_playoffs.json`. |
| 1384-1506 | groups_record, groups_playoff | Group contest (Poule Scudetto, Serie D winners) played as 96/97's type-2 contest in slot 3 (`GROUPS_INDEX 9`), header copied from 96/97's `_euroCup`; shape from `LeaguePlayoffs_GroupShape`. | skipped | Italy (spec) | skipped | COMPONENT (impl `spec_contest`) | Template = 96/97 `euroCup` record (`competitions/tournaments.json`). Slot 3 must be free. |
| 1764-1778 | WorldPlayoff_LevelWinner | A level tie without penalties: better placed goes through (spec level 0/2/5, or Serie B rule). Called from `diy_competition.c:10851/10879`. | no-op (`ours()` false) | as spec | no-op | DATA (+ impl) | spec round `level`. |
| 1780-1832, 1900-2031, 2044-2054 | WorldPlayoff_TieDecided, WorldPlayoff_ArrangeNextRound, WorldPlayoff_RoundSize | Next round's ties from the contest's rules (not the draw); Serie B semis/final fixed (l.2001-2030); series skips; final's e.t.+pens when level on points. | no-op | as described | no-op | COMPONENT (part of `CareerPlayoffContest`) | Called from `diy_competition.c:2862, 2894, 10922`. |
| 1834-1853 | k_round_names, WorldPlayoff_RoundStringIndex | Round names from the spec's `round_names` (index into `_cupRoundStringTable`). | -1 (game's) | `kWorld2627_*Rounds` | -1 | VIEW | `rules/league_playoffs.json` `round_names` if ever ported. |
| 1096-1112 | WorldPlayoff_StageWins | Stage titles won this season, for the chairman's season-end opinion (`career_season.c:705-710`, -2 each). | 0 | from stage titles / live contest | 0 | DATA | Follows `rules/league_playoffs.json` spec kind 8 + `rules/league_splits.json` zones. Affects career evaluation. |
| 1114-1216, 1218-1240, 1242-1259 | WorldPlayoff_RecordLines, WorldPlayoff_TitleWon, WorldPlayoff_IsTitleLine | Season record lines (titles won, stage titles, combined line, Super Cups) written into the current season's record lines 2-4; the trophy counted as the league's. | nothing written (lp NULL; `SuperCup_RecordLine` NULL with no Super Cup table) | `records`, `names`, `combined` | nothing | SAVE (+ DATA strings) | Writes `SeasonInformations` lines 2-4 (l.1185-1210; saved career history). Career only (`_gameType != 4` returns, l.1123). Strings from `rules/league_playoffs.json` `records`, `names`, `combined` (32 masks). |
| 982-1028 | WorldPlayoff_WinnerOf, WorldPlayoff_ItalianResult | SEASON WINNERS page queries. | -1 | live | -1 | VIEW | world_winners.c. |
| 1268-1297, 1516-1531 | WorldPlayoff_StageTitle, WorldPlayoff_PlayedTie, WorldPlayoff_PlayedTitle | The club's played results handed to the rules library (host callbacks). | none | yes | none | COMPONENT (glue of `SeasonEndPlayoffs`) | league_playoffs.c:272-291. |
| 2056-2082 | WorldPlayoff_RestoreEarlyTable | Season end: a division simulated early gets its early table back. | no-op (`early` 0) | yes | no-op | COMPONENT (part of `CareerPlayoffContest`) | Called `career_season.c:1722`. |
| 45-48 (h), 1508-1514, 2084-2093 | g_worldPlayoff, WorldPlayoff_GroupsRecord, WorldPlayoff_RestoreSlot | Contest state saved with the career and rebuilt into slot 3/4 on load. | kind 0 | saved | kind 0 | SAVE | `save_format_json.c:8561-8562, 8610`. |
| 166-215, 1620-1622 | force_place (and its call in league_ended) | Test switch `SWOS_TEST_PLAYOFF_PLACE`. | — | — | — | OUT_OF_SCOPE | debugging aid. |

## include/world_playoff.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 42-43 | (enum) | Contest kinds NONE, B_PLAYOUT, B_PLAYOFF, GROUP_TIE, C_PLAYOFF, PLACEMENT, SPEC, GROUPS. | NONE only | all | NONE only | COMPONENT | Implementations of `CareerPlayoffContest` (fixed_tier: B_PLAYOUT/B_PLAYOFF/C_PLAYOFF; spec: the rest). |
| 51-92, 96-102 | WorldPlayoffState, WorldStageTitle | Saved state of the contest and the 3 stage titles. | zero | used | zero | SAVE | Career save fields `world_playoff`, stage titles. |

(`WORLD_PLAYOFF_MAX` 32, `WORLD_PLAYOFF_EARLY` 9, `WORLD_PLAYOFF_EARLY_ROWS` 24: engine caps, same for every world: NOT_A_BRANCH.)

## src/league_playoffs.c

Gate of the whole season-end module: `playoffs_of(country)` (l.191-199) = `World_LeaguePlayoffs`. 96/97: NULL, 8485: count 0 -> every entry point returns early.

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 191-199, 423-433 | playoffs_of, LeaguePlayoffs_Decide | Does the season end run the Mod's play-offs/play-outs for this league? Needs the table and every division played out (`s_in.nd == lp->divisions`). | returns: original moves only | yes (35 countries) | returns | COMPONENT | `SeasonEndPlayoffs`: `none` (original promoted/relegated by place, `Division` counts in `competitions/leagues.json`), `fixed_tier_brackets` (`sr_italy`, src/rules/sr_dplay.c), `spec_knockouts` (`sr_knockouts` + `sr_final_moves`, src/rules/sr_gplay.c, sr_post.c). Selected by `rules/league_playoffs.json`. Called from `career_season.c:884`. |
| 228-256 | league_of | The league's rules handed to the library: divisions, promoted[]/relegated[] by place, italy, serieB, groupFirst, groups, spec, B-team rules (seconds, second[], secondTop, secondFlags). | — | `kWorld2627Playoffs` | — | DATA | `rules/league_playoffs.json` per country: `divisions`, `promoted[]`, `relegated[]`, `fixed_tiers`, `spec`, `b_teams {list:[ordinal, first_team_ordinal], top_level, flags}`. From world_data.c l.4229-4262. |
| 231, 244-251 | league_of | Promedio (relegation by average) seed history: points/matches of the two seasons before, by ordinal. | none | Argentina 43: `kWorld2627Promedio43` (30) | none | DATA | `rules/promedio.json`: per country `[{ordinal, points2, matches2, points1, matches1}]`. Saved running state `g_promedio` (SAVE). |
| 258-269, 312-327 | levels_of, season_of | Levels of parallel groups (from `LeagueGroups_ForCountry`); B teams in knockouts on (`b_in_ko = 1`); stage bank; winners page. | — | yes | — | DATA | Levels from `rules/league_groups.json`. `b_in_ko` constant 1 for every world that has the module (NOT a world branch). |
| 329-337 | LeaguePlayoffs_OutHeld, LeaguePlayoffs_PlayoffHeld | Serie B play-out held unless 16th leads by > 4 pts; play-off unless 3rd leads 4th by > 14 (`sr_dplay.c:127-128`). | unused | Italy | unused | COMPONENT (impl `fixed_tier_brackets`) | Constants live in the rules library; could become `fixed_tiers.playout_gap` 4 / `playoff_gap` 14 (unsure if worth it). |
| 341-349 | LeaguePlayoffs_TopTable | Top division's final table for other confederations' cups / Super Cups (`conti_season.c:99`, `world_supercup.c:102`). | table filled (Begin/KeepTable run in every world); unsure if callers run in 96/97 | yes | yes (unsure) | NOT_A_BRANCH | Callers are in the cups area. |
| 352-415 | LeaguePlayoffs_SpecAt, _ContestEntrants(Ex), _GroupShape, _ClubPlayoff, _ContestSeed, _ContestTie | Library entry points for the career club's contest. | NULL / -1 | spec | NULL / -1 | COMPONENT (glue) | `sr_spec_at`, `sr_contest_entrants`, `sr_group_shape_of`, `sr_club_playoff`, `sr_contest_seed`, `sr_contest_tie` (own LCG, not Rand2). |
| 434-439 | LeaguePlayoffs_Decide | Every club's final place kept for next season's cups (`WorldCups_KeepPlace`, `CupD_KeepPlace`). | skipped | yes | skipped | DATA (cups area) | Owned by the cups inventory. Note: in 8485 this does not run although 8485 has cup clubs (`World_CupClubs`); unsure how 8485 cups learn places. |
| 443-456 | LeaguePlayoffs_Decide | `lp->italy`: the Coppa Italia Serie C winner and the club's played Serie B/C results passed to `sr_italy`; a played group tie passed always. | skipped | Italy | skipped | COMPONENT (impl `fixed_tier_brackets`) | `fixed_tiers` presence. |
| 457-463 | LeaguePlayoffs_Decide | Rules library call `sr_italy` then `sr_knockouts`. | skipped | yes | skipped | COMPONENT | `SeasonEndPlayoffs` (see above). |
| 464-535 | LeaguePlayoffs_Decide | A title play-off's champion put first in the top-9 qualified grid (`_primaryQualifiedTeamGrids`, European places); a lower tier's title (no top-division entrant) gives no European place. | skipped | yes | skipped | COMPONENT (impl `spec_knockouts`) | Depends on 96/97's qualified grid of 9 (`DIY_CachePrimaryQualifiedTeamsGrid`). |
| 537-577 | LeaguePlayoffs_Decide | Each club's move: rules' move written into the scratch table; out of the lowest division -> pool swap (`TeamPool_Swap`, `LeagueGroups_Enter`, `CupD_NoteArrival`). | skipped | yes | skipped | COMPONENT (+ team pool area) | `sr_final_moves`; pools from the team-pool inventory. |
| 578-581 | LeaguePlayoffs_Decide | Promoted / relegated animation flags from the rules' move. | original flags | overridden | original | VIEW | — |
| 79-182 | LeaguePlayoffs_ResetStages, bank_of, LeaguePlayoffs_StageTable, LeaguePlayoffs_KeepStage | Stage tables (Apertura/Clausura/annual) and 'keep' tables kept through the season (called from league_split.c only). | never filled | yes | never filled | SAVE (+ DATA trigger) | `g_leagueStages` (24 banks) saved. Triggered by `rules/league_splits.json` stage `table`, `annual`, `keep`. |
| 43-75 | LeaguePlayoffs_Begin, LeaguePlayoffs_KeepTable | Final tables captured at the season end (`career_season.c:1706, 1728`). | runs (harmless) | runs | runs | NOT_A_BRANCH | — |

## include/league_playoffs.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 33-56 | LeaguePlayoffs (struct) | The per-country schema of the season-end rules. | absent | 35 entries | empty | DATA | Becomes `rules/league_playoffs.json` (fields listed above, plus `names`, `then`, `round_names`, `records`, `combined`, `winners`). |
| 24-31, 118-125 | LeagueWinLine, g_leagueWinners | SEASON WINNERS page lines; last season's winners saved. | none | `kWorld2627_*Winners` | none | VIEW (+ SAVE for `g_leagueWinners`) | `rules/league_playoffs.json` `winners` if ever ported. |
| 69-89 | LEAGUE_STAGE_TABLES, LEAGUE_STAGE_BANKS 24, g_leagueStages | Capacity of stage banks. | — | — | — | LIMIT (engine-wide) | Same in every world; `rules/limits.json` only if made per world (unsure). |
| 131-136 | LeaguePlayoffs_OutHeld / PlayoffHeld / TestHeld | See .c. `TestHeld` is a test switch. | — | — | — | OUT_OF_SCOPE (TestHeld) | — |

## src/league_split.c

Gate: `World_LeagueSplits()` (l.72, 115). 96/97: NULL; 8485: count 0; 2627: 38 tables.

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 110-125 | LeagueSplit_Init (before) | A division's own number of meetings (`playEachTeam = table[4]`), matched by country, division, clubs. Called `diy_competition.c:9425`. | league record's `rounds` | split table byte 4 (e.g. Scotland's 4) | record's `rounds` | DATA | `rules/league_splits.json` per (country, division, clubs): `meetings`. Overrides 96/97's `competitions/leagues.json` `rounds`. |
| 126-145 | LeagueSplit_Init (after) | Split on: days before/after, days' own weeks, state in the league (`LeagueSplitState`, magic). | no split | `kWorld2627Splits` | no split | DATA (+ SAVE) | `rules/league_splits.json`: `days_before`, `days_after`, `stages[]`, `weeks[]`, pairs list, host matrix. State lives in the league's saved tail. |
| 67-77, 150-222, 315-384 | active_of, slot_of, rows, calendar, LeagueSplit_Pair, LeagueSplit_Count | After the split, pairing and day counting from the split table; the career calendar rewritten. | engine's | split | engine's | COMPONENT | `LeagueFixturePairing`: `schedule_tables` (original `_fixtureSchedulePtrTable`), `split_days` (league_split.c), `league_phase_draw`, `fixed_calendar` (states.c). Called `diy_competition.c:1901, 2060`. |
| 241-306 | freeze | The split: table frozen into groups; points kept / halved up / halved down / from 0 / half of head-to-head (rules 0-4); flags reset, keep (Malta), perm, groups by membership (Panama), wins tie-break. | — | split table | — | DATA | `rules/league_splits.json` stage: `groups[{first, size, points_rule}]`, `flags {reset, keep, rows, members, wins, perm}`, `members[]`, `perm[]`. |
| 386-438 | LeagueSplit_AfterResult | Head-to-head points before the split (`g_leagueSplitH2H`, saved); stage table kept (`rec[23]`, `rec[24]` annual) -> `LeaguePlayoffs_StageTable`; stage-title knockout (`rec[22]`) -> `WorldPlayoff_StageEnded`. Called `diy_competition.c:11976`. | no-op | yes | no-op | DATA (+ SAVE for H2H) | `rules/league_splits.json` stage: `table`, `annual`, `after_stage_playoff`. |
| 440-471 | LeagueSplit_Sort | Table order after the split: by group, then points before rounding (or wins, MLS). Called `diy_competition.c:683`. | original order | split | original | COMPONENT | `LeagueTableOrder`: `points_gd_goals_name` (original `DIY_SortLeagueTableByStandings`), `split_groups` (league_split.c), `uefa_league_phase` (league_phase.c). |
| 476-512 | LeagueSplit_Pages, LeagueSplit_RestOfDayWithoutCareerClub | Table pages for > 22 clubs (VIEW); the rest of a day simulated when the career club rests after the split (`career_teams.c:2343`). | 0 | yes | 0 | VIEW (Pages) / COMPONENT (RestOfDay, part of `split_days`) | — |
| 514-605 | zones_of, LeagueSplit_ZoneFind, _ZoneOfTeam, _ZoneScale, _ZoneRecord, _InZones | League in zones by membership: the career club judged on its place and its more expensive clubs in its zone, scaled to the league (FindTeamInDIY, GetMoreExpensiveTeamsCount, season record, season-end opinion). | original place | Argentina, Panama, MLS (flag F_MEMBERS) | original | COMPONENT | `CareerLeaguePosition`: `league_place` (original), `zone_place_scaled` (league_split.c). Selected by the split stage's `members` flag. Callers: `diy_competition.c:10117, 10399-10419`, `career_season.c:705`, `statistics.c:1838`. |

## include/league_split.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 25-33 | LeagueSplit (struct), World_LeagueSplits | Per (country, division) split table = the Mod's SP_TAB bytes. | none | 38 | empty | DATA | `rules/league_splits.json` (structured fields rather than raw bytes, unsure). |
| 81-83 | LEAGUE_SPLIT_H2H 64, g_leagueSplitH2H | Head-to-head points of the career league (saved). | unused | used | unused | SAVE | Cap 64 = 8x8 clubs (head-to-head groups only in small leagues). |

## src/league_groups.c

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 108-116 | LeagueGroups_ForCountry | Does a country have levels of parallel groups? Used by world_playoff.c, league_playoffs.c. | NULL | Italy, Argentina, Australia, El Salvador | NULL (count 0) | DATA | `rules/league_groups.json` per country: `levels[{first, groups, sizes[], name}]`, `clubs[{slot, base_division, area[2]}]`. From `kWorld2627Groups` (world_data.c l.2348-2353). |
| 68-106, 118-125 | apply_one, LeagueGroups_Apply | Season end: one-division moves turned into level moves, each club into its area's group (`sr_pg_season`, src/rules/sr_groups.c); pool slots enter from below (`TeamPool_Area`). Called `career_init.c:1198`. | nothing (no table) | yes | nothing | COMPONENT | `LevelPlacement`: `division_chain` (original moves), `parallel_groups` (`sr_pg_season`). Selected by `rules/league_groups.json`. |

## include/league_groups.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 16-18 | LEAGUE_GROUPS_MAX_LEVELS 8, LEAGUE_GROUPS_MAX_GROUPS 9 | Capacity: levels per league, groups per level ("the World 2026-27's Serie D"). | — | sized for it | — | LIMIT | `rules/limits.json`: `league_levels_max` 8, `groups_per_level_max` 9 (2627's values; 96/97 would be 1 group per level). |
| 52-54 | World_CupClubs | A classic world's league clubs by division, for its cups from season 2 (used in world_cups.c). | NULL | NULL (empty) | `kWorld8485CupClubs` | DATA (cups area) | `rules/cup_clubs.json` (unsure name; the cups inventory owns it). |

## src/league_phase.c

Gate: `s_installed` (set by `LeaguePhase_InstallCups`, called from World_InstallBaked: 2627 `world_data.c:1869`; 96/97 only through a mod: world_baked.c:119). 8485 has none (its European cups are knockouts: `WorldEurope_Install`, other area).

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 486-494, 497-501 | ours_id, LeaguePhase_IsCup, LeaguePhase_StageDays, ours | Are the European cups (compId 0..2) played as a 36-club league phase? `SWOS_WORLDS && s_installed && compId <= CONFERENCE(2)`. 8 matchdays, Conference 6. | no (original cups) | yes | no (knockouts, world_europe.c) | COMPONENT | `EuropeanCupFormat`: `original_cups` (96/97 euroCup / CWC / UEFA records), `league_phase_36` (league_phase.c), `classic_knockouts` (world_europe.c, 8485). Selected by `rules/europe.json` (see InstallCups row). Called `diy_competition.c:941, 6070`. |
| 516-555 | LeaguePhase_StageStart | Pots from 16 players' prices + club coefficients (`UefaCoef_PotKey`), country cap, the draw with its own LCG seeded from compId, `_seasonPlaying`, teams. `World_FileCountry` for a second file's country (l.546). | — | yes | — | COMPONENT (impl `league_phase_36`) | Own LCG (l.20-25). Called `diy_competition.c:4555`. |
| 557-568 | LeaguePhase_Pair | Matches from the draw. | engine | draw | engine | COMPONENT | `LeagueFixturePairing` impl `league_phase_draw`. Called `diy_competition.c:1903`. |
| 581-626, 628-657 | cards, discipline, LeaguePhase_Result | Away goals/wins; disciplinary points: a simulated match draws cards with **Rand2()** (`cards`, l.596; 4 calls per simulated match, l.622-625); coefficients, prize money. Called `diy_competition.c:11977`. | — | yes | — | COMPONENT (impl `league_phase_36`) | Extra Rand2 calls per simulated league-phase match: RNG order differs from 96/97. Tables `kYellows`, `kReds` (l.583-584). |
| 659-697 | rank_rows, LeaguePhase_Sort | One table of 36, UEFA 2025-26 art. 18 criteria. Called `diy_competition.c:621`. | original sort | yes | original | COMPONENT | `LeagueTableOrder` impl `uefa_league_phase`. |
| 702-748 | LeaguePhase_GroupsEnd | Play-off ties and R16 seeds (`kPlayoffTies`, `kR16Seeds`, l.477-481), then 8 **Rand2()** coin flips (l.735, 740, in a loop of 4). Called `diy_competition.c:2231, 2234`. | — | yes | — | COMPONENT (impl `league_phase_36`) | RNG order differs. |
| 750-786 | LeaguePhase_KnockoutDraw | Knockouts after the phase keep their order; skips the standard shuffle (which draws Rand2, `diy_competition.c:~4375`). Called `diy_competition.c:4342`. | — | yes | — | COMPONENT (impl `league_phase_36`) | — |
| 789-801 | LeaguePhase_Showing, _PageHeader, _RowColour | Table screen of 36. | 0 | yes | 0 | VIEW | — |
| 801-822 | LeaguePhase_InstallCups | Career contest table entries 0..2 and `_europeTable.compCup` replaced by the world's league-phase records. | not called (96/97 records stay) | `kWorld2627LeaguePhases` | not called | DATA | `competitions/career.json` entries 0..2 point at the world's league-phase records (`competitions/tournaments.json` or new `rules/europe.json` `cups[3]`), plus `rules/europe.json` `places[{country, cl, el, uecl}]` (`kWorld2627EuropePlaces`, 54). |
| 834-851 | LeaguePhase_SetModEurope, LeaguePhase_ModCups, LeaguePhase_ModSeeds | `World_Current() != WORLD_9697` -> no mod cups; a mod's league phases only in 96/97. | mod's cups if a mod set them | world's own | none | OUT_OF_SCOPE | mod system (ModApi_SetLeaguePhases). |

## include/league_phase.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 18-19 | LEAGUE_PHASE_CLUBS 36, LEAGUE_PHASE_MATCHES 144 | Format constants of the league phase. | — | — | — | NOT_A_BRANCH | Part of the component. |
| 62-81 | World_LeaguePhaseCups, LeaguePhase_InstallCups, LeaguePhase_SetModEurope | See .c. | — | — | — | DATA / OUT_OF_SCOPE | — |

## src/league_zones.c

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 61-70 | load | Zone colours file per world: `data_json/zones/<World_Tag()>.json`. | not loaded | its file | its file (if present) | VIEW | Would be a package file (e.g. `view/zones.json`); not ported now. |
| 157-158 | LeagueZones_Begin | `_gameType != 4 \|\| World_Current() == WORLD_9697` -> no zone colours. | off | on | on | VIEW | — |
| 177-180 | LeagueZones_Begin | European places by UEFA association rank (`UefaCoef_AssociationRank`, `UefaAccess_PlaceColours`). | — | yes | (unsure) | VIEW | — |

## src/world_rules.c

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 39-42 | WorldRules_World | `SWOS_WORLDS && World_Current() != WORLD_9697`: Mod 4.1 match rules exist. | off | on | on | DATA | The world package lists the rules it offers. 96/97: none (the original game). |
| 34-37, 49-67 | kKeys, slug, WorldRules_On | Each rule's value: mod setting of mod `world<tag>`, default ON. | 0 always | setting, default 1 | setting, default 1 | DATA | `mod.json` `settings`: `stoppage_time`, `cpu_red_cards`, `cpu_injuries`, `goalkeeper_injuries`, `bench_info`, `opponent_squad` (default on). 96/97's package: none of them (or `rules/match_rules.json` `available: []`). |
| 71-93 | WorldRules_CareerBits, WorldRules_CareerLoaded | 3 career rules saved with a career, put back on load (an older save: ON). | not saved/applied | saved | saved | SAVE | `save_format_json.c:7719, 8795` "world_rules". |
| 103-131 | WorldRules_CpuOpen, _CpuAny, _CpuCountdown, _CpuDay | Computer clubs' cards/injuries recorded and counted down (original never does). Callers: `game_init.c:371/552`, `match_sim.c:426-427`, `career_status.c:327/363`, `career_teams.c:1869`, `tackle_foul.c:322/421`. | guards closed (original) | per setting | per setting | DATA (selects behaviour) | `mod.json` settings `cpu_red_cards`, `cpu_injuries`. The guard is a flag, not a different algorithm. |
| 133-203 | unavailable, swap_unavailable, WorldRules_CpuLineupsBegin/End | Computer club's injured/suspended starters swapped with fit subs for the match. | off | when CPU rules on | when on | DATA (selects behaviour) | as above. |
| 205-251, 253-274 | gk_rng, gk_apply, WorldRules_GkAfterMatch, WorldRules_GkAfterSim | Keeper injury 1 in 25 (`GK_ODDS`), 1-2 matches; own xorshift32 seeded from the game RNG state (read, not drawn); seed saved (`g_gkInjurySeed`). | off | on (setting) | on (setting) | DATA (+ SAVE seed) | `mod.json` `goalkeeper_injuries`. Does not touch Rand order. |

## include/world_rules.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 1-19, 26-34 | (doc), WRULE_* | Rule list and defaults; "In 96/97 ... every one is off". | off | on | on | DATA | as world_rules.c. |

## src/states.c

Whole file under `#if SWOS_WORLDS` (l.25 / 381-396 stubs). Gate inside: `World_States(country)` (2627: Brazil 46 only; 8485, 96/97: NULL).

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 25, 381-396 | (file), stubs | State championships compiled only with worlds. | stubs | real | real (no data) | NOT_A_BRANCH (build switch) | — |
| 81-84, 139-156 | career_states, States_SlotContest, States_RestoreContest | Career club in a state championship: its contest in slot 2 (Brazil's empty league-cup slot). `World_States(World_FileCountry(club))`. Callers `career_init.c:1437, 1820`. | none | Brazil's clubs | none | DATA | `rules/states.json` per country: `states_file`, `world`, `first_state`, `contests[]` (name, short, clubs, begin/end month, win points, meetings, stages, stage bytes, teams, calendar, draws, key). From `kWorld2627States46`. |
| 100-136 | make_contest | The state contest as 96/97's `_euroCup` header with the contest's stages. | — | yes | — | DATA | Template = 96/97 `euroCup` record. |
| 171-216 | States_StageDays, States_StageStart, States_Pair | First stage on a fixed calendar. Callers `diy_competition.c:945, 4556, 1905`. | 0 | yes | 0 | COMPONENT (impl `fixed_calendar` of `LeagueFixturePairing`) + DATA | `rules/states.json` contest `calendar`. |
| 221-329 | States_GroupsEnd, rank0, States_KnockoutStart, States_KnockoutEnd | Merit order, fixed knockout draws (labels, pairs, merit; host by stage-1 place). Callers `diy_competition.c:2236, 2238, 4594`. | no-op | yes | no-op | COMPONENT (knockout draw) + DATA | `rules/states.json` contest `draws`. |
| 331-379 | States_SeasonEnd | Every state's season simulated (`ss_season`), pools' swaps (`ri_season`, `TeamPool_Assign`). Season word seed `2026 + g_stateSeasons` (l.355). Called `career_season.c:887`. | no-op | Brazil | no-op | DATA | `rules/states.json`: `sim` table, `pools` table, `words_at[]`; `first_season_year` 2026 (hardcoded at l.355; should come from the world). |
| 31-32, 72-73 | g_stateChampions, g_stateSeasons, g_stateCup, g_stateRank0 | Saved state. | zero | saved | zero | SAVE | — |

## src/state_sim.c

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 16, 309-314 | (file) | Compiled only with worlds. | stub | real | real | NOT_A_BRANCH (build switch) | — |
| 189-200 | ss_season | Simulation of every state (own LCG seeded from the season word and the played ranking; strengths = prices). Runs only when `World_States` has the country. | not run | Brazil | not run | DATA (consumer of `rules/states.json`) | One algorithm; on when the data exists. Own LCG, no Rand2. |

## include/state_sim.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 10-12 | STATE_SIM_STATES 13, STATE_SIM_CLUBS 16, STATE_SIM_RANK | Capacity: states per country, clubs per state. | — | Brazil's | — | LIMIT | `rules/limits.json` `states_max` 13, `clubs_per_state_max` 16 (2627). |

## src/state_pool.c

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 23, 203 | (file) | Compiled only with worlds (no `#else`: nothing defined on a `SWOS_WORLDS=0` build; states.c's stubs never call it). | (not compiled when worlds are off) | real | real | NOT_A_BRANCH (build switch) | — |
| 35, 171-200 | RI_RULE, ri_season | State pools: relegated state-only clubs swap identities with queued pool clubs; `RI_RULE = 2` (draw weighted by weakness); seed from the season's league table (first 2000 bytes); a file entry runs only when its `world` byte equals `ws->world` (l.184). | not run | Brazil | not run | DATA | `rules/states.json` `pools` (with its `world` byte), `relegation_rule` 2. Saved `RI_STATE` (SAVE). |

## include/state_pool.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 30-34 | STATE_POOL_FILES 8, _POOLS 20, _QUEUE 192, _ENTRIES 16, _MAP 640 | Capacities of the state pools. | — | — | — | LIMIT | `rules/limits.json` (2627 values). |
| 38-48 | StatePoolState RI_STATE | Saved career state. | zero | saved | zero | SAVE | — |

## include/fixture_rounds.h

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 1-12, 71-75 | FixtureRounds_Install, FixtureRounds_Real | Odd-sized leagues (5..23): 96/97's pair tables are no rounds and a career league refuses them (panic 0x1313); a world installs real rounds (one club rests a day). Installed by `world_data.c:1870` (2627), `world_data_8485.c:626` (8485), `teamdb.c:332` (a team database). | original odd tables; odd career league panics | rounds | rounds | COMPONENT | `OddRoundRobinSchedule`: `original_pair_tables` (`_fixtureSchedulePtrTable`), `rounds_with_rest` (fixture_rounds.c `rounds()`, `FixtureRounds_Install`). Selected by `rules/fixtures.json` `odd_round_robin: "original" \| "rounds"`. |

## src/diy_competition.c (world references only)

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 383-405 | (league shuffle, test block) | `SWOS_TEST_REST_LAST` test with `FixtureRounds_Real`. | — | — | — | OUT_OF_SCOPE | debugging aid. |
| 621-622 | DIY_SortLeagueTableByStandings | League phase replaces the whole sort. | original | league phase cups | original | COMPONENT | `LeagueTableOrder`. |
| 683 | DIY_SortLeagueTableByStandings | Split groups re-order after the sort. | no-op | split leagues | no-op | COMPONENT | `LeagueTableOrder`. |
| 939-948 | DIYTournament_RecalculateMatchesPerTeam | A first stage's matchdays: league phase 8/6, state calendar days. | original count | yes | original | COMPONENT | `EuropeanCupFormat` / `rules/states.json` calendar. |
| 1002-1006 | GetNextCupRoundString | Round name from the world play-off, then the world cups (`WorldCups_RoundStringIndex`, cups area). | original | world's | world cups only (unsure) | VIEW | — |
| 1468-1478 | DIY_ValidateLeagueScratchBuffer | `#if SWOS_WORLDS`: a country over two team files checksums its whole global range (`World_SecondFile`). | one file (`World_SecondFile` 0) | per country | per country | LIMIT | `rules/limits.json` `clubs_per_team_file`; second file per country in `competitions/countries.json` (owned by the world/teams inventory). `WORLD_SLOTS` 512 = two files. |
| 1901-1906 | getCurrentFixtureTeamPtrs | Pairing source: split days, league-phase draw, state calendar, else schedule tables. | schedule tables | all | schedule tables | COMPONENT | `LeagueFixturePairing`. |
| 2060-2061 | DIY_AdvanceLeagueProgressAfterMatch | Split day counting. | engine | split | engine | COMPONENT | impl `split_days`. |
| 2231-2238 | DIY_SyncCompetitionTeamGridOrBuildBracket | Group-stage end hooks: league phase play-off ties, other confederations (`Conti_GroupsEnd`, cups area), state merit order / knockout winners. | original bracket | yes | (Conti: unsure) | COMPONENT | `EuropeanCupFormat`; states via `rules/states.json`. |
| 2861-2865 | DIY_UpdateCompetitionProgress | Next round's ties overridden: world play-off, world cups' first-season bracket, Coppa D, classic Europe's no same-country round 2. | original draw | play-off, cups, Coppa D | `WorldEurope_RoundPlayed` | COMPONENT | `CareerPlayoffContest` here; the rest belongs to the cups inventory (`CupDraw` components). |
| 2894 | DIY_UpdateCompetitionProgress | Only the play-off round's real ties counted. | no-op | yes | no-op | COMPONENT | part of `CareerPlayoffContest`. |
| 3314-3316 | DIY_RecordLeagueFixtureResultAndAdvanceProgress | No split during the calendar's walk. | no-op | yes | no-op | COMPONENT | impl `split_days`. |
| 3467-3485 | recordLeagueFixtures | (H)/(A) marker in the calendar entry for league-phase matches. | original | league phase | original | VIEW (unsure: the byte sits in the saved calendar cache) | — |
| 4341-4343 | DIY_ShuffleTeamsAndBuildCompTeamList | League phase knockouts skip the standard shuffle (its Rand2 calls). | shuffle | league phase | shuffle | COMPONENT | `EuropeanCupFormat` impl `league_phase_36`. RNG order differs. |
| 4555-4556, 4594 | DIY_LoadCurrentCompetitionSlot | Stage start hooks: league phase draw, state calendar, state knockout draw. | no-op | yes | no-op | COMPONENT | as above. |
| 6070-6072 | DesignDIYTournamentOnDraw | "8 OPPONENTS, ONCE" / "6 OPPONENTS, ONCE" text. | none | yes | none | VIEW | — |
| 7430-7440 | table_groups, table_per_group | Split league shown as zone pages. | original | yes | original | VIEW | — |
| 7643-7645, 7684-7700, 7830-7842 | DIYLeagueTournament16BeforeDraw | League phase headers/colours; zone colours and legend. | original | yes | zones only | VIEW | — |
| 9238-9240 | continental_record | A world's continental contests (Libertadores, UEFA ties, conti ties, UEFA Super Cup) count as European for suspensions. | 96/97 list only | yes | (unsure) | DATA (cups area) | A `continental` flag on competition records (owned by the cups inventory). |
| 9328-9335 | InitDIYCompetitionFromTableEntry | Odd career league panics 0x1313 unless rounds are installed. | panic on odd (original) | allowed | allowed | COMPONENT | `OddRoundRobinSchedule`. |
| 9424-9427 | InitDIYCompetitionFromTableEntry | Division's own meetings and split (`LeagueSplit_Init` before/after). | no-op | yes | no-op | DATA | `rules/league_splits.json`. |
| 9513-9514 | InitDIYCupFromRecord | World cups' byes/draw, classic Europe's no same-country draw. | no-op | `WorldCups_CupMade` | `WorldEurope_DrawMade` | COMPONENT (cups area) | owned by the cups inventory. |
| 9669-9677 | InitDIYLeagueFromRecord | `World_Current() == WORLD_9697`: a tournament's odd-sized league groups panic 0x1313; other worlds allow them (Poule Scudetto's groups of 3). | panic (original) | allowed | allowed | DATA | `rules/fixtures.json` `odd_tournament_groups: false/true`. Pairs with `OddRoundRobinSchedule`. |
| 10113-10118 | FindTeamInDIY | Career league's place = place in the club's zone, scaled. | league place | zones | league place | COMPONENT | `CareerLeaguePosition`. |
| 10394-10419 | GetMoreExpensiveTeamsCount | Count of more expensive clubs within the zone, scaled. | whole league | zones | whole league | COMPONENT | `CareerLeaguePosition`. |
| 10849-10856, 10877-10884 | ProcessDIYKnockoutMatchResultAndCareerUpdates | Level tie without penalties: the play-off's rule decides (else a draw/replay). | original | play-off | original | COMPONENT | `CareerPlayoffContest`. |
| 10918-10940 | ProcessDIYKnockoutMatchResultAndCareerUpdates | Tie decided hooks: world play-off, world cups, Coppa D, Super Cup, UEFA qualifying, conti. | no-ops | yes | some (unsure) | COMPONENT | play-off here; the rest owned by the cups inventory. |
| 11738 | DIY_PlayCupMatchAndAdvance | UEFA prize money on a cup won. | off | yes | (unsure) | DATA (finance area) | owned by the finance inventory. |
| 11976-11977 | DIY_PlayOrSimulateLeagueFixture | Split head-to-head / stage end; league phase away figures and discipline (Rand2 on simulated matches). | no-op | yes | no-op | COMPONENT | `split_days`, `league_phase_36`. |

## Summary

### Rows per class
(126 rows; a row with two classes is counted by its first)
- COMPONENT: 51
- DATA: 37
- VIEW: 15
- SAVE: 8 (plus about 10 rows whose second class is SAVE: saved state of splits, stages, promedio, states, rules bits)
- LIMIT: 5
- OUT_OF_SCOPE: 4
- NOT_A_BRANCH: 6 rows kept for context (the `#if SWOS_WORLDS` build switches of states.c / state_sim.c / state_pool.c, `LeaguePlayoffs_Begin/KeepTable`, `TopTable`, league-phase format constants). Not rowed: about 12 distinct `WORLD_*` names that are no world check — `WORLD_PLAYOFF_{NONE,B_PLAYOUT,B_PLAYOFF,GROUP_TIE,C_PLAYOFF,PLACEMENT,SPEC,GROUPS}`, `WORLD_PLAYOFF_{MAX,EARLY,EARLY_ROWS}`, `WORLD_SLOTS` — and 5 doc mentions of `WORLD_2026_27`.

### Proposed rules/ files
- `rules/league_playoffs.json` — per country: `divisions`, `promoted[]` / `relegated[]` by place, `fixed_tiers {second_division, group_first, groups}` (Italy), `spec` (play-offs in order: kind, entrants, rounds with legs/host/level/pairing/gap, member codes), `names[][2]`, `then[]`, `round_names[][12]`, `records[]`, `combined[32]`, `winners[]`, `b_teams {list, top_level, flags}`. From `kWorld2627Playoffs` (world_data.c l.4229-4262). 8485: empty. 96/97: absent.
- `rules/promedio.json` — per country `[{ordinal, points2, matches2, points1, matches1}]`. 2627: `kWorld2627Promedio43`.
- `rules/league_splits.json` — per (country, division, clubs): `meetings`, `days_before`, `days_after`, `stages[{first_day, groups[{first, size, points_rule}], flags, members[], perm[], table, annual, after_stage_playoff, weeks[]}]`, pairs and host matrix. 2627: `kWorld2627Splits` (38).
- `rules/league_groups.json` — per country `levels[{first, groups, sizes[], name}]`, `clubs[{slot, base_division, area[2]}]`. 2627: `kWorld2627Groups` (4).
- `rules/states.json` — per country: states file, world byte, first state, `sim`, `pools`, `words_at`, `contests[]` (calendar, draws), `relegation_rule` 2, `first_season_year` 2026. 2627: `kWorld2627States46`.
- `rules/europe.json` — European cups as league phases: the 3 records (or references into `competitions/tournaments.json`), `places[{country, cl, el, uecl}]`, which career contests 0..2 they replace. 2627: `kWorld2627LeaguePhases`, `kWorld2627EuropePlaces` (54). (8485's knockout Europe belongs to the cups inventory; merge there.)
- `rules/fixtures.json` — `odd_round_robin: "original" | "rounds"` (96/97 original; 2627, 8485 rounds), `odd_tournament_groups: bool` (96/97 false).
- `rules/match_rules.json` (or only `mod.json` `settings`) — which Mod match rules the world offers, with defaults: `stoppage_time`, `cpu_red_cards`, `cpu_injuries`, `goalkeeper_injuries`, `bench_info`, `opponent_squad` (all on); 96/97 offers none.
- `rules/limits.json` entries from this area: `league_levels_max` 8, `groups_per_level_max` 9, `states_max` 13, `clubs_per_state_max` 16, state-pool sizes (8/20/192/16/640), `clubs_per_team_file` (a country over two files: `WORLD_SLOTS` 512). These are engine caps today, sized for 2627.
- `rules/cup_clubs.json` — `World_CupClubs` (8485 only). Owned by the cups inventory.

### Proposed components
- `SeasonEndPlayoffs`: `none` (original moves by place); `fixed_tier_brackets` (`sr_italy`, src/rules/sr_dplay.c; `LeaguePlayoffs_Decide` l.443-457); `spec_knockouts` (`sr_knockouts`, `sr_final_moves`; `LeaguePlayoffs_Decide` l.457-577).
- `CareerPlayoffContest`: `original_slot4_playoffs` (`DIY_RebuildCompetitionAndTeamSelectionCaches`); `fixed_tier_contest` (world_playoff.c `league_ended`, `c_playoff`, `c_next_round`, `ArrangeNextRound` Serie B part); `spec_contest` (`group_tie`, `group_placement`, `k_make_ex`, `k_build`, `groups_playoff`, `WorldPlayoff_Advance`).
- `LevelPlacement`: `division_chain` (original); `parallel_groups` (`LeagueGroups_Apply` -> `sr_pg_season`).
- `LeagueFixturePairing`: `schedule_tables` (original); `split_days` (`LeagueSplit_Pair`, `LeagueSplit_Count`); `league_phase_draw` (`LeaguePhase_StageStart`, `LeaguePhase_Pair`); `fixed_calendar` (`States_StageStart`, `States_Pair`).
- `LeagueTableOrder`: `points_gd_goals_name` (`DIY_SortLeagueTableByStandings`); `split_groups` (`LeagueSplit_Sort`); `uefa_league_phase` (`LeaguePhase_Sort`, `LeaguePhase_Rank`).
- `OddRoundRobinSchedule`: `original_pair_tables` (96/97, odd career league panics); `rounds_with_rest` (`FixtureRounds_Install`).
- `EuropeanCupFormat`: `original_cups`; `league_phase_36` (league_phase.c: pots, draw, discipline, `GroupsEnd`, `KnockoutDraw`); `classic_knockouts` (world_europe.c, 8485; cups inventory).
- `CareerLeaguePosition`: `league_place` (original `FindTeamInDIY`, `GetMoreExpensiveTeamsCount`); `zone_place_scaled` (`LeagueSplit_ZoneFind`, `_ZoneOfTeam`, `_ZoneScale`, `_ZoneRecord`, `_InZones`, plus `WorldPlayoff_StageWins` in the season-end opinion).

### 96/97 hardcoded facts a world overrides
- `diy_competition.c:9672-9677` — odd tournament groups panic 0x1313, only in 96/97.
- `diy_competition.c:9332-9335` + `_fixtureSchedulePtrTable` — 96/97's odd pair tables are no rounds; replaced by `FixtureRounds_Install` (world_data.c:1870, world_data_8485.c:626).
- `world_playoff.c:98, 107, 1697` — 96/97's `_div1Playoffs` record is the template (dates, subs) and the date reference of every world play-off.
- `world_playoff.c:1393` and `states.c:104` — 96/97's `_euroCup` header is the template of group contests and state contests; slot 3 index 9 (`GROUPS_INDEX`, l.1382).
- `league_phase.c:477, 488, 803-820` — European cups are compIds 0..2 (96/97's Champions Cup, CWC, UEFA Cup); `_careerContests[0..2]` and `_europeTable.compCup` are replaced.
- `league_playoffs.c:504-527` — 96/97's qualified grid of 9 (`_primaryQualifiedTeamGrids`) gets a title play-off's champion first.
- `league_split.c:122-123` — the league record's `rounds` (meetings) is overridden per division.
- `diy_competition.c:615-683` — 96/97's table order is overridden (league phase, split).
- `world_playoff.c:1185-1210` — season record lines 2-4 are overwritten with world titles.
- `league_playoffs.c:578-581` — promoted/relegated animation flags overridden.
- `world_playoff.c:46` — compId 200 picked because 96/97's ids run to 161.

### Surprising or uncertain
- RNG order: the league phase calls `Rand2()` for simulated matches' cards (league_phase.c:596, 4 calls a match from l.622-625) and 8 times at the group end (l.735, 740), and skips the standard shuffle's Rand2. Spec play-offs and early simulations (`simulate_early`) run the game's match simulation at the league's end, and the season end simulates those divisions again (career_season.c:1720-1722). So the RNG stream differs per world. The rules library, `c_rng`, state sim and pools use their own LCGs.
- `states.c:355` hardcodes 2026 (World 2026-27's first year) as the state draws' season seed.
- `LeaguePlayoffs.promoted/relegated` may duplicate the leagues' `Division` counts in `competitions/leagues.json` (the header says the league record keeps balanced counts). Unsure which one the port should keep.
- 8485 has `World_CupClubs` data but no play-offs table, so `WorldCups_KeepPlace` (league_playoffs.c:437) never runs there. Unsure how 8485's cups learn final places.
- `WorldRules_World` is a direct world check, but the rules are mod settings. Upstream says the Mod plays them in 96/97 too; OpenSWOS turns them off there on purpose, to keep 96/97's matches original.
- `LeaguePhase_ModCups` lets a mod give 96/97 league-phase cups (out of scope). This means `EuropeanCupFormat` is data-selected even in 96/97.
- The Italian contests (Serie B/C) are fixed code, not spec data. The `italy` flag selects them. They are the largest block of non-data logic here.
- `state_pool.c` has no `#else`: with `SWOS_WORLDS=0` it defines nothing (states.c's stubs never call it).
