//! X.Borg's native casts and owner-supplied animation layers.
use crate::{audio, entity_key, entity_key_ro, EntityKey, Grid};
use mod_api_stable::*;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};

mod combat;
mod visuals;
mod voice;

type Pos = (u64, u64);
type Direction = (f64, f64);
const HERO: &str = "mlbb_xborg";
const ARMOR: &str = "mlbb_xborg_firaga_guard";
const SPIN_SPEED: &str = "mlbb_xborg_spin_speed";
const STATS: StatV1 = StatV1 {
    attack: 96,
    magic_power: 15,
    hp: 880,
    defence: 33,
    magic_resistance: 25,
    move_speed: 1040,
    hp_regen: 7,
    stack: 0,
    crit_chance: 0,
};
const GROWTH: StatV1 = StatV1 {
    attack: 6,
    magic_power: 5,
    hp: 100,
    defence: 5,
    magic_resistance: 3,
    move_speed: 4,
    hp_regen: 0,
    stack: 0,
    crit_chance: 0,
};

/// Prototype guard, isolated from the game's pooled shield layers.
/// The stable ABI cannot read or spend a shield layer by owner. The candidate
/// gives the armored form 20% reduction and depletes its separate reserve from
/// engine damage receipts. This is deliberately not an absorb-layer claim.
#[derive(Debug)]
struct Firaga {
    capacity: usize,
    reserve: usize,
    energy: usize,
    next_energy: usize,
}

impl Firaga {
    fn new(level: usize) -> Self {
        let capacity = Self::capacity(level);
        Self {
            capacity,
            reserve: capacity,
            energy: 0,
            next_energy: 0,
        }
    }

    fn capacity(level: usize) -> usize {
        (STATS.hp + GROWTH.hp * level.saturating_sub(1)) * 30 / 100
    }

    fn armored(&self) -> bool {
        self.reserve > 0
    }

    fn level(&mut self, level: usize) {
        let capacity = Self::capacity(level);
        if self.armored() {
            self.reserve = (self.reserve + capacity.saturating_sub(self.capacity)).min(capacity);
        }
        self.capacity = capacity;
    }

    fn spend(&mut self, now: usize) {
        self.reserve = 0;
        self.energy = 0;
        self.next_energy = now + 6;
    }

    fn hit(&mut self, damage: usize, now: usize) -> bool {
        if !self.armored() {
            return false;
        }
        self.reserve = self.reserve.saturating_sub(damage);
        if self.reserve != 0 {
            return false;
        }
        self.spend(now);
        true
    }

    fn recover(&mut self, amount: usize) {
        if self.armored() {
            self.reserve = (self.reserve + amount).min(self.capacity);
        } else {
            self.energy = (self.energy + amount).min(100);
        }
    }

    fn pickup(&mut self) {
        self.recover(if self.armored() {
            self.capacity * 12 / 100
        } else {
            12
        });
    }

    fn recharge(&mut self, now: usize) -> bool {
        if self.armored() {
            return false;
        }
        if now >= self.next_energy {
            let steps = (now - self.next_energy) / 6 + 1;
            self.energy = (self.energy + steps).min(100);
            self.next_energy += steps * 6;
        }
        if self.energy < 100 {
            return false;
        }
        self.reserve = self.capacity * 30 / 100;
        self.energy = 0;
        true
    }
}

#[derive(Clone, Copy)]
struct Receipt {
    amount: usize,
    attacker: usize,
    tick: usize,
}

struct State {
    armor: Firaga,
    visual: visuals::Visual,
    combat: combat::Combat,
    grid: Option<Grid>,
}

