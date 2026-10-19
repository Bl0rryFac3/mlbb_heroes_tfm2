// TFM2 0.6.4+ Stable API (ABI Level 10) Companion Module for MLBB Default Aldous (mlbb_aldous)
// Optional: Compile via `TFM2ModUploader.exe` or `cargo build --release` to enable:
//   1. Real Minion (+4), Jungle Monster (+10), and Champion (+12) Soul Steal stacking via `on_unit_kill` (0.6.4+)
//   2. Homing Ghostly Soul Steal Skull Wisp projectile (`s1_orb`) flying from slain units into Aldous's gauntlet
//   3. Contract: Transform Passive Shield every 3rd basic attack scaling with Soul Steal stacks (`500 + 3 * stacks`)
use mod_api_stable::*;

#[derive(Default)]
struct AldousContractPassive {
    soul_stacks: usize,
    basic_attack_count: usize,
}

impl AldousContractPassive {
    fn spawn_soul_wisp(&self, sim: &mut StableSim<'_>, victim: usize, owner_entity: usize) {
        let Some(v) = sim.get_entity(victim) else { return };
        let Some(me) = sim.get_entity(owner_entity) else { return };
        let (vx, vy) = v.pos();
        let _ = sim.spawn_projectile(
            "s1_orb",
            "mlbb_aldous:soul_absorb",
            &ProjectileSpawnV1 {
                caster_id: owner_entity,
                team: me.team(),
                x: vx,
                y: vy,
                radius: 8000,
                speed: 5500,
                move_kind: ProjectileMoveKindV1::Target.code(),
                target_id: owner_entity,
                casting_target: CastingTargetV1::AllyOnlySelf.code(),
                ..ProjectileSpawnV1::default()
            },
        );
    }
}

impl StablePassive for AldousContractPassive {
    fn on_attack(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize, _target: usize, damage: &mut usize) {
        self.basic_attack_count += 1;
        // Bonus Soul Steal stack damage (+5 per stack when s1_ready buff is active)
        if let Some(me) = sim.get_entity(entity) {
            let has_s1 = (0..me.buff_count())
                .filter_map(|i| me.buff_at(i))
                .any(|b| b.name() == "s1_ready");
            if has_s1 {
                *damage = damage.saturating_add(self.soul_stacks * 5);
            }
        }
        // Contract: Transform — Every 3rd basic attack grants a shield (140 + 3 * stacks) for 180 ticks (3.0s)
        if self.basic_attack_count % 3 == 0 {
            let shield_amt = 140 + self.soul_stacks * 3;
            let _ = sim.entity_add_shield(entity, shield_amt, 180);
        }
    }

    fn on_unit_kill(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize, victim: usize) {
        let kind = sim.get_entity(victim).and_then(|v| v.kind());
        let gained = match kind {
            Some(EntityKindV1::Jungle) => 10,
            _ => 4, // Minion / summon
        };
        self.soul_stacks = (self.soul_stacks + gained).min(1000);
        self.spawn_soul_wisp(sim, victim, entity);
    }

    fn on_kill(&mut self, sim: &mut StableSim<'_>, _player: usize, entity: usize, victim: usize) {
        self.soul_stacks = (self.soul_stacks + 12).min(1000);
        self.spawn_soul_wisp(sim, victim, entity);
    }
}

fn init(host: &StableHost) -> StableMod {
    host.log(LogLevel::Info, "MLBB Aldous v4.1.0 (0.6.4 ABI Level 10) initialized");
    let mut decl = StableMod::new("mlbb_aldous");
    decl.add_native_passive("mlbb_aldous:contract_transform", Box::new(AldousContractPassive::default()));
    decl
}

declare_stable_mod!(init);
