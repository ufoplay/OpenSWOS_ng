//! The comments upstream's table writes before the game's settings (src/swos_settings.c
//! `settings_build`), verbatim: a fresh file and an added key carry them.

/// The comment before `[gameplay] fire_hold_lockout`.
pub(crate) const GAMEPLAY_FIRE_HOLD_LOCKOUT: &str = concat!(
    "; fire_hold_lockout: true | false — true restores a BUG IN THE ORIGINAL\n",
    "; AMIGA GAME: an OPPONENT's tackle wipes your held fire button's shot charge,\n",
    "; and it cannot restart while the button is down, so holding fire through a\n",
    "; challenge never shoots until you let go and press again. Default false\n",
    "; (fixed). Your OWN tackle still spends the press either way — that half is\n",
    "; deliberate in the original. See team_control.c.",
);

/// The comment before `[gameplay] style`.
pub(crate) const GAMEPLAY_STYLE: &str = concat!(
    "; style: amiga | dos | xbox | amiga60 | moon. The Amiga release is the definitive one and what\n",
    "; this port is reverse engineered from; dos is the PC release's 70 FPS physics,\n",
    "; timers and formations, and its 320x200 view -- the view is part of how it\n",
    "; plays (what is on screen decides which players the game picks), so the\n",
    "; style sets both; xbox is SWOS 2020's XBOX style: 60 FPS, the Amiga's ball\n",
    "; physics and view, the DOS goalkeepers; amiga60 is SWOS 2020's AMIGA (60FPS): the\n",
    "; Amiga's game at 60 FPS; moon is its MOON SOCCER: the Amiga's, with a moon's\n",
    "; gravity. Online, the game plays the HOST's style.\n",
);

/// The comment before `[replays] auto_save`.
pub(crate) const REPLAYS_AUTO_SAVE: &str = concat!(
    "; auto_save: true | false — every match is recorded to LATEST.SRP in the save\n",
    "; folder and can be watched from HIGHLIGHTS -> LOAD. true also keeps a dated\n",
    "; copy of each (MMDDHHMM.SRP), so a match is not overwritten by the next one.",
);

/// The comment before `[replays] frame_overlay`.
pub(crate) const REPLAYS_FRAME_OVERLAY: &str = concat!(
    "; frame_overlay: true | false — the box at the bottom left of a recording being\n",
    "; watched (its speed, paused or not, the frame and a bar); I shows or hides it.\n",
    "; The keys: P pause, Up / Down speed, Left / Right backwards / forwards (paused: a\n",
    "; frame), Home / End the first / last frame, PgDn / PgUp a tenth back / on.",
);

/// The comment before `[replays] speed`.
pub(crate) const REPLAYS_SPEED: &str =
    "; speed: normal | half — the speed a recording starts at in the viewer.";

/// The comment before `[testing] no_injuries_cards`.
pub(crate) const TESTING_NO_INJURIES_CARDS: &str = concat!(
    "; no_injuries_cards: true | false -- FOR TESTING A CAREER: nobody is booked, sent off,\n",
    "; injured or suspended, so VIEW RESULT runs through a season without stopping.\n",
    "; Offline only: an online match always plays the real rules.",
);

/// The comment before `[general] language`.
pub(crate) const GENERAL_LANGUAGE: &str = "; language: en | de | fr | es | it | nl";

/// The comment before `[general] currency`.
pub(crate) const GENERAL_CURRENCY: &str = "; currency: pound | dollar | euro";

/// The comment before `[general] assets_reminder_off`.
pub(crate) const GENERAL_ASSETS_REMINDER_OFF: &str = concat!(
    "; assets_reminder_off: set by \"Don't remind me\" on the reminder that your game data was made by\n",
    "; an older OpenSWOS Setup -- the data versions it was about. Empty: remind (src/assets_check.c).",
);

/// The comment before `[general] world`.
pub(crate) const GENERAL_WORLD: &str = concat!(
    "; world: 9697 | 2627 -- the football world: its countries, clubs and competitions\n",
    "; (docs/WORLD_2026_27.md): SWOS 96/97, or World 2026-27 once OpenSWOS Setup has\n",
    "; imported its data. A save is played in the world it was made in.",
);