static STATES: LazyLock<Mutex<HashMap<EntityKey, State>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static RECEIPTS: LazyLock<Mutex<HashMap<EntityKey, Vec<Receipt>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn with_state(sim: &mut StableSim<'_>, id: usize, f: impl FnOnce(&mut StableSim<'_>, &mut State)) {
    let key = entity_key_ro(sim, id);
    let Some(mut state) = STATES.lock().unwrap().remove(&key) else {
        return;
    };
    f(sim, &mut state);
    if alive(sim, id) && entity_key_ro(sim, id) == key {
        STATES.lock().unwrap().insert(key, state);
    } else {
        RECEIPTS.lock().unwrap().remove(&key);
    }
}

fn forget(key: EntityKey) -> Option<State> {
    RECEIPTS.lock().unwrap().remove(&key);
    STATES.lock().unwrap().remove(&key)
}

fn position(sim: &StableSim<'_>, id: usize) -> Option<Pos> {
    sim.get_entity(id).map(|e| e.pos())
}
fn alive(sim: &StableSim<'_>, id: usize) -> bool {
    sim.get_entity(id)
        .is_some_and(|e| e.is_alive() && e.hp().0 > 0)
}
#[cfg(test)]
fn has_buff(sim: &StableSim<'_>, id: usize, name: &str) -> bool {
    sim.get_entity(id).is_some_and(|e| {
        (0..e.buff_count())
            .filter_map(|i| e.buff_at(i))
            .any(|b| b.name() == name)
    })
}
fn interrupted(sim: &StableSim<'_>, id: usize, movement: bool) -> bool {
    sim.get_entity(id).is_some_and(|e| {
        (0..e.cc_count()).filter_map(|i| e.cc_at(i)).any(|c| {
            let kind = CcKindV1::from_code(c.kind);
            matches!(
                kind,
                Some(CcKindV1::Stun | CcKindV1::Airborne | CcKindV1::BlockSkill)
            ) || movement
                && matches!(
                    kind,
                    Some(CcKindV1::Bind | CcKindV1::BlockMoveSkill | CcKindV1::ForceMove)
                )
        })
    })
}
fn delta(a: Pos, b: Pos) -> Direction {
    (b.0 as f64 - a.0 as f64, b.1 as f64 - a.1 as f64)
}
fn dist(a: Pos, b: Pos) -> f64 {
    let (x, y) = delta(a, b);
    x.hypot(y)
}
fn unit((x, y): Direction) -> Option<Direction> {
    let length = x.hypot(y);
    (length > 0.).then(|| (x / length, y / length))
}
fn advance((x, y): Pos, (dx, dy): Direction, length: f64) -> Pos {
    (
        (x as f64 + dx * length).max(0.).round() as u64,
        (y as f64 + dy * length).max(0.).round() as u64,
    )
}
fn aiming(sim: &StableSim<'_>, from: Pos, input: InputTargetV1) -> Option<Direction> {
    unit(match InputTargetKindV1::from_code(input.kind)? {
        InputTargetKindV1::Dir => (input.dir_x as f64, input.dir_y as f64),
        InputTargetKindV1::Target => delta(from, position(sim, input.target_id)?),
        InputTargetKindV1::Pos => delta(from, (input.x, input.y)),
        InputTargetKindV1::None => return None,
    })
}
fn clear_ground(
    sim: &StableSim<'_>,
    s: &mut State,
    from: Pos,
    dir: Direction,
    distance: u64,
) -> Pos {
    let grid = s.grid.get_or_insert_with(|| {
        let marks: Vec<_> = (0..sim.champion_count())
            .filter_map(|i| {
                let e = sim.get_entity(sim.champion_id_at(i))?;
                let (x, y) = e.pos();
                e.is_alive().then_some((x as f64, y as f64))
            })
            .collect();
        crate::choose_current_grid(&marks)
            .map(|(_, grid)| grid)
            .unwrap_or_else(|| Grid::builtin(GameModeKindV1::Moba))
    });
    let mut result = from;
    for segment in 1..=distance.div_ceil(1000) {
        let next = advance(from, dir, (segment * 1000).min(distance) as f64);
        if grid.solid(next.0 as f64, next.1 as f64) {
            break;
        }
        result = next;
    }
    result
}
fn guard(sim: &mut StableSim<'_>, id: usize) {
    let mut buff = BuffV1::named(ARMOR);
    buff.damaged_reduce = 20;
    sim.add_buff(id, &buff);
}
fn recoil(sim: &mut StableSim<'_>, id: usize, s: &mut State, dir: Direction) {
    if interrupted(sim, id, true) {
        return;
    }
    if let Some(from) = position(sim, id) {
        let to = clear_ground(sim, s, from, dir, 12_000);
        if to != from {
            sim.entity_set_pos(id, to.0, to.1);
        }
    }
}
fn trace(sim: &StableSim<'_>, id: usize, event: &str) {
    if sim
        .sim_origin()
        .and_then(|o| SimOriginKindV1::from_code(o.kind))
        == Some(SimOriginKindV1::ClientMatchView)
    {
        crate::wlog(format!("[XBORG] tick={} actor={id} {event}", sim.tick()));
    }
}

