//! The world's texts (`rules/strings.json`): the strings its names refer to, as the bytes the
//! game draws, and the names a league, a cup and a tournament fall back on. A name is the text of the symbol it
//! names, or itself when it names none (upstream save_format_json.c `dj_intern_names`).

use std::collections::BTreeMap;

use openswos_competitions_json_resource::Names;
use openswos_world_rules_resource::Strings;

use super::rules::at;
use crate::validation::{Field, Location, Problem, Problems};

/// The byte that ends a text, which no text holds.
const NUL: u8 = 0;

/// A name's bytes: none for a null name.
pub type NameBytes = Option<Vec<u8>>;

/// The world's texts, as bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Texts {
    /// Each symbol's text, by the symbol's name.
    symbols: BTreeMap<String, Vec<u8>>,
    /// The name a league's reset writes first.
    league_reset_name: Vec<u8>,
    /// The name a cup's reset writes first.
    cup_reset_name: Vec<u8>,
    /// The name a tournament's reset writes first.
    tournament_reset_name: Vec<u8>,
    /// Each division's default name, by index.
    division_names: Vec<Vec<u8>>,
    /// A cup's default name.
    cup_name: Vec<u8>,
    /// A tournament's default name.
    tournament_name: Vec<u8>,
    /// A cup's rounds' names, by index.
    cup_round_names: Vec<Vec<u8>>,
}

impl Texts {
    /// The texts of `strings`: each a byte a character. A text that is not is reported, and
    /// left out (as the empty name for a default).
    pub(crate) fn build(strings: &Strings, problems: &mut Problems) -> Self {
        let mut symbols = BTreeMap::new();
        for (symbol, text) in &strings.symbols {
            let location = || at::<Strings>(&Field::key("symbols").then(symbol));
            if let Some(bytes) = report(bytes(text), location, problems) {
                symbols.insert(symbol.clone(), bytes);
            }
        }
        let mut name = |text: &str, field: &Field| {
            report(resolve(&symbols, text), || at::<Strings>(field), problems).unwrap_or_default()
        };
        let league_reset_name = name(&strings.league_reset_name, &Field::key("league_reset_name"));
        let cup_reset_name = name(&strings.cup_reset_name, &Field::key("cup_reset_name"));
        let tournament_reset_name = name(
            &strings.tournament_reset_name,
            &Field::key("tournament_reset_name"),
        );
        let division_field = Field::key("division_names");
        let division_names = (0..)
            .zip(&strings.division_names)
            .map(|(index, text)| name(text, &division_field.index(index)))
            .collect();
        let cup_name = name(&strings.cup_name, &Field::key("cup_name"));
        let tournament_name = name(&strings.tournament_name, &Field::key("tournament_name"));
        let round_field = Field::key("cup_round_names");
        let cup_round_names = (0..)
            .zip(&strings.cup_round_names)
            .map(|(index, text)| name(text, &round_field.index(index)))
            .collect();
        Self {
            symbols,
            league_reset_name,
            cup_reset_name,
            tournament_reset_name,
            division_names,
            cup_name,
            tournament_name,
            cup_round_names,
        }
    }

    /// The bytes of `name`: its symbol's text, or its own.
    ///
    /// # Errors
    ///
    /// [`Problem::Unrepresentable`] for a name, not a symbol, with a character beyond a byte.
    pub(crate) fn resolve(&self, name: &str) -> Result<Vec<u8>, Problem> {
        resolve(&self.symbols, name)
    }

    /// A record's names, resolved: `None` (with the problems reported) when one does not.
    pub(crate) fn names(
        &self,
        names: &Names,
        location: impl Fn(usize) -> Location,
        problems: &mut Problems,
    ) -> Option<Vec<NameBytes>> {
        let resolved: Vec<Option<NameBytes>> = names
            .iter()
            .enumerate()
            .map(|(index, name)| match name {
                Some(name) => report(self.resolve(name), || location(index), problems).map(Some),
                None => Some(None),
            })
            .collect();
        resolved.into_iter().collect()
    }

