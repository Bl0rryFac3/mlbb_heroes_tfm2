//! Estes native gameplay, including a live, replay-safe supplied-art tether.
use mod_api_stable::*;

const SCRIPTURE_MAX: usize = 100;
const SCRIPTURE_TICKS_PER_CHARGE: usize = 30;
const LINK_SELF: &str = "mlbb_heroes_estes_link_self";
const LINK_TARGET: &str = "mlbb_heroes_estes_link_target";
const ULT_AURA: &str = "mlbb_heroes_estes_ult_aura";
const PASSIVE_SLOW: &str = "mlbb_heroes_estes_scripture_slow";
const LINK_RANGE: f64 = 120_000.0;
const ULT_LINK_RANGE: f64 = 105_000.0;
const SCRIPTURE_RICOCHET_RANGE: f64 = 45_000.0;
const SCRIPTURE_RICOCHET_MAX: usize = 2;
const TETHER_SEGMENT_SPACING: f64 = 12_000.0;
const TETHER_REFRESH_TICKS: usize = 2;
const TETHER_WIRE_EFFECTS: [&str; 12] = [
    "mlbb_heroes_estes_link_wire_0",
    "mlbb_heroes_estes_link_wire_1",
    "mlbb_heroes_estes_link_wire_2",
    "mlbb_heroes_estes_link_wire_3",
    "mlbb_heroes_estes_link_wire_4",
    "mlbb_heroes_estes_link_wire_5",
    "mlbb_heroes_estes_link_wire_6",
    "mlbb_heroes_estes_link_wire_7",
    "mlbb_heroes_estes_link_wire_8",
    "mlbb_heroes_estes_link_wire_9",
    "mlbb_heroes_estes_link_wire_10",
    "mlbb_heroes_estes_link_wire_11",
];

fn can_render_tether(sim: &StableSim<'_>) -> bool {
    sim.sim_origin()
        .and_then(|origin| SimOriginKindV1::from_code(origin.kind))
        .is_some_and(|kind| {
            matches!(
                kind,
                SimOriginKindV1::ClientMatchView
                    | SimOriginKindV1::ClientSpectate
                    | SimOriginKindV1::ClientReplay
            )
        })
}

fn draw_tether_sprites(
    sim: &mut StableSim<'_>,
    caster_id: usize,
    from: (u64, u64),
    to: (u64, u64),
) {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let length = dx.hypot(dy);
    if !length.is_finite() || length < 1.0 {
        return;
    }

    // Match Fanny's proven angle-bucketed view-effect technique, but use
    // Estes's seamless bright strand, spaced closely enough to overlap.
    let theta = dy.atan2(dx).rem_euclid(std::f64::consts::PI);
    let bucket = (((theta / std::f64::consts::PI) * 12.0).round() as usize) % 12;
    let count = (length / TETHER_SEGMENT_SPACING).ceil().clamp(1.0, 24.0) as usize;
    for i in 0..count {
        let t = ((i as f64) + 0.5) / count as f64;
        let x = (from.0 as f64 + dx * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let y = (from.1 as f64 + dy * t).round().clamp(0.0, u64::MAX as f64) as u64;
        let target = InputTargetV1::pos(x, y);
        let _ = sim.play_view_effect(TETHER_WIRE_EFFECTS[bucket], caster_id, &target, 0, 0, 0);
    }
}

fn has_named_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    sim.get_entity(entity_id).is_some_and(|entity| {
        (0..entity.buff_count())
            .filter_map(|i| entity.buff_at(i))
            .any(|buff| buff.name() == name)
    })
}

fn distance_sq(a: (u64, u64), b: (u64, u64)) -> f64 {
    let dx = a.0 as f64 - b.0 as f64;
    let dy = a.1 as f64 - b.1 as f64;
    dx * dx + dy * dy
}

fn distance(a: (u64, u64), b: (u64, u64)) -> f64 {
    distance_sq(a, b).sqrt()
}

fn inset_link_endpoints(
    from: (u64, u64),
    to: (u64, u64),
    inset: f64,
) -> Option<((u64, u64), (u64, u64))> {
    let dx = to.0 as f64 - from.0 as f64;
    let dy = to.1 as f64 - from.1 as f64;
    let length = dx.hypot(dy);
    if length <= inset * 2.0 + 1.0 {
        return None;
    }
    let ux = dx / length;
    let uy = dy / length;
    let start = (
        (from.0 as f64 + ux * inset).round().max(0.0) as u64,
        (from.1 as f64 + uy * inset).round().max(0.0) as u64,
    );
    let end = (
        (to.0 as f64 - ux * inset).round().max(0.0) as u64,
        (to.1 as f64 - uy * inset).round().max(0.0) as u64,
    );
    Some((start, end))
}

fn estes_base_stat(level: usize, stack: usize) -> StatV1 {
    let growth = level.saturating_sub(1).min(14);
    StatV1 {
        attack: 48 + 3 * growth,
        magic_power: 55 + 8 * growth,
        hp: 690 + 92 * growth,
        defence: 26 + 4 * growth,
        magic_resistance: 30 + 4 * growth,
        move_speed: 1030,
        hp_regen: 7,
        stack: stack.min(SCRIPTURE_MAX),
        crit_chance: 0,
    }
}

