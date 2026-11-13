//! The trait.

use std::fmt::Debug;

use openswos_math::DeltaScale;

use crate::values::{BallPhysics, DIVE_DELTAS, Goalkeeping, STRONGER_KEEPER_STEP, Timers};

/// A release of the original game whose code or data a style plays by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Release {
    /// The Amiga's, the definitive release.
    Amiga,
    /// The PC's.
    Dos,
}

/// Whether the stronger keepers tweak is on (`[tweaks] stronger_goalkeeper_ai`, SWOS 2020's
/// `--stronger-goalkeeper-ai`). Upstream turns every tweak off online and in its rigs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum KeeperReach {
    /// The style's own reach.
    #[default]
    Standard,
    /// The Amiga's goalkeepers reach a step further.
    Stronger,
}

/// How a match plays: the constants and the code paths that differ between the game's
/// releases and SWOS 2020's styles (upstream src/dos_gameplay.c: `ApplyGameStyle`, which
/// writes the constants at kick-off, `Gameplay_SetStyle` and `DosGameplay_TargetFps`). A
/// style is a stateless unit struct; the match holds a `&'static dyn GameStyle`, from the
/// registry ([`crate::by_id`], [`crate::by_code`]).
pub trait GameStyle: Debug + Sync {
    /// The stable id the settings store (`[gameplay] style`).
    fn id(&self) -> &'static str;

    /// Upstream's number for it (`GAMEPLAY_*`), which recordings and the online game carry:
    /// never renumbered.
    fn code(&self) -> u8;

    /// The name the game shows (`Gameplay_StyleName`).
    fn name(&self) -> &'static str;

    /// The match's frames a second (`DosGameplay_TargetFps`).
    fn frames_per_second(&self) -> u16;

    /// The release whose code the match runs where the releases' code differs (upstream
    /// `_gDosGameplay`).
    fn release(&self) -> Release;

    /// The release whose goalkeeper code the match runs (`_gDosKeepers`).
    fn goalkeepers(&self) -> Release;

    /// The release whose formations the teams use (`TacticsData_SelectRelease`).
    fn formations(&self) -> Release;

    /// The goalkeepers' constants.
    fn goalkeeping(&self) -> &'static Goalkeeping;

    /// The ball's physics.
    fn ball(&self) -> &'static BallPhysics;

    /// The match clock and the stoppage timers.
    fn timers(&self) -> &'static Timers;

    /// The scale of every sprite's step, so a game running at more frames a second does not
    /// play faster (`_gMoveDeltaScaleNum`, `_gMoveDeltaScaleDen`).
    fn delta_scale(&self) -> DeltaScale;

    /// The goalkeeper's reach table with the stronger keepers tweak applied: a step lower
    /// with the Amiga's goalkeepers, unchanged with the DOS game's (upstream
    /// `ApplyGameStyle`).
    fn dive_deltas(&self, reach: KeeperReach) -> [u32; DIVE_DELTAS] {
        let lower = match (self.goalkeepers(), reach) {
            (Release::Amiga, KeeperReach::Stronger) => STRONGER_KEEPER_STEP,
            (Release::Amiga, KeeperReach::Standard)
            | (Release::Dos, KeeperReach::Standard | KeeperReach::Stronger) => 0,
        };
        self.goalkeeping().dive_deltas.map(|delta| delta - lower)
    }
}

#[cfg(test)]
mod tests {
    use super::{GameStyle, KeeperReach};
    use crate::styles::{Amiga50Hz, Amiga60Hz, Dos70Hz, Moon50Hz, Xbox60Hz};
    use crate::values::STRONGER_KEEPER_STEP;

    #[test]
    fn the_stronger_keepers_reach_further_with_the_amigas_keepers() {
        let amiga_keepers: [&dyn GameStyle; 3] = [&Amiga50Hz, &Amiga60Hz, &Moon50Hz];
        for style in amiga_keepers {
            let standard = style.dive_deltas(KeeperReach::Standard);
            assert_eq!(standard, style.goalkeeping().dive_deltas, "{}", style.id());
            let stronger = style.dive_deltas(KeeperReach::Stronger);
            assert_eq!(stronger, standard.map(|delta| delta - STRONGER_KEEPER_STEP));
        }
        let dos_keepers: [&dyn GameStyle; 2] = [&Dos70Hz, &Xbox60Hz];
        for style in dos_keepers {
            let stronger = style.dive_deltas(KeeperReach::Stronger);
            assert_eq!(stronger, style.goalkeeping().dive_deltas, "{}", style.id());
        }
    }
}