    /// The name a league's reset writes first (upstream `_aDiyLeague`).
    #[must_use]
    pub fn league_reset_name(&self) -> &[u8] {
        &self.league_reset_name
    }

    /// The name a cup's reset writes first (upstream `_aDiyCup`).
    #[must_use]
    pub fn cup_reset_name(&self) -> &[u8] {
        &self.cup_reset_name
    }

    /// The name a tournament's reset writes first (upstream `_aDiyTournament`).
    #[must_use]
    pub fn tournament_reset_name(&self) -> &[u8] {
        &self.tournament_reset_name
    }

    /// The default name of division `division` (upstream `_divisionStringTable`), if the
    /// world has one.
    #[must_use]
    pub fn division_name(&self, division: usize) -> Option<&[u8]> {
        self.division_names.get(division).map(Vec::as_slice)
    }

    /// A cup's name when its record names none (upstream `_aCup_0`).
    #[must_use]
    pub fn cup_name(&self) -> &[u8] {
        &self.cup_name
    }

    /// A tournament's name when its record names none (upstream `_aLeague_1`).
    #[must_use]
    pub fn tournament_name(&self) -> &[u8] {
        &self.tournament_name
    }

    /// The cup round name of index `index` ([`round_name`](crate::competition::cup::round_name),
    /// [`stage_name`](crate::competition::cup::stage_name); upstream `_cupRoundStringTable`), if
    /// the world has one.
    #[must_use]
    pub fn cup_round_name(&self, index: u16) -> Option<&[u8]> {
        self.cup_round_names
            .get(usize::from(index))
            .map(Vec::as_slice)
    }
}

/// The bytes of `name` among `symbols`: its symbol's text, or its own.
fn resolve(symbols: &BTreeMap<String, Vec<u8>>, name: &str) -> Result<Vec<u8>, Problem> {
    match symbols.get(name) {
        Some(text) => Ok(text.clone()),
        None => bytes(name),
    }
}

/// The bytes of `text`, a character each.
///
/// # Errors
///
/// [`Problem::Unrepresentable`] for a character beyond a byte, or NUL.
pub(crate) fn bytes(text: &str) -> Result<Vec<u8>, Problem> {
    text.chars()
        .map(|character| {
            u8::try_from(character)
                .ok()
                .filter(|&byte| byte != NUL)
                .ok_or(Problem::Unrepresentable { character })
        })
        .collect()
}

