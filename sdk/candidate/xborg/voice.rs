//! Default English voice clips, routed through the shared match audio gates.
use crate::audio;
use mod_api_stable::StableSim;

pub(super) const MOVE: &[&str] = &[
    "mlbb_xborg_voice_move01",
    "mlbb_xborg_voice_move02",
    "mlbb_xborg_voice_move03",
    "mlbb_xborg_voice_move04",
    "mlbb_xborg_voice_move05",
    "mlbb_xborg_voice_move06",
    "mlbb_xborg_voice_move07",
    "mlbb_xborg_voice_move08",
    "mlbb_xborg_voice_move09",
    "mlbb_xborg_voice_move10",
    "mlbb_xborg_voice_move11",
];
pub(super) const BASIC: &[&str] = &[
    "mlbb_xborg_voice_basic01",
    "mlbb_xborg_voice_basic02",
    "mlbb_xborg_voice_basic03",
];
pub(super) const ULT: &[&str] = &["mlbb_xborg_voice_ult01", "mlbb_xborg_voice_ult02"];
pub(super) const DEATH: &[&str] = &["mlbb_xborg_voice_death01", "mlbb_xborg_voice_death02"];
pub(super) const RESPAWN: &[&str] = &["mlbb_xborg_voice_respawn01", "mlbb_xborg_voice_respawn02"];
pub(super) const KILL: &[&str] = &[
    "mlbb_xborg_voice_kill01",
    "mlbb_xborg_voice_kill02",
    "mlbb_xborg_voice_kill03",
];
pub(super) const OBJECTIVE: &[&str] = &["mlbb_xborg_voice_lord01", "mlbb_xborg_voice_lord02"];

pub(super) fn play(sim: &mut StableSim<'_>, id: usize, line: &str, priority: bool) {
    if audio::voice_in_match_view(sim) {
        if priority {
            audio::play_priority_voice(sim, id, line);
        } else {
            audio::play_voice(sim, id, line);
        }
    }
}
