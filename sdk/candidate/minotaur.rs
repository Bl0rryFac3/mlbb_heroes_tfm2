//! Minotaur's native kit. Body poses use the normal champion atlas; effects
//! are emitted by gameplay at the same contact ticks as the authored poses.

use crate::{
    audio, current_grid, entity_key, entity_key_ro, has_named_buff, units, EntityKey, Grid,
};
use mod_api_stable::*;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

type Pos = (u64, u64);
const HERO: &str = "mlbb_minotaur";
const MOTIVATION: &str = "mlbb_minotaur_motivated";
const ENRAGED: &str = "mlbb_minotaur_enraged";
const RAGE_WINDOW: &str = "mlbb_minotaur_rage_window";
const FURY: &str = "mlbb_minotaur_fury";
const S1_REACH: u64 = 48_000;
const S1_RADIUS: u64 = 22_000;
const S1_ENRAGED_RADIUS: u64 = 28_000;
const S1_LAND: usize = 24;
const STOMP_WAVE_TICKS: u64 = 30;
const ROAR_RADIUS: u64 = 58_000;
const FURY_RADIUS: u64 = 36_000;
const SLAMS: [usize; 3] = [18, 48, 78];
const FURY_DURATION: usize = 84;
const ENRAGED_DURATION: usize = 720;
const MOTIVATION_DURATION: usize = 180;
const CHARGE_DURATION: usize = 120;
const STATS: StatV1 = StatV1 {
    hp: 1000,
    attack: 84,
    magic_power: 20,
    defence: 38,
    magic_resistance: 30,
    move_speed: 1020,
    hp_regen: 7,
    stack: 0,
    crit_chance: 0,
};
const GROWTH: StatV1 = StatV1 {
    hp: 110,
    attack: 7,
    magic_power: 6,
    defence: 5,
    magic_resistance: 4,
    move_speed: 0,
    hp_regen: 0,
    stack: 0,
    crit_chance: 0,
};

#[derive(Clone, Copy)]
struct Unit {
    id: usize,
    team: usize,
    pos: Pos,
    hp: usize,
    max_hp: usize,
    stat: StatV1,
    kind: Option<EntityKindV1>,
    champion: bool,
    targetable: bool,
    structure: bool,
}

impl Unit {
    fn living(e: StableEntity<'_, '_>) -> Option<Self> {
        let (hp, max_hp) = e.hp();
        let kind = e.kind();
        (e.is_alive() && hp != 0).then(|| Self {
            id: e.id(),
            team: e.team(),
            pos: e.pos(),
            hp,
            max_hp,
            stat: e.stat(),
            kind,
            champion: e.is_champion(),
            targetable: e.is_targetable(),
            structure: e.is_tower() || matches!(kind, Some(EntityKindV1::Nexus)),
        })
    }

    fn from_sim(sim: &StableSim<'_>, id: usize) -> Option<Self> {
        sim.get_entity(id).and_then(Self::living)
    }

    fn at_or_below(self, pct: usize) -> bool {
        self.hp * 100 <= self.max_hp * pct
    }
}

fn distance_sq(a: Pos, b: Pos) -> u128 {
    let x = u128::from(a.0.abs_diff(b.0));
    let y = u128::from(a.1.abs_diff(b.1));
    x * x + y * y
}

fn near(a: Pos, b: Pos, radius: u64) -> bool {
    distance_sq(a, b) <= u128::from(radius).pow(2)
}

fn within_radius(sim: &StableSim<'_>, owner: Unit, pos: Pos, radius: u64) -> Vec<Unit> {
    let mut enemies: Vec<_> = (0..sim.entity_count())
        .filter_map(|i| Unit::living(sim.entity_at(i)?))
        .filter(|u| {
            u.team != owner.team && u.targetable && !u.structure && near(pos, u.pos, radius)
        })
        .collect();
    enemies.sort_unstable_by_key(|u| u.id);
    enemies.dedup_by_key(|u| u.id);
    enemies
}

fn champions<'s, 'sim>(sim: &'s StableSim<'sim>) -> impl Iterator<Item = Unit> + 's {
    (0..sim.champion_count())
        .filter_map(|i| Unit::from_sim(sim, sim.champion_id_at(i)))
        .filter(|u| u.champion)
}

