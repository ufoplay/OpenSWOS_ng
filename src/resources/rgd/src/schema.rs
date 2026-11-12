//! The frame's fields by name (upstream scripts/rgd_schema.py): the DOS structs' fields at
//! their offsets in a frame, what kind of number each is, the replay level that compares it
//! and how. A replay test compares the engine's state with a recording field by field,
//! by name, never by address: that is what lets the two trees' structs differ (upstream
//! docs/TESTS.md §5).
//!
//! Offsets are into the DOS structs as swos-port declares them, packed, little-endian.
//! Pointer fields are left out of the sprites; the team info's are recorded as identities.
//! Names are upstream's (the snake case of its structs' members).

use crate::version::{BENCH_LEN, FRAME_V1P0_LEN, RgdVersion};

/// Where the top team's general info starts in a frame.
pub const OFF_TOP_INFO: usize = 8;

/// Where the bottom team's general info starts.
pub const OFF_BOTTOM_INFO: usize = 164;

/// Where the top team's in-game data starts.
pub const OFF_TOP_GAME: usize = 320;

/// Where the bottom team's in-game data starts.
pub const OFF_BOTTOM_GAME: usize = 2024;

/// Where the top team's statistics start.
pub const OFF_TOP_STATS: usize = 3728;

/// Where the bottom team's statistics start.
pub const OFF_BOTTOM_STATS: usize = 3742;

/// Where the camera starts: its position, then its velocities, the clock, the goals and the
/// game's state.
pub const OFF_CAMERA: usize = 3756;

/// Where the sprites start.
pub const OFF_SPRITES: usize = 3774;

/// Bytes of a sprite.
pub const SPRITE_STRIDE: usize = 110;

/// The sprites of a frame.
pub const NUM_SPRITES: usize = 33;

/// The sprite of the corner flag: swos-port keeps four and picks one by the camera, so it
/// is no object of the port's (never compared).
pub const CORNER_FLAG_SPRITE: usize = 31;

/// The sprite of the current player's name, placed by the view (never compared).
pub const PLAYER_NAME_SPRITE: usize = 32;

/// Bytes of a player's in-game info.
pub const PLAYERINFO_STRIDE: usize = 61;

/// Where a team's in-game data has its players.
const TEAMGAME_PLAYERS_OFFSET: usize = 42;

/// The players of a team's in-game data.
pub const NUM_PLAYERS: usize = 16;

/// Where a team's general info has the DOS bench button (after the port's 144 bytes).
const TEAMINFO_BENCH_BUTTON: usize = 144;

/// Where the bench's state has its shirt numbers.
const BENCH_SHIRT_NUMBERS: usize = 39;

/// The bench's shirt numbers: two teams of 16.
const BENCH_SHIRT_NUMBERS_LEN: usize = 32;

/// The 1.4 additions' offsets: the key, fire blocked, the stats and result timers.
const V1P4_USER_KEY: usize = 0;
/// Fire blocked.
const V1P4_FIRE_BLOCKED: usize = 1;
/// The stats timer.
const V1P4_STATS_TIMER: usize = 2;
/// The result timer.
const V1P4_RESULT_TIMER: usize = 4;

/// The clock's digits: `d[0]` (always 0), hundreds, tens, units of minutes.
pub const GAME_TIME_DIGITS: usize = 4;

/// Where the clock starts after the camera's position (2 × 4 bytes) and velocities (2).
pub const CAMERA_CLOCK: usize = 10;

/// What kind of number a field is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// An unsigned byte.
    U8,
    /// A signed byte.
    S8,
    /// An unsigned word.
    U16,
    /// A signed word.
    S16,
    /// An unsigned long word.
    U32,
    /// A signed long word.
    S32,
}

impl FieldType {
    /// Its bytes.
    #[must_use]
    pub fn byte_len(self) -> usize {
        match self {
            Self::U8 | Self::S8 => 1,
            Self::U16 | Self::S16 => 2,
            Self::U32 | Self::S32 => 4,
        }
    }
}