/// The comment before `[general] file_sort`.
pub(crate) const GENERAL_FILE_SORT: &str = concat!(
    "; file_sort: name | date -- the order of the LOAD and SAVE screens' files (SWOS 2020's SORT\n",
    "; OPTION; their SORTED BY button switches it): by name, as the game always had them, or the\n",
    "; newest first.",
);

/// The comment before `[general] quick_access`.
pub(crate) const GENERAL_QUICK_ACCESS: &str = concat!(
    "; quick_access: true | false -- SWOS 2020's QUICK ACCESS after a friendly: SAVE HIGHLIGHTS,\n",
    "; FULL MATCH REPLAYS (the match just played) or CONTINUE.",
);

/// The comment before `[mods] enabled`.
pub(crate) const MODS_ENABLED: &str = concat!(
    "; enabled: the mods to play with, by their folder's name in the mods folder beside this file,\n",
    "; in that order: a file two of them have comes from the first, e.g. enabled = classics, hall.\n",
    "; Empty: none (docs/MODS.md). A downloaded (DLC) mod that is not cosmetic keeps the game off line.",
);

/// The comment before `[mods] disabled`.
pub(crate) const MODS_DISABLED: &str =
    "; disabled: the game's own (canonical) mods to turn off, by name; they are on by default.";

/// The comment before `[tweaks] cpu_cards`.
pub(crate) const TWEAKS_CPU_CARDS: &str = concat!(
    "; [tweaks]: small changes to the original's behaviour, each off by default and only off line\n",
    "; (online, and in the test rigs, the original plays): docs/MODS.md.\n",
    "; cpu_cards: true | false -- the computer's players are booked and sent off too\n",
    "; (the original cards only a side a human controls).",
);

/// The comment before `[tweaks] no_teammate_tackles`.
pub(crate) const TWEAKS_NO_TEAMMATE_TACKLES: &str = concat!(
    "; no_teammate_tackles: true | false -- a team-mate leaves your player on the ball alone\n",
    "; (in the original the nearest team-mate runs onto the ball and can slide or take it off you).",
);

/// The comment before `[tweaks] tackle_controlled_player`.
pub(crate) const TWEAKS_TACKLE_CONTROLLED_PLAYER: &str = concat!(
    "; tackle_controlled_player: true | false -- fire slides the player you control\n",
    "; (in the original a nearer team-mate may slide instead).",
);

/// The comment before `[tweaks] more_fouls`.
pub(crate) const TWEAKS_MORE_FOULS: &str = concat!(
    "; more_fouls: true | false -- one borderline tackle in 16, from the side, is a foul\n",
    "; (the original gives a foul only from behind; benefactorr's Career+).",
);

/// The comment before `[tweaks] ai_injury_subs`.
pub(crate) const TWEAKS_AI_INJURY_SUBS: &str = concat!(
    "; ai_injury_subs: true | false -- the computer replaces an injured player at a stoppage\n",
    "; (the original's computer never substitutes; benefactorr's Career+).",
);

/// The comment before `[tweaks] stronger_goalkeeper_ai`.
pub(crate) const TWEAKS_STRONGER_GOALKEEPER_AI: &str = concat!(
    "; stronger_goalkeeper_ai: true | false -- SWOS 2020's stronger goalkeeper AI: in the\n",
    "; gameplay styles with the Amiga's keepers (amiga, amiga60, moon) they reach one step\n",
    "; further for a shot (dos and xbox keep theirs). Off line only; GAMEPLAY OPTIONS' STRONGER KEEPERS.",
);

/// The comment before `[tweaks] unlimited_transfers`.
pub(crate) const TWEAKS_UNLIMITED_TRANSFERS: &str = concat!(
    "; unlimited_transfers: true | false -- SWOS 2020's UNLIMITED TRANSFERS: in a career,\n",
    "; BUY PLAYER is not held to the season's transfer quota (\"the club is unwilling to\n",
    "; purchase players\"), only to the room left in the career's player pool.",
);