fn allies(sim: &StableSim<'_>, caster: Unit, radius: u64) -> Vec<Unit> {
    let mut close: Vec<_> = champions(sim)
        .filter(|a| a.id != caster.id && a.team == caster.team && near(caster.pos, a.pos, radius))
        .collect();
    close.sort_unstable_by_key(|a| a.id);
    close.dedup_by_key(|a| a.id);
    close
}

fn weakest(allies: &[Unit], caster: Unit) -> Option<Unit> {
    allies.iter().copied().min_by(|a, b| {
        (a.hp * b.max_hp)
            .cmp(&(b.hp * a.max_hp))
            .then_with(|| distance_sq(a.pos, caster.pos).cmp(&distance_sq(b.pos, caster.pos)))
            .then(a.id.cmp(&b.id))
    })
}

fn blocked(sim: &StableSim<'_>, id: usize, movement: bool) -> bool {
    sim.get_entity(id).is_some_and(|e| {
        (0..e.cc_count()).filter_map(|i| e.cc_at(i)).any(|cc| {
            let kind = CcKindV1::from_code(cc.kind);
            matches!(
                kind,
                Some(
                    CcKindV1::Stun
                        | CcKindV1::Airborne
                        | CcKindV1::BlockSkill
                        | CcKindV1::ForceMove
                        | CcKindV1::Taunt
                        | CcKindV1::Fear
                        | CcKindV1::Charm
                )
            ) || movement && matches!(kind, Some(CcKindV1::Bind | CcKindV1::BlockMoveSkill))
        })
    })
}

fn fx(sim: &mut StableSim<'_>, id: usize, name: &str, anchor: Pos) {
    play_fx(sim, id, name, anchor, 0);
}

fn play_fx(sim: &mut StableSim<'_>, id: usize, name: &str, anchor: Pos, ticks: u64) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        != Some(SimOriginKindV1::ServerPresim)
    {
        sim.play_view_effect(
            &format!("{HERO}_{name}"),
            id,
            &InputTargetV1::pos(anchor.0, anchor.1),
            0,
            0,
            ticks,
        );
    }
}

fn voice(sim: &mut StableSim<'_>, id: usize, cast: Cast) {
    if !audio::voice_in_match_view(sim) {
        return;
    }
    match cast {
        Cast::Basic => audio::play_voice(sim, id, "mlbb_minotaur_voice_basic"),
        Cast::Stomp => audio::play_priority_voice(sim, id, "mlbb_minotaur_voice_stomp"),
        Cast::Roar => audio::play_priority_voice(sim, id, "mlbb_minotaur_voice_roar"),
        Cast::Fury => audio::play_priority_voice(sim, id, "mlbb_minotaur_voice_fury"),
    };
}

fn timed(sim: &mut StableSim<'_>, id: usize, name: &str, ticks: usize) {
    sim.entity_remove_buff(id, name);
    sim.add_buff(id, &BuffV1::timed(name, ticks));
}

fn motivate(sim: &mut StableSim<'_>, id: usize, pos: Pos) {
    timed(sim, id, MOTIVATION, MOTIVATION_DURATION);
    fx(sim, id, "motivation", (pos.0, pos.1.saturating_sub(8_000)));
}

fn slow(sim: &mut StableSim<'_>, target: usize, name: &str, pct: i32, ticks: usize) {
    let mut buff = BuffV1::timed(name, ticks);
    buff.move_speed_mult = -pct;
    sim.entity_remove_buff(target, name);
    sim.add_buff(target, &buff);
}

fn impact(sim: &mut StableSim<'_>, caster: Unit, target: Unit, damage: usize, kind: DamageTypeV1) {
    if kind == DamageTypeV1::Ad {
        sim.deal_damage(caster.id, target.id, damage, 0, AttackTypeV1::Skill);
    } else {
        sim.deal_damage_typed(caster.id, target.id, damage, kind, AttackTypeV1::Skill);
    }
}

fn airborne(sim: &mut StableSim<'_>, target: Unit, ticks: u64) {
    if !matches!(target.kind, Some(EntityKindV1::Epic | EntityKindV1::Serpen)) {
        sim.apply_cc(target.id, &CcV1::of_kind(CcKindV1::Airborne, ticks));
    }
}

fn bonus_hp(hp: usize) -> usize {
    10 + hp * 2 / 100
}

#[derive(Clone, Copy)]
struct Leap {
    start: usize,
    from: Pos,
    to: Pos,
    last: Pos,
    enraged: bool,
}

#[derive(Clone, Copy)]
struct Fury {
    start: usize,
    pulse: usize,
}

