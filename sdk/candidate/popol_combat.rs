//! Projectile receipts and Alpha bite payoff. All damage uses simulation events.
use super::*;
const SHOT_SLOTS: usize = 64;
const SHOT_LIFETIME: usize = 600;
pub(super) const ALPHA_BITE_RADIUS: f64 = 18_000.;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SpearKind {
    Basic,
    Command,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flight {
    target: EntityKey,
    kind: SpearKind,
    amount: usize,
    launched: usize,
    expires: usize,
    cancelled: bool,
}
#[derive(Default)]
struct Book {
    active: HashMap<usize, Flight>,
    cursor: usize,
    last_basic: Option<usize>,
    traces: usize,
}
static FLIGHTS: LazyLock<Mutex<HashMap<EntityKey, Book>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn effect(slot: usize) -> String {
    format!("mlbb_heroes:popol_spear_hit_{slot}")
}
fn viewer_trace(sim: &StableSim<'_>) -> bool {
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
pub(super) fn reset(key: EntityKey) {
    if let Some(book) = FLIGHTS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get_mut(&key)
    {
        // Keep cancelled in-flight slots reserved until their callback/expiry.
        // An old missile must not consume a new shot after an owner respawn.
        for flight in book.active.values_mut() {
            flight.cancelled = true;
        }
        book.last_basic = None;
    }
}

pub(super) fn launch(
    sim: &mut StableSim<'_>,
    owner: usize,
    target: usize,
    kind: SpearKind,
) -> bool {
    if !foe(sim, owner, target, kind == SpearKind::Basic) {
        return false;
    }
    let key = crate::entity_key(sim, owner);
    let target_key = crate::entity_key(sim, target);
    let now = sim.tick();
    let e = sim.get_entity(owner).unwrap();
    let start = e.pos();
    let team = e.team();
    let ad = e.stat().attack;
    let end = pos(sim, target).unwrap();
    let flight = Flight {
        target: target_key,
        kind,
        amount: if kind == SpearKind::Basic {
            ad
        } else {
            50 + ad / 2
        },
        launched: now,
        expires: now + SHOT_LIFETIME,
        cancelled: false,
    };
    let (slot, trace) = {
        let mut books = FLIGHTS.lock().unwrap_or_else(|e| e.into_inner());
        let book = books.entry(key).or_default();
        book.active.retain(|_, f| f.expires >= now);
        if kind == SpearKind::Basic && book.last_basic == Some(now) {
            return false;
        }
        let Some(slot) = (0..SHOT_SLOTS)
            .map(|n| (book.cursor + n) % SHOT_SLOTS)
            .find(|s| !book.active.contains_key(s))
        else {
            crate::wlog(format!(
                "[POPOL combat] spear rejected owner={owner} tick={now} reason=flight-cap"
            ));
            return false;
        };
        book.cursor = (slot + 1) % SHOT_SLOTS;
        book.active.insert(slot, flight);
        if kind == SpearKind::Basic {
            book.last_basic = Some(now);
        }
        let trace = book.traces < 16 || kind == SpearKind::Command;
        book.traces += 1;
        (slot, trace)
    };
    let ok = sim.spawn_projectile(
        "mlbb_pk_spear",
        &effect(slot),
        &ProjectileSpawnV1 {
            caster_id: owner,
            team,
            x: start.0,
            y: start.1,
            radius: 1000,
            speed: 6000,
            move_kind: ProjectileMoveKindV1::Target.code(),
            target_id: target,
            target_x: end.0,
            target_y: end.1,
            penetrate: false,
            attack_type: if kind == SpearKind::Basic {
                AttackTypeV1::BaseAttack.code()
            } else {
                AttackTypeV1::Skill.code()
            },
            casting_type: CastingTypeV1::Targeting.code(),
            casting_target: if kind == SpearKind::Basic {
                CastingTargetV1::Enemy.code()
            } else {
                CastingTargetV1::EnemyWithoutTower.code()
            },
        },
    );
    if !ok {
        if let Some(book) = FLIGHTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
        {
            if book.active.get(&slot) == Some(&flight) {
                book.active.remove(&slot);
            }
        }
    }
    if viewer_trace(sim) && (trace || !ok) {
        crate::wlog(format!("[POPOL combat] spear launch owner={owner} target={target} slot={slot} kind={kind:?} tick={now} raw={} spawned={ok}; no launch damage",flight.amount));
    }
    ok
}
#[derive(Clone, Copy)]
struct SpearHit(usize);
impl StableEffectType for SpearHit {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, owner: usize, input: InputTargetV1) {
        if input.kind != InputTargetKindV1::Target.code() {
            return;
        }
        let key = crate::entity_key_ro(sim, owner);
        // Each in-flight slot has its own native callback. Consume BEFORE damage,
        // so a repeated landing cannot steal another outstanding spear's receipt.
        let flight = FLIGHTS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
            .and_then(|b| b.active.remove(&self.0));
        let Some(flight) = flight else {
            return;
        };
        if flight.cancelled
            || sim.tick() > flight.expires
            || input.target_id != flight.target.entity_id
            || !alive(sim, flight.target)
            || !owned(sim, key)
            || !foe(sim, owner, input.target_id, flight.kind == SpearKind::Basic)
        {
            return;
        }
        let attack = if flight.kind == SpearKind::Basic {
            AttackTypeV1::BaseAttack
        } else {
            AttackTypeV1::Skill
        };
        let before = cooperation::health(sim, input.target_id);
        sim.deal_damage_typed(
            owner,
            input.target_id,
            flight.amount,
            DamageTypeV1::Ad,
            attack,
        );
        if let Some(state) = STATES
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key)
        {
            if cooperation::health(sim, input.target_id) < before
                && alive(sim, key)
                && state.kupa.is_some_and(|k| alive(sim, k))
                && !matches!(
                    sim.get_entity(input.target_id).and_then(|e| e.kind()),
                    Some(EntityKindV1::Tower | EntityKindV1::Nexus)
                )
            {
                state.cooperation.hit(flight.target, sim.tick(), false);
            }
            state.last_combat = sim.tick();
            state.prayer_started = None;
        }
        if viewer_trace(sim) && (flight.kind == SpearKind::Command || sim.tick() < 600) {
            crate::wlog(format!("[POPOL combat] spear impact owner={owner} target={} slot={} kind={:?} tick={} flight_ticks={} raw={}",input.target_id,self.0,flight.kind,sim.tick(),sim.tick().saturating_sub(flight.launched),flight.amount));
        }
    }
}
pub(super) fn register(module: &mut StableMod) {
    for slot in 0..SHOT_SLOTS {
        module.add_native_effect(&effect(slot), SpearHit(slot));
    }
}

