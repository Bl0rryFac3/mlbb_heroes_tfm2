//! Cosmetic state only. No rendered frame triggers gameplay or damage.
use super::*;
include!("popol_visual_bindings.rs");

#[derive(Clone, Default)]
pub(super) struct Visual {
    tick: Option<usize>,
    owner_pos: Option<Pos>,
    kupa_pos: Option<Pos>,
    owner_left: bool,
    kupa_left: bool,
    owner_moving: bool,
    kupa_moving: bool,
    pub owner_action: Option<(&'static str, usize)>,
    pub kupa_action: Option<(&'static str, usize)>,
    pub mounted_at: usize,
    pub dismount_until: usize,
    pub dismount_alpha: bool,
    pub shield_until: usize,
    pub shield_start_until: usize,
    pub revived_until: usize,
    owner_pose: Option<&'static str>,
    kupa_pose: Option<&'static str>,
    owner_fx: u8,
    kupa_fx: u8,
}
fn movement(previous: &mut Option<Pos>, current: Option<Pos>, left: &mut bool) -> bool {
    let moving = match (*previous, current) {
        (Some(a), Some(b)) => {
            if a.0.abs_diff(b.0) > 100 {
                *left = b.0 < a.0;
            }
            dist(a, b) > 100.
        }
        _ => false,
    };
    *previous = current;
    moving
}
fn selected(form: &str, motion: &str, left: bool) -> Option<&'static str> {
    BINDINGS
        .iter()
        .find(|(f, m, l, _)| *f == form && *m == motion && *l == left)
        .map(|x| x.3)
        .or_else(|| {
            BINDINGS
                .iter()
                .find(|(f, m, l, _)| *f == form && *m == "idle" && *l == left)
                .map(|x| x.3)
        })
}
fn cosmetics(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}
fn pose(
    sim: &mut StableSim<'_>,
    id: usize,
    form: Option<&str>,
    motion: &str,
    left: bool,
    current: &mut Option<&'static str>,
) {
    let wanted = form.and_then(|f| selected(f, motion, left));
    if *current == wanted {
        return;
    }
    if let Some(old) = current.take() {
        sim.entity_remove_buff(id, old);
    }
    if let Some(name) = wanted {
        sim.add_buff(id, &BuffV1::named(name));
    }
    *current = wanted;
}
fn fx(sim: &mut StableSim<'_>, id: usize, name: &str, on: bool, mask: &mut u8, bit: u8) {
    let present = *mask & bit != 0;
    if present == on {
        return;
    }
    let name = format!("mlbb_pk_fx_{name}");
    if on {
        sim.add_buff(id, &BuffV1::named(&name));
        *mask |= bit;
    } else {
        sim.entity_remove_buff(id, &name);
        *mask &= !bit;
    }
}
pub(super) fn clear(sim: &mut StableSim<'_>, owner: usize, state: &State) {
    if let Some(name) = state.visual.owner_pose {
        sim.entity_remove_buff(owner, name);
    }
    for (bit, name) in [(1, "shield_start"), (2, "shield_loop"), (4, "prayer")] {
        if state.visual.owner_fx & bit != 0 {
            sim.entity_remove_buff(owner, &format!("mlbb_pk_fx_{name}"));
        }
    }
    if let Some(k) = state.kupa.filter(|k| owned(sim, *k)) {
        if let Some(name) = state.visual.kupa_pose {
            sim.entity_remove_buff(k.entity_id, name);
        }
        for (bit, name) in [(1, "recall"), (2, "revival")] {
            if state.visual.kupa_fx & bit != 0 {
                sim.entity_remove_buff(k.entity_id, &format!("mlbb_pk_fx_{name}"));
            }
        }
    }
}
pub(super) fn once(
    sim: &mut StableSim<'_>,
    owner: usize,
    id: usize,
    name: &str,
    left: bool,
    ticks: u64,
) {
    if !cosmetics(sim) {
        return;
    }
    if let Some(p) = pos(sim, id) {
        sim.play_view_effect(
            &format!("mlbb_pk_once_{name}{}", if left { "_left" } else { "" }),
            owner,
            &InputTargetV1::pos(p.0, p.1),
            0,
            0,
            ticks,
        );
    }
}
pub(super) fn death(sim: &mut StableSim<'_>, owner: usize, state: &State) {
    once(sim, owner, owner, "popol_dead", state.visual.owner_left, 54);
    clear(sim, owner, state);
}

pub(super) fn show(sim: &mut StableSim<'_>, owner: usize, state: &mut State) {
    if !cosmetics(sim) {
        return;
    }
    let now = sim.tick();
    let k = state.kupa.filter(|k| alive(sim, *k));
    let owner_alive = sim.get_entity(owner).is_some_and(|e| e.is_alive());
    let v = &mut state.visual;
    if v.tick != Some(now) {
        v.owner_moving = movement(&mut v.owner_pos, pos(sim, owner), &mut v.owner_left);
        v.kupa_moving = movement(
            &mut v.kupa_pos,
            k.and_then(|k| pos(sim, k.entity_id)),
            &mut v.kupa_left,
        );
        v.tick = Some(now);
    }
    let riding = state.ride_until > now;
    let dismount = v.dismount_until > now;
    let alpha = state.alpha_until > now;
    let (form, motion) = if riding || dismount {
        (
            if (riding && alpha) || (dismount && v.dismount_alpha) {
                "mount_alpha"
            } else {
                "mount"
            },
            if dismount {
                "end"
            } else if now < v.mounted_at + 21 {
                "start"
            } else if v.owner_action.is_some_and(|(m, t)| m == "hit" && t > now) {
                "hit"
            } else if v.owner_moving {
                "run"
            } else {
                "idle"
            },
        )
    } else {
        (
            "popol",
            if state.prayer_started.is_some() {
                "pray"
            } else if let Some((m, _)) = v.owner_action.filter(|(_, t)| *t > now) {
                m
            } else if v.owner_moving {
                "run"
            } else {
                "idle"
            },
        )
    };
    pose(
        sim,
        owner,
        owner_alive.then_some(form),
        motion,
        v.owner_left,
        &mut v.owner_pose,
    );
    let shield = owner_alive
        && v.shield_until > now
        && sim.get_entity(owner).is_some_and(|e| e.shield() > 0);
    fx(
        sim,
        owner,
        "shield_start",
        shield && v.shield_start_until > now,
        &mut v.owner_fx,
        1,
    );
    fx(
        sim,
        owner,
        "shield_loop",
        shield && v.shield_start_until <= now,
        &mut v.owner_fx,
        2,
    );
    fx(
        sim,
        owner,
        "prayer",
        owner_alive && state.prayer_started.is_some(),
        &mut v.owner_fx,
        4,
    );
    if let Some(key) = state.kupa.filter(|k| owned(sim, *k)) {
        let motion = if let Some((m, _)) = v.kupa_action.filter(|(_, t)| *t > now) {
            m
        } else if v.kupa_moving {
            "run"
        } else {
            "idle"
        };
        pose(
            sim,
            key.entity_id,
            (k.is_some() && owner_alive && !riding && !dismount).then_some(if alpha {
                "kupa_alpha"
            } else {
                "kupa"
            }),
            motion,
            v.kupa_left,
            &mut v.kupa_pose,
        );
        fx(
            sim,
            key.entity_id,
            "recall",
            k.is_some() && state.recalling,
            &mut v.kupa_fx,
            1,
        );
        fx(
            sim,
            key.entity_id,
            "revival",
            k.is_some() && v.revived_until > now,
            &mut v.kupa_fx,
            2,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn left_motion_preserves_form() {
        assert_eq!(
            selected("mount_alpha", "run", true),
            Some("mlbb_pk_mount_alpha_run_left")
        );
    }
    #[test]
    fn missing_alpha_hit_uses_same_form_idle() {
        assert_eq!(selected("kupa_alpha", "hit", false), Some(ALPHA_BODY));
    }
    #[test]
    fn direction_updates_only_after_horizontal_movement() {
        let mut p = Some((1000, 1000));
        let mut left = false;
        assert!(movement(&mut p, Some((800, 1000)), &mut left));
        assert!(left);
        assert!(!movement(&mut p, Some((800, 1000)), &mut left));
        assert!(left);
    }
}