#[derive(Default)]
struct State {
    grid: Option<Grid>,
    leap: Option<Leap>,
    fury: Option<Fury>,
    enraged_until: usize,
    basics_left: u8,
    basics_until: usize,
    casts: [Option<usize>; 4],
    motivation_cast: Option<usize>,
    updated: Option<usize>,
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn take(key: EntityKey) -> Option<State> {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&key)
}

fn save(key: EntityKey, state: State) {
    STATES
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, state);
}

fn lerp(a: Pos, b: Pos, elapsed: u64, total: u64) -> Pos {
    let point = |i: u64, j: u64| {
        if j >= i {
            i + (j - i) * elapsed / total
        } else {
            i - (i - j) * elapsed / total
        }
    };
    (point(a.0, b.0), point(a.1, b.1))
}

fn landing(grid: &Grid, from: Pos, target: Pos) -> Pos {
    if near(from, target, 12_000) {
        return from;
    }
    let length = (distance_sq(from, target) as f64).sqrt().ceil() as u64;
    let limit = length.min(S1_REACH);
    let mut last = from;
    for step in 1..=limit.div_ceil(1_000) {
        let point = lerp(from, target, (step * 1_000).min(limit), length);
        if grid.solid(point.0 as f64, point.1 as f64) {
            break;
        }
        last = point;
    }
    last
}

fn land_stomp(sim: &mut StableSim<'_>, caster: Unit, s: &mut State, position: Pos, rage: bool) {
    let (radius, art, wave) = if rage {
        (S1_ENRAGED_RADIUS, "stomp_enraged", "stomp_wave_enraged")
    } else {
        (S1_RADIUS, "stomp", "stomp_wave")
    };
    let amount =
        55 + caster.stat.attack * 45 / 100 + if rage { bonus_hp(caster.max_hp) } else { 0 };
    fx(sim, caster.id, art, position);
    play_fx(sim, caster.id, wave, position, STOMP_WAVE_TICKS);
    s.basics_left = 2;
    s.basics_until = sim.tick() + CHARGE_DURATION;
    for target in within_radius(sim, caster, position, radius) {
        impact(sim, caster, target, amount, DamageTypeV1::Ad);
        if Unit::from_sim(sim, target.id).is_some() {
            airborne(sim, target, 24);
            slow(sim, target.id, "mlbb_minotaur_stomp_slow", 40, 84);
        }
    }
}

fn tick_leap(sim: &mut StableSim<'_>, caster: Unit, s: &mut State) {
    let Some(mut leap) = s.leap.take() else {
        return;
    };
    let now = sim.tick();
    if blocked(sim, caster.id, true) || !near(caster.pos, leap.last, 9_000) {
        return;
    }
    let elapsed = (now - leap.start).min(S1_LAND);
    let to = lerp(leap.from, leap.to, elapsed as u64, S1_LAND as u64);
    if to != caster.pos
        && (!sim.entity_set_pos(caster.id, to.0, to.1)
            || Unit::from_sim(sim, caster.id).is_none_or(|live| live.pos != to))
    {
        return;
    }
    if elapsed == S1_LAND {
        land_stomp(sim, caster, s, to, leap.enraged);
    } else {
        leap.last = to;
        s.leap = Some(leap);
    }
}

fn roar_heal(magic_power: usize) -> usize {
    220 + magic_power * 60 / 100
}

fn roar(sim: &mut StableSim<'_>, caster: Unit, rage: bool) {
    let around = allies(sim, caster, ROAR_RADIUS);
    let targets = if rage {
        around
    } else {
        weakest(&around, caster).into_iter().collect()
    };
    let shared = roar_heal(caster.stat.magic_power);
    let self_heal = shared + caster.max_hp.saturating_sub(caster.hp) * 8 / 100;
    fx(sim, caster.id, "roar_wave", caster.pos);
    for (target, heal) in
        std::iter::once((caster, self_heal)).chain(targets.into_iter().map(|a| (a, shared)))
    {
        if target.hp < target.max_hp {
            sim.heal(caster.id, target.id, heal);
            fx(sim, caster.id, "heal", target.pos);
        }
    }
}

fn fury_damage(attack: usize, final_hit: bool) -> usize {
    if final_hit {
        100 + attack * 55 / 100
    } else {
        65 + attack * 40 / 100
    }
}

