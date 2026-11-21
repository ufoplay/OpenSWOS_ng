//! World 2026-27's rosters, from its package (downloaded into `target/cache/`, D8): every
//! `teams/*.json` round-trips byte for byte (the clubs' and the national teams' dialects),
//! converts to typed teams and back (their players 17 on and years of birth too, a name longer
//! than its field cut as the game cuts it), and states the checksums its data gives. Skipped
//! when the package can be neither found in the cache nor downloaded.

use std::collections::BTreeSet;
use std::io::Read;

use openswos_common::{Exporter, Importer};
use openswos_team_json_resource::{
    EntryIds, Indent, PlayerKeyOrder, TeamEntry, TeamJsonFormat, team_checksum,
};
use openswos_team_resource::{COACH_NAME_LEN, PLAYER_NAME_LEN, TEAM_NAME_LEN};
use openswos_test_utils::cache::{WORLD_2627_PACKAGE, fetch};

/// The folder of the rosters in the package.
const TEAMS_DIR: &str = "/teams/";

/// The rosters of the package: (file name, bytes).
fn rosters() -> Option<Vec<(String, Vec<u8>)>> {
    let path = match fetch(&WORLD_2627_PACKAGE) {
        Ok(path) => path,
        Err(skip) => {
            openswos_test_utils::skip::report(&skip);
            return None;
        }
    };
    let file = std::fs::File::open(&path).expect("the cached package");
    let mut archive = zip::ZipArchive::new(file).expect("a zip");
    let mut rosters = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("a zip entry");
        let name = entry.name().expect("a readable entry name").into_owned();
        if name.contains(TEAMS_DIR) && name.ends_with(".json") {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("an entry's bytes");
            rosters.push((name, bytes));
        }
    }
    assert!(!rosters.is_empty(), "no roster in {}", path.display());
    Some(rosters)
}

#[test]
fn every_roster_round_trips_byte_for_byte() {
    let Some(rosters) = rosters() else { return };
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        let exported =
            TeamJsonFormat::export(&document).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(
            exported == *bytes,
            "{name}: the export differs from the file"
        );
    }
}

/// `text` as the game's loader reads it into a field of `len` bytes.
fn cut(text: &mut String, len: usize, cut_names: &mut usize) {
    if text.chars().count() > len {
        *text = text.chars().take(len).collect();
        *cut_names += 1;
    }
}

/// `entry` as the typed model holds it: its names cut to their fields. Its players' key
/// orders are the text's, which the typed model does not hold: `back` gets them.
fn as_typed(entry: &TeamEntry, back: &mut TeamEntry, cut_names: &mut usize) -> TeamEntry {
    let mut expected = entry.clone();
    cut(&mut expected.team.team_name, TEAM_NAME_LEN, cut_names);
    cut(&mut expected.team.coach_name, COACH_NAME_LEN, cut_names);
    let players = expected
        .players
        .iter_mut()
        .chain(expected.extra_players.iter_mut().flatten());
    let back_players = back
        .players
        .iter_mut()
        .chain(back.extra_players.iter_mut().flatten());
    for (player, back_player) in players.zip(back_players) {
        cut(&mut player.player_name, PLAYER_NAME_LEN, cut_names);
        back_player.key_order = player.key_order;
    }
    expected
}

#[test]
fn every_team_converts_to_typed_and_back() {
    let Some(rosters) = rosters() else { return };
    let (mut teams, mut cut_names) = (0, 0);
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let team = entry
                .to_team()
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let extras = entry
                .extras()
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            let mut back = TeamEntry::from_team(&team, EntryIds::of(entry)).with_extras(&extras);
            let expected = as_typed(entry, &mut back, &mut cut_names);
            assert_eq!(back, expected, "{name}: {}", team.name);
            teams += 1;
        }
    }
    eprintln!(
        "{} rosters, {teams} teams: all convert both ways ({cut_names} names cut to their field)",
        rosters.len()
    );
}

/// What World 2026-27's rosters hold beyond the record: players 17 on (in no checksum), years
/// of birth, looks; the new national teams' players written face first; every file indented
/// by one space (the sealed package: `scripts/worldgen/seal.py`).
#[test]
fn the_world_adds_squads_years_and_looks_and_its_national_teams_dialect() {
    let Some(rosters) = rosters() else { return };
    let (mut extras, mut born, mut looks, mut face_first, mut one_space) = (0, 0, 0, 0, 0);
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        if document.indent == Indent::One {
            one_space += 1;
        }
        for entry in &document.file.teams {
            let players: Vec<_> = entry
                .players
                .iter()
                .chain(entry.extra_players.iter().flatten())
                .collect();
            extras += entry.extra_players.as_ref().map_or(0, Vec::len);
            born += players
                .iter()
                .filter(|player| player.born.is_some())
                .count();
            looks += players
                .iter()
                .filter(|player| player.looks.is_some())
                .count();
            face_first += players
                .iter()
                .filter(|player| player.key_order == PlayerKeyOrder::FaceFirst)
                .count();
            assert!(
                players
                    .iter()
                    .filter_map(|player| player.born)
                    .all(|year| year.known().is_some()),
                "{name}: a year of birth the game would not take"
            );
        }
    }
    eprintln!(
        "{extras} players 17 on, {born} years of birth, {looks} looks, {face_first} players face \
         first, {one_space} files indented by one space"
    );
    assert!(extras > 0 && born > 0 && looks > 0 && face_first > 0);
    assert_eq!(one_space, rosters.len());
}

#[test]
fn every_team_states_the_checksum_its_data_gives() {
    let Some(rosters) = rosters() else { return };
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let team = entry
                .to_team()
                .unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(
                team_checksum(&team),
                entry.checksum,
                "{name}: {}",
                team.name
            );
        }
    }
}

#[test]
/// World 2026-27's ids are `uuid5(namespace, "2627/team/<club id>")` with the SWOS 96/97 Mod's
/// own club ids (upstream `scripts/world_import.py`), which the JSON does not carry: they can
/// only be read, never recomputed. Every one names a single team or player.
fn every_id_is_unique() {
    let Some(rosters) = rosters() else { return };
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for (name, bytes) in &rosters {
        let document =
            TeamJsonFormat::import(bytes).unwrap_or_else(|error| panic!("{name}: {error}"));
        for entry in &document.file.teams {
            let entry_ids = EntryIds::of(entry);
            for id in std::iter::once(entry_ids.team).chain(entry_ids.players) {
                count += 1;
                assert!(ids.insert(id), "{name}: {id} names two teams or players");
            }
        }
    }
    eprintln!("{count} ids, all distinct");
}
