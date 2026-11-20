//! Atlas renders through the same native pose and positional effect APIs as Hanzo.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Motion {
    pub name: &'static str,
    pub until: usize,
}

pub(super) fn motion(s: &mut State, name: &'static str, now: usize, duration: usize) {
    s.motion = Some(Motion {
        name,
        until: now + duration,
    });
    s.pose_dirty = true;
}

fn visible(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}

fn stamp(sim: &mut StableSim<'_>, id: usize, p: Pos, effect: &str) {
    sim.play_view_effect(effect, id, &InputTargetV1::pos(p.0, p.1), 0, 0, 0);
}

pub(super) fn fx(sim: &mut StableSim<'_>, id: usize, p: Pos, name: &str) {
    if visible(sim) {
        stamp(sim, id, p, &format!("mlbb_atlas_fx_{name}"));
    }
}

pub(super) fn death(sim: &mut StableSim<'_>, id: usize, p: Pos, ejected: bool, left: bool) {
    let name = if ejected { "pilot_dead" } else { "dead" };
    fx(sim, id, p, &format!("{name}{}", suffix(left)));
}

fn suffix(left: bool) -> &'static str {
    if left {
        "_left"
    } else {
        ""
    }
}

pub(super) fn pose(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let now = sim.tick();
    let active = s.motion.filter(|m| m.until > now);
    let moving = s.moving_until > now;
    let fallback = match (s.ejection.is_some(), moving) {
        (true, true) => "pilot_move",
        (true, false) => "pilot_idle",
        (false, true) => "run",
        (false, false) => "idle",
    };
    let name = active.map_or(fallback, |m| m.name);
    let binding = format!("mlbb_atlas_pose_{name}{}", suffix(s.left));
    if !s.pose_dirty && s.pose.as_deref() == Some(binding.as_str()) {
        return;
    }
    remove_pose(sim, id, s);
    sim.add_buff(id, &BuffV1::named(&binding));
    if name == "skill" {
        sim.add_buff(
            id,
            &BuffV1::timed(&format!("mlbb_atlas_s1_ice{}", suffix(s.left)), 36),
        );
    }
    s.pose = Some(binding);
    s.pose_dirty = false;
}

pub(super) fn sample(now: usize) -> bool {
    now % 4 == 0
}

pub(super) fn suit(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let now = sim.tick();
    let Some(e) = s.ejection.as_ref() else {
        return;
    };
    if now < s.sampled_at || !visible(sim) {
        return;
    }
    s.sampled_at = now + 4;
    // The hatch clip has the ejection pictured in the owner's frames.
    // Switch to separate world-space pilot and suit after those frames.
    if s.motion
        .is_some_and(|m| m.name == "skill2" && m.until > now)
    {
        return;
    }
    let (name, frame) = if let Some(begin) = e.suit_attack.filter(|t| *t + 36 > now) {
        ("suit_s1", (now - begin) / 4)
    } else if e.route.is_empty() {
        ("suit_follow", 0)
    } else {
        ("suit_follow", (now - e.since) / 4 % 9)
    };
    stamp(
        sim,
        id,
        e.suit,
        &format!("mlbb_atlas_{name}{}_{frame}", suffix(e.left)),
    );
    if !e.route.is_empty() && now % 20 == 0 {
        fx(sim, id, e.suit, "sentry_wake");
    }
}

pub(super) fn chains(sim: &mut StableSim<'_>, id: usize, p: Pos, links: &[EntityKey]) {
    if !visible(sim) {
        return;
    }
    let frame = sim.tick() / 4 % 9;
    for target in links {
        let Some(pos) = sim
            .get_entity(target.entity_id)
            .filter(|e| e.is_alive())
            .map(|e| e.pos())
        else {
            continue;
        };
        let angle = (pos.1 as f64 - p.1 as f64).atan2(pos.0 as f64 - p.0 as f64);
        let bucket = (angle.rem_euclid(std::f64::consts::PI) * 12. / std::f64::consts::PI).round()
            as usize
            % 12;
        let length = distance(p, pos);
        let count = (length / 14_000.).ceil().clamp(1., 5.) as usize;
        let name = format!("mlbb_atlas_chain_{bucket}_{frame}");
        for i in 0..count {
            let pos = toward(p, pos, length * (i as f64 + 0.5) / count as f64);
            stamp(sim, id, pos, &name);
        }
    }
}