fn slam(sim: &mut StableSim<'_>, caster: Unit, final_hit: bool) {
    fx(
        sim,
        caster.id,
        if final_hit { "finisher" } else { "slam" },
        caster.pos,
    );
    let amount = fury_damage(caster.stat.attack, final_hit);
    let kind = if final_hit {
        DamageTypeV1::Fixed
    } else {
        DamageTypeV1::Ad
    };
    for target in within_radius(sim, caster, caster.pos, FURY_RADIUS) {
        impact(sim, caster, target, amount, kind);
        if Unit::from_sim(sim, target.id).is_some() {
            if final_hit {
                airborne(sim, target, 42);
            } else {
                slow(sim, target.id, "mlbb_minotaur_fury_slow", 45, 39);
            }
        }
    }
}

fn end_fury(sim: &mut StableSim<'_>, id: usize, until: usize) {
    sim.entity_remove_buff(id, FURY);
    let remaining = until.saturating_sub(sim.tick());
    if remaining != 0 {
        timed(sim, id, ENRAGED, remaining);
    }
}

fn tick_fury(sim: &mut StableSim<'_>, caster: Unit, s: &mut State) {
    let Some(mut fury) = s.fury.take() else {
        return;
    };
    let age = sim.tick() - fury.start;
    // Immunity is granted by the host. If a host-specific disable gets through,
    // do not leave queued damage behind after it interrupted the windup.
    if blocked(sim, caster.id, false) {
        end_fury(sim, caster.id, s.enraged_until);
        return;
    }
    while SLAMS.get(fury.pulse).is_some_and(|&mark| age >= mark) {
        slam(sim, caster, fury.pulse == SLAMS.len() - 1);
        fury.pulse += 1;
        if Unit::from_sim(sim, caster.id).is_none() {
            return;
        }
    }
    if age >= FURY_DURATION {
        end_fury(sim, caster.id, s.enraged_until);
    } else {
        s.fury = Some(fury);
    }
}

fn attack(sim: &mut StableSim<'_>, owner: Unit, state: &mut State, input: InputTargetV1) {
    if input.kind != InputTargetKindV1::Target.code() {
        return;
    }
    let Some(target) = Unit::from_sim(sim, input.target_id)
        .filter(|u| u.team != owner.team && (u.targetable || u.structure))
    else {
        return;
    };
    let charged = !target.structure && state.basics_left > 0 && sim.tick() < state.basics_until;
    let damage = owner.stat.attack + if charged { bonus_hp(owner.max_hp) } else { 0 };
    voice(sim, owner.id, Cast::Basic);
    // The engine already range-checked this basic, including the tower's hit radius.
    sim.deal_damage(owner.id, target.id, damage, 0, AttackTypeV1::BaseAttack);
    if charged {
        state.basics_left -= 1;
        fx(sim, owner.id, "basic_bonus", target.pos);
    }
}

#[derive(Clone, Copy)]
struct Receipt {
    tick: usize,
    amount: usize,
    rate: usize,
}

#[derive(Default)]
struct Passive {
    owner: Option<EntityKey>,
    last_hp: usize,
    receipts: Vec<Receipt>,
    fraction: usize,
    last_motivation_cast: Option<usize>,
}

impl Passive {
    fn recover(&mut self, sim: &mut StableSim<'_>, caster: Unit) {
        let hp_loss = self.last_hp.saturating_sub(caster.hp);
        if hp_loss == 0 {
            // Some host paths may notify before applying HP loss in the same tick.
            self.receipts.retain(|receipt| receipt.tick == sim.tick());
            self.last_hp = caster.hp;
            if self.receipts.is_empty() && !has_named_buff(sim, caster.id, MOTIVATION) {
                self.fraction = 0;
            }
            return;
        }
        let receipts = std::mem::take(&mut self.receipts);
        if !has_named_buff(sim, caster.id, MOTIVATION) && receipts.is_empty() {
            self.fraction = 0;
        }
        // Never count the callback number on its own: the ABI does not promise
        // how much of it hit HP rather than a shield. Mixed eligible/ineligible
        // receipt intervals cannot be apportioned safely without a post-hit hook.
        let rate = receipts.iter().map(|r| r.rate).min().unwrap_or(0);
        if rate != 0 {
            let reported = receipts
                .iter()
                .fold(0usize, |sum, r| sum.saturating_add(r.amount));
            let total = self.fraction + hp_loss.min(reported) * rate;
            let heal = total / 100;
            self.fraction = total % 100;
            if heal != 0 {
                sim.heal(caster.id, caster.id, heal);
                fx(sim, caster.id, "heal", caster.pos);
            }
        }
        self.last_hp = Unit::from_sim(sim, caster.id).map_or(0, |u| u.hp);
    }
}