fn set_scripture_stack(sim: &mut StableSim<'_>, entity_id: usize, stack: usize) {
    let level = sim
        .get_entity(entity_id)
        .map_or(1, |entity| entity.level().max(1));
    let _ = sim.entity_set_base_stat(entity_id, &estes_base_stat(level, stack));
}

fn advance_scripture(charge: &mut usize, tick_accumulator: &mut usize, linked: bool) -> bool {
    *tick_accumulator += 1;
    if *tick_accumulator < SCRIPTURE_TICKS_PER_CHARGE {
        return false;
    }
    *tick_accumulator -= SCRIPTURE_TICKS_PER_CHARGE;
    let before = *charge;
    *charge = (*charge + if linked { 2 } else { 1 }).min(SCRIPTURE_MAX);
    *charge != before
}

#[derive(Clone, Default)]
struct EstesScripture {
    voice: crate::audio::VoiceLifecycle,
    charge: usize,
    charge_tick_accumulator: usize,
    empowered_pending: bool,
}

impl EstesScripture {
    fn skill_link_target(
        &self,
        sim: &StableSim<'_>,
        entity_id: usize,
        team: usize,
        origin: (u64, u64),
    ) -> Option<(usize, (u64, u64), f64)> {
        if !has_named_buff(sim, entity_id, LINK_SELF) {
            return None;
        }
        (0..sim.entity_count())
            .filter_map(|index| sim.entity_at(index))
            .filter(|ally| {
                ally.team() == team
                    && ally.is_alive()
                    && ally.is_champion()
                    && has_named_buff(sim, ally.id(), LINK_TARGET)
            })
            .map(|ally| (ally.id(), ally.pos()))
            .filter(|(id, _)| *id != entity_id)
            .map(|(id, pos)| (id, pos, distance(origin, pos)))
            .filter(|(_, _, dist)| *dist <= LINK_RANGE)
            .min_by(|a, b| a.2.total_cmp(&b.2))
    }

    fn ultimate_link_targets(
        sim: &StableSim<'_>,
        entity_id: usize,
        team: usize,
        origin: (u64, u64),
    ) -> Vec<(usize, (u64, u64))> {
        (0..sim.entity_count())
            .filter_map(|index| sim.entity_at(index))
            .filter(|ally| {
                ally.id() != entity_id
                    && ally.team() == team
                    && ally.is_alive()
                    && ally.is_champion()
            })
            .map(|ally| (ally.id(), ally.pos()))
            .filter(|(_, pos)| distance(origin, *pos) <= ULT_LINK_RANGE)
            .collect()
    }
}

impl StablePassive for EstesScripture {
    fn clone_box(&self) -> Box<dyn StablePassive> {
        Box::new(self.clone())
    }

    fn on_spawn(&mut self, sim: &mut StableSim<'_>, _player: usize, entity_id: usize) {
        self.voice
            .on_spawn(sim, entity_id, Some("mlbb_estes_voice_select"));
        self.charge = 0;
        self.charge_tick_accumulator = 0;
        self.empowered_pending = false;
        let _ = sim.entity_remove_buff(entity_id, LINK_SELF);
        let _ = sim.entity_remove_buff(entity_id, ULT_AURA);
        set_scripture_stack(sim, entity_id, 0);
    }

    fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        entity_id: usize,
    ) {
        let Some((team, pos, alive, caster_radius)) = sim
            .get_entity(entity_id)
            .map(|e| (e.team(), e.pos(), e.is_alive(), e.radius() as f64))
        else {
            return;
        };
        self.voice.on_update(
            sim,
            entity_id,
            &["mlbb_estes_voice_select"],
            &[
                "mlbb_estes_voice_move01",
                "mlbb_estes_voice_move02",
                "mlbb_estes_voice_move03",
                "mlbb_estes_voice_move04",
                "mlbb_estes_voice_move05",
                "mlbb_estes_voice_move06",
            ],
        );
        if !alive {
            return;
        }

        let skill_link = self.skill_link_target(sim, entity_id, team, pos);
        if advance_scripture(
            &mut self.charge,
            &mut self.charge_tick_accumulator,
            skill_link.is_some(),
        ) {
            set_scripture_stack(sim, entity_id, self.charge);
        }

        let ultimate_active = has_named_buff(sim, entity_id, ULT_AURA);
        let visual_targets = if ultimate_active {
            Self::ultimate_link_targets(sim, entity_id, team, pos)
        } else {
            skill_link
                .map(|(ally_id, ally_pos, _)| vec![(ally_id, ally_pos)])
                .unwrap_or_default()
        };
        let tick = sim.tick();
        if tick % TETHER_REFRESH_TICKS == 0 && can_render_tether(sim) {
            // Refresh short-lived native view sprites from current simulation
            // positions so match and replay tethers track both moving endpoints.
            for (ally_id, ally_pos) in visual_targets {
                let ally_radius = sim
                    .get_entity(ally_id)
                    .map_or(0.0, |ally| ally.radius() as f64);
                let inset = (caster_radius + ally_radius) * 0.72;
                if let Some((from, to)) = inset_link_endpoints(pos, ally_pos, inset) {
                    draw_tether_sprites(sim, entity_id, from, to);
                }
            }
        }
    }

    fn on_base_attack(
        &mut self,
        _sim: &mut StableSim<'_>,
        _rng_seed: u64,
        _player: usize,
        _entity: usize,
    ) {
        self.empowered_pending = self.charge >= SCRIPTURE_MAX;
    }

    fn on_attack(
        &mut self,
        sim: &mut StableSim<'_>,
        _player: usize,
        entity_id: usize,
        target_id: usize,
        _damage: &mut usize,
    ) {
        if !self.empowered_pending {
            return;
        }
        self.empowered_pending = false;
        if target_id == entity_id {
            return;
        }
        let (team, level, ap, attack, alive) = match sim.get_entity(entity_id) {
            Some(caster) => (
                caster.team(),
                caster.level().max(1),
                caster.stat().magic_power,
                caster.stat().attack,
                caster.is_alive(),
            ),
            None => return,
        };
        let Some(primary) = sim.get_entity(target_id) else {
            return;
        };
        if !alive || !primary.is_alive() || primary.team() == team {
            return;
        }
        let primary_pos = primary.pos();
        let primary_bonus = 65 + level * 11 + attack * 35 / 100 + ap;
        sim.deal_damage(entity_id, target_id, 0, primary_bonus, AttackTypeV1::Skill);
        let slow = BuffV1 {
            move_speed_mult: -40,
            ..BuffV1::timed(PASSIVE_SLOW, 45)
        };
        sim.add_buff(target_id, &slow);

        let mut nearby: Vec<(usize, f64)> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i))
            .filter(|e| {
                e.id() != entity_id
                    && e.id() != target_id
                    && e.team() != team
                    && e.is_alive()
                    && e.is_champion()
                    && e.is_targetable()
            })
            .map(|e| (e.id(), distance(primary_pos, e.pos())))
            .filter(|(_, d)| *d <= SCRIPTURE_RICOCHET_RANGE)
            .collect();
        nearby.sort_by(|a, b| a.1.total_cmp(&b.1));
        for (other_id, _) in nearby.into_iter().take(SCRIPTURE_RICOCHET_MAX) {
            sim.deal_damage(
                entity_id,
                other_id,
                0,
                (primary_bonus / 2).max(1),
                AttackTypeV1::Skill,
            );
            sim.add_buff(other_id, &slow);
        }
        let _ = sim.play_view_effect(
            "estes_scripture_proc",
            entity_id,
            &InputTargetV1::target(target_id),
            0,
            0,
            0,
        );
        self.charge = 0;
        set_scripture_stack(sim, entity_id, 0);
    }

    fn on_dead(&mut self, sim: &mut StableSim<'_>, player: usize) {
        self.voice.on_dead(
            sim,
            player,
            &[
                "mlbb_estes_voice_death01",
                "mlbb_estes_voice_death02",
                "mlbb_estes_voice_death03",
            ],
        );
        self.charge = 0;
        self.charge_tick_accumulator = 0;
        self.empowered_pending = false;
    }
}

pub fn register(module: &mut StableMod) {
    module.add_native_passive("mlbb_heroes:estes_scripture", EstesScripture::default());
}

#[cfg(test)]
mod tests {
    use super::{
        advance_scripture, inset_link_endpoints, SCRIPTURE_MAX, SCRIPTURE_TICKS_PER_CHARGE,
    };

    #[test]
    fn scripture_charge_caps_at_one_hundred() {
        assert_eq!((SCRIPTURE_MAX - 1 + 2).min(SCRIPTURE_MAX), SCRIPTURE_MAX);
    }

    #[test]
    fn scripture_charges_once_per_second_and_doubles_while_linked() {
        let mut charge = 0;
        let mut elapsed = 0;
        for _ in 0..SCRIPTURE_TICKS_PER_CHARGE - 1 {
            assert!(!advance_scripture(&mut charge, &mut elapsed, false));
        }
        assert!(advance_scripture(&mut charge, &mut elapsed, false));
        assert_eq!(charge, 1);
        for _ in 0..SCRIPTURE_TICKS_PER_CHARGE {
            advance_scripture(&mut charge, &mut elapsed, true);
        }
        assert_eq!(charge, 3);
    }

    #[test]
    fn tether_endpoints_inset_to_the_live_entity_edge_positions() {
        assert_eq!(
            inset_link_endpoints((10_000, 20_000), (30_000, 20_000), 2_000.0),
            Some(((12_000, 20_000), (28_000, 20_000)))
        );
        assert_eq!(inset_link_endpoints((0, 0), (3_000, 4_000), 2_500.0), None);
    }
}
