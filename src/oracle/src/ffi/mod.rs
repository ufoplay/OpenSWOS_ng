//! The C side, and the workspace's only `unsafe` (D3): upstream's units as bindgen declares
//! them, the Rust stubs of the externs they reference, and safe functions over both, one
//! module a unit. Every function here touches process-wide C globals: they are called only by
//! the holder of the oracle's lock ([`crate::Oracle`]).
#![allow(unsafe_code)]

pub(crate) mod cup;
pub(crate) mod cup_play;
pub(crate) mod draw;
pub(crate) mod league;
pub(crate) mod math;
pub(crate) mod play;
pub(crate) mod rng;
pub(crate) mod rules_library;
pub(crate) mod running;
pub(crate) mod sim;
pub(crate) mod sim_play;
pub(crate) mod sim_result;
pub(crate) mod stage;
mod stubs;
pub(crate) mod style;
pub(crate) mod table;
pub(crate) mod tables;
pub(crate) mod tournament;
pub(crate) mod unpack;

use std::hint::black_box;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use stubs::UpstreamPanic;

/// Upstream's units and headers, as bindgen declares them (`<file name>`, its dot an
/// underscore).
#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    dead_code,
    missing_docs,
    clippy::all,
    clippy::pedantic
)]
mod bindings {
    /// src/random.c.
    pub(crate) mod random_c {
        include!(concat!(env!("OUT_DIR"), "/random_c.rs"));
    }

    /// src/game_utils.c.
    pub(crate) mod game_utils_c {
        include!(concat!(env!("OUT_DIR"), "/game_utils_c.rs"));
    }

    /// src/math68k.c.
    pub(crate) mod math68k_c {
        include!(concat!(env!("OUT_DIR"), "/math68k_c.rs"));
    }

    /// include/math68k.h, its `static inline` functions through their wrappers.
    pub(crate) mod math68k_h {
        include!(concat!(env!("OUT_DIR"), "/math68k_h.rs"));
    }

    /// src/dos_gameplay.c.
    pub(crate) mod dos_gameplay_c {
        include!(concat!(env!("OUT_DIR"), "/dos_gameplay_c.rs"));
    }

    /// src/strings_data.c (data): the countries' table.
    pub(crate) mod strings_data_c {
        include!(concat!(env!("OUT_DIR"), "/strings_data_c.rs"));
    }

    /// src/swos_amiga_data_imports.c (data): the team files' global numbers.
    pub(crate) mod swos_amiga_data_imports_c {
        include!(concat!(env!("OUT_DIR"), "/swos_amiga_data_imports_c.rs"));
    }

    /// src/competition_data.c (data): the season end's countries.
    pub(crate) mod competition_data_c {
        include!(concat!(env!("OUT_DIR"), "/competition_data_c.rs"));
    }

    /// src/world_data.c: World 2026-27's baked tables.
    pub(crate) mod world_data_c {
        include!(concat!(env!("OUT_DIR"), "/world_data_c.rs"));
    }

    /// src/rnc.c: the RNC unpacker.
    pub(crate) mod rnc_c {
        include!(concat!(env!("OUT_DIR"), "/rnc_c.rs"));
    }

    /// src/diy_competition.c: the DIY competition engine's functions M5 ports.
    pub(crate) mod diy_competition_c {
        include!(concat!(env!("OUT_DIR"), "/diy_competition_c.rs"));
    }

    /// src/match_sim.c: the simulation of a match nobody plays.
    pub(crate) mod match_sim_c {
        include!(concat!(env!("OUT_DIR"), "/match_sim_c.rs"));
    }

    /// src/menus.c: the formations' players by how defensive they are.
    pub(crate) mod menus_c {
        include!(concat!(env!("OUT_DIR"), "/menus_c.rs"));
    }

