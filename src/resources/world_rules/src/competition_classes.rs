//! `rules/competition_classes.json`: the classes of competition the engine treats apart, each
//! a list of the competitions in it (upstream's lists of records, src/swos_amiga_ptr_imports.c,
//! and the ids upstream names in code).

use openswos_competitions_json_resource::SymbolId;
use serde::{Deserialize, Serialize};

use crate::file::RulesFile;

/// `rules/competition_classes.json`. Each list names competitions by id, or the copy a career
/// plays one in by the copy's name (`career.json`'s `competition_copy`): upstream's lists name
/// a record and its copy apart.
///
/// ```json
/// {
///   "national_teams": ["_worldCup", "_worldCup2Copy"],
///   "continental": ["_worldCup", "_euroCup", "_euroCupCopy", "_copaLibertadores"]
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompetitionClasses {
    /// The national teams' competitions: their names take no country before them (upstream
    /// diy_competition.c `IsNationalTeamCompetitionOrQualification`,
    /// `_nationalTeamCompetitionsAndQualifications1`).
    pub national_teams: Vec<SymbolId>,
    /// The contests whose suspensions are the continental track's, apart from the domestic
    /// one's (upstream diy_competition.c `continental_record`:
    /// `_europeanCupsandNationalTeamCompetitionsAndQualifications`, and the ids it names).
    pub continental: Vec<SymbolId>,
}

impl RulesFile for CompetitionClasses {
    const FILE_NAME: &'static str = "competition_classes.json";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::{read, write};

    /// A file of a class each, as this crate writes it.
    const TEXT: &str = "{\n  \"national_teams\": [\n    \"_worldCup2Copy\"\n  ],\n  \
        \"continental\": [\n    \"_euroCup\",\n    \"_copaLibertadores\"\n  ]\n}\n";

    #[test]
    fn it_reads_and_writes_back() {
        let classes: CompetitionClasses = read(TEXT.as_bytes()).expect("classes");
        let id = |text: &str| SymbolId(text.to_owned());
        assert_eq!(classes.national_teams, [id("_worldCup2Copy")]);
        assert_eq!(
            classes.continental,
            [id("_euroCup"), id("_copaLibertadores")]
        );
        assert_eq!(write(&classes), TEXT.as_bytes());
    }

    #[test]
    fn an_unknown_class_does_not_read() {
        let error = read::<CompetitionClasses>(
            br#"{"national_teams": [], "continental": [], "domestic": []}"#,
        )
        .expect_err("no such class");
        assert!(error.to_string().contains("domestic"), "{error}");
    }
}
