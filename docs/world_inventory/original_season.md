# World inventory: the original season end and career setup

Part of the [world inventory](../WORLD_INVENTORY.md) (docs/PLAN.md M4 #4). Upstream OpenSWOS at
`df643c2`; line numbers are at that commit. The `rules/` files and components proposed
here are merged into one design in the inventory, whose §12 maps each name used here to its final one.

Upstream: OpenSWOS at df643c2. Files: `src/career_season.c` (1875 lines), `src/career_init.c` (2173),
`src/career_eurocups.c` (1024), `src/competition_data.c` (2089, skimmed).

Conventions used below:

- "9697 / 2627 / 8485" columns say what each world does at that point.
- A hook "with no data" is a no-op in 9697 (its accessor returns NULL/0/false there).
- Team ids are `country << 8 | ordinal`. Regions (national teams) are countries 80..85 = 0x50..0x55:
  Europe, Africa, South America, North America, Asia, Oceania (in that order).
- Which world feeds which accessor (from `src/world_data.c` / `src/world_data_8485.c`):
  2627 has second team files, league groups, play-offs, team pools, world cups, league phases, other
  confederations' cups, states, UEFA seeds, league splits, Serie D cup, super cups. 8485 has its own
  season countries, 32 world cups, the national cycle (`World_Nationals`), European knockouts
  (`World_EuropeKnockouts`), cup clubs; its groups / play-offs / pools / splits are empty (count 0).
- Proposed `rules/` files are collected in the Summary.

---

## src/career_season.c

### World branch points

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 104, 278 | `Career_FillRandomTeamListExcludingSelected`, `Career_PrepareEuropeanCupQualificationState` | The country the career club plays for, read from its team file (`World_FileCountry`). | identity (file = country) | Italy's second file 197 counts as Italy | identity | DATA | `rules/team_files.json` `second_files: {file: country}`. 9697: empty. 2627: `WorldData2627_FileCountry` (world_data.c). |
| 292-306 | `Career_PrepareEuropeanCupQualificationState` | The three eligible-country lists: the 96/97 seeded draw, or one entry per place in the world's order (`World_LeaguePhaseCups`, capped at 39). | seeded draw stays | lists rewritten from `places[i][1..3]` | seeded draw runs, but the lists are not used (`koQualify` reads `WorldEurope_Places`) | COMPONENT | `european_qualification`: `seeded_draw` (9697), `league_phase_qualifying` (2627, `uefa_coef.c` + this block), `knockout_places` (8485, `koQualify`). |
| 441 | `Career_UpdateLastNationalityCall` | The club's country, for the national team that calls up (`World_FileCountry`). | identity | second file mapped | identity | DATA | `rules/team_files.json` (as above). |
| 703-710 | `ShowChairmanOpinionAboutTheSeason` | Stage titles of a league in zones count as cups won, -2 each (`LeagueSplit_InZones`, `WorldPlayoff_StageWins`). | no zones, no effect | Argentina / Panama zones | splits count 0, no effect | DATA | `rules/season_review.json` `stage_title_bonus: 2`; the zones come from the league-split data (2627 `WorldData2627_LeagueSplits`). |
| 811, 816, 835, 840, 1678, 1726, 1780-1791 | `careerCompetitionFinished`, slot updates | Winners passed to career mods (`ModHooks_CompetitionFinished`). | mods only | mods only | mods only | OUT_OF_SCOPE | Mods. |
| 843-844 | `Career_ProcessTertiaryCompetitionForNextSeason` | Whether the league cup winner keeps its European place (`WorldCups_LeagueCupEurope`). | always keeps (record is not a world cup) | per cup: `WorldCup.europe` | per cup (its 32 world cups) | DATA | `rules/european_cups.json` `league_cup_europe: [cup ids]`. 9697: every slot-2 cup (englishLeagueCup, scottishLeagueCup, japaneseEmperorsCup). 2627/8485: `WorldCup.europe` (world_data*.c). |
| 863, 868-872 | `Career_ProcessCompetitionForNextSeason` | Super Cups learn the top division's order and the cup's two finalists (`SuperCup_TableEnded`, `SuperCup_CupEnded`). | none | Super Cups | none | COMPONENT | `super_cups`: none (9697, 8485), `world_supercup` (2627, `world_supercup.c`). |
| 884-887 | `Career_ProcessCompetitionForNextSeason` | Play-offs/play-outs decided after the original's moves; last season's titles stamped; state championships (`LeaguePlayoffs_Decide`, `WorldWinners_SeasonDecided`, `States_SeasonEnd`). | no play-off data; titles page empty | play-offs, titles, Brazil's states | play-offs count 0 | COMPONENT | `promotion_playoffs`: `playoff_cup` (9697, see 1547-1656), `world_playoffs` (2627, `league_playoffs.c`, `world_playoff.c`). `state_championships`: none / `states.c` (2627). Titles page: VIEW (`world_winners.c`). |
| 1136-1137 | `setupWorldCupQualifyingGroups` | European WC qualifying groups: drawn, or the cycle's fixed groups (`WorldNationals_FillWorldCupGroups`). | price-sorted draw | price-sorted draw | the cycle's groups | COMPONENT | `national_qualifying_draw`: `price_sorted_column_fill` (9697, 2627), `fixed_cycle_groups` (8485, `world_nationals.c`). |
| 1286-1287 | `setCupPickList_Europe` | European Championship qualifying groups: drawn, or fixed (`WorldNationals_FillEuroGroups`). | draw | draw | the cycle's groups | COMPONENT | `national_qualifying_draw` (as above). |
| 1706, 1721-1722, 1728 | `processPrimaryCompetitionForNextSeason` | Play-off final tables kept; the club's group tables simulated early put back (`LeaguePlayoffs_Begin`, `WorldPlayoff_RestoreEarlyTable`, `LeaguePlayoffs_KeepTable`). | no data | used | count 0 | COMPONENT | `promotion_playoffs` (as above). |
| 1766, 1797, 1805, 1819, 1847, 1855 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Copa Libertadores: prepare its list, update its slot, qualify CONMEBOL clubs, add its 20 to the gate. Upstream extension, not the original. Active in 9697 when its data is installed and Brazil is a season country (`career_copa.c` `copa_in_world`). | active (SDL with JSON data) | replaced by its own confederation cups (`Conti_ReplacesCopa`) (unsure) | off (Brazil not a season country) (unsure) | COMPONENT | `european_qualification` `knockout_places` with places "league 1st and 2nd" per CONMEBOL country (Copa as data). Whether the World 1996-97 package keeps Copa is an open decision. |
| 1794-1795 | `Career_AdvanceToNextSeason_UpdateCompetitions` | UEFA Super Cup takes the three holders before they are cleared (`SuperCup_HoldersEnded`). | none | yes | none | COMPONENT | `super_cups` (as above). |
| 1802, 1815 | `Career_AdvanceToNextSeason_UpdateCompetitions` | The club's own country, processed first and skipped in the loop (`World_FileCountry`). | identity | second file mapped | identity | DATA | `rules/team_files.json`. |
| 1809-1810 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Which countries take part in the season end, and in what order (`World_SeasonCountries`, else `_careerCompetitionIdRemapTable`). | 65 countries of `_careerCompetitionIdRemapTable` | its list, incl. new countries | its 32 countries | DATA | `rules/season_end.json` `countries: [...]`. 9697: competition_data.c:1844-1850. 2627: `WorldData2627_SeasonCountries`. 8485: `kWorld8485SeasonCountries`. |
| 1828-1834 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Before the gate: the UEFA qualifying rounds fill the lists (`UefaQual_Fill`); other confederations' next editions (`Conti_SeasonEnd`). | none | both | none | COMPONENT | `european_qualification` `league_phase_qualifying` (2627). `other_confederation_cups`: none / `world_conti.c` + `conti_season.c` (2627). |
| 1835-1846 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Expected pool size before / after the holders join: 0x50 / 0x50; `UefaQual_Listed()` / 108; sum of places / places + holders (`WorldEurope_Active`, `World_EuropeKnockouts`). | 80 / 80 | listed / 108 | places / places + holders | DATA | Derived: before = what qualification appends, after = sum of cup sizes. 2627's 108 = 3 × 36 is itself hardcoded here; it should come from the league-phase records. Panics 0x8999 / 0x8998 become invariant errors. |
| 1852-1854 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Expected clubs per cup: 16 / 32 / 32; 36 / 36 / 36; `ko->cup[i]->header.teams`. | 16/32/32 | 36/36/36 (hardcoded) | record sizes | DATA | Each cup record's team count (`tournaments.json` euroCup, `cups.json` cupWinnersCup / uefaCup; 2627: league-phase records; 8485: `World_EuropeKnockouts` records). |
| 1858 | `Career_AdvanceToNextSeason_UpdateCompetitions` | The next season is not a world's first: its cups draw normally (`WorldCups_SeasonEnded`). | counter runs, unused | yes | yes | COMPONENT | `domestic_cup_field`: `whole_country` (9697), `world_cup_lists` (2627, 8485; `world_cups.c`). |
| 1859-1860 | `Career_AdvanceToNextSeason_UpdateCompetitions` | The career club's qualifying opponents' records (`UefaQual_Post`). | none | yes | none | COMPONENT | `european_qualification` `league_phase_qualifying`. |
| 1864-1867 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Debug log of the knockout counts. | - | - | - | OUT_OF_SCOPE | Debugging. |

### 96/97 hardcoded facts

| line(s) | function | fact | value | class | mapping |
|---|---|---|---|---|---|
| 100-168 | `Career_FillRandomTeamListExcludingSelected` | The seeded draw: each seed group is shuffled (Fisher-Yates, `Rand2`) unless all its entries are taken, then its first `copyCount` entries are listed. The career club's country is then forced in, from the end, until it has its quota. | algorithm | COMPONENT | `european_qualification` `seeded_draw`. The forced quota is "Faithful to the original". |
| 131-143 | `Career_FillRandomTeamListExcludingSelected` | The shuffle works in place on the static seed tables, so each season starts from last season's order. Upstream saves them (`save_format_json.c` `exact_seed_table`). | state | SAVE | The seed groups' current order is career state. |
| 198-206 | `Career_PrepareEuropeanCupQualificationState` | An accepted offer counts for the draw; a team of country 0x50..0x55 is a national team. | 0x50..0x55 | DATA | `rules/national_teams.json` `regions: [80..85]`. |
| 226-233, 248-255 | `Career_PrepareEuropeanCupQualificationState` | A European club's country gets at least 1 Champions' Cup and 1 Cup Winners' Cup place. "European" = continent byte 0 (first byte of `_countriesTable[n]`). | 1, continent 0 | DATA | `rules/european_cups.json` `career_country_min_places: [1, 1, quota]`, `continent: 0`. Continent per country: a `continent` field (new; `strings_data.c` `_countriesTable[n][0]`). |
| 234-235 | `Career_PrepareEuropeanCupQualificationState` | Champions' Cup seed groups and how many are taken: {4 take 4, 6 take 5, 11 take 5, 22 take 2} = 16. | seed lists competition_data.c:1856-1859; counts swos_amiga_ptr_imports.c:1403-1409 | DATA | `rules/european_cups.json` `seeded_draw.cups[0].groups: [{countries, take}]`. Not in `seed_buckets.json`. |
| 256-257 | `Career_PrepareEuropeanCupQualificationState` | Cup Winners' Cup groups: same countries, {4/4, 6/6, 11/11, 22/11} = 32. | competition_data.c:1861-1864; swos_amiga_ptr_imports.c:1411-1417 | DATA | `rules/european_cups.json` `seeded_draw.cups[1].groups`. |
| 270-287 | `Career_PrepareEuropeanCupQualificationState` | UEFA Cup buckets (a country repeated once per place) and how many are taken: {15 take 10, 18 take 10, 22 take 8, 22 take 4} = 32. The club's country keeps all its bucket entries. | buckets competition_data.c:1889-1894; counts swos_amiga_ptr_imports.c:1419-1425 | DATA | Buckets: `seed_buckets.json` (exists). Take-counts 10/10/8/4: new, `rules/european_cups.json` `seeded_draw.cups[2].take`. |
| 336-371 | `GetCompetitionAbbrevStr` | Short codes of national competitions: FR, WC, WCQ, EC, ECQ, NAC, CA, ANC, AC, OC, TNM, TR. Unknown id panics 0x2184. | strings | VIEW | A `short_code` per competition (new field; VIEW). |
| 382-386, 634-638 | `ShowSeasonOpinion`, `ShowChairmanOpinionAboutTheSeason` | Chairman message ids 27-31 / 13-17. | ids | NOT_A_BRANCH | Program text ids. |
| 489-517 | `adjustWeightsForCompetition` | Season review weight per competition class: World Cup 8, continental cups 6, WC qualifiers 4, friendly / invitation / tour 1 (score not multiplied), anything else (domestic) 3. | 8/6/4/1/3 | DATA | `rules/season_review.json` `weights` by class; class from the competition id (`comp_identity.h`). |
| 541-563 | `computeCategoryForCompetition` | National review thresholds: ratio < 30 / < 10 / <= -10 / <= -30; percentile <= 25 up one, >= 75 down one (guarded). Same values as `career_params.json` `board_*`, but hardcoded here. | 30/10/-10/-30, 25/75 | DATA | `rules/season_review.json` (or reuse `career_params.json` `board_*`; the guards differ from the club verdict, keep both). |
| 615-620 | `SeasonFinished_ComputeSeasonReview` | World Cup won: best verdict (0). Continental cup won: -2. | 0, -2 | DATA | `rules/season_review.json` `world_cup_win`, `continental_win`. |
| 754 | `BeginContractNegotiationSequence` | Negotiation time after a match. | 2 | NOT_A_BRANCH | Career mechanic, not a world fact (a `const`). |
| 801-805, 825-829 | secondary / tertiary processing | A country's 2nd and 3rd competitions must be cups (type 1); else panic 0x3003. | type 1 | NOT_A_BRANCH | Data validation on load. |
| 858-878 | `Career_ProcessCompetitionForNextSeason` | A country's table: [0] primary, [1] skipped (-2), [2] national cup, [3] league cup; -1 ends it. The cup is played only after the league; the league cup only if there is a national cup. | layout | DATA | `country_competitions.json` (exists: leagues, cups). The "league cup only after a cup" order is implied. |
| 993-1042 | `pickRandomNationalTeamId` | Opponent picking: up to 100 tries within the price band, then any; no duplicate, never the coach's team. | 99 retries | DATA | `rules/national_teams.json` `pick_retries: 100`. |
| 1115-1132 | `setupWorldCupQualifyingGroups` | WC qualifying draw only in a WC-qualifying season (flag 0x40). Europe's group 8 has 6 places, minus a European holder / host, made even. | 0x40, 6 | DATA | `rules/national_cycle.json` `wc_qualifying.europe.last_group_size: 6`. |
| 1139-1185 | `setupWorldCupQualifyingGroups` | European teams sorted by average player price, filled column by column over 8 groups; the coach's team into a random group (`Rand2() & 0x1C`); 0x30 - 6 + group-8 size slots. | 8 groups, 48 max | COMPONENT | `national_qualifying_draw` `price_sorted_column_fill`. Group count 8: DATA `rules/national_cycle.json`. |
| 1188-1191 | `setupWorldCupQualifyingGroups` | Other regions filled in order: North America, South America, Africa, Asia, then Oceania. | 0x53, 0x52, 0x51, 0x54, 0x55 | DATA | `rules/national_cycle.json` `wc_qualifying.region_order`. |
| 1194-1205 | `setupWorldCupQualifyingGroups` | Oceania's group: 4 places, minus an Oceanian holder / host, made even. | 4 | DATA | `rules/national_cycle.json` `wc_qualifying.oceania_size: 4`. |
| 1212-1235 | `setCupPickListCommon` | Non-European national competitions by season flag: 0x80 WC year (finals list if qualified, else friendlies), 0x40 continental cup + WC qualifying (finalist list if holder/host, else non-finalist), 0x02 continental cup year, 0 invitation + friendly. Other flags panic 0x3775. | flags | DATA | `rules/national_cycle.json` `pick_lists[region]` (competition_data.c:1900-1933). |
| 1249-1272 | `setCupPickList_Europe` | Europe by flag: 0x80 WC, 0x02 European Championship, 0x40 WC qualifying, 0x01 EC qualifying. | flags | DATA | `rules/national_cycle.json` `pick_lists[80]` (competition_data.c:1942-1949). |
| 1284-1341 | `setCupPickList_Europe` | EC qualifying draw (flag 0x01): price-sorted, column fill of 8 groups, 46 slots (group 8 takes 4); the coach's team into a random group among the first 7 only (`Rand2() * 7 >> 8`). | 8 groups, 46, 7 | DATA + COMPONENT | Sizes: `rules/national_cycle.json` `ec_qualifying`. Fill: `national_qualifying_draw`. The 7-of-8 is "Faithful to the original". |
| 1346-1401 | `setupFriendlyCupTeams`, `setupInvitationTournamentTeams` | Friendly and invitation opponents: average player price within 75%..125% of the coach's team. | 75, 125 | DATA | `rules/national_teams.json` `friendly_price_band_pct: [75, 125]`. |
| 1405-1427 | `setupTourCupTeams` | Tour: a random region of 6 (`Rand() * 6 >> 8`); opponents of that region priced 0..0x31; the tour named by region. | 6, 49 | DATA | `rules/national_teams.json` `tour_max_price: 49`; names: `_tourStringTable` (competition_data.c:1965, VIEW). |
| 1435-1471 | `Career_SetCupPickList_ForSelectedRegion` | Region dispatch 0x50..0x55; another value panics 0x4434. | 6 regions | DATA | `rules/national_teams.json` `regions`. |
| 1547-1656 | `processDiyLeagueQualificationEntry` | Only England's divisions 1..3 feed a play-off (div1Playoffs, div2Playoffs, div3Playoffs) with qualifier index pairs; checked by `COMPID_englishLeague` and a switch on the division. | England, 3 play-offs | DATA + COMPONENT | Links: `leagues.json` (englishLeague `playoffLinkPtrs`, `promotionLinkData`) and `league_blocks.json` (englishLeagues1/2 `playoffs`, `qualifierIndexPairs`) — exist; resolve per division from data, no country check. Behaviour: `promotion_playoffs` `playoff_cup`. |
| 1666-1680 | `processPrimaryCompetitionForNextSeason` | A cup primary (type 2) is played through the league auto-advance path (`Career_AutoAdvanceLeagueCompetitionFromRecord`). | quirk | NOT_A_BRANCH | "Faithful to the original" (comment at 1674-1675). |
| 1715-1724 | `processPrimaryCompetitionForNextSeason` | Only division 0's table gives European candidates (top-9 grid). | division 0 | DATA | `rules/european_cups.json` `candidates_division: 0`. |
| 1771-1775 | `Career_AdvanceToNextSeason_UpdateCompetitions` | A national coach: Europe (0x50) updates the European Championship; every region updates the World Cup. | 0x50 | DATA | `rules/national_cycle.json` `continental_qualifying: {80: ecQualifyingGroups}`. |
| 1777-1779 | `Career_AdvanceToNextSeason_UpdateCompetitions` | European cups finished in order Champions' Cup, Cup Winners' Cup, UEFA Cup — before any domestic season. | order | DATA | `rules/season_end.json` `steps`. |
| 1799 | `Career_AdvanceToNextSeason_UpdateCompetitions` | The team pool is emptied: it will hold next season's European fields. | 0 | NOT_A_BRANCH | Mechanics. |
| 1801-1820 | `Career_AdvanceToNextSeason_UpdateCompetitions` | A club coach's own country first, then the season countries in order, each followed by European qualification. A national coach: all countries in order. | order | DATA | `rules/season_end.json` `own_country_first: true`, `countries`. |
| 1822-1827, 1845-1847 | `Career_AdvanceToNextSeason_UpdateCompetitions` | Pool gate: 16 + 32 + 32 = 0x50 clubs appended, else panic 0x8999. | 80 | DATA | Derived from the three cup sizes (no field). Panic: invariant error. |
| 1852-1856, 1868-1869 | `Career_AdvanceToNextSeason_UpdateCompetitions` | After the holders: 16 / 32 / 32 per cup and still 0x50 in the pool, else panic 0x8998. | 16/32/32 | DATA | Cup records' team counts (`tournaments.json`, `cups.json`). |

---

## src/career_init.c

### World branch points

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 316-332, 722-738 | `ChooseTeamAndStartCareer`, `InitCareer` | A new career resets the world modules: team pools, world cups, Serie D cup, super cups, cup record, UEFA coefficients and prize, FFP, CPU replacements, squad extras, B teams, play-off stages, stage titles, winners page, other confederations, keeper-injury generator, states. | all no-ops (no data; `WorldRules_*` off in 9697) | all live | the world cups' state live; most others empty (unsure) | COMPONENT | Each optional component's `new_career()` (none in 9697). Modules: `team_pool.c`, `world_cups.c`, `world_cupd.c`, `world_supercup.c`, `cup_record.c`, `uefa_coef.c`, `world_finance.c`, `cpu_repl.c`, `squad_extras.c`, `bteam.c`, `league_playoffs.c`, `world_playoff.c`, `world_winners.c`, `world_conti.c`, `world_rules.c`, `states.c`. |
| 488 | `Career_InitializeSelectedTeamAndStartingBalance` | A world club's players 17.. join the squad (`SquadExtras_ClubLoaded`). | 16 players | squads up to 30 | (unsure) | DATA | Players per team: team file data + `rules/limits.json` `players_per_squad`. |
| 713-714 | `InitCareer` | The World Cup holder at the career's start: 20994, then a classic world's own (`WorldNationals_CareerStart`). | 0x5202 (probably Brazil, the 1994 holder; unsure) | 0x5202 | `World_Nationals()->holder` | DATA | `rules/national_cycle.json` `world_cup_holder_at_start`. 9697: career_init.c:713. 8485: `kWorld8485Nationals` (world_data_8485.c). |
| 786-791 | `InitCareer` | The first season's European fields: the EUROCUP / EUROCWC / UEFACUP .TMD files, or the world's own lists (`Career_LoadWorldEuropeTmd`, which tests `World_Current() == WORLD_9697` in career_teams.c). | .TMD files | its league-phase records' lists | its knockout records' lists | DATA | `rules/european_cups.json` `first_season_pool` (9697: `teams/EUROCUP.json`, `EUROCWC.json`, `UEFACUP.json`); worlds: the cup records' `team_seed_list`. |
| 860, 868 | `ContinueCareerNextSeason` | B-team call-ups go back before the season end (`BTeam_ReturnAll`). | no-op | yes | (unsure) | COMPONENT | `b_teams`: none / `bteam.c`. |
| 1196-1200 | `InitializeNewSeason` | Before the league tables are copied back: one-division moves become one-level moves for side-by-side groups (`LeagueGroups_Apply`). | none | yes | count 0 | COMPONENT | `promotion_relegation_moves`: `division_ladder` (9697, 8485), `grouped_levels` (2627, `league_groups.c`). |
| 1272, 1601 | `InitializeNewSeason`, `InitNewSeason` | Last season's club play-off cleared (`WorldPlayoff_Reset`). | none | yes | none | COMPONENT | `promotion_playoffs` `world_playoffs`. |
| 1347 | `InitializeNewSeason` | A national coach's cycle hosts, from the world (`WorldNationals_Hosts`). | 96/97 host table | 96/97 table | its cycle's hosts | DATA | `rules/national_cycle.json` `hosts[cycle]`. 9697: competition_data.c:1817-1818. 8485: `World_Nationals()` cycles. |
| 1355 | `InitializeNewSeason` | The club's country from its file (`World_FileCountry`). | identity | second file | identity | DATA | `rules/team_files.json`. |
| 1399, 1419, 1627, 1633 | `InitializeNewSeason`, `InitNewSeason` | Slot 1 / 2 only if the club is in the cup (`WorldCups_Plays`). | always | world cup lists | world cup lists | COMPONENT | `domestic_cup_field` (as above). |
| 1436-1449 | `InitializeNewSeason` | Slot 2 empty: the club's state championship (`States_SlotContest`). | none | Brazil | none | COMPONENT | `state_championships`. |
| 1456-1460 | `InitializeNewSeason` | Slot 3: first the club's UEFA qualifying tie, or else its other-confederation tie (`UefaQual_SeasonTie`, `Conti_SeasonTie`); then the European cups. | none | yes | none | COMPONENT | `european_qualification` `league_phase_qualifying`; `other_confederation_cups`. |
| 1464 | `InitializeNewSeason` | Slot 3 last resort: Copa Libertadores (upstream extension). | when installed | (unsure) | off | COMPONENT | As the Copa row of career_season.c. |
| 1540-1546 | `InitializeNewSeason` | Season start hooks: FFP before/after finances, Serie D cup rounds, Super Cup tie, cup record (`Ffp_*`, `CupD_SeasonStart`, `SuperCup_SeasonStart`, `CupRecord_SeasonStart`). | no-ops | live | cup record live; others no-op (unsure) | COMPONENT | `financial_fair_play`, `serie_d_cup`, `super_cups`; cup record is VIEW. |
| 833, 872-878, 882, 372 | `InitCareer`, `ContinueCareerNextSeason`, `ContinueCareer` | Career mod hooks (`ModHooks_*`). | - | - | - | OUT_OF_SCOPE | Mods. |
| 1814-1820, 1838-1840, 1876-1881 | `Rebuild_CompetitionSlotPointers` | After load: slots re-derived through the same hooks (`WorldCups_Plays`, `States_RestoreContest`, `WorldPlayoff_RestoreSlot`, `CupD_RestoreSlot`, `SuperCup_RestoreSlot`, `UefaQual_RestoreTie`, `Conti_RestoreTie`, `WorldPlayoff_GroupsRecord`). | slot index 0..3 | 0..9 | 0..2 | SAVE | The save stores slot identities; `_euroCupSlotIndex` codes: 0-2 the European cups, 3 Copa, 4-6 UEFA ties, 7-8 other-confederation ties, 9 a world's group contest. |

### 96/97 hardcoded facts

| line(s) | function | fact | value | class | mapping |
|---|---|---|---|---|---|
| 114-117 | `Career_InitTransferMarketState` | Three random market offsets 0..15 (`Rand2() & 0xF`), in the order 1, 2, 0. | 0xF | NOT_A_BRANCH | Mechanics; RNG order kept. |
| 134-155 | `Career_InitSeasonTransferMarketState` | Season transfer quota = (432 - foreign players - (7 - seasons completed) × 16) / (num_seasons - season). 432 is the foreign pool size; half for swaps, half for the pool. | 432, 7, 16, 6 (negotiation time) | DATA | `career_params.json` (exists: `foreign_pool_capacity` 432, `num_seasons`, `transfer_quota_mode`); 7 and 16 as new career knobs (not world facts). |
| 172-189 | `Career_RandomizeInvitationTournamentCup` | Each invitation tournament in the pick list becomes a tour with 50% (`Rand() & 1`). | 1/2 | DATA | `rules/national_teams.json` `invitation_to_tour_pct: 50`. |
| 234, 242 | `SetName`, `SetSurname` | Manager name 9 chars, surname 13. | 9, 13 | LIMIT | `rules/limits.json` (VIEW input). |
| 263-288 | `CareerYearsToggle` | Career length choices 20 / 30 / 40 (SDL only; the original is 20). | 20/30/40 | VIEW | Menu choices over `career_params.json` `num_seasons`. |
| 346-351 | `LoadLiverpool` | Cheat: start with Liverpool (0x811). | 0x811 | OUT_OF_SCOPE | Debugging. |
| 366, 1171-1172, 1550, 1556 | `ContinueCareer`, `InitializeNewSeason` | Retirement after the last season; season counter wraps at the career length. | `num_seasons` (20) | DATA | `career_params.json` `num_seasons` (exists). |
| 477 | `Career_InitializeSelectedTeamAndStartingBalance` | Team control byte: 2 coach, 3 player-coach. | 2/3 | NOT_A_BRANCH | Mechanics. |
| 483-487 | `Career_InitializeSelectedTeamAndStartingBalance` | 16 players in a career squad. | 16 | LIMIT | `rules/limits.json` `players_per_squad: 16`. |
| 521-528 | `Career_ContinueToNextSeasonInitTeamAndFunds` | A new club's funds: offered funds / 1000 × (80 + 0..24)% × 1000. | 80, 25 | DATA | Career knob: `career_params.json` new fields (like `start_balance_pct_*`). Not a world fact. |
| 713 | `InitCareer` | `_worldCupHostTeam` is really the World Cup holder (the winner replaces it in a WC year, career_eurocups.c:943-944). | 0x5202 | DATA | `rules/national_cycle.json` `world_cup_holder_at_start`. |
| 716-721 | `InitCareer` | League state table: one byte per global team number. | 2000 (`GLOBAL_TEAM_NUMBERS`) | LIMIT | `rules/limits.json` `global_team_numbers`. |
| 800-808 | `InitCareer` | The career's competitions are copied from 34 templates (`_careerContests`). | 34 | DATA | `career.json` (exists). |
| 811-822 | `InitCareer` | Slot states A/B/C = 3/2/8 for slots 0..3. | 3/2/8 | NOT_A_BRANCH | Mechanics (menu/slot state). |
| 823-832 | `InitCareer` | A new career starts at season index -1 and year - 1; `InitializeNewSeason` steps both. First year 96 is set in `menus.c:465`. | 96 | DATA | `rules/calendar.json` `first_year: 1996` (upstream `World_FirstYear`; 2627/8485: theirs). |
| 1176-1186 | `InitializeNewSeason` | 4 unavailable-player entries cleared. | 4 | LIMIT | `rules/limits.json`. |
| 1192-1194 | `InitializeNewSeason` | Year counter wraps at 100 (two digits). | 100 | VIEW | Display year; the port keeps a full year. |
| 1198-1200 | `InitializeNewSeason` | The copy-back of league tables: the season's promotions and relegations (scratch) become next season's divisions (`DIY_SaveLeagueScratchToTable`). | step | DATA | `rules/season_end.json` step `copy_back_league_tables` (in `InitializeNewSeason`, not in the season end). |
| 1233-1238 | `InitializeNewSeason` | The club's division = its league + its move in the state table. | rule | NOT_A_BRANCH | Mechanics. |
| 1253-1257 | `InitializeNewSeason` | Last match index per competition id: 256 entries. | 256 | LIMIT | `rules/limits.json` `competition_ids: 256`. |
| 1260-1264 | `InitializeNewSeason` | Five competition slots: league, national cup, league cup, continental cup, play-off. | 5 | LIMIT | `rules/limits.json` `competition_slots: 5`; their roles: `rules/season_start.json`. |
| 1290-1307 | `InitializeNewSeason` | National coach iff country 80..85. Season flags from a 20-entry table per region (Europe its own, the other five share one), `season % 20`. | 80..85; tables competition_data.c:1820-1821, map swos_amiga_ptr_imports.c:602-609 | DATA | `rules/national_cycle.json` `season_flags[region]` (period 4: Europe {0x40, 0x80, 0x01, 0x02}; others {0x42, 0x80, 0x02, 0}). |
| 1342-1346 | `InitializeNewSeason` | Hosts per 4-season cycle, 5 cycles, then wrap: WC host-qualified team all -1 (none); EC host 0x5005, 0x5025, 0x5022, 0x5011, 0x500B. | competition_data.c:1817-1818 | DATA | `rules/national_cycle.json` `hosts: [{wc, ec}]`, `cycle_seasons: 4`. |
| 1363-1432 | `InitializeNewSeason` | Slots from the country table: 0 primary (with division), skip the -2 entry, 1 national cup, 2 league cup. | layout | DATA | `country_competitions.json` (exists). |
| 1456-1467 | `InitializeNewSeason` | Slot 3 tried in order: Champions' Cup, Cup Winners' Cup, UEFA Cup (then Copa). | order | DATA | `rules/season_start.json` `continental_slot_order`. |
| 1469-1470, 1095-1138 | `DIY_SelectCompetitionSlot4FromCurrentDivision`, `slot4_link` | Slot 4: England's division 1/2/3 play-off (`COMPID_englishLeague` + switch). | England only | DATA | Per-division play-off link from `leagues.json` / `league_blocks.json` (exist). |
| 1474-1482 | `InitializeNewSeason` | `_leagueFirstRoundNumber` = record byte 9 for a league, which is `divisionsCount`; else 1. | byte 9 | NOT_A_BRANCH | Mechanics (unsure: the name suggests a round, the byte is the divisions count). |
| 1502-1522 | `InitializeNewSeason` | Job-offer phase bias = the first fixture's week key; wage divisor = last key - first (min 1). | derived | NOT_A_BRANCH | Mechanics (calendar keys come from the competition records). |
| 1549-1561 | `InitializeNewSeason` | Chairman message: last season or greeting, staying or new club. | num_seasons - 1 | NOT_A_BRANCH | Mechanics. |
| 1586 | `InitNewSeason` | A new club's country table chosen by the record's `teamNumber`. | rule | NOT_A_BRANCH | Mechanics. |
| 2110-2117 | `Career_ViewOrEditCompetitionSlot` | VIEW WORLD shows the European Championship / World Cup templates (ids 22 / 4) through the played copies (ids 21 / 3). | 22→21, 4→3 | VIEW | `career.json` an `alias` field (new) (unsure). |

---

## src/career_eurocups.c

### World branch points

| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 284-300 | `worldLeaguePhases`, `Career_AppendTeamToEuroCupListN` | League phases exist (`World_LeaguePhaseCups`); the qualifying rounds append clubs to cup N. | no | yes | no | COMPONENT | `european_qualification` `league_phase_qualifying`. |
| 346 | `Career_ProcessCompetitionForEuropeanCupQualification` | Every country's table is seen by the other confederations' cups (`Conti_Country`). | no-op | yes | no-op | COMPONENT | `other_confederation_cups`. |
| 352-364 | `Career_ProcessCompetitionForEuropeanCupQualification` | A European country's clubs into the cups: qualifying rounds (`UefaQual_Country`), its places (`WorldEurope_Places` + `koQualify`), or the 96/97 per-cup picks. | `qualifyTeamFor*` | `UefaQual_Country` | `koQualify` | COMPONENT | `european_qualification`: `seeded_draw`, `league_phase_qualifying`, `knockout_places`. |
| 307-341 | `koTaken`, `koNext`, `koQualify` | Knockout places: Champions' Cup from the league top 9, Cup Winners' Cup from the cup final (2) then the league, UEFA from the league; each club once; holders not listed. | - | - | yes | COMPONENT | `european_qualification` `knockout_places`; per-country places: 8485 packs' `europe_places` (`World_EuropeKnockouts`). |
| 562-577 | `Career_UpdateEuroCupCompetitionSlot` | Champions' Cup record of type 1 (a knockout) is played as a cup. | type 2 tournament | type 2 (league phase) | type 1 | NOT_A_BRANCH | Driven by the record's type (`tournaments.json` / world records). |
| 589-594, 608-622, 653-667 | slot updates | League-phase cups (type 2): reload the cup's selection on a loaded career. | type 1 CWC/UEFA | type 2 | type 1 | NOT_A_BRANCH | Driven by record type; the reload is a SAVE fix. |
| 730-735 | `drawRandomTeamFromPoolAndCopyGrid` | The club's country from its file (`World_FileCountry`). | identity | second file | identity | DATA | `rules/team_files.json`. |
| 758-775 | `Career_DrawEuropeanCupTeamsAndCopyGrids` | How last season's holders enter: swapped into a random place, or appended (league phases: EC and CWC holders to the Champions' list, UEFA holder to the CWC list unless `UefaQual_KeepsUeclHolder`; knockouts: each to its own cup). | swap | append crosswise | append own | COMPONENT | `european_holder_entry`: `swap_into_random_place` (9697, `drawRandomTeamFromPoolAndCopyGrid`), `append_to_cups` with a holder→cup map (2627: [0,0,1]; 8485: [0,1,2]). The map is DATA. |
| 865-886 | `Career_UpdateEuropeanChampionshipCompetitions` | EC finals list: the host, then each cycle group's places (`WorldNationals_Cycle`, `WorldNationals_RefreshGroups`). | fixed places 2×7 + 1 | fixed places | cycle places | DATA | `rules/national_cycle.json` `ec_qualifying.places`; 8485: `World_Nationals()` cycles `ecq[g].places`. |
| 958-1002 | `Career_UpdateWorldCupCompetitions` | WC finals list: first cycle the world's holder; holder (if a national team), host, cycle group places, then the direct qualifiers until `wcTeams`. | holder + fixed places | same as 9697 | cycle + direct list | COMPONENT | `national_finals_field`: `group_places` (9697), `cycle_places_then_direct` (8485, `world_nationals.c`). Sizes / places: DATA. |
| 876-884, 995-1000 | EC / WC updates | Logs of the lists. | - | - | - | OUT_OF_SCOPE | Debugging. |