    /// src/swos_rules_post.c (src/rules/sr_post.c): the season's moves.
    pub(crate) mod swos_rules_post_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_post_c.rs"));
    }

    /// src/swos_rules_cups.c (src/rules/sr_cups.c): a cup's clubs.
    pub(crate) mod swos_rules_cups_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_cups_c.rs"));
    }

    /// src/swos_rules_groups.c (src/rules/sr_groups.c): parallel groups.
    pub(crate) mod swos_rules_groups_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_groups_c.rs"));
    }

    /// src/swos_rules_cupd.c (src/rules/sr_cupd.c): the grouped cup's later rounds.
    pub(crate) mod swos_rules_cupd_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_cupd_c.rs"));
    }

    /// src/swos_rules_dplay.c (src/rules/sr_dplay.c): the fixed tiers' brackets.
    pub(crate) mod swos_rules_dplay_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_dplay_c.rs"));
    }

    /// src/swos_rules_check.c (src/rules/sr_check.c): the checks of a league's rules.
    pub(crate) mod swos_rules_check_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_check_c.rs"));
    }

    /// src/swos_rules_gplay.c (src/rules/sr_gplay.c): the pack's knockouts.
    pub(crate) mod swos_rules_gplay_c {
        include!(concat!(env!("OUT_DIR"), "/swos_rules_gplay_c.rs"));
    }

    /// src/career_status.c: whether a player is available.
    pub(crate) mod career_status_c {
        include!(concat!(env!("OUT_DIR"), "/career_status_c.rs"));
    }

    /// src/test_rules.c: the testers' rule.
    pub(crate) mod test_rules_c {
        include!(concat!(env!("OUT_DIR"), "/test_rules_c.rs"));
    }

    /// src/tactics_data.c (data): the tactics table and the price tables.
    pub(crate) mod tactics_data_c {
        include!(concat!(env!("OUT_DIR"), "/tactics_data_c.rs"));
    }

    /// src/career_team_scalars.c (data): the career club's players.
    pub(crate) mod career_team_scalars_c {
        include!(concat!(env!("OUT_DIR"), "/career_team_scalars_c.rs"));
    }

    /// src/career_teams.c: a tournament's next stage.
    pub(crate) mod career_teams_c {
        include!(concat!(env!("OUT_DIR"), "/career_teams_c.rs"));
    }

    /// src/fixture_rounds.c: a world's odd round robins as real rounds.
    pub(crate) mod fixture_rounds_c {
        include!(concat!(env!("OUT_DIR"), "/fixture_rounds_c.rs"));
    }

    /// src/fixture_data.c (data): the original's round robins.
    pub(crate) mod fixture_data_c {
        include!(concat!(env!("OUT_DIR"), "/fixture_data_c.rs"));
    }

    /// src/swos_amiga_ptr_imports.c (data): the round robins by size, the divisions' default
    /// names, the lists of records.
    pub(crate) mod swos_amiga_ptr_imports_c {
        include!(concat!(env!("OUT_DIR"), "/swos_amiga_ptr_imports_c.rs"));
    }

    /// src/string_symbols.c (data): the compiled strings by name.
    pub(crate) mod string_symbols_c {
        include!(concat!(env!("OUT_DIR"), "/string_symbols_c.rs"));
    }

    /// src/swos_amiga_strings.c (data): the name a running competition's reset writes.
    pub(crate) mod swos_amiga_strings_c {
        include!(concat!(env!("OUT_DIR"), "/swos_amiga_strings_c.rs"));
    }

    /// src/swos_data_bulk_imports.c (data): the European Championships' round robin.
    pub(crate) mod swos_data_bulk_imports_c {
        include!(concat!(env!("OUT_DIR"), "/swos_data_bulk_imports_c.rs"));
    }
}

/// Calls `call`, turning upstream's halt into its reason code.
fn halting(call: impl FnOnce()) -> Result<(), u16> {
    catch_unwind(AssertUnwindSafe(call)).map_err(|payload| {
        match payload.downcast::<UpstreamPanic>() {
            Ok(halt) => halt.0,
            Err(other) => resume_unwind(other),
        }
    })
}

