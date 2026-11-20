//! Owner art only. Pose changes and emission/hit events drive bounded cosmetics.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Motion {
    Idle,
    Run,
    Fire,
    Rondo,
    Cannon,
    Roll,
    Hit,
}
#[derive(Clone, Debug, Default)]
pub(super) struct Visual {
    pub action: Option<(Motion, usize)>,
    pub left: bool,
    previous: Option<Pos>,
    pose: Option<&'static str>,
    pub restart: bool,
}
fn cosmetics(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}
fn binding(m: Motion, left: bool) -> &'static str {
    use Motion::*;
    match (m, left) {
        (Idle, false) => "mlbb_granger_pose_idle",
        (Idle, true) => "mlbb_granger_pose_idle_left",
        (Run, false) => "mlbb_granger_pose_run",
        (Run, true) => "mlbb_granger_pose_run_left",
        (Fire, false) => "mlbb_granger_pose_fire",
        (Fire, true) => "mlbb_granger_pose_fire_left",
        (Rondo, false) => "mlbb_granger_pose_rondo",
        (Rondo, true) => "mlbb_granger_pose_rondo_left",
        (Cannon, false) => "mlbb_granger_pose_cannon",
        (Cannon, true) => "mlbb_granger_pose_cannon_left",
        (Roll, false) => "mlbb_granger_pose_roll",
        (Roll, true) => "mlbb_granger_pose_roll_left",
        (Hit, false) => "mlbb_granger_pose_hit",
        (Hit, true) => "mlbb_granger_pose_hit_left",
    }
}
pub(super) fn action(
    s: &mut State,
    motion: Motion,
    now: usize,
    ticks: usize,
    direction: (f64, f64),
) {
    s.visual.action = Some((motion, now + ticks));
    if direction.0.abs() > 0.01 {
        s.visual.left = direction.0 < 0.;
    }
    s.visual.restart = true;
}
pub(super) fn once(sim: &mut StableSim<'_>, owner: usize, at: Pos, name: &str, ticks: u64) {
    if cosmetics(sim) {
        sim.play_view_effect(
            &format!("mlbb_granger_fx_{name}"),
            owner,
            &InputTargetV1::pos(at.0, at.1),
            0,
            0,
            ticks,
        );
    }
}
pub(super) fn emitted(
    sim: &mut StableSim<'_>,
    owner: usize,
    s: &mut State,
    kind: ShotKind,
    target: usize,
) {
    let Some(from) = pos(sim, owner) else { return };
    let direction = unit(from, pos(sim, target).unwrap_or(from));
    // Dash/roll and cannon body poses take precedence over concurrent S1 rounds.
    let busy = s.visual.action.is_some_and(|(m, t)| {
        t > sim.tick() && matches!(m, Motion::Rondo | Motion::Roll | Motion::Cannon)
    });
    if kind == ShotKind::Cannon {
        action(s, Motion::Cannon, sim.tick(), 12, direction);
    } else if !busy {
        action(s, Motion::Fire, sim.tick(), 18, direction);
    }
    let left = direction.0 < 0.;
    let at = (
        if left {
            from.0.saturating_sub(28000)
        } else {
            from.0.saturating_add(28000)
        },
        from.1.saturating_sub(26000),
    );
    // Cannon sheet already contains its muzzle flash; keep the supplied external
    // cannon muzzle available without stacking a second flash on the same shot.
    if kind != ShotKind::Cannon {
        once(
            sim,
            owner,
            at,
            if left {
                "gun_muzzle_left"
            } else {
                "gun_muzzle"
            },
            9,
        );
    }
}
pub(super) fn clear(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    if let Some(old) = s.visual.pose.take() {
        sim.entity_remove_buff(owner, old);
    }
}
pub(super) fn death(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    if let Some(p) = pos(sim, owner) {
        once(
            sim,
            owner,
            p,
            if s.visual.left { "dead_left" } else { "dead" },
            54,
        );
    }
    clear(sim, owner, s);
    s.visual.action = None;
}
pub(super) fn show(sim: &mut StableSim<'_>, owner: usize, s: &mut State) {
    if !cosmetics(sim) {
        return;
    }
    let Some(p) = pos(sim, owner) else { return };
    let v = &mut s.visual;
    let moving = v.previous.is_some_and(|a| dist(a, p) > 100.);
    let action = v.action.filter(|(_, t)| *t > sim.tick());
    if action.is_none() {
        if let Some(a) = v.previous {
            if a.0.abs_diff(p.0) > 100 {
                v.left = p.0 < a.0;
            }
        }
    }
    v.previous = Some(p);
    let m = action
        .map(|(m, _)| m)
        .unwrap_or(if moving { Motion::Run } else { Motion::Idle });
    let wanted = binding(m, v.left);
    if v.pose == Some(wanted) && !v.restart {
        return;
    }
    v.restart = false;
    if let Some(old) = v.pose.take() {
        sim.entity_remove_buff(owner, old);
    }
    sim.add_buff(owner, &BuffV1::named(wanted));
    v.pose = Some(wanted);
}
