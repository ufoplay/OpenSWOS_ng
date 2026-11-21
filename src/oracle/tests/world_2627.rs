//! World 2026-27's baked install (`openswos_oracle::tables::world_2627`), against what upstream's
//! src/world_data.c states. A file of its own: the install writes 96/97's tables for good, so
//! this test runs in a process of its own (cargo runs each test file apart).

#![cfg(not(oracle_unavailable))]

use openswos_oracle::Oracle;
use openswos_oracle::tables::world_2627;

/// The clubs `resize_country(20, 256)` gives Italy.
const ITALY_CLUBS: u16 = 256;

/// The clubs `resize_country(0, 10)` gives Albania.
const ALBANIA_CLUBS: u16 = 10;

/// The European cups the league phases replace.
const EUROPEAN_CUPS: usize = 3;

/// The clubs of a league phase (include/league_phase.h `LEAGUE_PHASE_CLUBS`).
const LEAGUE_PHASE_CLUBS: usize = 36;

/// World 2026-27's national and league cups (`kWorld2627Cups`).
const WORLD_CUPS: usize = 101;

/// Its league cups: those in slot 2.
const LEAGUE_CUPS: usize = 14;

/// The slot of a league cup (include/world_cups.h `WorldCup.slot`).
const LEAGUE_CUP_SLOT: u8 = 2;

/// The clubs of the Coppa Italia's real bracket (`kWorld2627Cups`).
const COPPA_ITALIA_CLUBS: usize = 16;

/// The clubs of the English cup's real bracket, past the record's 32
/// (`kWorld2627_englishCupExt`).
const ENGLISH_CUP_CLUBS: usize = 64;

/// The rule bytes of a cup record (include/game_types.h `CompetitionCup.roundRuleFlags`).
const ROUND_RULES: usize = 6;

/// The odd sizes `FixtureRounds_Install` rewrites (src/fixture_rounds.c `FIRST_ODD`,
/// `LAST_ODD`).
const FIRST_ROUNDS: u8 = 5;

/// The last of them.
const LAST_ROUNDS: u8 = 23;

/// The countries of World 2026-27's season end (`kWorld2627SeasonCountries`: 96/97's 65 and
/// its 26 new countries).
const SEASON_COUNTRIES: usize = 91;

#[test]
fn the_install_resizes_adds_renames_and_switches() {
    let baked = world_2627().expect("built");
    let range = |country: usize| baked.bases[country + 1] - baked.bases[country];
    assert_eq!(range(20), ITALY_CLUBS);
    assert_eq!(range(0), ALBANIA_CLUBS);
    let entry = |number: u8| {
        baked
            .countries
            .iter()
            .find(|(found, _)| *found == number)
            .map(|(_, entry)| entry.clone())
    };
    let andorra = entry(47).expect("a new country");
    assert_eq!(
        (andorra.continent, andorra.name.as_slice()),
        (0, &b"ANDORRA"[..])
    );
    assert_eq!(entry(41).expect("renamed").name, b"SERBIA");
    assert_eq!(baked.season_countries.len(), SEASON_COUNTRIES);
    assert!(baked.second_files.is_empty(), "none at the pin");
    assert!(baked.squad_numbers && baked.league_phases);
    // `FixtureRounds_Install`: the odd sizes 5 to 23 as real rounds, the same pairs in another
    // order; the even ones and 3 as they were.
    for schedule in &baked.schedules {
        let odd = schedule.teams % 2 == 1;
        let rewritten = odd && (FIRST_ROUNDS..=LAST_ROUNDS).contains(&schedule.teams);
        assert_eq!(schedule.rounds, !odd || rewritten, "{}", schedule.teams);
    }
    let five = baked.schedules.iter().find(|schedule| schedule.teams == 5);
    assert_eq!(
        five.map(|schedule| schedule.pairs.first().copied()),
        Some(Some((4, 1))),
        "the circle method: club 0 meets the dummy and rests, then 4 v 1, swapped by its place"
    );
    // `kWorld2627LeaguePhases`: the Champions League first, 36 clubs in 2 groups of 18.
    let [champions, ..] = baked.league_phase_cups.as_slice() else {
        panic!("three league phases");
    };
    assert_eq!(baked.league_phase_cups.len(), EUROPEAN_CUPS);
    assert_eq!((champions.comp.comp_id, champions.comp.kind), (0, 2));
    assert_eq!(champions.rounds.first(), Some(&[36, 2, 18]));
    assert_eq!(
        champions.names[0].as_deref(),
        Some(&b"UEFA CHAMPIONS LEAGUE"[..])
    );
    assert_eq!(champions.team_seed_list.len(), LEAGUE_PHASE_CLUBS);
    assert_eq!(champions.team_seed_list.first(), Some(&(1, 0)), "LASK");
    // `kWorld2627Cups`: the Coppa Italia first, its record `kWorld2627_italianCup`.
    let [italian, ..] = baked.cups.as_slice() else {
        panic!("the world's cups");
    };
    assert_eq!(baked.cups.len(), WORLD_CUPS);
    let league_cups = baked.cups.iter().filter(|cup| cup.slot == LEAGUE_CUP_SLOT);
    assert_eq!(league_cups.count(), LEAGUE_CUPS);
    assert_eq!((italian.country, italian.slot), (20, 1));
    let record = &italian.record;
    assert_eq!(
        (
            record.comp.comp_id,
            record.comp.kind,
            record.comp.team_number
        ),
        (132, 1, 20)
    );
    assert_eq!((record.teams, record.subs_and_points_for_win), (16, 53));
    assert_eq!(record.round_rule_flags, [84, 84, -108, 84, 0, 0]);
    assert_eq!(
        record.names,
        [Some(b"COPPA ITALIA".to_vec()), Some(b"COPPA".to_vec())]
    );
    assert_eq!(record.team_seed_list.len(), COPPA_ITALIA_CLUBS);
    assert_eq!(record.team_seed_list.first(), Some(&(20, 7)));
    let english = baked
        .cups
        .iter()
        .find(|cup| (cup.country, cup.slot) == (8, 1))
        .expect("the English cup");
    assert_eq!(english.record.comp.comp_id, 118);
    assert_eq!(english.record.round_rule_flags.len(), ROUND_RULES);
    assert_eq!(english.record.team_seed_list.len(), ENGLISH_CUP_CLUBS);
    assert_eq!(english.record.team_seed_list.first(), Some(&(8, 51)));
    assert_eq!(
        english.record.team_seed_list.last(),
        Some(&(8, 12)),
        "the list continues past the record"
    );
    assert_eq!(world_2627().expect("remembered"), baked, "installed once");
    let locked = std::panic::catch_unwind(|| drop(Oracle::lock()));
    assert!(
        locked.is_err(),
        "the oracle serves nothing else after a world's install"
    );
}