/// The comment before `[tweaks] contest_own_ratings`.
pub(crate) const TWEAKS_CONTEST_OWN_RATINGS: &str = concat!(
    "; The tweaks below are ON by default (RULES OPTIONS); off line only, like every tweak.\n",
    "; contest_own_ratings: true | false -- when two players contest the ball, each side is\n",
    "; rated by the man at it. Both originals rate one side by a team-mate (the player with\n",
    "; the opponent's line-up number): docs/BALL_CONTEST.md.",
);

/// The comment before `[tweaks] league_crash_fixes`.
pub(crate) const TWEAKS_LEAGUE_CRASH_FIXES: &str = concat!(
    "; league_crash_fixes: true | false -- SWOS 2020's career fixes 1-3: a division sized by the\n",
    "; teams in its file, and when one grew a longer season and fewer meetings. They change\n",
    "; nothing in 96/97's own leagues; they guard against a database whose leagues differ.",
);

/// The comment before `[tweaks] league_rules_2020`.
pub(crate) const TWEAKS_LEAGUE_RULES_2020: &str = concat!(
    "; league_rules_2020: true | false -- SWOS 2020's career fixes 4-7 in 96/97: Greece's season\n",
    "; from October (its cup September to June), Israel plays each other twice (not three times),\n",
    "; Italy relegates and promotes 3 (not 4), Scotland plays each other three times (not four).",
);

/// The comment before `[tweaks] three_points_for_a_win`.
pub(crate) const TWEAKS_THREE_POINTS_FOR_A_WIN: &str = concat!(
    "; three_points_for_a_win: true | false -- every 96/97 league gives 3 points for a win\n",
    "; (SWOS 2020's --always-points-per-win-3).",
);

/// The comment before `[tweaks] five_substitutes`.
pub(crate) const TWEAKS_FIVE_SUBSTITUTES: &str = concat!(
    "; five_substitutes: true | false -- every 96/97 league and cup: 5 substitutes from a bench\n",
    "; of 5 (SWOS 2020's, with its 3 points). These four apply to a career started with them:\n",
    "; a saved career keeps the rules it was started with.",
);

/// The comment before `[general] skip_intro`.
pub(crate) const GENERAL_SKIP_INTRO: &str = concat!(
    "; skip_intro: true | false -- straight to the menus, no intro (SWOS 2020's --no-intro;\n",
    "; GAMEPLAY OPTIONS' SKIP INTRO).",
);

/// The comment before `[general] skip_image_reels`.
pub(crate) const GENERAL_SKIP_IMAGE_REELS: &str = concat!(
    "; skip_image_reels: true | false -- the intro is its title picture and tune alone, without\n",
    "; the pictures before and after it (SWOS 2020's --no-image-reels).",
);

/// The comment before `[general] no_load_pause`.
pub(crate) const GENERAL_NO_LOAD_PAUSE: &str = concat!(
    "; no_load_pause: true | false -- the stadium screen before a match goes at once instead of\n",
    "; after its 3 seconds (SWOS 2020's --no-load-pause). Off line only.",
);

/// The comment before `[gameplay] cpu_tactics`.
pub(crate) const GAMEPLAY_CPU_TACTICS: &str = concat!(
    "; cpu_tactics: normal | expert -- expert plays SWOS 2020's improved formations (its\n",
    "; --improved-cpu-tactics: every team's 4-4-2 to DEFEND, named 4132, 5131, ...). Off line only.",
);

/// The comment before `[options] game_length`.
pub(crate) const OPTIONS_GAME_LENGTH: &str = concat!(
    "; The OPTIONS screen, as it was left (a loaded career plays its own).\n",
    "; game_length: 3 | 5 | 7 | 10 (minutes)",
);

/// The comment before `[options] pitch_type`.
pub(crate) const OPTIONS_PITCH_TYPE: &str =
    "; pitch_type: seasonal | random | frozen | muddy | wet | soft | normal | dry | hard";

/// The comment before `[options] auto_replays`.
pub(crate) const OPTIONS_AUTO_REPLAYS: &str =
    "; auto_replays, all_teams_equal, auto_save_highlights, menu_music: true | false";