/// Resets the globals the units touch to upstream's values, and keeps every stub linked.
pub(crate) fn reset() {
    stubs::reset();
    sim_play::reset();
    rng::reset();
    style::reset();
    black_box(stubs::PanicHaltLoop as extern "C-unwind" fn() -> !);
    black_box(
        bindings::career_teams_c::DIY_AdvanceToNextCompetitionSlot
            as unsafe extern "C-unwind" fn(),
    );
    black_box(stubs::TacticsData_SelectRelease as extern "C-unwind" fn(_));
    black_box(stubs::Tweaks_On as extern "C-unwind" fn(_) -> _);
    black_box(stubs::World_Current as extern "C-unwind" fn() -> _);
    black_box(stubs::LeaguePhase_InstallCups as extern "C-unwind" fn(_));
    black_box(stubs::World_SyncSecondFiles as extern "C-unwind" fn());
    black_box(stubs::LeaguePhase_Sort as extern "C-unwind" fn() -> _);
    black_box(stubs::LeagueSplit_Sort as extern "C-unwind" fn());
    black_box(stubs::VerifyTeamLoaded as extern "C-unwind" fn(_) -> _);
    black_box(stubs::LeagueSplit_Init as extern "C-unwind" fn(_, _, _));
    black_box(stubs::SuperCup_UefaTie as extern "C-unwind" fn() -> _);
    black_box(stubs::WorldCups_CupMade as extern "C-unwind" fn(_));
    black_box(stubs::WorldEurope_DrawMade as extern "C-unwind" fn(_));
    black_box(stubs::LeaguePhase_StageDays as extern "C-unwind" fn(_, _, _) -> _);
    black_box(stubs::States_StageDays as extern "C-unwind" fn(_, _, _) -> _);
    black_box(stubs::LeaguePhase_StageStart as extern "C-unwind" fn());
    black_box(stubs::LeaguePhase_KnockoutDraw as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::States_StageStart as extern "C-unwind" fn());
    black_box(stubs::States_KnockoutStart as extern "C-unwind" fn());
    black_box(stubs::InitInGameTeams_WithMatchParam as unsafe extern "C-unwind" fn(_, _));
    black_box(stubs::WorldRules_CpuCountdown as extern "C-unwind" fn(_) -> _);
    black_box(stubs::SetupPlayers as extern "C-unwind" fn(_, _, _, _, _, _));
    black_box(stubs::CupRecord_Match as extern "C-unwind" fn(_, _, _, _, _, _, _, _));
    black_box(stubs::UpdateCareerPlayersFormAfterMatch as extern "C-unwind" fn(_, _, _, _, _));
    black_box(stubs::BeginContractNegotiationSequence as extern "C-unwind" fn(_, _, _));
    black_box(stubs::Career_UpdateStatsAfterMatch as extern "C-unwind" fn(_, _, _, _, _, _, _));
    black_box(stubs::ModHooks_CareerEvent as extern "C-unwind" fn(_, _, _, _));
    black_box(stubs::LongBench_LeagueMatch as extern "C-unwind" fn(_));
    black_box(stubs::CalculateViewResult as unsafe extern "C-unwind" fn(_, _, _, _, _, _, _, _, _));
    black_box(
        stubs::InitializeInGameTeamsAndStartGame as extern "C-unwind" fn(_, _, _, _, _, _, _),
    );
    black_box(stubs::LeagueSplit_Pair as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::LeaguePhase_Pair as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::States_Pair as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::LeagueSplit_Count as extern "C-unwind" fn() -> _);
    black_box(stubs::LeagueSplit_AfterResult as extern "C-unwind" fn(_, _, _, _));
    black_box(stubs::LeaguePhase_Result as extern "C-unwind" fn(_, _, _, _));
    black_box(stubs::ExtraTimeSecondPhaseScoring as unsafe extern "C-unwind" fn(_, _));
    black_box(stubs::PenaltyShootoutScorerPair as unsafe extern "C-unwind" fn(_, _));
    black_box(stubs::WorldPlayoff_LevelWinner as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::WorldPlayoff_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::WorldCups_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::CupD_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::SuperCup_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::UefaQual_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::Conti_TieDecided as extern "C-unwind" fn(_, _, _));
    black_box(stubs::UefaPrize_CupWon as extern "C-unwind" fn(_, _));
    black_box(stubs::WorldPlayoff_ArrangeNextRound as extern "C-unwind" fn());
    black_box(stubs::WorldCups_ArrangeNextRound as extern "C-unwind" fn());
    black_box(stubs::CupD_ArrangeNextRound as extern "C-unwind" fn());
    black_box(stubs::WorldEurope_RoundPlayed as extern "C-unwind" fn());
    black_box(stubs::WorldPlayoff_RoundSize as extern "C-unwind" fn());
    black_box(stubs::WorldCups_RoundStringIndex as extern "C-unwind" fn(_, _, _) -> _);
    black_box(stubs::WorldPlayoff_RoundName as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::StringCopy as unsafe extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::LeaguePhase_GroupsEnd as extern "C-unwind" fn(_, _));
    black_box(stubs::Conti_GroupsEnd as extern "C-unwind" fn(_, _));
    black_box(stubs::States_GroupsEnd as extern "C-unwind" fn(_, _));
    black_box(stubs::States_KnockoutEnd as extern "C-unwind" fn());
    black_box(stubs::LeaguePhase_Showing as extern "C-unwind" fn() -> _);
    black_box(stubs::LeagueSplit_Walking as extern "C-unwind" fn(_));
    black_box(bindings::match_sim_c::CalcTeamViewOffset as unsafe extern "C-unwind" fn(_) -> _);
    black_box(
        bindings::match_sim_c::CalcTeamPriceRankPercentOffset
            as unsafe extern "C-unwind" fn(_, _) -> _,
    );
    black_box(
        bindings::match_sim_c::InitPlayerRemapTableForTeam as unsafe extern "C-unwind" fn(_, _),
    );
    black_box(
        bindings::career_status_c::GetNumberOfAvailablePlayers
            as unsafe extern "C-unwind" fn(_, _, _) -> _,
    );
    black_box(
        bindings::career_status_c::GetNumberOfAvailableSubstitutes
            as unsafe extern "C-unwind" fn(_, _) -> _,
    );
    black_box(stubs::ReplayTest_Armed as extern "C-unwind" fn() -> _);
    black_box(stubs::ShouldApplyRandomPlayerStatusAfterSim as extern "C-unwind" fn() -> _);
    black_box(stubs::WorldRules_CpuLineupsBegin as extern "C-unwind" fn(_, _, _, _));
    black_box(stubs::WorldRules_CpuLineupsEnd as extern "C-unwind" fn(_));
    black_box(stubs::WorldRules_GkAfterSim as extern "C-unwind" fn(_, _));
    black_box(stubs::Career_ApplyPostMatchPlayerStatModsIfCareerTeam as extern "C-unwind" fn(_, _));
    black_box(bindings::menus_c::InitializeTacticsPositions as unsafe extern "C-unwind" fn());
    black_box(stubs::LeagueSplit_ZoneFind as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::LeagueSplit_ZoneOfTeam as extern "C-unwind" fn(_, _, _, _) -> _);
    black_box(stubs::LeagueSplit_ZoneScale as extern "C-unwind" fn(_, _, _, _) -> _);
    black_box(stubs::LongBench_FrOrder as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::SquadExtras_BenchRecord as extern "C-unwind" fn(_, _) -> _);
    black_box(stubs::WorldRules_CpuOpen as extern "C-unwind" fn(_) -> _);
    black_box(stubs::WorldRules_CpuAny as extern "C-unwind" fn() -> _);
    black_box(stubs::ReplayTest_Running as extern "C-unwind" fn() -> _);
    black_box(stubs::Rollback_Enabled as extern "C-unwind" fn() -> _);
}
