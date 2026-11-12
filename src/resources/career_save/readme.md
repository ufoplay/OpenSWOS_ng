# openswos_career_save_resource

Reader and writer for OpenSWOS's career saves (`.CAR`), byte for byte (D4): the format
upstream's golden careers hash, so that our engine's saves can be compared with them.
Built on `openswos_save_json_resource` (the saves' JSON dialect).

## The save

`CareerSave` has one field per top-level member, in the order upstream's writer
(`SaveJson_WriteCareer`) writes them. The members it writes only in some careers (a
tournament's stage, the parked selection, World 2026-27's world cups, play-offs and
coefficients, the mods' sections…) are options. Unknown members do not read.

The layout is the writer's (`CareerSave::layout`): one member a line, the header written
`"key": value`, the stages' cup lists and `cache_scores` on the line of the member
before.

## The members (M2 #11b–g)

| member | type |
|---|---|
| `schema_version`, `file_kind` | `SchemaVersion`, `CareerKind`: only 3 and `career` read |
| `world` | the world's tag; absent in saves made before worlds (then World 1996-97's) |
| `season_informations` | `SeasonInformation`: the manager's record, a season each (35) |
| `primary_country_code` | the current competition's country, 255 without one |
| `career_team`, `current_team`, `selected_teams`, `selected_teams_parked`, `tmd_teams` | `SaveTeam`: the team crate's `Team` |
| `career_team_reserves`, `current_team_reserves` | `SavePlayer`: the team crate's `Player` |
| `career_foreign_market_players` | `MarketPlayer`: the market's header and a `SavePlayer` |
| `transfer_offer_table` | `TransferOffer`: clubs' offers for the manager's players (5) |
| `new_job_offers`, `job_offers` | `JobOffer`: the job-offer queues (5 at least) |
| `team_pools`, `state_pools` | `TeamPool`, `StatePools`: a world's club pools below the leagues |
| `world_cups`, `world_cupd`, `conti_holders`, `conti_edition` | `WorldCups`, `WorldCupD`, `ContiHolders`, `ContiEdition`: a world's cups |
| `world_playoff`, `stage_titles`, `season_winners`, `league_stages`, `promedio` | `WorldPlayoff`, `StageTitles`, `SeasonWinners`, `LeagueStage`, `Promedio`: a world's league contests |
| `competition_overrides`, `secondary_league_rows`…`euro_league_rows` | `CompetitionOverrides`, `LeagueRow`: the slots' league tables by team identity, a world league's split |
| `comp_copy_dynamic`, `comp_copy_teamlists` | `CompCopyDynamic`, byte lists: what the competition copies change from their templates |
| `cache_strides`, `cache_scores` | the team selection key cache's strides and its games' scores (`[packed, aggregate, penalties]`) |
| `comp_slots` | `CompSlots`: the five competition slots, each a `CompetitionSlot` |
| `comp_slot_stage` … `stage_score_pen`, `<slot>_comp_slot_stage…` | a tournament's current stage: its slot, cup team ids, standings (`StageRow`, `StageStandingsRow`) or knockout scores |
| `uefa_coef`, `uefa_qualifying` | `UefaCoef`, `UefaQualifying`: a world's UEFA coefficients and qualifying rounds |
| `career_scalars` | `CareerScalars`: the 249 fields of upstream's scalar table, the user's tactics as `SaveTactics` (the `tac` crate's `TeamTactics`) |
| `mods` | the mods' sections, kept as written (`SaveValue`): they are the mods' |

The counts the save states must agree with what it holds: `num_selected_teams` with
`selected_teams`, `career_scalars.tmd_team_count` with `tmd_teams`.

- **Teams and players** are the 684- and 38-byte records, written field by field (the
  writer's `jb_player`, `emit_team_file_ex_body`): bit fields and nibbles split, the
  global team number as a number. They convert through the record into the team crate's
  types, so a code outside its set (tactics, kit colour, team control…) does not read,
  and the team crate's checks apply.
- **Signed bytes.** The writer prints some bytes signed (`(int8_t)`): a player's
  nationality, shirt number, passing, price and form, a market player's ordinal. Either
  sign reads; they are written signed.
- **Names** are written up to their first NUL. Faithful to the original: the bytes after
  it are not written (`jb_key_str` zeroes them), so a name's leftover bytes do not
  survive a save. A name longer than its slot, or holding a NUL, does not read.
- **Season information and offers** keep their numbers in their C widths (an ordinal
  of `-1` for none, `-1`/`-2` marking a haggle's progress); what they mean is the
  career's (M7). A job offer's team name is 18 raw bytes, as written: `0xFF` marks it
  unset, so it is no NUL-ended name.
- **A world's sections** (only in the careers whose world has them) are the saved state
  of its modules (team pools, cups, play-offs, coefficients…), field by field in the
  writer's order and their C widths. Lists keep the length written. Records the writer
  flattens into one array (a table's rows of 6, a club's 7 coefficients…) read as
  fixed-size records (`flat`): a length that is not a whole number of them does not
  read.
- **League tables and copies.** The overrides carry each league table's rows by team
  identity (-1 the career club, otherwise an index among the selected teams), 64 a
  table. A copy's team list is kept as its bytes: `{country, ordinal}` pairs, though the
  writer sizes it from the copy's template.
- **Competition slots.** A slot is a header (`SlotHeader`: type, teams, months, round
  state, name, team ids…) and what its type adds (`SlotData`): a league's progress, a
  cup's bracket and scores, a tournament's stage slots, or the match data alone for
  another type. A slot reads by its type and must write back exactly as read, so a
  member that is unknown, missing, out of place or of another type does not read.
- **The scalar table** (`career_scalars`, upstream `s_career_scalars`) comes in the
  table's order, each field typed by its field type: numbers in their C widths, names,
  byte buffers, word arrays. The five competition slots' state
  (`*_competition_context`) and a few other buffers are written as they lie in memory, in
  the x86_64 layout upstream's golden careers are made with: they are kept as bytes, and
  what they hold is the competitions' (M5, M7).
- **The transfer list's two keys.** Upstream saved it as raw bytes
  (`players_on_transfer_market`) until 2026-11-05 (commit `5769efd`), and as words
  (`players_on_transfer_market_ids`) since. The pinned loader ignores the old key (the
  list is lost); this crate keeps whichever is written, and requires exactly one.
- **Season names.** Upstream's loader keeps one byte less of a season's names than their
  slots hold (17 of a club's 18, 22 of a scorer's 23); the save keeps what was written.

## Sources

Upstream OpenSWOS: `src/save_format_json.c` (`SaveJson_WriteCareer`,
`emit_season_informations`, `jb_player`, `s_player_fields`, `emit_team_file_ex_body`,
`s_team_file_fields`, `emit_market_player_body`, `emit_struct_object`,
`s_transfer_offer_fields`, `s_job_offer_fields`, the world sections, `wp_emit_*`,
`emit_competition_overrides`, `emit_ctx_league_standings`, `emit_comp_copy_dynamic`,
`emit_comp_copy_teamlists`, `emit_cache_strides`, `emit_comp_slot`, `emit_ctx_stage`,
`emit_stage_cupfull`, `s_career_scalars`, `emit_scalar_table`, `emitted_len`), the
world modules' state (`include/team_pool.h`, `state_pool.h`, `states.h`, `world_cups.h`,
`world_cupd.h`, `world_conti.h`, `world_playoff.h`, `league_playoffs.h`, `uefa_coef.h`,
`uefa_access.h`, the rules library's `swosrules.h`), `include/game_types.h`
(`SeasonInformations`, `PlayerFile`, `TeamFile`, `MarketPlayer`, `TransferOfferEntry`,
`JobOffer`, `DIYTeamStats`, `LeagueSplitState`, `DIYCompetition`, `DIYCompetitionEx` and its
league and cup views, `DIYTournamentSlot`).

## Tests

- Unit tests: each typed member against the writer's text, the save's layout, the
  checks, and a property test (any player record round-trips, its name up to the first
  NUL).
- `tests/fixtures.rs`: every team of the committed team files reads back as written.
- `tests/upstream.rs`: with upstream's checkout in the cache, `CareerScalars` is exactly
  upstream's scalar table, in order (the struct was generated from it).
- `tests/real_saves.rs`: every career save in `OPENSWOS_SAVES_DIR` (`just test_saves
  <folder>`) reads as a typed career and writes back byte for byte. Saves are never
  committed.
