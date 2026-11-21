//! A world's European cups as league phases (upstream src/league_phase.c
//! `LeaguePhase_InstallCups`). The world's three records join `tournaments.json`. The career
//! index's first three contests name them in place of 96/97's Champions', Cup Winners' and
//! UEFA Cups, and so does Europe's group (`_europeTable`), the one upstream patches. 96/97's
//! records stay: upstream's world modules copy them as templates, and the world does not play
//! them (`openswos_core::World::is_played`).

use std::collections::BTreeMap;

use openswos_competitions_json_resource::{
    CompCupEntry, CompetitionsDocument, CompetitionsFile, Round, SymbolId, Tournament,
    TournamentKind,
};
use openswos_oracle::tables::TournamentRecord;
use openswos_team_json_resource::TeamJsonDocument;

use super::records::{ID_PREFIX, header, names, round_rules, seeds, signed, subs_and_points};
use crate::error::ExtractError;

/// The tournaments' file.
const TOURNAMENTS: &str = "tournaments.json";

/// The career index's file.
const CAREER: &str = "career.json";

/// The qualifying groups' file.
const QUALIFYING: &str = "qualifying.json";

/// The group upstream points at the league phases: Europe's (src/league_phase.c,
/// `_europeTable.compCup`).
const EUROPE_GROUP: &str = "_europeTable";

/// Installs `records` (the world's league phases, the Champions', Cup Winners' and UEFA Cup's
/// places in that order) into `competitions`, their seeds named after their clubs in
/// `teams`. A world without league phases changes nothing.
pub(super) fn install(
    competitions: &mut BTreeMap<String, CompetitionsDocument>,
    records: &[TournamentRecord],
    teams: &BTreeMap<String, TeamJsonDocument>,
) -> Result<(), ExtractError> {
    if records.is_empty() {
        return Ok(());
    }
    let tournaments = records
        .iter()
        .map(|record| tournament(record, teams))
        .collect::<Result<Vec<_>, _>>()?;
    let ids: Vec<SymbolId> = tournaments
        .iter()
        .map(|tournament| tournament.id.clone())
        .collect();
    let Some(CompetitionsFile::Tournaments(file)) = competitions
        .get_mut(TOURNAMENTS)
        .map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(TOURNAMENTS));
    };
    if let Some(taken) = ids
        .iter()
        .find(|id| file.competitions.iter().any(|found| &found.id == *id))
    {
        return Err(ExtractError::IdTaken(taken.as_str().to_owned()));
    }
    file.competitions.extend(tournaments);
    file.count = file.competitions.len();
    let Some(CompetitionsFile::Career(career)) = competitions
        .get_mut(CAREER)
        .map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(CAREER));
    };
    let mut replaced = Vec::new();
    for (contest, id) in career.contests.iter_mut().zip(&ids) {
        replaced.push(contest.competition.replace(id.clone()));
    }
    let Some(CompetitionsFile::Qualifying(groups)) = competitions
        .get_mut(QUALIFYING)
        .map(|document| &mut document.file)
    else {
        return Err(ExtractError::NoCompetitionFile(QUALIFYING));
    };
    for group in groups
        .groups
        .iter_mut()
        .filter(|group| group.id.as_str() == EUROPE_GROUP)
    {
        for entry in &mut group.comp_cup {
            let CompCupEntry::Competition(named) = entry else {
                continue;
            };
            let position = replaced
                .iter()
                .position(|was| was.as_ref() == Some(&*named));
            if let Some(id) = position.and_then(|position| ids.get(position)) {
                *named = id.clone();
            }
        }
    }
    Ok(())
}

/// `record` as a tournament of `tournaments.json`, its id made of its name, its seeds named
/// after their clubs in `teams`.
fn tournament(
    record: &TournamentRecord,
    teams: &BTreeMap<String, TeamJsonDocument>,
) -> Result<Tournament, ExtractError> {
    let names = names(&record.names)?;
    let name = names
        .first()
        .cloned()
        .flatten()
        .ok_or(ExtractError::Unnamed)?;
    let (substitutes, points_for_win) = subs_and_points(record.subs_and_points_for_win);
    Ok(Tournament {
        id: id_of(&name),
        kind: TournamentKind::Tournament,
        comp: header(&record.comp)?,
        seeding_type: signed(record.seeding_type),
        menu_state: signed(record.menu_state),
        play_each_team: signed(record.play_each_team),
        away_goals_mode: signed(record.away_goals_mode),
        substitutes,
        points_for_win,
        round_string_base: signed(record.round_string_base),
        rounds: record
            .rounds
            .iter()
            .map(|&[teams, groups, a_group]| Round {
                team_count: signed(teams),
                group_count: signed(groups),
                team_group_count: signed(a_group),
            })
            .collect(),
        header_reserved: signed(record.reserved),
        round_rule_flags: round_rules(&record.round_rule_flags),
        name: names,
        team_seed_list: seeds(&record.team_seed_list, teams),
    })
}