/// What a pointer field identifies (the recorder writes an identity, not an address).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerKind {
    /// 0 the top team's data, 1 the bottom team's.
    Team,
    /// 0 the top team's in-game data, 1 the bottom team's.
    TeamGame,
    /// 0 the first team's statistics, 1 the second's.
    Stats,
    /// 0 the first team's sprites table, 1 the second's.
    SpritesTable,
    /// -1 none, else a sprite of the recording's 33 (only the players, 4..=25).
    Sprite,
    /// -1 none, 0 the outfield table, else a goalkeeper's skill table + 1.
    ShotChance,
}

/// How a replay compares a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Compare {
    /// Exactly.
    Exact,
    /// Its truth only: the exact non-zero value is an implementation detail.
    Truth,
    /// Only at the recording's view size (320 × 200): it depends on the view.
    View,
    /// Never: the port has no counterpart.
    NotApplicable,
    /// As a sprite-bank index, less the recording's offset (114).
    SpriteIndex,
    /// As a pointer's identity.
    Pointer(PointerKind),
    /// Recorded input, injected rather than compared.
    Input,
    /// Bookkeeping, neither injected nor compared.
    Meta,
}

/// How much of a frame a replay compares; each level includes those before it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// The frame's scalars: inputs, camera, clock, goals, state.
    Scalars,
    /// The teams' in-game data, statistics and bench.
    Teams,
    /// The teams' general info.
    Info,
    /// The sprites.
    Sprites,
}

/// A field of a frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FrameField {
    /// Its name (`top_game.players[3].goals_scored`).
    pub name: String,
    /// Its number's kind.
    pub field_type: FieldType,
    /// Its offset in the frame.
    pub offset: usize,
    /// The level that compares it.
    pub level: Level,
    /// How it is compared.
    pub compare: Compare,
}

