//! What a world's career keeps of the SWOS 96/97 Mod 4.1's rules: its UEFA prize money and
//! financial fair play (upstream include/world_finance.h), its CPU replacements
//! (include/cpu_repl.h), its squads of 30 (include/squad_extras.h), its B-team call-ups
//! (include/bteam.h) and its match rules (include/world_rules.h). Written by every world's
//! career but World 1996-97's.
//!
//! What the numbers mean is the career's (M7); they are kept as written, in their fields' C
//! widths. Lists keep the length written.

use openswos_save_json_resource::SaveString;
use serde::{Deserialize, Serialize};

/// The UEFA prize money paid to the career club this season (upstream `UefaPrizeState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UefaPrizes {
    /// 0: an older save, its first event counts as paid what is reached.
    pub ok: u8,
    /// The cup paid (0..2).
    pub cup: u8,
    /// The season stamp paid.
    pub season: u8,
    /// 1 participation, 2 won, 4 the ranking pillar.
    pub flags: u8,
    /// The league-phase wins paid.
    pub wins: u8,
    /// The league-phase draws paid.
    pub draws: u8,
    /// The furthest knockout stage paid (1: the play-offs).
    pub stage: u8,
    /// 1: the prizes switched off.
    pub off: u8,
}

/// The financial fair play (upstream `FfpState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ffp {
    /// The rule on for this career.
    pub on: u8,
    /// Sanctioned this season: purchases refused while over the limit.
    pub block: u8,
    /// The income covers the whole season (0: an older save loaded mid-season).
    pub valid: u8,
    /// The season's gates and TV base so far.
    pub income: i32,
    /// Last season's income (0: none of ours).
    pub last_income: i32,
    /// The fine taken at this season's start.
    pub fine: i32,
    /// The season's match charges.
    pub charges: u16,
}

/// The CPU replacements (upstream `CpuReplState`): whether a club sold from replaces its
/// player from its squad's players 17.., and whose were used.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuReplacements {
    /// 1: FROM SQUAD; 0: INVENTED (the game's).
    pub from_squad: u8,
    /// The clubs whose squad players were used (at most 64, upstream `CPU_REPL_CLUBS`).
    pub used: Vec<UsedSquad>,
}

/// A club whose squad players replaced some sold. Written `[team, mask]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u16, u16)", into = "(u16, u16)")]
pub struct UsedSquad {
    /// The club, as the game packs it (file << 8 | ordinal).
    pub team: u16,
    /// Its squad players 17.. already used (bit 0: the 17th).
    pub mask: u16,
}

impl From<(u16, u16)> for UsedSquad {
    fn from((team, mask): (u16, u16)) -> Self {
        Self { team, mask }
    }
}

impl From<UsedSquad> for (u16, u16) {
    fn from(UsedSquad { team, mask }: UsedSquad) -> Self {
        (team, mask)
    }
}

/// The squads of 30 (upstream `SqxState`): the clubs' players 17.. gone from them, so they
/// are never offered or brought back again (the Mod's lists XS and XL).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SquadExtras {
    /// The next entry to overwrite once the list is full.
    pub ring: u16,
    /// The players gone (at most 256, upstream `SQX_LEFT_MAX`).
    pub left: Vec<DepartedPlayer>,
}

/// A club's player 17.. gone from it. Written `[team, index, name]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u16, u8, SaveString)", into = "(u16, u8, SaveString)")]
pub struct DepartedPlayer {
    /// The club, as the game packs it (file << 8 | ordinal).
    pub team: u16,
    /// His place - 1 (16..).
    pub index: u8,
    /// His name (the place is checked against it).
    pub name: SaveString,
}

impl From<(u16, u8, SaveString)> for DepartedPlayer {
    fn from((team, index, name): (u16, u8, SaveString)) -> Self {
        Self { team, index, name }
    }
}

impl From<DepartedPlayer> for (u16, u8, SaveString) {
    fn from(DepartedPlayer { team, index, name }: DepartedPlayer) -> Self {
        (team, index, name)
    }
}