### 96/97 hardcoded facts

| line(s) | function | fact | value | class | mapping |
|---|---|---|---|---|---|
| 118-129 | `tryConsumeCompetitionIdFromList` | A country takes one place per entry in a cup's eligible list; a used entry becomes 0xFE. | rule | COMPONENT | `european_qualification` `seeded_draw`. |
| 147-160 | `clearEuropeanCupTitleHolderIfMatchTeam` | A holder that qualifies normally loses its holder place (first match only: EC, then CWC, then UEFA). | rule | COMPONENT | `european_holder_entry` `swap_into_random_place`. |
| 190-199 | `qualifyTeamForEuroCup` | Champions' Cup: the league champion (grid slot 0). | slot 0 | DATA | `rules/european_cups.json` `seeded_draw.picks[0]: "league:1"`. |
| 213-233 | `qualifyTeamForCupWinnersCup` | Cup Winners' Cup: from the cup grid (slot 9 on), skipping the league champion and the Champions' Cup holder; the walk may run past the 3 cup slots into the next buffer. | slot 9.. | DATA | `rules/european_cups.json` `picks[1]: ["cup:1", "cup:2", ...]`, skip rules. The overrun is "Faithful to the original" (unsure what follows in memory). |
| 238-274 | `qualifyTeamsForUefaCup` | UEFA Cup, once per eligible entry: the league cup winner first (if any), else from the league runner-up; skip the champion, the cup winner, the two holders, clubs already in CWC / UEFA; wrap back to slot 1 at the holder grid. | rule | DATA | `rules/european_cups.json` `picks[2]` candidate order + skip rules. |
| 349-351 | `Career_ProcessCompetitionForEuropeanCupQualification` | Only European countries (continent byte 0) send clubs. | continent 0 | DATA | `rules/european_cups.json` `continent: 0`; countries' continent (new field). |
| 389-392 | `DIY_CachePrimaryQualifiedTeamsGrid` | European candidates from the league: the top 9 (fewer if the division is smaller). | 9 | LIMIT | `rules/limits.json` `europe_league_candidates: 9` (also read by `koQualify` and 2627's qualifying). |
| 412-419 | `DIY_CacheSecondaryQualifiedTeamsGrid` | European candidates from the national cup: 3 (grid slots 9..11). | 3 | LIMIT | `rules/limits.json` `europe_cup_candidates: 3`. |
| 432-444 | `DIY_CacheTertiaryQualifiedTeamGrid` | The league cup gives 1 candidate (its winner). | 1 | DATA | `rules/european_cups.json` `league_cup_europe`. |
| 702-751 | `drawRandomTeamFromPoolAndCopyGrid` | A holder replaces a random listed club not of the holder's country nor the career club's country (`Rand2() * count >> 8`, retried); pool size must not change (panic 0x8997). | rule | COMPONENT | `european_holder_entry` `swap_into_random_place`. |
| 776-786 | `Career_DrawEuropeanCupTeamsAndCopyGrids` | Holders swapped in order: Champions' Cup, CWC, UEFA. | order | DATA | Part of the component; order kept for RNG. |
| 848-849 | `Career_UpdateEuropeanChampionshipCompetitions` | Only in an EC-qualifying season (flag 0x01). | 0x01 | DATA | `rules/national_cycle.json` `season_flags`. |
| 861-862, 894-901 | `Career_UpdateEuropeanChampionshipCompetitions` | EC finals: the host, then 2 from each of groups 1-7 and 1 from group 8 = 16. | 1 + 2×7 + 1 | DATA | `rules/national_cycle.json` `ec_qualifying.places: [2,2,2,2,2,2,2,1]`, `host_first: true`. |
| 943-944 | `Career_UpdateWorldCupCompetitions` | In a WC season (0x80) the primary's winner becomes the holder. | 0x80 | DATA | `rules/national_cycle.json` `season_flags`. |
| 946-947, 1003-1023 | `Career_UpdateWorldCupCompetitions` | WC finals (in a WC-qualifying season 0x40): the holder, 2 from each of 8 European groups, North America 3, South America 4, Africa 5, Asia 2, Oceania 1 = 32. | 1 + 16 + 15 | DATA | `rules/national_cycle.json` `wc_qualifying.places`. |
| 969 | `Career_UpdateWorldCupCompetitions` | (8485 path) a holder outside 80..85 is left out. | 80..85 | DATA | `rules/national_teams.json` `regions`. |
| 964 | `Career_UpdateWorldCupCompetitions` | (8485 path) first cycle = seasons 0..3. | 4 | DATA | `rules/national_cycle.json` `cycle_seasons: 4`. |

---

## src/competition_data.c

### What it holds

All compiled competition data of 96/97: about 76 `CompetitionCup`, 66 `CompetitionLeague`, 13
`CompetitionTournament`, 7 `Playoffs`, 7 `Leagues` blocks, 10 `QualifyingGroup`, `_careerContests`
(34 entries + sentinel, room to `CAREER_CONTESTS_CAP`), the master pointer arrays, country tables,
`_competitionsTable[256]`, season flag tables, the European seed lists, the national pick lists, and
small lookup tables. It also holds two code accessors (`comp_name_table`, `comp_team_seed_list`) and
`Comp_AppendCareerContest` / `Comp_Register` / `Comp_ById` (a runtime registry). The `*Copy` records
are only `extern`s here (runtime copies in the career buffer: SAVE).

### World branch points

None (no `World_Current`, no `WORLD_*`, no `SWOS_WORLDS`). Worlds patch the tables from outside
(`world.c`, `world_packs.c`, `world_cups.c`, `world_data*.c` write `_competitionsTable[]` /
`_countriesTable[]`).

### Coverage by competitions/*.json

| table (lines) | covered by | notes |
|---|---|---|
| Cup / league / tournament / playoff / league-block / qualifying records (~200-1486) | `cups.json`, `leagues.json`, `tournaments.json`, `playoffs.json`, `league_blocks.json`, `qualifying.json` | Covered. |
| `_careerContests` (1574-1611) | `career.json` | Covered (template → copy index; the copy is derived). |
| `_nationalTeamCompetitionsAndQualifications`, `_continentalCups` (1656-1686) | no | Used in `teams.c` (preset menus). Derivable from `qualifying.json` + `career.json` (unsure). |
| Region / world / qualifying tables (1692-1740) | `qualifying.json` | Covered, incl. `_europeTable.countries` and `_worldTable` region order {80, 83, 82, 81, 84, 85}. |
| Country tables (1746-1811), `_competitionsTable` (1984-2061) | `country_competitions.json` | Covered. Specials 72/100 (customs), 252-254: covered as qualifying groups (unsure for the index mapping). `_italyTable[5]` has a spare slot for a world's league cup. |
| `_seasonCycleNationalFlagsA/B` (1817-1818) | no | → `rules/national_cycle.json` `hosts`. |
| `_careerTeamFlagsBySeason0/1` (1820-1821) | no | → `rules/national_cycle.json` `season_flags`. The region → table map is in `swos_amiga_ptr_imports.c:602-609`. |
| `_nationalSelectorSeasonMaskA/B` (1823-1824) | no | Used in `teams.c` (not my files). → `rules/national_cycle.json` (unsure of meaning). |
| `_allCountriesList` (1830-1842) | `countries.json` | Covered. |
| `_careerCompetitionIdRemapTable` (1844-1850) | no | → `rules/season_end.json` `countries` (65 + 0xFF). Its order differs from `_allCountriesList` and `_europeTable`. |
| `_euroCupQualSeedList*`, `*Reduced*` (1856-1864) | no | → `rules/european_cups.json` `seeded_draw` (with take-counts from `swos_amiga_ptr_imports.c:1403-1425`). |
| `_euroCupQualSeedBucketBlock` (1889-1894) | `seed_buckets.json` | Covered, but not the take-counts (10/10/8/4). |
| `_cupPick*` (1900-1949) | no | → `rules/national_cycle.json` `pick_lists[region]`. |
| `_ecQualifyingGroupsTable`, `_worldCupQualifyingGroupsTable` (1935-1956) | derivable | The 8 group copies: `qualifying.json` + `career.json`. |
| `_tourStringTable` (1965) | no | VIEW: tour name per region. |
| `_careerRegionPickWeightsTable/2` (1968-1969) | no | Used in `career_teams.c` (not my files). |
| `_diyCup*Choice*`, `_diyEdit*` (2067-2087) | n/a | Program state (zero-init), not data. |

### 96/97 hardcoded facts

| line(s) | function | fact | value | class | mapping |
|---|---|---|---|---|---|
| 1817 | data | WC host-qualified team per cycle: none. | -1 ×5 | DATA | `rules/national_cycle.json` `hosts[].wc` (unsure: France 98 not modelled). |
| 1818 | data | EC host per cycle. | 0x5005, 0x5025, 0x5022, 0x5011, 0x500B | DATA | `rules/national_cycle.json` `hosts[].ec`. |
| 1820-1821 | data | Season flags, period 4, 20 seasons: Europe 0x40, 0x80, 0x01, 0x02; others 0x42, 0x80, 0x02, 0. | tables | DATA | `rules/national_cycle.json` `season_flags`. |
| 1844-1850 | data | Season-end countries, in order. | 65 countries | DATA | `rules/season_end.json` `countries`. |
| 1856-1864 | data | Champions' Cup / CWC seed groups: 4 {14, 20, 35, 8}, 6 {2, 13, 29, 30, 31, 41}, 11 {...}, 22 {...}. The two sets are equal. | lists | DATA | `rules/european_cups.json` `seeded_draw.groups`. |
| 1889-1894 | data | UEFA buckets (places per country by tier). | buckets | DATA | `seed_buckets.json` (exists). |
| 1900-1949 | data | National competitions per region and season kind (pick lists of 5: primary, -2, then up to 2 more). | lists | DATA | `rules/national_cycle.json` `pick_lists`. |
| 1609, 1614 | data | 34 career contests compiled; the table holds `CAREER_CONTESTS_CAP`. | 34 / cap | LIMIT | `rules/limits.json` `career_contests` (upstream: 48 today, 2627 needs ~85). |
| 1984 | data | Country table index: 256 entries. | 256 | LIMIT | `rules/limits.json` `countries: 256`. |

---

## The season end, step by step

From `ContinueCareer` (career_init.c:364) to the end of `InitializeNewSeason`. "Hooks" are the world
calls at that point (no-ops in 9697 unless said).

1. **Retire or go on** — `ContinueCareer` (career_init.c:366-374). Last season (`num_seasons - 1`): retirement message, career reset, exit. Hooks: `ModHooks_CareerBeforeContinue` (mods).
2. **Verdict** — `ContinueCareerNextSeason` (career_init.c:855-867). Offer accepted: leaving message (unless sacked). Else: `ShowChairmanOpinionAboutTheSeason` (club: league ratio, percentile, cup top-2 bonuses; national: season review). Hooks: `LeagueSplit_InZones` + `WorldPlayoff_StageWins` (2627 zones).
3. **B teams back** — `BTeam_ReturnAll` (career_init.c:860 / 868). Hook only (2627).
4. **Load league state** — `DIY_LoadLeagueScratchFromTable` (career_season.c:1760): the league state table into the scratch buffer.
5. **Draw next season's European eligibility** — `Career_PrepareEuropeanCupQualificationState` (career_season.c:191-311): seeded draws for Champions' Cup (16), CWC (32), UEFA (32), in that order, with `Rand2`; the career club's country gets its quota. An accepted offer counts. Hooks: `World_FileCountry`; `World_LeaguePhaseCups` rewrites the lists (2627).
6. **Copa list** — `Career_PrepareCopaLibertadoresQualificationState` (career_season.c:1766). Upstream extension.
7. **National competitions** (national coach only) — career_season.c:1771-1775: Europe finishes the European Championship qualifying groups and fills the finals list (`Career_UpdateEuropeanChampionshipCompetitions`, EC-qualifying season only); every region updates the World Cup (`Career_UpdateWorldCupCompetitions`: the winner becomes holder in a WC season; in a WC-qualifying season the groups are finished and the finals list filled). Hooks: `WorldNationals_Cycle`, `WorldNationals_RefreshGroups` (8485).
8. **Finish the European cups** — `Career_UpdateEuroCupCompetitionSlot`, `Career_UpdateCupWinnersCupCompetitionSlot`, `Career_UpdateUefaCupCompetitionSlot` (career_season.c:1777-1779): played if the club's slot 3, else simulated; each winner's grid and id kept as holder. Hooks: record type 1/2 (8485 / 2627); `ModHooks_CompetitionFinished` (mods); `SuperCup_HoldersEnded` (2627).
9. **Finish Copa** — `Career_UpdateCopaLibertadoresCompetitionSlot` (career_season.c:1797). Upstream extension.
10. **Empty the team pool** — `_careerFileBuffer = 0` (career_season.c:1799).
11. **Each country's season** (club coach: own country first, career_season.c:1801-1806; then every season country in order, skipping the own one, 1809-1820; national coach: all of them) — for each country:
    1. `Career_ProcessCompetitionForNextSeason` (career_season.c:851-888):
       1. **League** — `processPrimaryCompetitionForNextSeason` (1662-1736): for each division, played (the club's) or simulated; division 0 caches the top-9 grid; promotion/relegation counts into the scratch; the qualifiers list appended. Hooks: `LeaguePlayoffs_Begin`, `WorldPlayoff_RestoreEarlyTable`, `LeaguePlayoffs_KeepTable` (2627); `ModHooks_CompetitionFinished`.
       2. **Division play-offs** — `processDiyLeagueQualificationEntry` per division (1547-1656): England's divisions 1-3: the play-off cup, played or simulated, and its promotion weights.
       3. **Super Cup (league)** — `SuperCup_TableEnded` (863). Hook only (2627).
       4. **National cup** — `Career_ProcessSecondaryCompetitionForNextSeason` (799-818): played or simulated; the 3-club grid cached. Hook: `SuperCup_CupEnded` (2627).
       5. **League cup** — `Career_ProcessTertiaryCompetitionForNextSeason` (823-845): played or simulated; the winner cached. Hook: `WorldCups_LeagueCupEurope` (worlds).
       6. **Sum check** — `DIY_ValidateLeagueScratchBuffer` (880): panics 0x4444 / 0x5555.
       7. **World play-offs, titles, states** — `LeaguePlayoffs_Decide`, `WorldWinners_SeasonDecided`, `States_SeasonEnd` (884-887). Hooks only (2627).
    2. **European qualification** — `Career_ProcessCompetitionForEuropeanCupQualification` (career_eurocups.c:344-365): European countries only; 96/97: champion → Champions' Cup, cup grid → CWC, league cup winner / league from 2nd → UEFA, per eligible entry. Hooks: `Conti_Country` (2627), `UefaQual_Country` (2627), `koQualify` (8485).
    3. **Copa qualification** — `Career_ProcessCompetitionForCopaLibertadoresQualification`. Upstream extension.
12. **Qualifying rounds / other confederations** — career_season.c:1828-1834. Hooks: `UefaQual_Fill`, `Conti_SeasonEnd` (2627).
13. **Pool gate** — career_season.c:1845-1847: pool = 0x50 (+ Copa), else panic 0x8999.
14. **Holders join** — `Career_DrawEuropeanCupTeamsAndCopyGrids` (career_eurocups.c:756-787): 96/97 swaps each holder into a random place (EC, CWC, UEFA order). Hooks: league phases (2627) / knockouts (8485) append instead.
15. **Count gate** — career_season.c:1852-1856: 16 / 32 / 32 (+ Copa 20) and pool unchanged, else panic 0x8998.
16. **Record moves** — `Career_StorePromotionRelegationInSeasonInfo` (1857). Hooks: `WorldCups_SeasonEnded` (worlds), `UefaQual_Post` (2627).
17. **New club or new budget** — offer accepted: `Career_ContinueToNextSeasonInitTeamAndFunds` (career_init.c:509-529: market cleared, foreign pool rebuilt, funds 80-104%, the new club set up). Else: `InitializeStartingBalanceForNewSeason` (other file). Hooks: `ModHooks_CareerEvent` (mods).
18. **New season** — `InitializeNewSeason` (career_init.c:1162-1571):
    1. Season and year counters step (wrap at `num_seasons` / 100).
    2. **Copy-back of league tables** — `DIY_SaveLeagueScratchToTable` (1200). Hook before it: `LeagueGroups_Apply` (2627).
    3. Board, season info, the club's division, transfer market state.
    4. **Competition slots** — national coach: season flags, cycle hosts, pick lists and qualifying draws (`Career_SetCupPickList_ForSelectedRegion`); club: the country table. Slot 0 league, 1 national cup, 2 league cup, 3 continental (EC, CWC, UEFA, Copa), 4 England's play-off. Hooks: `WorldNationals_Hosts` (8485), `World_FileCountry`, `WorldCups_Plays` (worlds), `States_SlotContest`, `UefaQual_SeasonTie`, `Conti_SeasonTie` (2627), `WorldPlayoff_Reset`.
    5. Budget ranks, calendar keys, teams, manager name, markets, finances. Hooks: `Ffp_SeasonStartBefore/After`, `CupD_SeasonStart`, `SuperCup_SeasonStart` (2627), `CupRecord_SeasonStart` (worlds, VIEW).
    6. Chairman message; trialists; national squad.
19. **Mods** — `ModHooks_CareerSeasonEnd` (career_init.c:882).

A package would state steps 4-18 as an ordered list of step names, with the European cup order, the
country order and the components chosen per step. The UI parts (1-2, 19) stay in the engine.

---

## Summary

### Rows per class

| class | rows |
|---|---|
| DATA | 73 |
| COMPONENT | 33 |
| NOT_A_BRANCH | 15 |
| LIMIT | 10 |
| OUT_OF_SCOPE | 5 |
| VIEW | 4 |
| SAVE | 2 |

140 rows: 48 world branch points, 92 facts. The 2 rows marked "DATA + COMPONENT" count in both.
By file: career_season.c 34 DATA / 15 COMPONENT; career_init.c 18 / 9 (+ 6 LIMIT); career_eurocups.c
14 / 9; competition_data.c 7 DATA, 2 LIMIT (no world branches).

### Proposed rules/ files

- `rules/season_end.json`: `steps` (ordered, see above), `countries` (96/97: `_careerCompetitionIdRemapTable`; worlds: `World_SeasonCountries`), `own_country_first`, and the European cup order (EC, CWC, UEFA).
- `rules/season_start.json`: competition slot roles (league, national cup, league cup, continental, play-off) and `continental_slot_order` (EC, CWC, UEFA, then Copa if kept).
- `rules/european_cups.json`: cup ids and their sizes (taken from the records); `continent: 0` (eligibility); the qualification component per world; `seeded_draw` (EC / CWC seed groups with take-counts 4/5/5/2 and 4/6/11/11; UEFA take-counts 10/10/8/4 over `seed_buckets.json`; the career country's minimum places); `picks` per cup (candidate order and skip rules); `candidates_division: 0`; `league_cup_europe`; the holder-entry component and its holder→cup map; `first_season_pool` (EUROCUP / EUROCWC / UEFACUP).
- `rules/national_cycle.json`: `cycle_seasons: 4`, `season_flags` per region, `hosts` per cycle (5), `world_cup_holder_at_start` (0x5202), WC qualifying (8 European groups, group-8 size 6, region order, Oceania 4, places), EC qualifying (8 groups, 46 slots, places 2×7 + 1, host first), `pick_lists` per region, which region plays the EC qualifiers (80).
- `rules/national_teams.json`: `regions` 80..85 (order, and continent = region - 80), price band 75-125%, tour max price 49, pick retries 100, invitation → tour 50%.
- `rules/season_review.json`: weights by competition class (8/6/4/1/3), national thresholds (30/10/-10/-30, 25/75), WC win → 0, continental win → -2, stage-title bonus 2.
- `rules/team_files.json`: second team files (file → country; 9697 empty).
- `rules/calendar.json`: `first_year` (1996).
- `rules/limits.json`: global team numbers 2000, competition ids 256, countries 256, competition slots 5, career contests (34 / cap), players per squad 16, unavailable list 4, European league candidates 9, cup candidates 3, eligible lists 17 / 33 / 33 in the original (40 with worlds, 39 entries: `swos_data_bulk_imports.c:55-57`, career_season.c:301), name lengths 9 / 13.
- New field in an existing file: `countries.json` `continent` (from `_countriesTable[n][0]`).

### Proposed components

- `european_qualification`: `seeded_draw` (9697: `Career_PrepareEuropeanCupQualificationState`, `Career_FillRandomTeamListExcludingSelected`, `qualifyTeamForEuroCup`, `qualifyTeamForCupWinnersCup`, `qualifyTeamsForUefaCup`), `league_phase_qualifying` (2627: `UefaQual_*` in `uefa_coef.c`, the list rewrite in career_season.c:292-306), `knockout_places` (8485: `koQualify`, `WorldEurope_Places`; Copa could be another instance).
- `european_holder_entry`: `swap_into_random_place` (9697: `drawRandomTeamFromPoolAndCopyGrid`, `clearEuropeanCupTitleHolderIfMatchTeam`), `append_to_cups` with a holder→cup map (2627, 8485: `Career_DrawEuropeanCupTeamsAndCopyGrids`).
- `national_qualifying_draw`: `price_sorted_column_fill` (9697: `setupWorldCupQualifyingGroups`, `fillWorldCupQualifyingGroupTeams`, `setCupPickList_Europe`), `fixed_cycle_groups` (8485: `WorldNationals_FillWorldCupGroups`, `WorldNationals_FillEuroGroups`).
- `national_finals_field`: `group_places` (9697: `updateECSlot`, `updateWCSlot` lists), `cycle_places_then_direct` (8485: `world_nationals.c`).
- `domestic_cup_field`: `whole_country` (9697), `world_cup_lists` (2627, 8485: `WorldCups_Plays`, `WorldCups_SeasonEnded`, `world_cups.c`).
- `promotion_playoffs`: `playoff_cup` (9697 England: `processDiyLeagueQualificationEntry`, `slot4_link`), `world_playoffs` (2627: `LeaguePlayoffs_*`, `WorldPlayoff_*`).
- `promotion_relegation_moves`: `division_ladder` (9697, 8485), `grouped_levels` (2627: `LeagueGroups_Apply`).
- Optional, none in 9697: `super_cups` (`SuperCup_*`), `state_championships` (`States_*`), `other_confederation_cups` (`Conti_*`), `serie_d_cup` (`CupD_*`), `financial_fair_play` (`Ffp_*`), `b_teams` (`BTeam_*`, `SquadExtras_*`).

### Surprising or uncertain

- Upstream's 9697 is not the pure original: Copa Libertadores (`career_copa.c`) runs in the season end when its data is installed. It adds a simulated competition (RNG) and 20 clubs to the gates. The World 1996-97 package must decide whether to keep it.
- The seeded draw shuffles the static seed tables in place. Their order is career state (upstream saves it).
- The European cups are finished before any domestic season. The club's own country goes first, then the list order. Both orders matter for the RNG and for which country takes which eligible entry.
- The CWC pick walks past the 3 cup slots into the next buffer, and the UEFA pick wraps at the holder grid. Both depend on memory layout (unsure what lies after slot 11).
- The EC qualifying draw puts the coach's team in a random group among 7 of 8. The WC draw uses all 8.
- `_worldCupHostTeam` is the World Cup holder. 0x5202 is probably Brazil (unsure). The WC host table is all -1.
- `_leagueFirstRoundNumber` reads `divisionsCount` (unsure).
- 2627's 36 / 108 counts are hardcoded in career_season.c:1846-1854, not taken from its records.
- England's play-offs are found by `COMPID_englishLeague` and a switch on the division, though the links are in the data.
- The national season review hardcodes the board thresholds that the club verdict reads from `career_params.json`.
