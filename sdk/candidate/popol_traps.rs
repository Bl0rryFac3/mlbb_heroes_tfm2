//! Bounded automatic traps, a TFM2 adaptation of the fourth MLBB active.
use super::*;
const ARM: usize = 30;
const LIFE: usize = 900;
const COOLDOWN: usize = 600;
const FIELD_LIFE: usize = 180;
const STEP: usize = 8;
const TRIGGER: f64 = 12_000.;
const FROST: f64 = 20_000.;
#[derive(Clone, Copy, Debug)]
struct Trap {
    id: usize,
    p: Pos,
    born: usize,
    armed: usize,
    until: usize,
}
#[derive(Clone, Copy, Debug)]
struct Field {
    p: Pos,
    born: usize,
    until: usize,
}
#[derive(Clone, Default)]
pub(super) struct Traps {
    active: Vec<Trap>,
    fields: Vec<Field>,
    affected: Vec<EntityKey>,
    next_place: usize,
    next_try: usize,
    next_scan: usize,
    next_view: usize,
    next_id: usize,
}
impl Traps {
    pub(super) fn may_attempt(&self, now: usize) -> bool {
        self.active.len() < 2 && now >= self.next_place && now >= self.next_try
    }
}
fn buff(key: EntityKey) -> String {
    format!("mlbb_pk_frost_{}_{}", key.entity_id, key.copy)
}
fn visible(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
}
fn effect(sim: &mut StableSim<'_>, owner: usize, p: Pos, name: &str, time: usize) {
    if visible(sim) {
        sim.play_view_effect(
            name,
            owner,
            &InputTargetV1::pos(p.0, p.1),
            0,
            0,
            time as u64,
        );
    }
}
fn trace(sim: &StableSim<'_>, message: String) {
    if visible(sim) {
        crate::wlog(message);
    }
}
pub(super) fn clear(sim: &mut StableSim<'_>, key: EntityKey, state: &mut Traps) {
    let name = buff(key);
    for target in state.affected.drain(..) {
        if owned(sim, target) {
            sim.entity_remove_buff(target.entity_id, &name);
        }
    }
    state.active.clear();
    state.fields.clear();
}
fn legal(grid: &crate::Grid, p: Pos) -> bool {
    [
        (0., 0.),
        (-8000., 0.),
        (8000., 0.),
        (0., -8000.),
        (0., 8000.),
    ]
    .into_iter()
    .all(|(x, y)| !grid.solid(p.0 as f64 + x, p.1 as f64 + y))
}
fn placement(sim: &StableSim<'_>, owner: usize, nav: &mut Navigation) -> Option<Pos> {
    let a = pos(sim, owner)?;
    let team = sim.get_entity(owner)?.team();
    let target = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| {
            foe(sim, owner, e.id(), false)
                && dist(a, e.pos()) <= LEASH
                && sim.is_visible(team, e.id())
        })
        .min_by_key(|e| (!e.is_champion(), dist(a, e.pos()) as u64, e.id()))?
        .id();
    let b = pos(sim, target)?;
    let (_, grid, changed) = nav.layout.select(sim, a, b);
    if changed {
        nav.clear_route();
    }
    let dx = a.0 as f64 - b.0 as f64;
    let dy = a.1 as f64 - b.1 as f64;
    let n = dx.hypot(dy).max(1.);
    [8000., 0., 16000.]
        .into_iter()
        .map(|offset| {
            (
                (b.0 as f64 + dx / n * offset).max(0.) as u64,
                (b.1 as f64 + dy / n * offset).max(0.) as u64,
            )
        })
        .find(|p| dist(*p, a) <= LEASH && legal(&grid, *p))
}
pub(super) fn tick(
    sim: &mut StableSim<'_>,
    key: EntityKey,
    state: &mut Traps,
    nav: &mut Navigation,
    allow_place: bool,
) -> bool {
    if state.active.is_empty()
        && state.fields.is_empty()
        && state.affected.is_empty()
        && !allow_place
    {
        return false;
    }
    let now = sim.tick();
    let owner = key.entity_id;
    if !alive(sim, key) {
        clear(sim, key, state);
        return false;
    }
    let old_fields = state.fields.len();
    state.fields.retain(|f| now < f.until);
    state.active.retain(|t| now < t.until);
    if allow_place && state.active.len() < 2 && now >= state.next_place && now >= state.next_try {
        state.next_try = now + 30;
        if let Some(p) = placement(sim, owner, nav) {
            if state.active.iter().all(|t| dist(t.p, p) >= TRIGGER) {
                let id = state.next_id;
                state.next_id += 1;
                state.active.push(Trap {
                    id,
                    p,
                    born: now,
                    armed: now + ARM,
                    until: now + LIFE,
                });
                state.next_place = now + COOLDOWN;
                trace(sim,format!("[POPOL traps] placed owner={owner} id={id} tick={now} p={p:?} arms={} expires={}",now+ARM,now+LIFE));
            }
        }
    }
    let scan = now >= state.next_scan;
    let mut triggered = false;
    if scan {
        state.next_scan = now + STEP;
        // One entity snapshot only while a trap/field is active, never a UI scan.
        if !state.active.is_empty() || !state.fields.is_empty() {
            let enemies: Vec<(usize, Pos)> = (0..sim.entity_count())
                .filter_map(|i| sim.entity_at(i))
                .filter(|e| foe(sim, owner, e.id(), false))
                .map(|e| (e.id(), e.pos()))
                .collect();
            let mut keep = Vec::with_capacity(2);
            for trap in std::mem::take(&mut state.active) {
                let victim = (now >= trap.armed)
                    .then(|| {
                        enemies
                            .iter()
                            .filter(|(_, p)| dist(*p, trap.p) <= TRIGGER)
                            .min_by_key(|(id, p)| (dist(*p, trap.p) as u64, *id))
                            .map(|(id, _)| *id)
                    })
                    .flatten();
                if let Some(target) = victim.filter(|t| foe(sim, owner, *t, false)) {
                    let amount = 25 + sim.get_entity(owner).unwrap().stat().attack / 5;
                    sim.deal_damage_typed(
                        owner,
                        target,
                        amount,
                        DamageTypeV1::Ad,
                        AttackTypeV1::Skill,
                    );
                    if sim.get_entity(target).is_some_and(|e| e.is_alive()) {
                        sim.apply_cc(target, &CcV1::of_kind(CcKindV1::Bind, 36));
                    }
                    effect(sim, owner, trap.p, "mlbb_pk_trap_trigger", 36);
                    if state.fields.len() == 2 {
                        state.fields.remove(0);
                    }
                    state.fields.push(Field {
                        p: trap.p,
                        born: now,
                        until: now + FIELD_LIFE,
                    });
                    triggered = true;
                    trace(sim,format!("[POPOL traps] triggered owner={owner} id={} target={target} tick={now} raw={amount} frost_until={}",trap.id,now+FIELD_LIFE));
                } else {
                    keep.push(trap);
                }
                if !alive(sim, key) {
                    clear(sim, key, state);
                    return triggered;
                }
            }
            state.active = keep;
            refresh(sim, key, state, &enemies);
        } else if !state.affected.is_empty() {
            clear(sim, key, state);
        }
    } else if old_fields != state.fields.len() {
        // Expiry cleanup must not leave the previous short buff lease behind.
        let enemies: Vec<_> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i))
            .filter(|e| foe(sim, owner, e.id(), false))
            .map(|e| (e.id(), e.pos()))
            .collect();
        refresh(sim, key, state, &enemies);
    }
    if now >= state.next_view && visible(sim) {
        state.next_view = now + STEP;
        for t in &state.active {
            effect(
                sim,
                owner,
                t.p,
                &format!("mlbb_pk_trap_idle_{}", ((now - t.born) / STEP) % 9),
                STEP,
            );
        }
        for f in &state.fields {
            effect(
                sim,
                owner,
                f.p,
                &format!("mlbb_pk_trap_frost_loop_{}", ((now - f.born) / STEP) % 9),
                STEP,
            );
        }
    }
    triggered
}
fn refresh(sim: &mut StableSim<'_>, key: EntityKey, state: &mut Traps, enemies: &[(usize, Pos)]) {
    let name = buff(key);
    let mut wanted = Vec::new();
    for &(id, p) in enemies {
        if foe(sim, key.entity_id, id, false) && state.fields.iter().any(|f| dist(p, f.p) <= FROST)
        {
            wanted.push(crate::entity_key(sim, id));
        }
    }
    wanted.sort_by_key(|k| k.entity_id);
    wanted.dedup();
    for target in &state.affected {
        if !wanted.contains(target) && owned(sim, *target) {
            sim.entity_remove_buff(target.entity_id, &name);
        }
    }
    for target in &wanted {
        sim.entity_remove_buff(target.entity_id, &name);
        sim.add_buff(
            target.entity_id,
            &BuffV1 {
                move_speed_mult: -20,
                defence_mult: -12,
                ..BuffV1::timed(&name, STEP + 1)
            },
        );
    }
    state.affected = wanted;
}

#[cfg(test)]
impl Traps {
    pub(super) fn seed_trap(&mut self, p: Pos, armed: usize, until: usize) {
        let id = self.next_id;
        self.next_id += 1;
        self.active.push(Trap {
            id,
            p,
            born: 0,
            armed,
            until,
        });
    }
    pub(super) fn seed_field(&mut self, p: Pos, until: usize) {
        self.fields.push(Field { p, born: 0, until });
    }
    pub(super) fn counts(&self) -> (usize, usize) {
        (self.active.len(), self.fields.len())
    }
    pub(super) fn deadline(&self) -> usize {
        self.next_place
    }
    pub(super) fn name(key: EntityKey) -> String {
        buff(key)
    }
}
