# World inventory: the career, squads and the match

Part of the [world inventory](../WORLD_INVENTORY.md) (docs/PLAN.md M4 #4). Upstream OpenSWOS at
`df643c2`; line numbers are at that commit. The `rules/` files and components proposed
here are merged into one design in the inventory, whose §12 maps each name used here to its final one.

Scope: the files below. Line numbers are at the pin. "settings.X" means the world
mod's `mod.json` `settings` key X. `world_rules.c` reads it under the slug "world" + tag. But
`world_finance.c` and `cpu_repl.c` hardcode the slug "world2627" (see Summary). Every setting
defaults to on, except `cpu_replacements` (default "invented"). In 96/97 every setting is forced
off by a `World_Current() != WORLD_9697` gate.

Short names used below:
- "extras" = a club's players 17..30, from `teams/<COUNTRY>.json` (`DataJson_LoadTeamExtras`).
- "FileCountry" = `World_FileCountry` / `World_SecondFile` (a country spread over two team files).
  At this pin both worlds' generated tables are empty, so it is the identity everywhere
  (world_data.c:1568-1593, world_data_8485.c:495-520).

## src/career_teams.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 818-863 (caller career_init.c:786-791) | Career_LoadWorldEuropeTmd | where a new career's first-season foreign clubs (the TMD pool for job offers, foreign bids and the foreign market) come from | returns 0: `LoadSomeEuroCup` x3 loads EUROCUP/EUROCWC/UEFACUP.TMD records | the 3 league phases' 36 clubs (`World_LeaguePhaseCups`, world_data.c:8070 kWorld2627LeaguePhases), records read from the team files | the 3 knockout cups' seed lists (`World_EuropeKnockouts`, world_data_8485.c:3379 kWorld8485EuropeKO), records from the team files | DATA | `competitions/career.json` `first_season_foreign_clubs`: team ids in order. 9697: the ids in the 3 .TMD files. 2627/8485: the seed lists of the world's European records. Unsure whether a .TMD record equals its team-file record; if not, also add `source: tmd`. |
| 1046-1058 | fillLeagueTeamSelectionFromFile | a division's clubs are also read from the country's second team file when the first is short | none (macro 0) | none at this pin (empty table) | none | DATA | `competitions/countries.json` `second_team_file` (absent = none). Source: world_data.c `WorldData2627_SecondFile`. |
| 1442-1445 | Career_InitCompetitionTeamSelectionFromTableHeaderSpec | a cup's entrants: a fixed world-cup list (byes, padded bracket) instead of the record's count | the record's count | `World_Cups` (world_data.c:7420, 101 cups) | `World_Cups` (world_data_8485.c:3135, 32 cups) | DATA | `competitions/cups.json`, world-cup fields (fixed list, real ties, byes, quotas). Competitions inventory. |
| 1865-1866 | Career_AdvanceToNextTeamSelectionKeyCacheEntry | one-match B-team call-ups go back at each calendar step | no-op (no call-ups) | bteam.c | no-op unless the package has bteams.json (unsure) | DATA | `competitions/bteams.json` (no B teams = nothing to return). |
| 1867-1869 | Career_AdvanceToNextTeamSelectionKeyCacheEntry | computer clubs' injury/card countdowns run once a calendar day (calls `CareerStatus_CountdownTeam`, which uses Rand2) | off | on (settings cpu_red_cards / cpu_injuries) | on (same settings, slug world8485) | DATA | `mod.json` settings.cpu_red_cards, settings.cpu_injuries (9697: false). |
| 1870-1878 | Career_AdvanceToNextTeamSelectionKeyCacheEntry | daily steps of world-only contests: Coppa Italia Serie D, super cups, UEFA qualifying, other confederations' cups, play-offs | all no-ops | tables in world_data.c | tables in world_data_8485.c | DATA | competitions packages (CupD, SuperCups, UefaSeeds, ContiSpec, LeaguePlayoffs). Competitions inventory. |
| 2052-2055, 2176-2179 | Career_ProcessCurrentCompetitionSlotAfterMatch, Career_OpenCompetitionSlotMenuAndApplyChanges | the caches are not rebuilt while slot 4 holds a world play-off / Serie D cup / super cup tie | always rebuilt | as the data says | as the data says | DATA | same competitions data as above. |
| 2343 | Career_AutoSimulateLeagueFixturesUntilHumanTeam | after a league split, stop at the end of the day | never | `World_LeagueSplits` | `World_LeagueSplits` | DATA | league split data. Competitions inventory. |
| 2642-2650 | DIY_AdvanceToNextCompetitionSlot | group-stage end hooks: league phase play-offs, other confederations' main rounds, state championships | no-ops | world tables | world tables | DATA | competitions data (league phase, conti, states). Competitions inventory. |
| 3131-3135 | addNextRoundCupEntry | a world cup's round names in the key cache entry | engine's `GetNextCupRoundStringIndex` | world cup's names (world_cups.c) | same | VIEW | Round name only. It is stored in the key cache entry, so it may also be SAVE (unsure). |

## src/career_status.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 325-328 | maybeDecayPlayerStatusBucketForTeam | a computer club's injury buckets decay (Rand2 inside) | skipped for computer clubs | runs if settings.cpu_injuries | same (slug world8485) | DATA | `mod.json` settings.cpu_injuries (9697: false). |
| 359-364 | cyclePlayerStatusFlagsForTeam | a computer club's cards/injuries count down | skipped for computer clubs | runs if settings.cpu_red_cards | same | DATA | `mod.json` settings.cpu_red_cards (9697: false). |
| 606-625; used at 646, 676, 760, 801, 844 | fr_list, fr_player in GetNumberOfSubstitutes, GetNumberOfSubstitutesInSelectionList, GetNumberOfAvailablePlayers, GetNumberOfAvailableSubstitutes, GetNumberOfAvailableStartingPlayers | squad-legality counts for a friendly/DIY club read its long-bench order (16 + extras) instead of its 16 | always the record's 16 | the long-bench order when the match bench is over 5 (`LongBench_FrOrder`) | none (long benches are 2627 only) | LIMIT | `rules/limits.json` bench size via `rules/benches.json`, plus extras in `teams/*.json`. With no long bench the list stays the 16. |
| 1366-1370 | UpdateCareerFinancesAfterMatch | a gate over 0xFFFF saturates instead of wrapping | wraps (`d7 &= 0xFFFF`, faithful) | saturates | saturates | DATA | `rules/finance.json` `gate_saturates: bool` (9697: false; world_finance.c `world()`). |
| 1379, 1381 | UpdateCareerFinancesAfterMatch | gates counted into FFP income | not counted | counted | counted | DATA | `rules/finance.json` `financial_fair_play` present. |
| 1692 | ApplyWeeklyFinancialDeductions | weekly TV + commercial income | none | `kTvBase` by level + 1.1 x wages | same | DATA | `rules/finance.json` `tv_income` (world_finance.c:54 kTvBase {3000000,1200000,500000,200000}; 9697: absent). |
| 2128-2142 | UpdateBoardConfidenceModifiers | trophy bonus flags for euro slot 7/8 (other confederations), 9 (group contest), 4-6 (qualifying ties) | slots 0-3 only (never 4-9) | conti cup tier (`Conti_TrophyTier`) | same | DATA | conti cups' `rank` (world_data.c `World_ContiCups`). Competitions inventory. |
| 2410 | Career_UpdatePlayerWeeklyWageCache | the managed club's own extras are paid at the reserve rate | never (no extras) | if the club has extras | if extras exist (unsure: 8485 data) | DATA | `teams/<COUNTRY>.json` extras + `career_params.json` `wage_reserve_pct`. |
| 2425-2426 | Career_UpdatePlayerWeeklyWageCache | a one-match B-team call-up is paid 0 | never | bteam.c | if bteams.json exists | DATA | `competitions/bteams.json`. |
| 2958-2989 | clubBusinessInitOriginal | statement rows SPONSORS AND TV RIGHTS, UEFA PRIZE MONEY | hidden | shown | TV shown, prizes hidden | VIEW | Shown when `rules/finance.json` tv_income / uefa_prizes are active. |
| 3175-3216 | clubBusinessWorldRows, ClubBusinessFfpSwitch, ClubBusinessUefaPrizesSwitch | FFP fine row, ratio, ON/OFF switches | hidden | shown | FFP shown, prizes hidden | VIEW | The switches flip `g_ffp.on` / `g_uefaPrize.off`. That career state is SAVE. |
| 3255 | Career_InitializeNewSeasonFinances | TV share period reset | no-op | resets | resets | DATA | part of `rules/finance.json` `tv_income`. |
| 3323-3324 | Career_InitializeNewSeasonFinances | the season-start euro-cup bonus is skipped while UEFA prizes pay instead | paid | skipped for slots 0-2, 4-6 when prizes are on | paid (prizes off: no league phase) | DATA | `rules/finance.json` `uefa_prizes.replaces_season_start_bonus` + settings.uefa_prizes. |
| 3328-3331 | Career_InitializeNewSeasonFinances | euro bonus tier for slots 7/8/9 | not reached | conti tier | same | DATA | conti cups' rank. Competitions inventory. |

## src/career_autopilot.c, include/career_autopilot.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 92, 956 | view_winners, the menu driver | golden-test screenshots of SEASON WINNERS when the world has titles | off | on | on | OUT_OF_SCOPE | test aid |
| 690-691, 891-893 | the menu driver | test call-ups and test purchases (SWOS_TEST_*) | n/a | test | test | OUT_OF_SCOPE | test aid |

The header has no branch points.

## src/transfer_market.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 638-648 | LoadPoolplyrDat | which foreign players' pool file is loaded | data/POOLPLYR.DAT | the package's `teams/POOLPLYR.DAT` | the package's, else 96/97's with a warning | DATA | new package file `teams/POOLPLYR.DAT` (a resource; or convert it to `teams/foreign_pool.json`). 9697: game data. |
| 754-756 | AddCareerTeamPlayer | same-country test for the home market's random pick | own file | FileCountry (identity at this pin) | identity | DATA | `competitions/countries.json` `second_team_file`. |
| 765-771 | AddCareerTeamPlayer | home market pick may be a club's extra. Costs one more Rand2 when the club has extras | 0..15 only, no extra Rand2 | 16 + extras equally likely | same if extras exist (unsure) | DATA | `teams/<COUNTRY>.json` extras (no extras = no extra draw). |
| 1458 | Career_CommitNewBalanceAndResetFinanceTotals | TV share period reset | no-op | resets | resets | DATA | `rules/finance.json` `tv_income`. |
| 1899 | BuyPlayerInit | CPU REPL. button | hidden | shown | shown | VIEW | Shows the CPU-replacement component choice (see cpu_repl.c). |
| 2253-2304, 2307 on | bop_page_layout, BuyOtherPlayersFinish, BuyOtherPlayersInit, BuyOtherPlayersPage | BUY OTHER PLAYERS page 2 (extras); a B team shown in place of the offering club | page 1 only | page 2 when there are extras; B squad browsing | as data | VIEW | extras + `competitions/bteams.json`. |
| 2515 | BuyPlayerMenu | purchases refused while FFP sanctions the club | never | `Ffp_PurchasesBlocked` | same | DATA | `rules/finance.json` `financial_fair_play` + settings.financial_fair_play. |
| 3092-3101, 3165-3180, 3204-3213 | PlayerMarketItemBeforeDraw, BuyOtherPlayersCommon, BuyOtherPlayersAfterDrawCommon | an extra's record (index 16..) for market rows and BUY OTHER PLAYERS page 2; a picked B-team row opens the call-up | never 16.. | extras, B teams | as data | VIEW | Drawing and menus. `BTeam_PickPlayer` starts the call-up (bteam.c). |
| 5521-5526 | PlayerMarketBuildVisiblePlayerList | the market filters read an extra's record for index 16.. | never 16.. | extras | as data | DATA | `teams/<COUNTRY>.json` extras. |
| 4253-4254 | Career_BuildForeignMarketPoolFromSelectedTeam | on a job change, the club's extras now in its 16 go on the departed list | no-op | runs | runs if extras | DATA | extras. The departed list is career state ("squad_extras" in the save). |
| 4645-4653 | Career_RemovePlayerAndAddAsTrialist | what fills a computer club's slot after the career club buys its player | invented trialist (`Career_ConvertPlayerToTrialistFromName`) | an extra bought: no replacement. Else invented, or FROM SQUAD (`CpuRepl_AfterSale`) | same choice, but no extras (unsure) | COMPONENT | `CpuSaleReplacement`: Invented / FromSquad / None-for-extra. Default from settings.cpu_replacements. |
| 4692-4696, 4708-4712 | pickRandomTeamRecordAvoidingTeam | same-country test for a generated replacement's donor club | own file | FileCountry (identity) | identity | DATA | `competitions/countries.json` `second_team_file`. |
| 5194-5197, 5220-5223 | TrainingMatch_AdjustSelectedTeamRosterForTrialists | a B-team call-up (PF_LOAN) is not dropped as a surplus filler | n/a | kept | kept if call-ups | DATA | `competitions/bteams.json`. |
| 5636-5650; called 5696-5697, 5722-5723 | offerRuleA in CreateRandomTransferOfferForPlayer | a club bids for your player only if he would be in its best 12 (fewer than 12 of its 16 priced above his price + 2) | no filter | filter | filter | DATA | `rules/market.json` `outgoing_offer_rank: {top_n: 12, price_margin: 2}` (transfer_market.c:5639-5640 OFFER_RULE_N, OFFER_RULE_MARGIN; 9697: absent). |

## src/job_offers.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 1296-1298, 1359-1362 | Career_MaybeGenerateJobOffer | same-country test for home and foreign job offers | own file | FileCountry (identity) | identity | DATA | `competitions/countries.json` `second_team_file`. |

## src/world_finance.c, include/world_finance.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 40-43, 59-62 | world, WorldFinance_On | the whole module is on: any world but 96/97, career only | off | on | on | DATA | `rules/finance.json` present (9697: absent). |
| 54, 66-75, 77-88 | kTvBase, tv_level, WorldFinance_Charge | TV + commercial income per weekly charge: the level's base / the season's charges, + 1.1 x the charge's wages. Level = league-groups level, capped at 3 | none | yes | yes | DATA | `rules/finance.json` `tv_income: {base_by_level: [3000000,1200000,500000,200000], wage_share_pct: 110}`. Levels from the league-groups data. |
| 104-112 | WorldFinance_PeriodTv | the statement's TV row | no row | row | row | VIEW | |
| 126-140, 143-158, 203-258 | kEuros, kValueEuros, SCALE_DIV, kMarket, amount, pay_raw, assoc_rank, market, value_pillar | UEFA 2026/27 prize table at 1/20, value pillar, media-market order | none | used | unused (no league phase) | DATA | `rules/finance.json` `uefa_prizes: {scale_div: 20, cups: [[12 amounts] x3], value: [[2] x3], market_first: [8,35,14,20,13]}`. Coefficients from `World_UefaSeeds` (league phase data). |
| 162-173 | UefaPrize_On, UefaPrize_NewCareer | prizes on: world, career, `UefaCoef_On` (league phase + UEFA seeds), switch on. Initial switch from setting "uefa_prizes" of slug "world2627" | off | on | off (`UefaCoef_On` false) | DATA | settings.uefa_prizes + `rules/finance.json` uefa_prizes present + league phase data. |
| 261-335 | UefaPrize_Result, _PhaseEnd, _Stage, _CupWon | prize payments (called from league_phase.c, diy_competition.c) | none | paid | none | DATA | as above. |
| 337-342 | UefaPrize_SkipSeasonStartTerm | euro slots 0-2, 4-6 skip the season-start term | n/a | yes | n/a | DATA | `uefa_prizes.replaces_season_start_bonus`. |
| 344-352 | UefaPrize_ShowRow, UefaPrize_Shown | statement row, CLUB BUSINESS switch | hidden | shown | hidden | VIEW | |
| 360, 365-371, 379-383, 407-415, 441-471 | FFP_LIMIT, Ffp_NewCareer, Ffp_Gate, Ffp_SeasonStartBefore/After, Ffp_PurchasesBlocked | financial fair play: wages over 40 % of last season's income close the market, fine = half the excess rounded down to thousands. Switch from setting "financial_fair_play" of slug "world2627" | off | on | on | DATA | `rules/finance.json` `financial_fair_play: {limit_pct: 40, fine_divisor: 2, fine_round: 1000}` + settings.financial_fair_play. |
| 417-439, 473-514 | ffp_memo, Ffp_Shown, Ffp_RatioText | FFP memo, ratio text | none | shown | shown | VIEW | |
| 33-41, 70-78 (header) | UefaPrizeState g_uefaPrize, FfpState g_ffp | prize and FFP state kept in the career | none | saved | saved | SAVE | the career save's world items. |

## src/world_nationals.c, include/world_nationals.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 40-66 | WorldNationals_Install | World Cup and European Championship formats + points per win | game's records | game's (`WorldData2627_Nationals` returns 0, world_data.c:2392) | WC2 header copied from `_worldCup` (24 teams, 1986 format). EC: 8 teams from kEcStages (world_nationals.c:37). 2 points a win | DATA | `competitions/tournaments.json` (the WC/EC records) + `rules/nationals.json` `points_for_win`. Source: world_data_8485.c:1809 kWorld8485Nationals. |
| 67-113 | WorldNationals_Install | national teams table: the era's nations only, each its nationality and home league; no nationality = no job | game's table | game's | filtered by 143 rules (kWorld8485NatRules) | DATA | `competitions/national_teams.json` fields `unused`, `nationality`, `home_league`. |
| 117-124 | WorldNationals_CareerStart | World Cup holder at career start | game's | game's | 0x5017 | DATA | `rules/nationals.json` `holder`. |
| 126-138, 27-33 | WorldNationals_Hosts, cycle | this season's WC and EC hosts | game's table (career_init.c) | game's | the 4-season cycle's (season / 4, last repeats) | DATA | `rules/nationals.json` `cycles[].wc_host/ec_host`. Cycle length 4 hardcoded at 29. |
| 148-207 | fill, WorldNationals_FillWorldCupGroups, WorldNationals_FillEuroGroups | qualifying groups: the game's draw or the cycle's fixed groups (minus holder and hosts) | game's draw (career_season.c:1136, 1286) | game's draw | fixed groups | COMPONENT | `NationalQualifyingDraw`: GameDraw (career_season.c) / FixedCycleGroups (world_nationals.c fill). Data: `rules/nationals.json` `cycles[].wcq[8], ecq[8]` {teams, places}. |
| 211-225, 228-236 | WorldNationals_RefreshGroups, WorldNationals_Cycle | after a load, groups' points/places again; season-end finalists by cycle (career_eurocups.c:865, 958) | none | none | applied | DATA | same `rules/nationals.json` (`direct` list used at career_eurocups.c:985). |

## src/bteam.c, include/bteam.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 127-190 | load | the world's B teams and call-up rules from `competitions/bteams.json` | none (skipped) | package file (20 clubs per header) | file if present (unsure) | DATA | `competitions/bteams.json`: `rules[]` {over_age_cap, u23, gk_any_age, windows[[from,to]]}, `teams[]` {b, parent, rules}. |
| 207-211 | BTeam_Shown | B TEAM button: world, career, not national, club has a B team | hidden | shown when data | as data | VIEW | |
| 214-226 | BTeam_Born | a B player's year of birth | n/a | `teams/*.json` "born" | as data | DATA | `teams/<COUNTRY>.json` born. |
| 247-301 | BTeam_Can | call-up checks: 4 a match, 3 a window, squad 30, 10 held, over-age cap by age this season | n/a | yes | as data | LIMIT | `rules/limits.json` `bteam_per_match: 4`, `bteam_per_window: 3`, `bteam_held: 10` (bteam.h:28-30). Squad 30 is the game's. Age uses `World_SeasonYear` (WORLD.ini `first_year`). |
| 303-350, 364-405 | BTeam_Add, back, BTeam_ReturnMatch, BTeam_ReturnAll | call-up (loan, wages, the B team gets a generated player) and return | n/a | yes | as data | DATA | `competitions/bteams.json`. State `g_bteam` is SAVE ("b_team"). |
| 450-478 | BTeam_PickPlayer | call-up dialog | n/a | dialog | dialog | VIEW | |

## src/squad_extras.c, include/squad_extras.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 48-55 | SquadExtras_On | extras on: world, career, not national | off | on | on (does nothing without extras) | DATA | extras in `teams/<COUNTRY>.json` (none = off). |
| 139-160 | block | load a club's extras (8-club cache) | n/a | team JSON | team JSON | DATA | `teams/<COUNTRY>.json` players 17.. |
| 162-190 | SquadExtras_ClubLoaded | the managed club starts with its extras in places 17.. (and no invented reserves, via TakeAdded) | game's 10 invented reserves | real extras | as data | DATA | extras. 14 extras max (SQX_MAX). |
| 230-246, 266-273 | SquadExtras_BenchPlayer, SquadExtras_BenchRecord | a club's extras for a long bench in ANY world match (not only a career) | none (`== WORLD_9697`) | yes | yes if extras (but no long bench there) | DATA | extras. |
| 247-264 | SquadExtras_Checksum | extras in the netplay team check | 0 | checksum | checksum | OUT_OF_SCOPE | netplay |
| 286-297 | SquadExtras_MarketPick | the market pick may be an extra. One more Rand2 | none | yes | if extras | DATA | extras. RNG order: the extra Rand2 happens only when the club has extras. |
| 299-366 | SquadExtras_Left, _OwnLeaving, _JobChange, _Bought | departed list (ring of 256) | n/a | yes | yes | SAVE | "squad_extras" in the save. LIMIT `extras_departed_max: 256` (squad_extras.h:32). |

## src/long_bench.c, include/long_bench.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 106-109 | world_benches | long benches on, by world id (not by table presence) | off | on | off | DATA | `rules/benches.json` present (2627 only). |
| 114-170 | division_bench, club_bench, country_bench, title_bench | named substitutes by (country, division), the country's own, or the contest title. National/custom/edited teams: none | n/a | tables | n/a | DATA | `rules/benches.json` `divisions[]` {country, division or "own", named, et}, `titles[]` {country or 0, name, named, et}. Source: world_benches.c:8-214 (107 divisions, 93 titles). |
| 173-203 | match_bench | career: title, then league division (`LongBench_LeagueMatch`), else the country's. Friendly/DIY: the longer of the two clubs' | game's bench | as tables | game's | DATA | same file. |
| 243-283, 214-220 | LongBench_MatchStart, LongBench_PreMatch | bench over 5: 11 + named places, subs allowed 5. Bench of 1..5: bench = named, subs = min(named, 5). Cap 15 | game's maxSubs/minSubs | competition's | game's | LIMIT | `rules/limits.json` `bench_max: 15` (LB_MAX_SUBS), `subs_with_long_bench: 5` (LB_USED). Bench sizes from `rules/benches.json`. |
| 71-82 | LongBench_Last, LongBench_SubsAllowed | cursor's last place; one more sub in extra time where `et` | 5; maxSubs | bench; +1 in ET | 5; maxSubs | LIMIT | `rules/benches.json` `et`. SubsAllowed is core (bench_subs.c). |
| 294-353 | LongBench_InitPlace, LongBench_Record | in-match places 17..26: the managed club's squad order, others' extras; injured/suspended = empty seat | not built | built | not built | LIMIT | places = 11 + bench. Extras from `teams/*.json`. |
| 420-436, 438-496 | fr_eligible, LongBench_FrActivate/FrOrder/FrSwap/FrRotate | a friendly/DIY club's own order of 16 + extras (offline, by world id 2627) | none | yes | none | LIMIT | same long-bench data. Netplay exclusion: OUT_OF_SCOPE. |
| 498-501, 505-640 | LongBench_FrontRow, MirroredArrow, name sprites | dugout front row, name sprites | n/a | drawn | n/a | VIEW | |
| 649-670 | LongBench_ReplayWrite/Read | BNCH replay chunk only with a long bench | none | written | none | SAVE | replay file (.SRP). |
| 687-720, 731-760 | LongBench_TestCallBench/TestInput, LongBench_SelfTest | tests | | | | OUT_OF_SCOPE | test aid |

## src/world_benches.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 8-117, 119-214 | g_worldBenchDivs, g_worldBenchTitles | World 2026-27's bench table (generated from the Mod's bench2627.json) | n/a | the table | n/a (table exists but long_bench.c gates it to 2627) | DATA | `rules/benches.json` (2627 package). |

## src/cpu_repl.c, include/cpu_repl.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 39-46 | CpuRepl_NewCareer | a new career's CPU replacement choice, from setting "cpu_replacements" of slug "world2627" (default invented) | invented | setting | reads world2627's setting | DATA | `mod.json` settings.cpu_replacements selects the `CpuSaleReplacement` component. |
| 55-58 | CpuRepl_Shown | the choice exists: world, career | no | yes | yes | VIEW | button gate (`CpuRepl_AfterSale` also checks it). |
| 61-171 | role, used_mask, pick, CpuRepl_AfterSale | FROM SQUAD: the same role's most valuable extra, else any outfield (a keeper only for a keeper), lowest place on a tie, names in the 16 skipped | n/a | yes | no extras: invented stays (unsure) | COMPONENT | `CpuSaleReplacement::FromSquad` (cpu_repl.c pick, CpuRepl_AfterSale). `used[]` is SAVE: LIMIT `cpu_repl_clubs: 64` (cpu_repl.h:21). EXTRAS_MAX 16 at 53. |
| 195-218 | label, CpuRepl_MenuInit, BuyPlayerCpuReplacements | button | hidden | shown | shown | VIEW | |

## src/team_pool.c, include/team_pool.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 43-51 | pool_for | a country has an off-file pool below its lowest division | none | 6 pools (world_data.c:4405-4412 kWorld2627Pools: Italy 42, Brazil 43, Spain 4, Germany 4, Holland 6, Portugal 5) | none (count 0, world_data_8485.c:1855) | DATA | `rules/team_pools.json` `pools[]` {country, roster file (`teams/ITALY_POOL.json`...), count, areas[[2]]}. |
| 108-134 | load_records | pool and second-file records | n/a | JSON | n/a | DATA | pool rosters in `teams/<X>_POOL.json`; second file from `competitions/countries.json`. |
| 157-175, 177-201, 203-220, 233-260, 262-277 | TeamPool_ApplyToFile, _Swap, _Assign, _SwapsDone, _Area | identity swaps at season end (queue), applied to every loaded record in a career | n/a | yes | n/a | DATA | same file. Map + queue are SAVE. LIMIT `team_pool_queue: 64`, `team_pool_countries: 8`, second file 16 clubs (team_pool.h:38, 56; team_pool.c:38). |

## src/world_stadium.c, include/world_stadium.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 35-59, 69-106 | load, WorldStadium_Name | stadium name under the team bars (neutral venue first, else home club) | none | from `competitions/stadiums.json` if present | same | VIEW | `competitions/stadiums.json`. |

## src/match_sim.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 419-436 (used 440, 615) | cpu_kept in ApplyRandomPlayerStatusAfterSim, InitPlayerRemapTableForTeam | a simulated result's random cards/injuries for a computer club (RNG calls), each kind kept only if its rule is on | computer clubs skipped | per settings | per settings | DATA | settings.cpu_red_cards, settings.cpu_injuries. |
| 950, 1136 | CalculateViewResult | computer clubs' injured/suspended starters swapped out for the result | no | if either rule | if either rule | DATA | same settings. |
| 980-981 | CalculateViewResult | goalkeeper injuries after a simulated result (own generator) | no | settings.goalkeeper_injuries | same | DATA | `mod.json` settings.goalkeeper_injuries. |

## src/statistics.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 1726-1760 | Career_FillCurrentSeasonInfo | trophy flags for super cups, titles won after the league, league-phase cups' bits 3/4 swapped, other confederations' trophies | game's flags | adjusted | adjusted | SAVE | `SeasonInformations.trophyFlags` in the save. Driven by competitions data. |
| 2013-2033 | SetTrophySprites_Internal | trophies shown for an older season's lines | none | shown | shown | VIEW | |
| 2258 | FillManagementRecordInfo | season title years | 1996 + n | 2026 + n | 1984 + n | VIEW | WORLD.ini `first_year` (world.c:17 kFirstYear). |

## src/stoppage.c, include/stoppage.h
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 62-75 | Stoppage_Wanted | stoppage time on: world, not training, not arcade, setting (online: host switch) | off | settings.stoppage_time | same | DATA | `mod.json` settings.stoppage_time (9697: false). Netplay part: OUT_OF_SCOPE. |
| 54-55, 82-102 | kEraToday, kEra1996, Stoppage_Reset | caps (1st half, 2nd half, ET half, base) and dead-ball divisor. Today's only for a career with first year >= 2000 | n/a | career {3,5,1,1, KD 233}; other matches {2,3,1,1, KD 0} | {2,3,1,1, KD 0} | DATA | `rules/match.json` `stoppage: {career: {caps:[3,5,1,1], dead_ball_div: 233}, other: {caps:[2,3,1,1], dead_ball_div: 0}}`. |
| 104-215 | Stoppage_Delta ... Stoppage_GoalMinute | the added-time mechanics (no RNG) | inert | active | active | DATA | same switch. Mechanics are one engine. |
| 232-308 | Stoppage_ClockSlot, Stoppage_ClockSprite | "+N" clock sprite | n/a | drawn | drawn | VIEW | |

## src/tackle_foul.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 322 | TryBookingThePlayer | computer players are booked too | only human teams (unless tweak cpu_cards) | settings.cpu_red_cards | same | DATA | `mod.json` settings.cpu_red_cards. |
| 420-422 | TrySendingOffThePlayer | computer players are sent off too | only human teams | settings.cpu_red_cards | same | DATA | same. |

## src/bench_subs.c
| line(s) | function | what is decided | 9697 | 2627 | 8485 | class | mapping |
|---|---|---|---|---|---|---|---|
| 182-186 | piFromOffset | places 16.. live outside TeamGame | 16 places | up to 26 | 16 | LIMIT | bench size. |
| 356-368, 390 | aiReplaceInjuredPlayer | AI sub: subs allowed (+1 in ET), every bench place scanned | 5 places, maxSubs | competition's | game's | LIMIT | `rules/benches.json` + `rules/limits.json`. Core. |
| 1041-1049 | isThisPlayerOkToSelect | a sub is allowed (count vs SubsAllowed) | maxSubs | +1 in ET where `et` | maxSubs | LIMIT | same. Core. |
| 259-263, 940-945, 1722, 2010-2012, 2156-2160, 3091 | bench_window_first, the menu cursor, name swaps, drawBenchPlayersAndCoach | 5-place window, front row, names | game's | long bench | game's | VIEW | |
| 2694-2701, 2727-2770 | strip_record, benchInfoStrip | substitute's value and best skills strip | off | settings.bench_info | same | VIEW | `mod.json` settings.bench_info. |
| 2769-2840 | opponentListAllowed, opponentListControls, drawOpponentList | OPPONENT squad list in the bench menu (off online) | off | settings.opponent_squad | same | VIEW | `mod.json` settings.opponent_squad. |
| 522, 765 | (tests) | SWOS_TEST_LONGBENCH | | | | OUT_OF_SCOPE | test aid |

## src/player_looks.c, include/player_looks.h, src/shirt_numbers.c, src/player_face3.c, src/shirt_edit.c
No branch points. Looks (record byte 25), face 3 (blond) and shirt numbers 12..99 are read from
each player record. Only World 2026-27's data carries them, so they need no switch:
`teams/<COUNTRY>.json` looks/faces/numbers (DATA, no rows). `World_SquadNumbers` (kept squad numbers)
is called from teams.c:1337, outside this area.

## Summary

### Rows per class
- DATA: 67
- COMPONENT: 3 rows (2 components)
- LIMIT: 9
- SAVE: 4
- VIEW: 21
- OUT_OF_SCOPE: 5
- NOT_A_BRANCH (no rows): 28 references. 19 `#if SWOS_WORLDS` build guards (world_nationals x9,
  squad_extras 50/77, long_bench 102/422/734, long_bench.h:52, bteam 95, cpu_repl 48,
  world_stadium 24, world_benches 6). 7 per-world cache keys (bteam 129/131/219/221,
  squad_extras 141, world_stadium 37/42). 2 header comments (world_finance.h:6, bteam.h:20).

### Proposed rules/ files
- `rules/finance.json`. Absent in 9697; present in 2627 and 8485 (upstream turns it on for any non-96/97 world).
  - `tv_income`: {base_by_level [3000000,1200000,500000,200000], wage_share_pct 110}.
  - `gate_saturates`: bool.
  - `financial_fair_play`: {limit_pct 40, fine_divisor 2, fine_round 1000}.
  - `uefa_prizes`: {scale_div 20, per-cup amounts (kEuros), value pillar (kValueEuros), market_first [8,35,14,20,13], replaces_season_start_bonus}. Only takes effect where a league phase exists (2627).
- `rules/market.json`: `outgoing_offer_rank` {top_n 12, price_margin 2}. Absent in 9697.
- `rules/benches.json`: `divisions[]` and `titles[]` {named, et}. 2627 only (world_benches.c).
- `rules/team_pools.json`: `pools[]` {country, roster file, count, areas}. 2627: 6 pools (world_data.c kWorld2627Pools).
- `rules/nationals.json`: holder, points_for_win, cycles[] {wc_host, ec_host, wcq[8], ecq[8], direct[]}. 8485 only (kWorld8485Nationals).
- `rules/match.json`: stoppage caps and dead-ball divisor, career vs other matches.
- `rules/limits.json`: bench_max 15, subs_with_long_bench 5, bteam_per_match 4, bteam_per_window 3,
  bteam_held 10, extras_max 14, extras_departed_max 256, cpu_repl_clubs 64, team_pool_queue 64,
  team_pool_countries 8.
- Fields in existing files:
  - `mod.json` settings: stoppage_time, cpu_red_cards, cpu_injuries, goalkeeper_injuries, bench_info, opponent_squad, financial_fair_play, uefa_prizes, cpu_replacements. All false/absent in 9697.
  - `WORLD.ini` `first_year` (1996/2026/1984, world.c:17).
  - `competitions/countries.json` `second_team_file`.
  - `competitions/career.json` `first_season_foreign_clubs`.
  - `competitions/national_teams.json` nationality/home/unused.
  - New package file `teams/POOLPLYR.DAT` (foreign pool).

### Proposed components
- `CpuSaleReplacement`: what fills a computer club's slot after you buy its player.
  - Invented: `Career_ConvertPlayerToTrialistFromName` (transfer_market.c:4651).
  - FromSquad: cpu_repl.c `CpuRepl_AfterSale` / `pick`.
  - NoneForExtra: `SquadExtras_Bought` (transfer_market.c:4648).
  - Selected per career; the default comes from settings.cpu_replacements.
- `NationalQualifyingDraw`: how the WC/EC qualifying groups are built.
  - GameDraw: career_season.c, around 1136/1286.
  - FixedCycleGroups: world_nationals.c `fill`, `WorldNationals_FillWorldCupGroups/FillEuroGroups/RefreshGroups`, 4-season `cycle()`.

### 96/97 facts that a world overrides
- transfer_market.c:633 `_aDataPoolplyrDa` data/POOLPLYR.DAT: the foreign pool file. Worlds use teams/POOLPLYR.DAT.
- career_init.c:787-790: the 3 .TMD files (EUROCUP/EUROCWC/UEFACUP) as first-season foreign clubs.
- career_status.c:1370: the gate wraps at 16 bits.
- career_status.c:3333-3339: season-start euro bonus multipliers 62644/41668/50011, skipped under UEFA prizes.
- tackle_foul.c:322/421, career_status.c:327/363, match_sim.c:424: computer clubs never booked, injured or counted down.
- Game bench of 5 (competition maxSubs) vs long_bench.h:34/38 LB_MAX_SUBS 15 / LB_USED 5.
- world_nationals.c:37 kEcStages (8-team EC) and 47-51 (24-team WC copied from `_worldCup`) replace 96/97's `_europeanChampionships2` / `_worldCup2`.
- world.c:17 kFirstYear; stoppage.c:84 era rule "career && first year >= 2000".
- bench_subs.c:191: the original's `maxSubs > 5` goalkeeper-only branch, now tested against 127.

### Surprising or uncertain
- At this pin both worlds' SecondFile/FileCountry tables are empty. Every FileCountry row (career_teams, transfer_market, job_offers, world_finance, team_pool, statistics) is inert.
- `WorldData2627_Nationals` returns 0. World 2026-27's national-coach career plays 96/97's hosts, draws and national table.
- Settings slug "world2627" is hardcoded in world_finance.c:35 and cpu_repl.c:44. world_rules.c uses "world"+tag. So 8485 reads 2627's uefa_prizes / financial_fair_play / cpu_replacements.
- Long benches and the friendly order are gated on `== WORLD_2627` (long_bench.c:108, 424), not on table presence.
- Finance (TV income, gate cap, FFP) also runs in 8485, since `world()` is "not 96/97".
- transfer_market.c:5695 (home-market offers) compares the raw country byte, not FileCountry, unlike line 754. This is inconsistent for two-file countries (inert today).
- cpu_repl.c:53 EXTRAS_MAX 16 vs squad_extras.h:31 SQX_MAX 14.
- Non-career matches use the 1996 stoppage caps in every world.
- Unsure whether 8485's package has extras or bteams.json. If not, those modules are no-ops there.
- `ModHooks_*` calls in transfer_market.c (2815, 4415, 4645, 4887, 5090) are the mod system, not world branches. No rows.