/// A field of a struct: its name, number kind, offset in the struct and comparison.
pub(crate) type StructField = (&'static str, FieldType, usize, Compare);

/// A sprite's fields (110 bytes; pointers left out).
pub(crate) const SPRITE_FIELDS: &[StructField] = &[
    ("team_number", FieldType::S16, 0, Compare::Exact),
    ("player_ordinal", FieldType::S16, 2, Compare::Exact),
    ("frame_offset", FieldType::S16, 4, Compare::Exact),
    ("starting_direction", FieldType::S16, 10, Compare::Exact),
    ("player_state", FieldType::U8, 12, Compare::Exact),
    ("player_down_timer", FieldType::S8, 13, Compare::Exact),
    ("field_e", FieldType::S16, 14, Compare::Exact),
    ("field_10", FieldType::S16, 16, Compare::Exact),
    ("frame_index", FieldType::S16, 22, Compare::Exact),
    ("frame_delay", FieldType::S16, 24, Compare::Exact),
    ("cycle_frames_timer", FieldType::S16, 26, Compare::Exact),
    ("frame_switch_counter", FieldType::S16, 28, Compare::Exact),
    ("x", FieldType::S32, 30, Compare::Exact),
    ("y", FieldType::S32, 34, Compare::Exact),
    ("z", FieldType::S32, 38, Compare::Exact),
    ("direction", FieldType::S16, 42, Compare::Exact),
    ("speed", FieldType::S16, 44, Compare::Exact),
    ("delta_x", FieldType::S32, 46, Compare::Exact),
    ("delta_y", FieldType::S32, 50, Compare::Exact),
    ("delta_z", FieldType::S32, 54, Compare::Exact),
    ("dest_x", FieldType::S16, 58, Compare::Exact),
    ("dest_y", FieldType::S16, 60, Compare::Exact),
    ("field_3e[0]", FieldType::U8, 62, Compare::Exact),
    ("field_3e[1]", FieldType::U8, 63, Compare::Exact),
    ("field_3e[2]", FieldType::U8, 64, Compare::Exact),
    ("field_3e[3]", FieldType::U8, 65, Compare::Exact),
    ("field_3e[4]", FieldType::U8, 66, Compare::Exact),
    ("field_3e[5]", FieldType::U8, 67, Compare::Exact),
    ("visible", FieldType::S16, 68, Compare::Exact),
    ("image_index", FieldType::S16, 70, Compare::SpriteIndex),
    ("save_sprite", FieldType::S16, 72, Compare::Exact),
    ("ball_distance", FieldType::S32, 74, Compare::Exact),
    ("field_4e", FieldType::S16, 78, Compare::Exact),
    ("field_50", FieldType::S16, 80, Compare::Exact),
    ("full_direction", FieldType::S16, 82, Compare::Exact),
    ("on_screen", FieldType::S16, 84, Compare::View),
    ("field_56", FieldType::S16, 86, Compare::Exact),
    ("field_58", FieldType::S16, 88, Compare::Exact),
    ("field_5a", FieldType::S16, 90, Compare::Exact),
    ("player_direction", FieldType::S16, 92, Compare::Exact),
    ("is_moving", FieldType::S16, 94, Compare::Truth),
    ("tackle_state", FieldType::S16, 96, Compare::Exact),
    ("heading", FieldType::S16, 98, Compare::Exact),
    ("dest_reached_state", FieldType::S16, 100, Compare::Exact),
    ("cards", FieldType::S16, 102, Compare::Exact),
    ("injury_level", FieldType::S16, 104, Compare::Exact),
    ("tackling_timer", FieldType::S16, 106, Compare::Exact),
    ("sent_away", FieldType::S16, 108, Compare::Exact),
];

/// A team's general info (`TeamGeneralInfo`, 145 bytes), its pointers as identities.
pub(crate) const TEAMINFO_FIELDS: &[StructField] = &[
    (
        "opponents_team",
        FieldType::S32,
        0,
        Compare::Pointer(PointerKind::Team),
    ),
    (
        "in_game_team_ptr",
        FieldType::S32,
        10,
        Compare::Pointer(PointerKind::TeamGame),
    ),
    (
        "team_stats_ptr",
        FieldType::S32,
        14,
        Compare::Pointer(PointerKind::Stats),
    ),
    (
        "sprites_table",
        FieldType::S32,
        20,
        Compare::Pointer(PointerKind::SpritesTable),
    ),
    (
        "shot_chance_table",
        FieldType::S32,
        24,
        Compare::Pointer(PointerKind::ShotChance),
    ),
    (
        "controlled_player",
        FieldType::S32,
        32,
        Compare::Pointer(PointerKind::Sprite),
    ),
    (
        "pass_to_player_ptr",
        FieldType::S32,
        36,
        Compare::Pointer(PointerKind::Sprite),
    ),
    (
        "last_heading_tackling_player",
        FieldType::S32,
        72,
        Compare::Pointer(PointerKind::Sprite),
    ),
    (
        "passing_kicking_player",
        FieldType::S32,
        104,
        Compare::Pointer(PointerKind::Sprite),
    ),
    ("player_number", FieldType::S16, 4, Compare::Exact),
    ("player_coach_number", FieldType::S16, 6, Compare::Exact),
    ("is_pl_coach", FieldType::S16, 8, Compare::Exact),
    ("team_number", FieldType::S16, 18, Compare::Exact),
    ("tactics", FieldType::S16, 28, Compare::Exact),
    ("update_player_index", FieldType::S16, 30, Compare::Exact),
    ("player_has_ball", FieldType::S16, 40, Compare::Exact),
    ("allowed_directions", FieldType::S16, 42, Compare::Exact),
    (
        "current_allowed_direction",
        FieldType::S16,
        44,
        Compare::Exact,
    ),
    ("direction", FieldType::S16, 46, Compare::Exact),
    ("quick_fire", FieldType::S8, 48, Compare::Exact),
    ("normal_fire", FieldType::S8, 49, Compare::Exact),
    ("fire_pressed", FieldType::S8, 50, Compare::Exact),
    ("fire_this_frame", FieldType::S8, 51, Compare::Truth),
    ("header_or_tackle", FieldType::S16, 52, Compare::Exact),
    ("fire_counter", FieldType::S16, 54, Compare::Exact),
    (
        "controlled_pl_direction",
        FieldType::S16,
        56,
        Compare::Exact,
    ),
    ("shooting", FieldType::S16, 58, Compare::Exact),
    ("field_3c", FieldType::S8, 60, Compare::Exact),
    ("pl_very_close_to_ball", FieldType::S8, 61, Compare::Exact),
    ("pl_close_to_ball", FieldType::S8, 62, Compare::Exact),
    ("pl_not_far_from_ball", FieldType::S8, 63, Compare::Exact),
    ("ball_less_equal4", FieldType::S8, 64, Compare::Exact),
    ("ball4_to8", FieldType::S8, 65, Compare::Exact),
    ("ball8_to12", FieldType::S8, 66, Compare::Exact),
    ("ball12_to17", FieldType::S8, 67, Compare::Exact),
    ("ball_above17", FieldType::S8, 68, Compare::Exact),
    (
        "prev_pl_very_close_to_ball",
        FieldType::S8,
        69,
        Compare::Exact,
    ),
    ("field_46", FieldType::S16, 70, Compare::Truth),
    (
        "goalkeeper_saved_comment_timer",
        FieldType::S16,
        76,
        Compare::Exact,
    ),
    ("field_4e", FieldType::S16, 78, Compare::Exact),
    (
        "goalkeeper_diving_right",
        FieldType::S16,
        80,
        Compare::Exact,
    ),
    ("goalkeeper_diving_left", FieldType::S16, 82, Compare::Exact),
    (
        "ball_out_of_play_or_keeper",
        FieldType::S16,
        84,
        Compare::Truth,
    ),
    ("goalie_playing_or_out", FieldType::S16, 86, Compare::Truth),
    ("passing_ball", FieldType::S16, 88, Compare::Exact),
    ("passing_to_player", FieldType::S16, 90, Compare::Exact),
    ("player_switch_timer", FieldType::S16, 92, Compare::Exact),
    ("ball_in_play", FieldType::S16, 94, Compare::Exact),
    ("ball_out_of_play", FieldType::S16, 96, Compare::Exact),
    ("ball_x", FieldType::S16, 98, Compare::Exact),
    ("ball_y", FieldType::S16, 100, Compare::Exact),
    ("pass_kick_timer", FieldType::S16, 102, Compare::Exact),
    (
        "ball_control_turn_timer",
        FieldType::S16,
        108,
        Compare::Exact,
    ),
    (
        "ball_can_be_controlled",
        FieldType::S16,
        110,
        Compare::Exact,
    ),
    (
        "ball_controlling_player_direction",
        FieldType::S16,
        112,
        Compare::Exact,
    ),
    ("field_72", FieldType::S16, 114, Compare::Exact),
    ("field_74", FieldType::S16, 116, Compare::Exact),
    ("spin_timer", FieldType::S16, 118, Compare::Exact),
    ("left_spin", FieldType::S16, 120, Compare::Exact),
    ("right_spin", FieldType::S16, 122, Compare::Exact),
    ("long_pass", FieldType::S16, 124, Compare::Exact),
    ("long_spin_pass", FieldType::S16, 126, Compare::Exact),
    ("pass_in_progress", FieldType::S16, 128, Compare::Exact),
    ("ai_timer", FieldType::S16, 130, Compare::Exact),
    ("field_84", FieldType::S16, 132, Compare::Exact),
    (
        "ai_after_touch_strength",
        FieldType::S16,
        134,
        Compare::Exact,
    ),
    (
        "ai_ball_spin_direction",
        FieldType::S16,
        136,
        Compare::Exact,
    ),
    ("won_the_ball_timer", FieldType::S16, 138, Compare::Exact),
    ("goalkeeper_playing", FieldType::S16, 140, Compare::Exact),
    ("reset_controls", FieldType::S16, 142, Compare::Exact),
];

/// A player's in-game info (`PlayerInfo`, 61 bytes; the names, static, left out).
pub(crate) const PLAYERINFO_FIELDS: &[StructField] = &[
    ("skip", FieldType::S8, 0, Compare::Exact),
    ("player_index", FieldType::S8, 1, Compare::Exact),
    ("goals_scored", FieldType::S8, 2, Compare::Exact),
    ("shirt_number", FieldType::S8, 3, Compare::Exact),
    ("position", FieldType::S8, 4, Compare::Exact),
    ("face", FieldType::S8, 5, Compare::Exact),
    ("is_injured", FieldType::S8, 6, Compare::Exact),
    ("field_7", FieldType::S8, 7, Compare::Exact),
    ("field_8", FieldType::S8, 8, Compare::Exact),
    ("previous_cards", FieldType::S8, 9, Compare::Exact),
    ("num_yellow_cards", FieldType::S8, 10, Compare::Exact),
    ("field_b", FieldType::S8, 11, Compare::Exact),
    ("passing", FieldType::S8, 27, Compare::Exact),
    ("shooting", FieldType::S8, 28, Compare::Exact),
    ("heading", FieldType::S8, 29, Compare::Exact),
    ("tackling", FieldType::S8, 30, Compare::Exact),
    ("ball_control", FieldType::S8, 31, Compare::Exact),
    ("speed", FieldType::S8, 32, Compare::Exact),
    ("finishing", FieldType::S8, 33, Compare::Exact),
    ("goalie_skill", FieldType::S8, 34, Compare::Exact),
    ("injuries_bitfield", FieldType::S8, 35, Compare::Exact),
    ("half_played", FieldType::S8, 36, Compare::Exact),
    ("face2", FieldType::S8, 37, Compare::Exact),
];

/// A team's in-game header (`TeamGame`, before its players at +42).
pub(crate) const TEAMGAME_FIELDS: &[StructField] = &[
    ("pr_shirt_type", FieldType::S16, 0, Compare::Exact),
    ("pr_shirt_col", FieldType::S16, 2, Compare::Exact),
    ("pr_stripes_col", FieldType::S16, 4, Compare::Exact),
    ("pr_shorts_col", FieldType::S16, 6, Compare::Exact),
    ("pr_socks_col", FieldType::S16, 8, Compare::Exact),
    ("sec_shirt_type", FieldType::S16, 10, Compare::Exact),
    ("sec_shirt_col", FieldType::S16, 12, Compare::Exact),
    ("sec_stripes_col", FieldType::S16, 14, Compare::Exact),
    ("sec_shorts_col", FieldType::S16, 16, Compare::Exact),
    ("sec_socks_col", FieldType::S16, 18, Compare::Exact),
    ("marked_player", FieldType::S16, 20, Compare::Exact),
    ("num_own_goals", FieldType::S8, 40, Compare::Exact),
];

/// A team's statistics (`TeamStatsData`, 14 bytes).
pub(crate) const TEAMSTATS_FIELDS: &[StructField] = &[
    ("ball_possession", FieldType::S16, 0, Compare::Exact),
    ("corners_won", FieldType::S16, 2, Compare::Exact),
    ("fouls_conceded", FieldType::S16, 4, Compare::Exact),
    ("bookings", FieldType::S16, 6, Compare::Exact),
    ("sendings_off", FieldType::S16, 8, Compare::Exact),
    ("goal_attempts", FieldType::S16, 10, Compare::Exact),
    ("on_target", FieldType::S16, 12, Compare::Exact),
];

/// The bench menu's state (`BenchDataV1`, 71 bytes, v1.3+), before its shirt numbers.
pub(crate) const BENCH_FIELDS: &[StructField] = &[
    ("pl1_tap_count", FieldType::U8, 0, Compare::Exact),
    ("pl2_tap_count", FieldType::U8, 1, Compare::Exact),
    ("pl1_prev_direction", FieldType::S8, 2, Compare::Exact),
    ("pl2_prev_direction", FieldType::S8, 3, Compare::Exact),
    ("pl1_block_hold", FieldType::U8, 4, Compare::Exact),
    ("pl2_block_hold", FieldType::U8, 5, Compare::Exact),
    ("controls", FieldType::S8, 6, Compare::NotApplicable),
    ("team_data", FieldType::S8, 7, Compare::NotApplicable),
    ("team_game", FieldType::S8, 8, Compare::NotApplicable),
    ("state", FieldType::U8, 9, Compare::Exact),
    ("go_to_bench_timer", FieldType::S32, 10, Compare::Exact),
    ("bench1_called", FieldType::U8, 14, Compare::Exact),
    ("bench2_called", FieldType::U8, 15, Compare::Exact),
    ("block_directions", FieldType::U8, 16, Compare::Exact),
    ("fire_timer", FieldType::S32, 17, Compare::Exact),
    ("block_fire", FieldType::U8, 21, Compare::Exact),
    ("last_direction", FieldType::S8, 22, Compare::NotApplicable),
    ("movement_delay_timer", FieldType::S32, 23, Compare::Exact),
    ("training_top_team", FieldType::U8, 27, Compare::Exact),
    ("teams_swapped", FieldType::U8, 28, Compare::Exact),
    ("alternate_teams_timer", FieldType::S32, 29, Compare::Exact),
    ("arrow_player_index", FieldType::S8, 33, Compare::Exact),
    (
        "selected_menu_player",
        FieldType::S8,
        34,
        Compare::NotApplicable,
    ),
    ("player_to_enter", FieldType::S8, 35, Compare::Exact),
    ("player_to_sub_pos", FieldType::S8, 36, Compare::Exact),
    ("player_to_sub_ord", FieldType::S8, 37, Compare::Exact),
    ("selected_formation", FieldType::S8, 38, Compare::Exact),
];

fn field(
    name: String,
    field_type: FieldType,
    offset: usize,
    level: Level,
    compare: Compare,
) -> FrameField {
    FrameField {
        name,
        field_type,
        offset,
        level,
        compare,
    }
}

/// `table`'s fields at `base`, named `prefix.name`, at `level`.
fn struct_fields(
    out: &mut Vec<FrameField>,
    prefix: &str,
    base: usize,
    table: &[StructField],
    level: Level,
) {
    for &(name, field_type, offset, compare) in table {
        out.push(field(
            format!("{prefix}.{name}"),
            field_type,
            base + offset,
            level,
            compare,
        ));
    }
}

/// A recording's frame fields up to `level`, in upstream's order (`build_schema`).
#[must_use]
pub fn frame_fields(version: RgdVersion, level: Level) -> Vec<FrameField> {
    let mut out = Vec::new();
    let scalars = Level::Scalars;
    out.push(field(
        "frame_no".into(),
        FieldType::U32,
        0,
        scalars,
        Compare::Meta,
    ));
    out.push(field(
        "p1_controls".into(),
        FieldType::S16,
        4,
        scalars,
        Compare::Input,
    ));
    out.push(field(
        "p2_controls".into(),
        FieldType::S16,
        6,
        scalars,
        Compare::Input,
    ));
    out.push(field(
        "top_bench_button".into(),
        FieldType::U8,
        OFF_TOP_INFO + TEAMINFO_BENCH_BUTTON,
        scalars,
        Compare::Input,
    ));
    out.push(field(
        "bottom_bench_button".into(),
        FieldType::U8,
        OFF_BOTTOM_INFO + TEAMINFO_BENCH_BUTTON,
        scalars,
        Compare::Input,
    ));
    for (name, field_type, offset) in [
        ("camera_x", FieldType::S32, 0),
        ("camera_y", FieldType::S32, 4),
        ("camera_x_vel", FieldType::S8, 8),
        ("camera_y_vel", FieldType::S8, 9),
    ] {
        out.push(field(
            name.into(),
            field_type,
            OFF_CAMERA + offset,
            scalars,
            Compare::View,
        ));
    }
    let clock = OFF_CAMERA + CAMERA_CLOCK;
    for digit in 0..GAME_TIME_DIGITS {
        out.push(field(
            format!("game_time[{digit}]"),
            FieldType::U8,
            clock + digit,
            scalars,
            Compare::Exact,
        ));
    }
    for (index, name) in ["team1_goals", "team2_goals", "game_state", "game_state_pl"]
        .into_iter()
        .enumerate()
    {
        out.push(field(
            name.into(),
            FieldType::U8,
            clock + GAME_TIME_DIGITS + index,
            scalars,
            Compare::Exact,
        ));
    }
    if version.has_bench() && level >= Level::Teams {
        struct_fields(
            &mut out,
            "bench",
            FRAME_V1P0_LEN,
            BENCH_FIELDS,
            Level::Teams,
        );
        for index in 0..BENCH_SHIRT_NUMBERS_LEN {
            out.push(field(
                format!("bench.shirt_numbers[{index}]"),
                FieldType::U8,
                FRAME_V1P0_LEN + BENCH_SHIRT_NUMBERS + index,
                Level::Teams,
                Compare::NotApplicable,
            ));
        }
    }
    if version.has_v1p4() {
        let base = FRAME_V1P0_LEN + BENCH_LEN;
        for (name, field_type, offset, compare) in [
            ("user_key", FieldType::U8, V1P4_USER_KEY, Compare::Input),
            (
                "fire_blocked",
                FieldType::U8,
                V1P4_FIRE_BLOCKED,
                Compare::Exact,
            ),
            (
                "stats_timer",
                FieldType::U16,
                V1P4_STATS_TIMER,
                Compare::Truth,
            ),
            (
                "result_timer",
                FieldType::U16,
                V1P4_RESULT_TIMER,
                Compare::Exact,
            ),
        ] {
            out.push(field(
                name.into(),
                field_type,
                base + offset,
                scalars,
                compare,
            ));
        }
    }
    if level >= Level::Teams {
        for (prefix, base) in [("top_game", OFF_TOP_GAME), ("bottom_game", OFF_BOTTOM_GAME)] {
            struct_fields(&mut out, prefix, base, TEAMGAME_FIELDS, Level::Teams);
            for player in 0..NUM_PLAYERS {
                let player_base = base + TEAMGAME_PLAYERS_OFFSET + player * PLAYERINFO_STRIDE;
                struct_fields(
                    &mut out,
                    &format!("{prefix}.players[{player}]"),
                    player_base,
                    PLAYERINFO_FIELDS,
                    Level::Teams,
                );
            }
        }
        for (prefix, base) in [
            ("top_stats", OFF_TOP_STATS),
            ("bottom_stats", OFF_BOTTOM_STATS),
        ] {
            struct_fields(&mut out, prefix, base, TEAMSTATS_FIELDS, Level::Teams);
        }
    }
    if level >= Level::Info {
        for (prefix, base) in [("top_info", OFF_TOP_INFO), ("bottom_info", OFF_BOTTOM_INFO)] {
            struct_fields(&mut out, prefix, base, TEAMINFO_FIELDS, Level::Info);
        }
    }
    if level >= Level::Sprites {
        for sprite in 0..NUM_SPRITES {
            let base = OFF_SPRITES + sprite * SPRITE_STRIDE;
            for &(name, field_type, offset, compare) in SPRITE_FIELDS {
                let compare = if sprite == CORNER_FLAG_SPRITE || sprite == PLAYER_NAME_SPRITE {
                    Compare::NotApplicable
                } else {
                    compare
                };
                out.push(field(
                    format!("sprite[{sprite}].{name}"),
                    field_type,
                    base + offset,
                    Level::Sprites,
                    compare,
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(minor: u8) -> RgdVersion {
        RgdVersion::new(1, minor).expect("a version")
    }

    #[test]
    fn the_tables_are_upstreams() {
        let tables = [
            SPRITE_FIELDS.len(),
            TEAMINFO_FIELDS.len(),
            PLAYERINFO_FIELDS.len(),
            TEAMGAME_FIELDS.len(),
            TEAMSTATS_FIELDS.len(),
            BENCH_FIELDS.len(),
        ];
        assert_eq!(tables, [48, 70, 23, 12, 7, 27]);
    }

    #[test]
    fn levels_add_fields() {
        let counts: Vec<usize> = [Level::Scalars, Level::Teams, Level::Info, Level::Sprites]
            .into_iter()
            .map(|level| frame_fields(version(1), level).len())
            .collect();
        // 17 scalars; + 2 × (12 + 16 × 23) + 2 × 7; + 2 × 70; + 33 × 48.
        assert_eq!(counts, [17, 791, 931, 2515]);
        let v1p5 = frame_fields(version(5), Level::Teams).len();
        assert_eq!(
            v1p5,
            791 + 27 + 32 + 4,
            "the bench, its shirt numbers, the 1.4 words"
        );
    }

    #[test]
    fn every_field_lies_in_its_frame() {
        for minor in [0, 1, 3, 4, 5] {
            let version = version(minor);
            let frame_len = version.frame_len().expect("a known size");
            for field in frame_fields(version, Level::Sprites) {
                assert!(
                    field.offset + field.field_type.byte_len() <= frame_len,
                    "{}",
                    field.name
                );
            }
        }
    }

    #[test]
    fn names_are_unique_and_upstreams() {
        let fields = frame_fields(version(5), Level::Sprites);
        let mut names: Vec<&str> = fields.iter().map(|field| field.name.as_str()).collect();
        assert!(names.contains(&"top_game.players[15].face2"));
        assert!(names.contains(&"sprite[32].sent_away"));
        assert!(names.contains(&"bench.shirt_numbers[31]"));
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        let corner = fields
            .iter()
            .find(|field| field.name == "sprite[31].x")
            .expect("a field");
        assert_eq!(corner.compare, Compare::NotApplicable);
    }
}
