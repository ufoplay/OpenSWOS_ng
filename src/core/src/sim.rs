//! Results of matches nobody plays (docs/PLAN.md M5 #8; upstream src/match_sim.c): a team in a
//! match ([`MatchTeam`]), its players' match values and its strength ([`match_value`],
//! [`strength`], M5 #8a), its standing in its competition and its form on the day
//! ([`price_rank`], [`form`]), its players' availability and the status buckets a match gives
//! them ([`available_players`], [`status_buckets`], M5 #8b), the simulated result, its extra
//! time and its shoot-out ([`simulate`], [`extra_time`], [`penalties`], M5 #8c), by the engine's
//! [`tables`]; the countdown of the teams' statuses before a match ([`countdown`]) and a
//! competition's matches simulated, the squads kept ([`Simulation`], M5 #8d).

mod countdown;
mod formation;
mod play;
mod result;
mod stand_ins;
mod standing;
mod status;
pub mod tables;
mod team;
mod value;

pub use countdown::countdown;
pub use formation::{FORMATIONS, defensive_order, formation_orders};
pub use play::{Simulation, Squads};
pub use result::{Session, Simulated, extra_time, penalties, simulate};
pub use stand_ins::{NONE, StandIns, stand_ins};
pub use standing::{Day, Form, form, price_rank};
pub use status::{
    Availability, ComputerStatuses, GameType, SimRules, available_keepers, available_players,
    status_buckets,
};
pub use team::{
    CAREER_LINE_UP, CAREER_RECORD_LEN, CareerLineUp, MatchTeam, PLAYER_LEN, PLAYERS, RECORD_LEN,
};
pub use value::{
    MatchValue, SimError, StatusRules, match_value, nearness, player_price, strength, unavailable,
};