impl StablePassive for Passive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(Self::default())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize) {
        if let Some(old) = self.owner.take() {
            take(old);
        }
        let Some(caster) = Unit::from_sim(sim, id) else {
            return;
        };
        *self = Self {
            owner: Some(entity_key(sim, id)),
            last_hp: caster.hp,
            ..Self::default()
        };
        save(self.owner.unwrap(), State::default());
    }

    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _: usize,
        id: usize,
        attacker: usize,
        amount: usize,
    ) {
        let victim = sim.get_entity(id);
        if self.owner != Some(entity_key_ro(sim, id)) || amount == 0 || victim.is_none() {
            return;
        }
        let victim = victim.unwrap();
        let recent = has_named_buff(sim, id, MOTIVATION);
        let source = sim.get_entity(attacker);
        let enemy_hero = source
            .as_ref()
            .is_some_and(|e| e.is_champion() && e.team() != victim.team());
        let enraged = has_named_buff(sim, id, RAGE_WINDOW);
        let rate = if recent && enemy_hero {
            (5 + victim.level()).min(12) * if enraged { 2 } else { 1 }
        } else {
            0
        };
        self.receipts.push(Receipt {
            tick: sim.tick(),
            amount,
            rate,
        });
        if *TRACE_RECEIPTS
            && sim
                .sim_origin()
                .and_then(|o| SimOriginKindV1::from_code(o.kind))
                == Some(SimOriginKindV1::ClientMatchView)
        {
            crate::wlog(format!("[MINOTAUR] tick={} victim={id} attacker={attacker} kind={:?} notified={amount} hp={} prior_hp={} shield={} eligible_pct={rate}",
                sim.tick(), source.and_then(|e| e.kind()), victim.hp().0, self.last_hp, victim.shield()));
        }
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        let Some(caster) = Unit::from_sim(sim, id).filter(|_| !sim.is_end()) else {
            self.on_dead(sim, player);
            return;
        };
        let key = entity_key_ro(sim, id);
        if self.owner != Some(key) {
            self.on_spawn(sim, player, id);
        }
        let Some(key) = self.owner else { return };
        let Some(mut state) = take(key) else { return };
        let now = sim.tick();
        if state.updated != Some(now) {
            state.updated = Some(now);
            let release = state.motivation_cast;
            if release != self.last_motivation_cast {
                if self
                    .last_motivation_cast
                    .is_none_or(|then| release.is_some_and(|r| r >= then + MOTIVATION_DURATION))
                {
                    self.fraction = 0;
                }
                self.last_motivation_cast = release;
            }
            self.recover(sim, caster);
            tick_leap(sim, caster, &mut state);
            tick_fury(sim, caster, &mut state);
        }
        if Unit::from_sim(sim, id).is_some() && entity_key_ro(sim, id) == key {
            save(key, state);
        }
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        if let Some(key) = self.owner.take() {
            take(key);
            if key == entity_key_ro(sim, key.entity_id) {
                for name in [MOTIVATION, FURY, ENRAGED, RAGE_WINDOW] {
                    sim.entity_remove_buff(key.entity_id, name);
                }
            }
        }
        *self = Self::default();
    }
}

static TRACE_RECEIPTS: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("MLBB_MINOTAUR_RECEIPTS").is_some());

#[derive(Clone, Copy, Eq, PartialEq)]
enum Cast {
    Basic,
    Stomp,
    Roar,
    Fury,
}

impl Cast {
    fn name(self) -> &'static str {
        ["attack", "skill", "skill2", "ult"][self as usize]
    }
    fn range(self) -> u64 {
        [24_000, S1_REACH, ROAR_RADIUS, FURY_RADIUS][self as usize]
    }
    fn target(self) -> CastingTargetV1 {
        [
            CastingTargetV1::Enemy,
            CastingTargetV1::EnemyChampion,
            CastingTargetV1::AllyOnlySelf,
            CastingTargetV1::EnemyChampion,
        ][self as usize]
    }
    fn slot(self) -> SkillSlotV1 {
        [
            SkillSlotV1::Attack,
            SkillSlotV1::Skill,
            SkillSlotV1::Skill2,
            SkillSlotV1::Ult,
        ][self as usize]
    }
}

