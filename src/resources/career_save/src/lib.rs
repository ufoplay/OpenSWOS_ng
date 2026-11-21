#![doc = include_str!("../readme.md")]
// Resource crates handle untrusted data: every access is checked.
#![deny(clippy::indexing_slicing)]

pub mod competitions;
pub mod contests;
pub mod continuation;
pub mod flat;
pub mod offers;
pub mod player;
pub mod pools;
pub mod record;
pub mod save;
pub mod scalars;
pub mod season;
pub mod signed;
pub mod slots;
pub mod supercups;
pub mod team;
#[cfg(test)]
mod testing;
pub mod uefa;
pub mod world_career;
pub mod world_cups;

pub use competitions::{CompCopyDynamic, CompetitionOverrides, LeagueRow, LeagueSplit};
pub use contests::{LeagueStage, Promedio, SeasonWinners, StageTitles, WorldPlayoff};
pub use continuation::ExactContinuation;
pub use offers::{JOB_OFFER_TEAM_NAME_LEN, JobOffer, TransferOffer};
pub use player::{MarketPlayer, SavePlayer};
pub use pools::{StatePools, TeamPool};
pub use record::RecordTextError;
pub use save::{
    CUP_PICK_LIST_LEN, CareerKind, CareerSave, CareerSaveError, CareerSaveFormat, NATIONAL_FILES,
};
pub use scalars::{CareerScalars, SaveTactics};
pub use season::SeasonInformation;
pub use slots::{
    CompSlots, CompetitionSlot, CupSlot, LeagueSlot, OtherSlot, SlotData, SlotHeader, StageRow,
    StageStandingsRow, TournamentSlot, TournamentStage,
};
pub use supercups::{CupMatch, CupRecord, EuroTitleHolders, WorldSuperCup};
pub use team::SaveTeam;
pub use uefa::{ASSOCIATIONS, QUALIFYING_LIST_LEN, UefaCoef, UefaQualifying};
pub use world_career::{
    BTeamCall, BTeamCalls, CpuReplacements, DepartedPlayer, Ffp, SquadExtras, UefaPrizes,
    UsedSquad, WorldRules,
};
pub use world_cups::{ContiEdition, ContiHolders, WorldCupD, WorldCups};
