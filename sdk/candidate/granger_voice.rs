//! Event selection only; actual playback uses the shared live-frame voice gate.
use super::*;
const MOVES: &[&str] = &[
    "mlbb_granger_voice_move01",
    "mlbb_granger_voice_move02",
    "mlbb_granger_voice_move03",
    "mlbb_granger_voice_move04",
    "mlbb_granger_voice_move05",
    "mlbb_granger_voice_move06",
    "mlbb_granger_voice_move08",
    "mlbb_granger_voice_move09",
    "mlbb_granger_voice_move10",
];
#[derive(Clone, Debug, Default)]
pub(super) struct Voice {
    intro_at: usize,
    pub(super) introduced: bool,
    respawn: bool,
    pub(super) next_skill: usize,
    next_move: usize,
    move_cursor: usize,
    ult_cursor: usize,
    kill_cursor: usize,
    respawns: usize,
    previous: Option<Pos>,
}
fn viewer(sim: &StableSim<'_>) -> bool {
    matches!(
        sim.sim_origin()
            .and_then(|o| SimOriginKindV1::from_code(o.kind)),
        Some(
            SimOriginKindV1::ClientMatchView
                | SimOriginKindV1::ClientSpectate
                | SimOriginKindV1::ClientReplay
        )
    )
}
impl Voice {
    pub fn reset(&mut self, now: usize, respawn: bool) {
        self.intro_at = now + 60;
        self.introduced = false;
        self.respawn = respawn;
        if respawn {
            self.respawns += 1;
        }
        self.next_move = now + 720;
        self.previous = None;
    }
    fn shot_line(&mut self, now: usize, kind: ShotKind) -> Option<&'static str> {
        if !self.introduced && now >= self.intro_at {
            self.introduced = true;
            self.next_move = now + 720;
            self.next_skill = now + 300;
            return Some(if self.respawn {
                if self.respawns % 2 == 1 {
                    "mlbb_granger_voice_respawn01"
                } else {
                    "mlbb_granger_voice_respawn02"
                }
            } else {
                "mlbb_granger_voice_select"
            });
        }
        if now < self.next_skill {
            return None;
        }
        let cue = match kind {
            ShotKind::Round(1) => "mlbb_granger_voice_skill01",
            ShotKind::Cannon => {
                self.ult_cursor += 1;
                if self.ult_cursor % 2 == 1 {
                    "mlbb_granger_voice_ult01"
                } else {
                    "mlbb_granger_voice_ult02"
                }
            }
            _ => return None,
        };
        self.next_skill = now + 360;
        Some(cue)
    }
    fn movement_line(&mut self, now: usize, p: Pos) -> Option<&'static str> {
        let moving = self.previous.is_some_and(|a| dist(a, p) > 500.);
        self.previous = Some(p);
        if !self.introduced || !moving || now < self.next_move {
            return None;
        }
        let cue = MOVES[self.move_cursor % MOVES.len()];
        self.move_cursor += 1;
        self.next_move = now + 720;
        Some(cue)
    }
}
pub(super) fn emitted(sim: &mut StableSim<'_>, owner: usize, s: &mut State, kind: ShotKind) {
    if !viewer(sim) {
        return;
    }
    if let Some(line) = s.voice.shot_line(sim.tick(), kind) {
        crate::audio::play_voice(sim, owner, line);
    }
}
pub(super) fn tick(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    if !viewer(sim) {
        return;
    }
    if let Some(p) = pos(sim, owner) {
        if let Some(line) = s.voice.movement_line(sim.tick(), p) {
            crate::audio::play_voice(sim, owner, line);
        }
    }
}
pub(super) fn death(sim: &mut StableSim<'_>, owner: usize) {
    if viewer(sim) {
        crate::audio::play_priority_voice(sim, owner, "mlbb_granger_voice_death01");
    }
}
pub(super) fn kill(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    if !viewer(sim) {
        return;
    }
    let cue = [
        "mlbb_granger_voice_kill01",
        "mlbb_granger_voice_kill02",
        "mlbb_granger_voice_kill03",
    ][s.voice.kill_cursor % 3];
    s.voice.kill_cursor += 1;
    crate::audio::play_voice(sim, owner, cue);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intro_is_a_fresh_shot_opportunity_not_spawn_or_repeating_basic_chatter() {
        let mut v = Voice::default();
        v.reset(0, false);
        assert_eq!(v.shot_line(0, ShotKind::Basic), None);
        assert_eq!(
            v.shot_line(60, ShotKind::Basic),
            Some("mlbb_granger_voice_select")
        );
        for n in 61..600 {
            assert_eq!(v.shot_line(n, ShotKind::Basic), None);
        }
    }
    #[test]
    fn six_rounds_offer_one_skill_line_and_cannon_is_throttled() {
        let mut v = Voice::default();
        v.introduced = true;
        assert_eq!(
            v.shot_line(100, ShotKind::Round(1)),
            Some("mlbb_granger_voice_skill01")
        );
        for round in 2..=6 {
            assert_eq!(v.shot_line(100 + 18 * round, ShotKind::Round(round)), None);
        }
        assert_eq!(v.shot_line(200, ShotKind::Cannon), None);
        assert_eq!(
            v.shot_line(460, ShotKind::Cannon),
            Some("mlbb_granger_voice_ult01")
        );
        assert_eq!(
            v.shot_line(820, ShotKind::Cannon),
            Some("mlbb_granger_voice_ult02")
        );
    }
    #[test]
    fn movement_requires_real_motion_and_has_twelve_second_spacing() {
        let mut v = Voice::default();
        v.introduced = true;
        assert_eq!(v.movement_line(100, (1000, 1000)), None);
        assert_eq!(v.movement_line(101, (1001, 1000)), None);
        assert_eq!(v.movement_line(102, (2000, 1000)), Some(MOVES[0]));
        assert_eq!(v.movement_line(103, (3000, 1000)), None);
        assert_eq!(v.movement_line(822, (4000, 1000)), Some(MOVES[1]));
    }
    #[test]
    fn respawn_line_waits_for_new_combat_and_rotates() {
        let mut v = Voice::default();
        v.reset(100, true);
        assert_eq!(v.shot_line(100, ShotKind::Basic), None);
        assert_eq!(
            v.shot_line(160, ShotKind::Basic),
            Some("mlbb_granger_voice_respawn01")
        );
        v.reset(900, true);
        assert_eq!(
            v.shot_line(960, ShotKind::Basic),
            Some("mlbb_granger_voice_respawn02")
        );
    }
}