impl StableEffectType for Cast {
    fn apply(&self, sim: &mut StableSim<'_>, _: u64, id: usize, input: InputTargetV1) {
        let Some(caster) = Unit::from_sim(sim, id) else {
            return;
        };
        let key = entity_key_ro(sim, id);
        let Some(mut state) = take(key) else { return };
        let now = sim.tick();
        let cast_index = *self as usize;
        if state.leap.is_none()
            && state.fury.is_none()
            && state.casts[cast_index] != Some(now)
            && (*self == Self::Basic || !blocked(sim, id, *self == Self::Stomp))
        {
            let rage = now < state.enraged_until;
            state.casts[cast_index] = Some(now);
            match self {
                Self::Basic => attack(sim, caster, &mut state, input),
                Self::Stomp => {
                    let aim = match InputTargetKindV1::from_code(input.kind) {
                        Some(InputTargetKindV1::Pos) => Some((input.x, input.y)),
                        Some(InputTargetKindV1::Target) => Unit::from_sim(sim, input.target_id)
                            .filter(|u| u.champion && u.team != caster.team && u.targetable)
                            .map(|u| u.pos),
                        _ => None,
                    };
                    if let Some(aim) = aim {
                        let grid = state.grid.get_or_insert_with(|| current_grid(&units(sim)));
                        let to = landing(grid, caster.pos, aim);
                        state.leap = Some(Leap {
                            start: now,
                            from: caster.pos,
                            to,
                            last: caster.pos,
                            enraged: rage,
                        });
                        state.motivation_cast = Some(now);
                        motivate(sim, id, caster.pos);
                        voice(sim, id, Self::Stomp);
                    }
                }
                Self::Roar => {
                    state.motivation_cast = Some(now);
                    motivate(sim, id, caster.pos);
                    roar(sim, caster, rage);
                    voice(sim, id, Self::Roar);
                }
                Self::Fury => {
                    state.enraged_until = now + ENRAGED_DURATION;
                    state.fury = Some(Fury {
                        start: now,
                        pulse: 0,
                    });
                    sim.entity_remove_buff(id, ENRAGED);
                    let mut immune = BuffV1::timed(FURY, FURY_DURATION);
                    immune.cc_immune = true;
                    sim.add_buff(id, &immune);
                    timed(sim, id, RAGE_WINDOW, ENRAGED_DURATION);
                    state.motivation_cast = Some(now);
                    motivate(sim, id, caster.pos);
                    voice(sim, id, Self::Fury);
                }
            }
        }
        if Unit::from_sim(sim, id).is_some() && entity_key_ro(sim, id) == key {
            save(key, state);
        }
    }

    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            match self {
                Self::Basic => stat.attack,
                Self::Stomp => 55 + stat.attack * 45 / 100,
                Self::Roar => 0,
                Self::Fury => fury_damage(stat.attack, false) * 2 + fury_damage(stat.attack, true),
            },
            0,
        )
    }

    fn expected_heal(&self, stat: &StatV1) -> usize {
        if *self == Self::Roar {
            roar_heal(stat.magic_power)
        } else {
            0
        }
    }

    fn expected_cc_time(&self) -> Option<usize> {
        match self {
            Self::Stomp => Some(24),
            Self::Fury => Some(42),
            _ => None,
        }
    }
}

fn visible_enemies(sim: &StableSim<'_>, owner: Unit, radius: u64) -> Vec<Unit> {
    champions(sim)
        .filter(|e| {
            e.team != owner.team
                && e.targetable
                && near(owner.pos, e.pos, radius)
                && sim.is_visible(owner.team, e.id)
        })
        .collect()
}

fn wants_roar(sim: &StableSim<'_>, caster: Unit, enraged: bool) -> bool {
    let close = allies(sim, caster, ROAR_RADIUS);
    caster.at_or_below(70)
        || close.iter().any(|u| u.at_or_below(68))
        || enraged && close.iter().filter(|u| u.at_or_below(80)).count() >= 2
}

fn wants_fury(sim: &StableSim<'_>, caster: Unit) -> bool {
    let enemies = visible_enemies(sim, caster, FURY_RADIUS);
    if enemies.len() >= 2 {
        return true;
    }
    enemies.first().is_some_and(|e| {
        allies(sim, caster, ROAR_RADIUS)
            .iter()
            .any(|a| a.at_or_below(50) && near(a.pos, e.pos, 24_000))
    })
}