/// An id made of `name`: an underscore, then its words, the first in lower case and every
/// other capitalised (`UEFA CHAMPIONS LEAGUE`: `_uefaChampionsLeague`).
fn id_of(name: &str) -> SymbolId {
    let mut id = String::from(ID_PREFIX);
    for (index, word) in name
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .enumerate()
    {
        let lower = word.to_ascii_lowercase();
        let mut characters = lower.chars();
        if index > 0
            && let Some(first) = characters.next()
        {
            id.push(first.to_ascii_uppercase());
            id.extend(characters);
        } else {
            id.push_str(&lower);
        }
    }
    SymbolId(id)
}

#[cfg(test)]
mod tests {
    use openswos_competitions_json_resource::CompetitionType;
    use openswos_test_utils::world_1996_97_path;

    use super::*;
    use crate::assemble::committed;
    use crate::complete::records::tests::{base, oceania};

    /// The Champions League's record as world_data.c compiles it, its list cut to two clubs.
    fn champions_league() -> TournamentRecord {
        TournamentRecord {
            comp: base(),
            seeding_type: 0,
            menu_state: 0,
            play_each_team: 1,
            away_goals_mode: 3,
            subs_and_points_for_win: 53,
            round_string_base: 6,
            rounds: vec![
                [36, 2, 18],
                [16, 0, 16],
                [16, 0, 16],
                [8, 0, 8],
                [4, 0, 4],
                [2, 0, 2],
            ],
            reserved: 1,
            round_rule_flags: vec![0, -108, -108, -108, -108, 20],
            names: vec![
                Some(b"UEFA CHAMPIONS LEAGUE".to_vec()),
                Some(b"CHAMPIONS LEAGUE".to_vec()),
            ],
            team_seed_list: vec![(85, 0), (85, 1)],
        }
    }

    #[test]
    fn an_id_is_made_of_a_name() {
        assert_eq!(
            id_of("UEFA CHAMPIONS LEAGUE").as_str(),
            "_uefaChampionsLeague"
        );
        assert_eq!(id_of("CONFERENCE LEAGUE").as_str(), "_conferenceLeague");
        assert_eq!(id_of("A-B. C").as_str(), "_aBC");
    }

    #[test]
    fn a_record_is_the_tournament_upstream_compiles() {
        let tournament = tournament(&champions_league(), &oceania()).expect("a tournament");
        assert_eq!(tournament.id.as_str(), "_uefaChampionsLeague");
        assert_eq!(tournament.comp.kind, CompetitionType::Tournament);
        assert_eq!(i16::from(tournament.comp.team_number), -1);
        assert_eq!(
            (
                u8::from(tournament.substitutes),
                u8::from(tournament.points_for_win)
            ),
            (3, 5),
            "53 is 0x35"
        );
        assert_eq!(tournament.rounds[0].team_count.byte(), 36);
        // -108 is 0x94: two legs, extra time, penalties.
        assert_eq!(u8::from(tournament.round_rule_flags[1]), 0x94);
        assert_eq!(
            tournament.name,
            [
                Some("UEFA CHAMPIONS LEAGUE".to_owned()),
                Some("CHAMPIONS LEAGUE".to_owned())
            ]
        );
        assert_eq!(
            tournament.team_seed_list[0].team.as_deref(),
            Some("AUSTRALIA")
        );
    }

    #[test]
    fn the_league_phases_take_the_european_cups_places() {
        let mut competitions = committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions;
        let mut records = vec![champions_league(); 3];
        for (record, name) in records.iter_mut().zip(["A", "B", "C"]) {
            record.names[0] = Some(name.as_bytes().to_vec());
        }
        install(&mut competitions, &records, &oceania()).expect("installed");
        let Some(CompetitionsFile::Career(career)) = competitions.get(CAREER).map(|d| &d.file)
        else {
            panic!("the career index");
        };
        let named: Vec<_> = career
            .contests
            .iter()
            .take(4)
            .map(|contest| contest.competition.as_ref().map(SymbolId::as_str))
            .collect();
        assert_eq!(
            named,
            [
                Some("_a"),
                Some("_b"),
                Some("_c"),
                Some("_invitationTournament")
            ]
        );
        let Some(CompetitionsFile::Qualifying(groups)) =
            competitions.get(QUALIFYING).map(|d| &d.file)
        else {
            panic!("the groups");
        };
        let europe = groups
            .groups
            .iter()
            .find(|group| group.id.as_str() == EUROPE_GROUP)
            .expect("Europe's group");
        let names: Vec<&str> = europe
            .comp_cup
            .iter()
            .filter_map(|entry| match entry {
                CompCupEntry::Competition(id) => Some(id.as_str()),
                CompCupEntry::Marker(_) => None,
            })
            .collect();
        assert_eq!(names, ["_europeanChampionships", "_a", "_b", "_c"]);
    }

    #[test]
    fn an_id_already_taken_is_refused() {
        let mut competitions = committed(&world_1996_97_path())
            .expect("the committed package")
            .competitions;
        let mut record = champions_league();
        record.names[0] = Some(b"EURO CUP".to_vec());
        let error = install(&mut competitions, &[record], &oceania()).expect_err("_euroCup");
        assert!(error.to_string().contains("_euroCup"), "{error}");
    }
}