pub(super) fn alpha_bite(
    sim: &mut StableSim<'_>,
    owner: usize,
    center: Pos,
    amount: usize,
) -> Vec<usize> {
    let mut victims: Vec<usize> = (0..sim.entity_count())
        .filter_map(|i| sim.entity_at(i))
        .filter(|e| foe(sim, owner, e.id(), false) && dist(e.pos(), center) <= ALPHA_BITE_RADIUS)
        .map(|e| e.id())
        .collect();
    victims.sort_unstable();
    victims.dedup();
    let mut damaged = Vec::new();
    for &target in &victims {
        let before = cooperation::health(sim, target);
        sim.deal_damage_typed(owner, target, amount, DamageTypeV1::Ad, AttackTypeV1::Skill);
        if cooperation::health(sim, target) < before {
            damaged.push(target);
        }
        if sim.get_entity(target).is_some_and(|e| e.is_alive()) {
            sim.apply_cc(target, &CcV1::of_kind(CcKindV1::Airborne, 36));
        }
    }
    if viewer_trace(sim) {
        crate::wlog(format!("[POPOL combat] Alpha S1 area owner={owner} tick={} center={center:?} radius={ALPHA_BITE_RADIUS} raw={amount} victims={victims:?}",sim.tick()));
    }
    damaged
}

#[cfg(test)]
pub(super) fn impact(sim: &mut StableSim<'_>, effect_name: &str, owner: usize, target: usize) {
    let slot = effect_name
        .strip_prefix("mlbb_heroes:popol_spear_hit_")
        .unwrap()
        .parse()
        .unwrap();
    SpearHit(slot).apply(sim, 0, owner, InputTargetV1::target(target));
}