#[derive(Default)]
struct Passive {
    owner: Option<EntityKey>,
    voice: audio::VoiceLifecycle,
}

impl StablePassive for Passive {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(Self::default())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, player: usize, id: usize) {
        if let Some(old) = self.owner.take() {
            forget(old);
        }
        if !alive(sim, id) {
            return;
        }
        let key = entity_key(sim, id);
        let entity = sim.get_entity(id).unwrap();
        let (level, pos) = (entity.level(), entity.pos());
        let mut state = State {
            armor: Firaga::new(level),
            visual: visuals::Visual::new(pos),
            combat: combat::Combat::default(),
            grid: None,
        };
        sim.entity_remove_buff(id, ARMOR);
        sim.entity_remove_buff(id, SPIN_SPEED);
        guard(sim, id);
        visuals::draw(sim, id, &mut state);
        STATES.lock().unwrap().insert(key, state);
        self.owner = Some(key);
        if audio::voice_in_match_view(sim) {
            let deaths = sim.get_player(player).map_or(0, |player| player.deaths());
            let line = if deaths == 0 {
                "mlbb_xborg_voice_select"
            } else {
                voice::RESPAWN[(deaths - 1) % voice::RESPAWN.len()]
            };
            self.voice.on_spawn(sim, id, Some(line));
        }
        trace(sim, id, "spawn armored");
    }

    fn on_damaged(
        &mut self,
        sim: &mut StableSim<'_>,
        _: usize,
        id: usize,
        attacker: usize,
        damage: usize,
    ) {
        let Some(key) = self.owner else { return };
        if key == entity_key_ro(sim, id) && damage > 0 {
            RECEIPTS
                .lock()
                .unwrap()
                .entry(key)
                .or_default()
                .push(Receipt {
                    amount: damage,
                    attacker,
                    tick: sim.tick(),
                });
        }
    }

    fn on_update(&mut self, sim: &mut StableSim<'_>, _: u64, player: usize, id: usize) {
        if !alive(sim, id) {
            self.on_dead(sim, player);
            return;
        }
        if self.owner != Some(entity_key_ro(sim, id)) {
            self.on_spawn(sim, player, id);
        }
        let Some(key) = self.owner else { return };
        let receipts = RECEIPTS.lock().unwrap().remove(&key).unwrap_or_default();
        with_state(sim, id, |sim, s| {
            let now = sim.tick();
            s.armor.level(sim.get_entity(id).unwrap().level());
            for receipt in receipts {
                if s.armor.hit(receipt.amount, receipt.tick) {
                    sim.entity_remove_buff(id, ARMOR);
                    combat::broken(sim, id, s, receipt.attacker);
                    break;
                }
                if !s.combat.is_spinning() && !s.combat.is_firing() {
                    let name =
                        visuals::form_name(s.armor.armored(), "armored_hurt", "armorless_hurt");
                    if !s.visual.action_active(now) {
                        s.visual.play(name, now);
                    }
                }
            }
            combat::update(sim, id, s);
            if alive(sim, id) && s.armor.recharge(now) {
                guard(sim, id);
                if !s.combat.is_firing() {
                    s.visual.play("armor_restore", now);
                }
                voice::play(sim, id, "mlbb_xborg_voice_passive", false);
                trace(sim, id, "rearm");
            }
            visuals::draw(sim, id, s);
        });
        if audio::voice_in_match_view(sim) && alive(sim, id) {
            self.voice.on_update(sim, id, &[], voice::MOVE);
        }
    }

    fn on_kill(&mut self, sim: &mut StableSim<'_>, _: usize, id: usize, victim: usize) {
        let Some(target) = sim.get_entity(victim) else {
            return;
        };
        let lines = if target.is_champion() {
            voice::KILL
        } else if matches!(
            target.kind(),
            Some(EntityKindV1::Epic | EntityKindV1::Serpen)
        ) {
            voice::OBJECTIVE
        } else {
            return;
        };
        let line = lines[sim.tick() % lines.len()];
        voice::play(sim, id, line, false);
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, _: usize) {
        let Some(key) = self.owner.take() else { return };
        let state = forget(key);
        let id = key.entity_id;
        if entity_key_ro(sim, id) == key {
            sim.entity_remove_buff(id, ARMOR);
            sim.entity_remove_buff(id, SPIN_SPEED);
            if let Some(pos) = position(sim, id) {
                visuals::death(sim, id, pos, state.is_some_and(|s| s.visual.left));
            }
            let line = voice::DEATH[sim.tick() % voice::DEATH.len()];
            voice::play(sim, id, line, true);
            trace(sim, id, "death cleanup");
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cast {
    Basic,
    Fire,
    Stakes,
    Spin,
    BasicHit,
}

impl Cast {
    fn slot(self) -> &'static str {
        match self {
            Self::Basic | Self::BasicHit => "attack",
            Self::Fire => "skill",
            Self::Stakes => "skill2",
            Self::Spin => "ult",
        }
    }
    fn reach(self) -> u64 {
        match self {
            Self::Basic | Self::BasicHit => 24_000,
            Self::Fire => 50_000,
            Self::Stakes => 55_000,
            Self::Spin => 62_000,
        }
    }
    fn target(self) -> CastingTargetV1 {
        match self {
            Self::Basic | Self::BasicHit => CastingTargetV1::Enemy,
            Self::Spin => CastingTargetV1::EnemyChampion,
            _ => CastingTargetV1::EnemyWithoutTower,
        }
    }
}

impl StableEffectType for Cast {
    fn apply(&self, sim: &mut StableSim<'_>, seed: u64, id: usize, input: InputTargetV1) {
        if alive(sim, id) {
            with_state(sim, id, |sim, s| {
                combat::cast(*self, sim, id, input, s, seed)
            });
        }
    }
    fn expected_damage(&self, stat: &StatV1) -> (usize, usize) {
        (
            match self {
                Self::Basic | Self::BasicHit => stat.attack,
                Self::Fire => combat::fire_damage(*stat) * 4,
                Self::Stakes => combat::stake_damage(*stat),
                Self::Spin => combat::spin_damage(*stat) * 4 + 140,
            },
            0,
        )
    }
    fn may_adjust_range(&self) -> bool {
        matches!(self, Self::Fire | Self::Stakes)
    }
    fn range_adjust(&self, caster: &StableEntityView<'_>, _: &StableEntityView<'_>) -> u64 {
        if self.may_adjust_range() && !caster.has_buff(ARMOR) {
            15_000
        } else {
            0
        }
    }
}

impl StableAction for Cast {
    fn clone_box(&self) -> Box<dyn StableAction> {
        Box::new(*self)
    }
    fn action_name(&self) -> String {
        self.slot().into()
    }
    fn duration(&self) -> usize {
        if *self == Self::Basic {
            18
        } else {
            1
        }
    }
    fn cancelable(&self) -> bool {
        true
    }
    fn cooltime(&self, _: &StatV1, _: usize) -> usize {
        match self {
            Self::Basic | Self::BasicHit => 66,
            Self::Fire => 300,
            Self::Stakes => 600,
            Self::Spin => 2640,
        }
    }
    fn casting_target(&self) -> CastingTargetV1 {
        self.target()
    }
    fn description(&self) -> String {
        format!(
            "#asset/base/text/champion?description.{HERO}.{}",
            self.slot()
        )
    }
    fn can_use_with_move(&self) -> bool {
        *self != Self::Basic
    }
    fn effect(&self) -> Option<StableEffectSpec> {
        let basic = *self == Self::Basic;
        Some(StableEffectSpec {
            range: self.reach(),
            growth_range: 0,
            start_timing: if basic { 5 } else { 0 },
            casting: if basic {
                CastingTypeV1::Targeting
            } else {
                CastingTypeV1::Direction
            },
            target: self.target(),
            attack_type: if basic {
                AttackTypeV1::BaseAttack
            } else {
                AttackTypeV1::Skill
            },
            effect: Box::new(*self),
        })
    }
    fn can_activate(&self, sim: &StableSim<'_>, id: usize) -> bool {
        STATES
            .lock()
            .unwrap()
            .get(&entity_key_ro(sim, id))
            .is_some_and(|s| combat::allows(*self, s))
    }
    fn target_constraint(&self, sim: &StableSim<'_>, id: usize, target: usize) -> bool {
        if *self != Self::Spin {
            return true;
        }
        position(sim, id)
            .zip(position(sim, target))
            .is_some_and(|(a, b)| dist(a, b) <= self.reach() as f64)
    }
}

struct Xborg;
impl StableChampion for Xborg {
    fn id(&self) -> String {
        HERO.into()
    }
    fn name(&self) -> String {
        "#asset/base/text/champion?description.mlbb_xborg.name".into()
    }
    fn skill_icon(&self, i: usize) -> (String, String) {
        let slot = ["attack", "skill", "skill2", "ult"]
            .get(i)
            .unwrap_or(&"passive");
        (
            format!("asset/mlbb_heroes/icons/mlbb_xborg_{slot}"),
            String::new(),
        )
    }
    fn category(&self) -> ChampionCategoryV1 {
        ChampionCategoryV1::Melee
    }
    fn tags(&self) -> Vec<ChampionTagV1> {
        vec![
            ChampionTagV1::Ad,
            ChampionTagV1::Melee,
            ChampionTagV1::Cc,
            ChampionTagV1::Dot,
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
        Box::new(Cast::Fire)
    }
    fn skill2(&self) -> Box<dyn StableAction> {
        Box::new(Cast::Stakes)
    }
    fn ult(&self) -> Option<Box<dyn StableAction>> {
        Some(Box::new(Cast::Spin))
    }
    fn passive(&self) -> Option<Box<dyn StablePassive>> {
        Some(Box::new(Passive::default()))
    }
    fn lane_prior(&self) -> Option<Vec<(LaneV1, usize)>> {
        Some(vec![
            (LaneV1::Top, 100),
            (LaneV1::Jungle, 40),
            (LaneV1::Mid, 20),
            (LaneV1::Bottom, 5),
            (LaneV1::Support, 0),
        ])
    }
}

pub(crate) fn register(module: &mut StableMod) {
    module.add_champion(Xborg);
    for cast in [Cast::Basic, Cast::Fire, Cast::Stakes, Cast::Spin] {
        module.add_native_effect(&format!("mlbb_heroes:xborg_{}", cast.slot()), cast);
    }
    module.add_native_effect("mlbb_heroes:xborg_basic_hit", Cast::BasicHit);
    module.add_native_passive("mlbb_heroes:xborg_passive", Passive::default());
}

#[cfg(test)]
mod tests;
