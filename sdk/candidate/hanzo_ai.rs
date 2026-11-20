//! Skill-only substitutions on the native AI's current combat target.
//! None means pass through: never create Move/Return/hold/escape inputs here.
use super::*;
pub(super) fn choices(
    sim: &StableSim<'_>,
    id: usize,
    s: &State,
    base: Option<InputV1>,
) -> Vec<InputV1> {
    let Some(base) = base else {
        return vec![];
    };
    if matches!(
        InputKindV1::from_code(base.kind),
        Some(InputKindV1::Move | InputKindV1::Return)
    ) {
        return vec![];
    }
    let Some(e) = sim.get_entity(id).filter(|e| e.is_alive()) else {
        return vec![];
    };
    let now = sim.tick();
    if s.form.input_locked()
        || s.dash.is_some()
        || s.snatch.is_some()
        || s.feast.is_some()
        || visual::locked(s, now)
    {
        return vec![];
    }
    // Preserve native targets, task planning, recalls and ongoing actions.
    if base.target.kind != InputTargetKindV1::Target.code()
        || !foe(sim, id, base.target.target_id, false)
    {
        return vec![];
    }
    let target = base.target.target_id;
    let ready = |slot| {
        e.cooldown(slot)
            .is_some_and(|c| c.unlocked && c.charges > 0 && c.remaining == 0)
    };
    let spirit = s.form.projected();
    let mut out = vec![];
    if ready(SkillSlotV1::Ult) {
        if let Form::Spirit { since, .. } = s.form {
            if now >= since + 120 && e.hp().0 * 4 < e.hp().1 {
                out.push(InputV1::action(InputKindV1::Ult, InputTargetV1::default()));
            }
        } else if !s.blocked_architecture
            && now >= s.ready[2]
            && sim.get_entity(target).is_some_and(|t| t.is_champion())
        {
            out.push(InputV1::action(InputKindV1::Ult, InputTargetV1::default()));
        }
    }
    let edge = edge_distance(sim, id, target);
    if ready(SkillSlotV1::Skill)
        && now >= combat::clock(s, 0)
        && combat::can_feast(sim, id, s, target)
        && edge <= if spirit { 75000. } else { 28000. }
    {
        let five = s
            .marks
            .get(&crate::entity_key_ro(sim, target))
            .is_some_and(|(n, end)| *n == 5 && *end > now);
        if !spirit
            || five
            || edge > 18000.
            || sim
                .get_entity(target)
                .is_some_and(|t| {
                    t.hp().0 <= combat::snatch_base(e.level().saturating_sub(1), bonus(sim, id))
                })
        {
            out.push(InputV1::action(
                InputKindV1::Skill,
                InputTargetV1::target(target),
            ));
        }
    }
    if ready(SkillSlotV1::Skill2) && now >= combat::clock(s, 1) {
        let to = position(sim, target).unwrap();
        let d = distance(e.pos(), to);
        if d > if spirit { 45000. } else { 35000. } && d < 90000. {
            let u = direction(e.pos(), to);
            out.push(InputV1::action(
                InputKindV1::Skill2,
                InputTargetV1::dir((u.0 * 100000.) as i64, (u.1 * 100000.) as i64),
            ));
        }
    }
    // An unavailable skill can fall back to a basic on this same native target.
    if base.kind != InputKindV1::Attack.code()
        && edge <= 18000.
        && e.cooldown(SkillSlotV1::Attack)
            .is_none_or(|c| c.remaining == 0)
    {
        out.push(InputV1::action(
            InputKindV1::Attack,
            InputTargetV1::target(target),
        ));
    }
    out
}