/// The B-team call-ups (upstream `BTeamState`, the Mod's item BU).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BTeamCalls {
    /// The players called up (at most 10, upstream `BTEAM_MAX`).
    pub called: Vec<BTeamCall>,
}

/// A player called up from the career club's B team. Written `[kind, index, team, name]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "(u8, u8, u16, SaveString)", into = "(u8, u8, u16, SaveString)")]
pub struct BTeamCall {
    /// Bits 0-1: 1 next match, 2 the season; bit 3: over age; bits 4-5: a season call-up's
    /// window (1..).
    pub kind: u8,
    /// His place in the B team: 0..15 its record, 16.. its players 17...
    pub index: u8,
    /// The B team, as the game packs it (file << 8 | ordinal).
    pub team: u16,
    /// His name (his career squad slot is found by it).
    pub name: SaveString,
}

impl From<(u8, u8, u16, SaveString)> for BTeamCall {
    fn from((kind, index, team, name): (u8, u8, u16, SaveString)) -> Self {
        Self {
            kind,
            index,
            team,
            name,
        }
    }
}

impl From<BTeamCall> for (u8, u8, u16, SaveString) {
    fn from(
        BTeamCall {
            kind,
            index,
            team,
            name,
        }: BTeamCall,
    ) -> Self {
        (kind, index, team, name)
    }
}

/// The match rules kept with a world's career (upstream `WorldRules_CareerBits`: the Mod's
/// CPU RED CARDS, CPU INJURIES and GOALKEEPER INJURIES) and the keepers' injuries' generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldRules {
    /// The computer's players are booked and sent off: 1 on.
    pub cpu_red_cards: u8,
    /// The computer's players are injured.
    pub cpu_injuries: u8,
    /// Goalkeepers are injured.
    pub goalkeeper_injuries: u8,
    /// The keepers' injuries' generator's seed (signed, as the Amiga's `long`).
    pub gk_seed: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{rejects, round_trip};

    #[test]
    fn the_money_round_trips() {
        let prizes: UefaPrizes = round_trip(
            r#"{"ok":1,"cup":0,"season":2,"flags":5,"wins":3,"draws":1,"stage":2,"off":0}"#,
        );
        assert_eq!(prizes.flags, 5);
        let ffp: Ffp = round_trip(
            r#"{"on":1,"block":0,"valid":1,"income":-5,"last_income":2147483647,"fine":0,"charges":7}"#,
        );
        assert_eq!(ffp.income, -5);
        assert!(rejects::<Ffp>(
            r#"{"on":1,"block":0,"valid":1,"income":0,"last_income":0,"fine":0,"charges":65536}"#
        ));
    }

    #[test]
    fn the_squads_lists_round_trip() {
        let replacements: CpuReplacements =
            round_trip(r#"{"from_squad":1,"used":[[5123,3],[300,1]]}"#);
        assert_eq!(
            replacements.used.first(),
            Some(&UsedSquad {
                team: 5123,
                mask: 3
            })
        );
        let extras: SquadExtras = round_trip(r#"{"ring":0,"left":[[5123,17,"MARIO ROSSI"]]}"#);
        assert_eq!(
            extras.left.first().map(|player| player.name.clone()),
            Some(SaveString::from("MARIO ROSSI"))
        );
        let calls: BTeamCalls = round_trip(r#"{"called":[[1,16,5164,"LUCA BIANCHI"]]}"#);
        assert_eq!(calls.called.first().map(|call| call.team), Some(5164));
        assert!(rejects::<SquadExtras>(r#"{"ring":0,"left":[[5123,17]]}"#));
    }

    #[test]
    fn the_match_rules_round_trip() {
        let rules: WorldRules = round_trip(
            r#"{"cpu_red_cards":1,"cpu_injuries":0,"goalkeeper_injuries":1,"gk_seed":-1}"#,
        );
        assert_eq!(rules.gk_seed, -1);
    }
}