/// `result`'s value, or `None` with its problem reported at `location`.
fn report<T>(
    result: Result<T, Problem>,
    location: impl FnOnce() -> Location,
    problems: &mut Problems,
) -> Option<T> {
    result
        .map_err(|problem| problems.push(location(), problem))
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strings of `symbols`, the league's reset name `reset` and default names `divisions`,
    /// and the cup's and the tournament's reset names and default names `others`.
    fn strings(
        symbols: &[(&str, &str)],
        reset: &str,
        divisions: &[&str],
        others: [&str; 4],
    ) -> Strings {
        let [cup_reset, cup_name, tournament_reset, tournament_name] = others;
        Strings {
            symbols: symbols
                .iter()
                .map(|&(name, text)| (name.to_owned(), text.to_owned()))
                .collect(),
            league_reset_name: reset.to_owned(),
            cup_reset_name: cup_reset.to_owned(),
            tournament_reset_name: tournament_reset.to_owned(),
            division_names: divisions.iter().map(|&name| name.to_owned()).collect(),
            cup_name: cup_name.to_owned(),
            tournament_name: tournament_name.to_owned(),
            cup_round_names: vec!["_aDivision1".to_owned(), "FINAL".to_owned()],
        }
    }

    fn built(strings: &Strings) -> Result<Texts, String> {
        let mut problems = Problems::default();
        let texts = Texts::build(strings, &mut problems);
        problems
            .finish(Some(texts))
            .map_err(|invalid| invalid.to_string())
    }

    #[test]
    fn a_name_is_its_symbols_text_or_itself() {
        let texts = built(&strings(
            &[
                ("_aDivision1", "DIVISION 1"),
                ("_aDiyLeague", "DIY LEAGUE"),
                ("_aCup_0", "CUP"),
            ],
            "_aDiyLeague",
            &["_aDivision1", "SOUTH"],
            ["DIY CUP", "_aCup_0", "DIY TOURNAMENT", "LEAGUE"],
        ))
        .expect("valid");
        assert_eq!(texts.resolve("_aDivision1"), Ok(b"DIVISION 1".to_vec()));
        assert_eq!(texts.resolve("SERIE A"), Ok(b"SERIE A".to_vec()));
        // Not a symbol of the world: the name itself, as upstream's.
        assert_eq!(texts.resolve("_aSerieA"), Ok(b"_aSerieA".to_vec()));
        assert_eq!(texts.league_reset_name(), b"DIY LEAGUE");
        assert_eq!(texts.cup_reset_name(), b"DIY CUP");
        assert_eq!(texts.cup_name(), b"CUP");
        assert_eq!(texts.tournament_reset_name(), b"DIY TOURNAMENT");
        assert_eq!(texts.tournament_name(), b"LEAGUE");
        assert_eq!(texts.cup_round_name(0), Some(&b"DIVISION 1"[..]));
        assert_eq!(texts.cup_round_name(1), Some(&b"FINAL"[..]));
        assert_eq!(texts.cup_round_name(2), None);
        assert_eq!(texts.division_name(0), Some(&b"DIVISION 1"[..]));
        assert_eq!(texts.division_name(1), Some(&b"SOUTH"[..]));
        assert_eq!(texts.division_name(2), None);
    }

    #[test]
    fn a_text_is_a_byte_a_character() {
        assert_eq!(bytes("ÉIRE"), Ok(vec![0xC9, b'I', b'R', b'E']));
        assert_eq!(bytes("Ł"), Err(Problem::Unrepresentable { character: 'Ł' }));
        assert_eq!(
            bytes("A\0"),
            Err(Problem::Unrepresentable { character: '\0' })
        );
    }

    #[test]
    fn a_text_beyond_a_byte_is_reported_where_it_is() {
        let error = built(&strings(
            &[("_aCity", "ŁÓDŹ")],
            "DIY ✓",
            &["ok", "Ω"],
            ["DIY CUP ✓", "Ω", "Ω", "Ω"],
        ))
        .expect_err("seven texts beyond a byte");
        assert!(
            error.contains("rules/strings.json: symbols._aCity"),
            "{error}"
        );
        for field in [
            "league_reset_name",
            "cup_reset_name",
            "division_names[1]",
            "cup_name",
            "tournament_reset_name",
            "tournament_name",
        ] {
            let location = format!("rules/strings.json: {field}");
            assert!(error.contains(&location), "{field}: {error}");
        }
    }

    #[test]
    fn a_records_names_resolve_or_are_reported() {
        let texts = built(&strings(&[("_aCup", "CUP")], "", &[], ["", "", "", ""])).expect("valid");
        let names = vec![Some("_aCup".to_owned()), None, Some("COPA".to_owned())];
        let mut problems = Problems::default();
        let resolved = texts.names(&names, |_| Location::file("x"), &mut problems);
        assert_eq!(
            resolved,
            Some(vec![Some(b"CUP".to_vec()), None, Some(b"COPA".to_vec())])
        );
        let names = vec![Some("Ω".to_owned())];
        assert_eq!(
            texts.names(&names, |_| Location::file("x"), &mut problems),
            None
        );
        assert!(problems.finish(Some(())).is_err());
    }
}
