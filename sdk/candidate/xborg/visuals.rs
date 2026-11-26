//! One live actor, one body pose, and short-lived world-effect samples.
use super::*;

include!("../xborg_body.rs");

const FIRAGA_GAUGE_HEIGHT: u64 = 36_000;

pub(super) struct Visual {
    pub left: bool,
    pub last_pos: Pos,
    moving: bool,
    next_frame: usize,
    action: Option<(&'static str, usize)>,
}

impl Visual {
    pub fn new(pos: Pos) -> Self {
        Self {
            left: false,
            last_pos: pos,
            moving: false,
            next_frame: 0,
            action: None,
        }
    }

    pub fn play(&mut self, name: &'static str, start: usize) {
        self.action = Some((name, start));
        self.next_frame = start;
    }

    pub fn action_active(&self, now: usize) -> bool {
        self.action
            .is_some_and(|(name, start)| now < start + clip(name).1)
    }

    pub fn face(&mut self, dir: Direction) {
        if dir.0.abs() > 0.08 {
            self.left = dir.0 < 0.;
        }
    }
}

pub(super) fn form_name(armored: bool, full: &'static str, slim: &'static str) -> &'static str {
    if armored {
        full
    } else {
        slim
    }
}

pub(super) fn visible(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}

pub(super) fn effect(sim: &mut StableSim<'_>, id: usize, name: &str, at: Pos, ticks: usize) {
    if visible(sim) {
        sim.play_view_effect(
            &format!("mlbb_xborg_{name}"),
            id,
            &InputTargetV1::pos(at.0, at.1),
            0,
            0,
            ticks as u64,
        );
    }
}

pub(super) fn angle(dir: Direction) -> usize {
    (dir.1.atan2(dir.0).rem_euclid(std::f64::consts::TAU) * 16. / std::f64::consts::TAU).round()
        as usize
        % 16
}

pub(super) fn draw(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    if !alive(sim, id) {
        return;
    }
    if !visible(sim) {
        return;
    }
    let now = sim.tick();
    if now < s.visual.next_frame {
        return;
    }
    let Some(at) = position(sim, id) else { return };
    let action = s.visual.action.filter(|(name, t)| now < *t + clip(name).1);
    let attacking = matches!(
        action,
        Some(("armored_basic_attack" | "armorless_basic_attack", _))
    );
    let walking = dist(s.visual.last_pos, at) > 120.;
    if walking && !attacking && !s.combat.is_firing() && !s.combat.is_spinning() {
        if let Some(dir) = unit(delta(s.visual.last_pos, at)) {
            s.visual.face(dir);
        }
    }
    s.visual.last_pos = at;
    s.visual.moving = walking;
    let full = s.armor.armored();
    let movement = if s.visual.moving {
        form_name(full, "armored_run", "armorless_walk")
    } else {
        form_name(full, "armored_idle", "armorless_idle")
    };
    let active = s.combat.body_exclusive(now, full, walking);
    // The form break is visible while a running S1 retains its own clock.
    let breaking = action.filter(|(name, _)| *name == "armor_break" && !s.combat.is_spinning());
    let (name, start, looped) = breaking
        .map(|(n, t)| (n, t, false))
        .or(active)
        .or_else(|| action.map(|(n, t)| (n, t, false)))
        .unwrap_or((movement, 0, true));
    let (len, duration) = clip(name);
    let age = now.saturating_sub(start);
    let frame = (if looped {
        age % duration
    } else {
        age.min(duration - 1)
    }) * len
        / duration;
    let facing = if s.visual.left { "_left" } else { "" };
    effect(
        sim,
        id,
        &format!("body_{name}{facing}_{frame}"),
        at,
        FRAME_TICKS,
    );
    let (mode, fill) = if full {
        let fill = (s.armor.reserve * GAUGE_STEPS + s.armor.capacity / 2) / s.armor.capacity;
        ("armor", fill.clamp(1, GAUGE_STEPS))
    } else {
        ("recharge", s.armor.energy * GAUGE_STEPS / 100)
    };
    effect(
        sim,
        id,
        &format!("firaga_gauge_{mode}_{fill}"),
        (at.0, at.1.saturating_sub(FIRAGA_GAUGE_HEIGHT)),
        FRAME_TICKS,
    );
    if name == "armored_ult_spin" && frame == 0 {
        trace(
            sim,
            id,
            if walking {
                "R body loop moving"
            } else {
                "R body loop stationary"
            },
        );
    }
    s.visual.next_frame = now + FRAME_TICKS;
    s.combat.draw_effects(sim, id, at, full);
}

pub(super) fn death(sim: &mut StableSim<'_>, id: usize, at: Pos, left: bool) {
    let name = if left { "death_left" } else { "death" };
    effect(sim, id, name, at, 44);
}
