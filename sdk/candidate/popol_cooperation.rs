//! Bounded paired-hit bookkeeping and live companion stats. No rendering decisions.
use super::*;
pub(super) const BUFF: &str = "mlbb_popol_cooperation";
#[derive(Clone, Copy)]
struct Exchange {
    target: EntityKey,
    popol: Option<usize>,
    kupa: Option<usize>,
}
#[derive(Clone, Default)]
pub(super) struct Cooperation {
    exchanges: Vec<Exchange>,
    pub stacks: usize,
    until: usize,
    applied: (usize, bool),
    applied_until: usize,
    inherited: Option<(usize, usize, usize)>,
    next_sync: usize,
    next_heal: usize,
}
impl Cooperation {
    pub fn hit(&mut self, target: EntityKey, now: usize, from_kupa: bool) {
        if now >= self.until {
            self.stacks = 0;
        }
        self.exchanges.retain(|e| {
            e.popol
                .into_iter()
                .chain(e.kupa)
                .any(|t| now.saturating_sub(t) <= 120)
        });
        let index = match self.exchanges.iter().position(|e| e.target == target) {
            Some(i) => i,
            None => {
                if self.exchanges.len() == 8 {
                    self.exchanges.remove(0);
                }
                self.exchanges.push(Exchange {
                    target,
                    popol: None,
                    kupa: None,
                });
                self.exchanges.len() - 1
            }
        };
        let e = &mut self.exchanges[index];
        if from_kupa {
            e.kupa = Some(now);
        } else {
            e.popol = Some(now);
        }
        if let (Some(a), Some(b)) = (e.popol, e.kupa) {
            if a.abs_diff(b) <= 120 {
                self.stacks = (self.stacks + 1).min(3);
                self.until = now + 240;
                // Both receipts are consumed: one spear cannot pair with many bites.
                e.popol = None;
                e.kupa = None;
            }
        }
    }
    pub fn clear(&mut self, sim: &mut StableSim<'_>, owner: usize, pet: Option<usize>) {
        if self.applied.0 > 0 {
            sim.entity_remove_buff(owner, BUFF);
        }
        if self.applied.0 > 0 {
            if let Some(pet) = pet {
                sim.entity_remove_buff(pet, BUFF);
            }
        }
        *self = Self::default();
    }
    pub fn tick(
        &mut self,
        sim: &mut StableSim<'_>,
        owner: usize,
        pet: usize,
        alpha: bool,
        last_combat: usize,
    ) {
        let now = sim.tick();
        if now >= self.until {
            self.stacks = 0;
        }
        let wanted = (self.stacks, alpha && self.stacks > 0);
        if wanted != self.applied || (self.stacks > 0 && self.applied_until != self.until) {
            for id in [owner, pet] {
                sim.entity_remove_buff(id, BUFF);
                if self.stacks > 0 {
                    let factor = if alpha { 2 } else { 1 };
                    sim.add_buff(
                        id,
                        &BuffV1 {
                            attack_speed_mult: (self.stacks * 5 * factor) as i32,
                            move_speed_mult: (self.stacks * 2 * factor) as i32,
                            ..BuffV1::timed(BUFF, self.until.saturating_sub(now))
                        },
                    );
                }
            }
            self.applied = wanted;
            self.applied_until = self.until;
        }
        if now >= self.next_sync {
            self.next_sync = now + 30;
            let stat = companion_stat(sim, owner);
            let desired = (stat.hp, stat.defence, stat.magic_resistance);
            if self.inherited != Some(desired) {
                if sim.entity_set_base_stat(pet, &stat) {
                    self.inherited = Some(desired);
                    if let Some(e) = sim.get_entity(pet) {
                        let (hp, max) = e.hp();
                        if hp > max {
                            sim.entity_set_hp(pet, max);
                        }
                    }
                }
            }
        }
        if now.saturating_sub(last_combat) < 300 {
            self.next_heal = now + 60;
        } else if now >= self.next_heal {
            self.next_heal = now + 60;
            if let Some(e) = sim.get_entity(pet) {
                let (hp, max) = e.hp();
                if hp < max {
                    sim.heal(owner, pet, max * 8 / 100);
                }
            }
        }
    }
}
pub(super) fn companion_stat(sim: &StableSim<'_>, owner: usize) -> StatV1 {
    let e = sim.get_entity(owner).unwrap();
    let level = e.level().saturating_sub(1);
    let stat = e.stat();
    StatV1 {
        hp: 560 + level * 80 + stat.hp.saturating_sub(660 + level * 90) / 2,
        defence: stat.defence,
        magic_resistance: stat.magic_resistance,
        ..Default::default()
    }
}
pub(super) fn health(sim: &StableSim<'_>, target: usize) -> usize {
    sim.get_entity(target)
        .map_or(0, |e| e.hp().0.saturating_add(e.shield()))
}
pub(super) fn interval(sim: &StableSim<'_>, owner: usize) -> usize {
    let speed = sim.get_entity(owner).map_or(100, |e| e.attack_speed_mult());
    (INTERVAL * 100 / speed.max(1)).clamp(20, INTERVAL * 4)
}
