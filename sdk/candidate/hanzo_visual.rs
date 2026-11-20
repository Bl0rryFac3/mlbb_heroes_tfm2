//! Owner animation sheets, bounded pose changes and simulation-tick effects.
use super::*;
#[derive(Clone, Debug, Default)]
pub(super) struct Visual {
    pub action: Option<(&'static str, usize)>,
    pub moving: bool,
    pub changed: bool,
}
pub(super) fn locked(s: &State, now: usize) -> bool {
    s.visual.action.is_some_and(|(name,end)| end>now && matches!(name,"human_attack"|"spirit_attack"|"human_feast"|"spirit_snatch"|"human_dash"|"spirit_dash"))
}
pub(super) fn action(s: &mut State, name: &'static str, now: usize, ticks: usize) {
    s.visual.action = Some((name, now + ticks));
    s.visual.changed = true;
}
pub(super) fn fx(sim: &mut StableSim<'_>, id: usize, p: Pos, name: &str, ticks: u64) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
    {
        sim.play_view_effect(
            &format!("mlbb_hanzo_fx_{name}"),
            id,
            &InputTargetV1::pos(p.0, p.1),
            0,
            0,
            ticks,
        );
    }
}
pub(super) fn pose(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    let name = s
        .visual
        .action
        .filter(|(_, until)| *until > sim.tick())
        .map(|(n, _)| n)
        .unwrap_or(if s.form.projected() {
            if s.visual.moving {
                "spirit_move"
            } else {
                "spirit_idle"
            }
        } else if s.visual.moving {
            "human_run"
        } else {
            "human_idle"
        });
    let binding = super::bindings::pose(name, s.left);
    if s.pose != Some(binding) || s.visual.changed {
        if let Some(old) = s.pose.take() {
            sim.entity_remove_buff(id, old);
        }
        sim.add_buff(id, &BuffV1::named(binding));
        s.pose = Some(binding);
        s.visual.changed = false;
    }
}
pub(super) fn death(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    if let Some(p) = position(sim, id) {
        let name = if s.form.projected() {
            "spirit_break"
        } else {
            "human_death"
        };
        fx(
            sim,
            id,
            p,
            &format!("{name}{}", if s.left { "_left" } else { "" }),
            54,
        );
    }
}

fn stamp(sim: &mut StableSim<'_>, id: usize, name: &str, p: Pos, _ticks: u64) {
    sim.play_view_effect(
        &format!("mlbb_hanzo_overlay_{name}"),
        id,
        &InputTargetV1::pos(p.0, p.1),
        0,
        0,
        0,
    );
}
pub(super) fn overlays(sim: &mut StableSim<'_>, id: usize, s: &mut State) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        == Some(SimOriginKindV1::ServerPresim)
        || sim.tick() < s.overlay_next
    {
        return;
    }
    let now = sim.tick();
    s.overlay_next = now + 6;
    if s.form != Form::Human {
        let age = now.saturating_sub(s.body_since);
        let name = if matches!(s.form, Form::Returning { .. }) {
            "body_merge"
        } else if age < 18 {
            "human_project"
        } else {
            "parked_body"
        };
        let count = bindings::world(name).0.max(1);
        let frame = if let Form::Returning { until, .. } = s.form {
            ((18 - until.saturating_sub(now).min(18)) * count / 18).min(count - 1)
        } else if age < 18 {
            (age * count / 18).min(count - 1)
        } else {
            (age / 8) % count
        };
        stamp(
            sim,
            id,
            &format!("{name}{}_{frame}", if s.body_left { "_left" } else { "" }),
            s.body,
            6,
        );
        let ward_count = bindings::world("hanzo_body_ward").0.max(1);
        stamp(
            sim,
            id,
            &format!("hanzo_body_ward_{}", age / 8 % ward_count),
            s.body,
            6,
        );
        if let Some(to) = position(sim, id) {
            let a = (s.body.0, s.body.1.saturating_sub(24000));
            let b = (to.0, to.1.saturating_sub(24000));
            let d = distance(a, b);
            if d > 2000. {
                let angle = ((b.1 as f64 - a.1 as f64).atan2(b.0 as f64 - a.0 as f64))
                    .rem_euclid(std::f64::consts::PI);
                let bucket = ((angle / std::f64::consts::PI * 12.).round() as usize) % 12;
                let segments = (d / 48000.).ceil().clamp(1., 16.) as usize;
                let count = bindings::world("hanzo_spirit_link").0.max(1);
                let f = age / 6 % count;
                for i in 0..segments {
                    let t = (i as f64 + 0.5) / segments as f64;
                    let p = (
                        (a.0 as f64 + (b.0 as f64 - a.0 as f64) * t).max(0.) as u64,
                        (a.1 as f64 + (b.1 as f64 - a.1 as f64) * t).max(0.) as u64,
                    );
                    stamp(sim, id, &format!("link_{bucket}_{f}"), p, 6);
                }
            }
        }
        if s.overlay_logs < 2 {
            s.overlay_logs += 1;
            trace(
                sim,
                id,
                s,
                "NATIVE_OVERLAY body/link emitted via positional view effects",
            );
        }
    }
    // Marks share the same native path as the already visible hit/skill effects.
    if now >= s.mark_next {
        s.mark_next = now + 12;
        let rows: Vec<_> = s
            .marks
            .iter()
            .filter(|(k, (_, end))| *end > now && owned(sim, **k))
            .filter_map(|(k, (n, _))| position(sim, k.entity_id).map(|p| (p, *n)))
            .take(16)
            .collect();
        for (p, n) in rows {
            stamp(
                sim,
                id,
                &format!("mark_{}", n.clamp(1, 5)),
                p,
                12,
            );
        }
    }
}