fn stomp_target(
    sim: &StableSim<'_>,
    caster: Unit,
    grid: Option<&Grid>,
    enraged: bool,
) -> Option<Unit> {
    let party = allies(sim, caster, 85_000);
    let radius = if enraged {
        S1_ENRAGED_RADIUS
    } else {
        S1_RADIUS
    };
    visible_enemies(sim, caster, S1_REACH)
        .into_iter()
        .filter(|e| grid.is_none_or(|g| near(landing(g, caster.pos, e.pos), e.pos, radius)))
        .filter(|e| {
            near(caster.pos, e.pos, 27_000) || party.iter().any(|a| near(a.pos, e.pos, 55_000))
        })
        .min_by_key(|e| {
            let peel = party
                .iter()
                .any(|a| a.at_or_below(68) && near(a.pos, e.pos, 24_000));
            (!peel, distance_sq(caster.pos, e.pos), e.id)
        })
}

impl StableAction for Cast {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(*self)
    }
    fn action_name(&self) -> String {
        self.name().into()
    }
    fn duration(&self) -> usize {
        [24, 40, 28, FURY_DURATION][*self as usize]
    }
    fn cooltime(&self, _: &StatV1, _: usize) -> usize {
        [72, 450, 540, 3480][*self as usize]
    }
    fn cancelable(&self) -> bool {
        *self == Self::Basic
    }
    fn casting_target(&self) -> CastingTargetV1 {
        self.target()
    }
    fn description(&self) -> String {
        format!(
            "#asset/base/text/champion?description.{HERO}.{}",
            self.name()
        )
    }
    fn effect(&self) -> Option<StableEffectSpec> {
        Some(StableEffectSpec {
            range: self.range(),
            growth_range: 0,
            start_timing: [15, 0, 12, 0][*self as usize],
            casting: [
                CastingTypeV1::Targeting,
                CastingTypeV1::Position,
                CastingTypeV1::None,
                CastingTypeV1::None,
            ][*self as usize],
            target: self.target(),
            attack_type: if *self == Self::Basic {
                AttackTypeV1::BaseAttack
            } else {
                AttackTypeV1::Skill
            },
            effect: Box::new(*self),
        })
    }
    fn can_activate(&self, sim: &StableSim<'_>, id: usize) -> bool {
        let Some(caster) = Unit::from_sim(sim, id) else {
            return false;
        };
        let states = STATES.lock().unwrap_or_else(|e| e.into_inner());
        let Some(s) = states.get(&entity_key_ro(sim, id)) else {
            return *self == Self::Basic;
        };
        if s.leap.is_some() || s.fury.is_some() {
            return false;
        }
        let rage = sim.tick() < s.enraged_until;
        match self {
            Self::Basic => true,
            Self::Stomp => {
                !blocked(sim, id, true)
                    && stomp_target(sim, caster, s.grid.as_ref(), rage).is_some()
            }
            Self::Roar => !blocked(sim, id, false) && wants_roar(sim, caster, rage),
            Self::Fury => !blocked(sim, id, false) && wants_fury(sim, caster),
        }
    }
    fn target_constraint(&self, sim: &StableSim<'_>, id: usize, target: usize) -> bool {
        if *self != Self::Stomp {
            return true;
        }
        let Some((owner, enemy)) = Unit::from_sim(sim, id).zip(Unit::from_sim(sim, target)) else {
            return false;
        };
        let lock = STATES.lock().unwrap_or_else(|e| e.into_inner());
        let state = lock.get(&entity_key_ro(sim, id));
        enemy.champion
            && enemy.team != owner.team
            && enemy.targetable
            && sim.is_visible(owner.team, target)
            && state.and_then(|s| s.grid.as_ref()).is_none_or(|g| {
                let radius = if state.is_some_and(|s| sim.tick() < s.enraged_until) {
                    S1_ENRAGED_RADIUS
                } else {
                    S1_RADIUS
                };
                near(landing(g, owner.pos, enemy.pos), enemy.pos, radius)
            })
    }
}

struct Ai;
impl StablePlayerAi for Ai {
    fn clone_box(&self) -> Box<dyn StablePlayerAi> {
        Box::new(Self)
    }
    fn id(&self) -> String {
        format!("{HERO}_ai")
    }
    fn priority(&self) -> i32 {
        175
    }
    fn matches(&self, init: &StableAiInit) -> bool {
        init.champion_name == HERO
    }
    fn think(&mut self, ctx: &mut StableAiContext<'_>, base: Option<InputV1>) -> Option<InputV1> {
        if base.is_some_and(|i| i.kind == InputKindV1::Return.code()) {
            return None;
        }
        let player = ctx.player_id();
        let inputs = {
            let sim = ctx.sim()?;
            let entity = sim.get_player(player)?.champion()?;
            let caster = Unit::from_sim(&sim, entity.id())?;
            if blocked(&sim, caster.id, false) {
                return None;
            }
            let lock = STATES.lock().unwrap_or_else(|e| e.into_inner());
            let state = lock.get(&entity_key_ro(&sim, caster.id))?;
            if state.leap.is_some() || state.fury.is_some() {
                return None;
            }
            let enraged = sim.tick() < state.enraged_until;
            let ready = |cast: Cast| {
                entity
                    .cooldown(cast.slot())
                    .is_some_and(|c| c.unlocked && c.remaining == 0 && c.charges > 0)
            };
            let mut inputs = Vec::new();
            if ready(Cast::Fury) && wants_fury(&sim, caster) {
                inputs.push(InputV1::action(InputKindV1::Ult, InputTargetV1::default()));
            }
            if ready(Cast::Roar) && wants_roar(&sim, caster, enraged) {
                inputs.push(InputV1::action(
                    InputKindV1::Skill2,
                    InputTargetV1::default(),
                ));
            }
            if ready(Cast::Stomp) && !blocked(&sim, caster.id, true) {
                if let Some(enemy) = stomp_target(&sim, caster, state.grid.as_ref(), enraged) {
                    inputs.push(InputV1::action(
                        InputKindV1::Skill,
                        InputTargetV1::pos(enemy.pos.0, enemy.pos.1),
                    ));
                }
            }
            inputs
        };
        inputs.into_iter().find(|i| ctx.is_valid_input(i))
    }
}

struct Minotaur;
impl StableChampion for Minotaur {
    fn id(&self) -> String {
        HERO.into()
    }
    fn name(&self) -> String {
        format!("#asset/base/text/champion?description.{HERO}.name")
    }
    fn skill_icon(&self, i: usize) -> (String, String) {
        let slot = ["attack", "skill", "skill2", "ult"]
            .get(i)
            .unwrap_or(&"passive");
        (
            format!("asset/mlbb_heroes/icons/{HERO}_{slot}"),
            String::new(),
        )
    }
    fn category(&self) -> ChampionCategoryV1 {
        ChampionCategoryV1::Melee
    }
    fn tags(&self) -> Vec<ChampionTagV1> {
        vec![
            ChampionTagV1::Ad,
            ChampionTagV1::Ap,
            ChampionTagV1::Melee,
            ChampionTagV1::Cc,
            ChampionTagV1::Heal,
            ChampionTagV1::Tank,
        ]
    }
    fn stat(&self) -> StatV1 {
        STATS
    }
    fn growth(&self) -> StatV1 {
        GROWTH
    }
    fn attack(&self) -> Box<dyn StableAction> {
        Box::new(Cast::Basic)
    }
    fn skill(&self) -> Box<dyn StableAction> {
        Box::new(Cast::Stomp)
    }
    fn skill2(&self) -> Box<dyn StableAction> {
        Box::new(Cast::Roar)
    }
    fn ult(&self) -> Option<Box<dyn StableAction>> {
        Some(Box::new(Cast::Fury))
    }
    fn passive(&self) -> Option<Box<dyn StablePassive>> {
        Some(Box::new(Passive::default()))
    }
    fn lane_prior(&self) -> Option<Vec<(LaneV1, usize)>> {
        Some(vec![
            (LaneV1::Support, 100),
            (LaneV1::Top, 25),
            (LaneV1::Jungle, 5),
            (LaneV1::Mid, 5),
            (LaneV1::Bottom, 5),
        ])
    }
}

pub(crate) fn register(mod_: &mut StableMod) {
    mod_.add_champion(Minotaur);
    for cast in [Cast::Basic, Cast::Stomp, Cast::Roar, Cast::Fury] {
        mod_.add_native_effect(&format!("mlbb_heroes:minotaur_{}", cast.name()), cast);
    }
    mod_.add_native_passive("mlbb_heroes:minotaur_passive", Passive::default());
    mod_.add_player_input_ai(Ai);
}

#[cfg(test)]
mod tests;
