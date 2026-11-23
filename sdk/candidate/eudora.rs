//! Revamped Eudora. Authoritative damage/timers live in the simulation.
//! Returned PixelLab art is cosmetic; no duplicated data-layer skill damage.
use crate::EntityKey;
use mod_api_stable::*;
use std::collections::{HashMap};
use std::sync::{LazyLock, Mutex};

type Pos = (u64,u64);
pub const MARK_TICKS: usize = 300;
pub const LINK_TICKS: usize = 60;
pub const LINK_PERIOD: usize = 15;
pub const REFUND: usize = 90;
pub const BURST_DELAY: usize = 36;
pub const S1_RANGE: f64 = 70_000.;
const S1_WINDUP_TICKS: usize = 8;
const S1_HALF_ANGLE_DEGREES: f64 = 50.;
pub const S2_RANGE: f64 = 75_000.;
pub const ULT_RANGE: f64 = 80_000.;
const BREAK_RANGE: f64 = 95_000.;
const S2_RADIUS: f64 = 22_000.;
const INNER_RADIUS: f64 = 15_000.;
const OUTER_RADIUS: f64 = 30_000.;
const BURST_RADIUS: f64 = 18_000.;
const SPEED: &str = "mlbb_eudora_link_speed";
const MR: &str = "mlbb_eudora_mr";
// (base damage, AP percent). Shared by real hits and AI damage estimates.
const FORK_HERO: (usize, usize) = (120, 65);
const FORK_HIT: (usize, usize) = (100, 55);
const LINK_PULSE: (usize, usize) = (4, 2);
const BALL_HIT: (usize, usize) = (115, 45);
const ULT_CENTER: (usize, usize) = (260, 110);
const ULT_OUTER: (usize, usize) = (130, 55);
const THUNDERBURST: (usize, usize) = (160, 72);

fn pct(v: usize,p: usize)->usize {v.saturating_mul(p)/100}
fn damage(base:usize,ratio:usize,ap:usize)->usize {base.saturating_add(pct(ap,ratio))}
fn spell(profile:(usize,usize),ap:usize)->usize {damage(profile.0,profile.1,ap)}
fn d2(a:Pos,b:Pos)->f64 {(a.0 as f64-b.0 as f64).powi(2)+(a.1 as f64-b.1 as f64).powi(2)}
fn within(a:Pos,b:Pos,r:f64)->bool {d2(a,b)<=r*r}
fn named(sim:&StableSim<'_>,id:usize,n:&str)->bool {
    sim.get_entity(id).is_some_and(|e|(0..e.buff_count()).filter_map(|i|e.buff_at(i)).any(|b|b.name()==n))
}
fn marker(k:EntityKey)->String {format!("mlbb_eudora_mark_{}_{}",k.entity_id,k.copy)}
fn eligible_kind(champ:bool,minion:bool,tower:bool,kind:Option<EntityKindV1>)->bool {
    !tower && !matches!(kind,Some(EntityKindV1::Nexus|EntityKindV1::Tower)) &&
    (champ||minion||matches!(kind,Some(EntityKindV1::Jungle|EntityKindV1::Epic|EntityKindV1::Serpen|EntityKindV1::Summon)))
}
fn enemy(sim:&StableSim<'_>,c:usize,t:usize)->bool {
    let (Some(a),Some(b))=(sim.get_entity(c),sim.get_entity(t)) else{return false};
    a.team()!=b.team() && b.is_alive() && b.is_targetable() && eligible_kind(b.is_champion(),b.is_minion(),b.is_tower(),b.kind())
}
fn basic_target(sim:&StableSim<'_>,c:usize,t:usize)->bool {
    let (Some(a),Some(b))=(sim.get_entity(c),sim.get_entity(t)) else{return false};
    a.team()!=b.team()&&b.is_alive()&&b.is_targetable()
}
fn markable(sim:&StableSim<'_>,id:usize)->bool {
    sim.get_entity(id).is_some_and(|e|e.is_champion()||matches!(e.kind(),Some(EntityKindV1::Jungle|EntityKindV1::Epic|EntityKindV1::Serpen)))
}
fn foes(sim:&StableSim<'_>,c:usize)->Vec<usize> {
    let mut ids:Vec<_>=(0..sim.entity_count()).filter_map(|i|sim.entity_at(i)).map(|e|e.id()).filter(|t|enemy(sim,c,*t)).collect();
    ids.sort_unstable(); ids
}
fn pos(sim:&StableSim<'_>,id:usize)->Option<Pos>{sim.get_entity(id).map(|e|e.pos())}
fn power(sim:&StableSim<'_>,id:usize)->usize{sim.get_entity(id).map_or(0,|e|e.stat().magic_power)}
fn fx(sim:&mut StableSim<'_>,name:&str,c:usize,p:Pos,time:usize) {
    // Unknown origin is common in 0.7.2. Let the host decide whether it can render.
    // Suppress known headless contexts without disabling actual matches on older hosts.
    if sim.sim_origin().and_then(|o|SimOriginKindV1::from_code(o.kind)).is_some_and(|k|matches!(k,SimOriginKindV1::ServerPresim|SimOriginKindV1::Tool)){return}
    sim.play_view_effect(name,c,&InputTargetV1::pos(p.0,p.1),0,0,time as u64);
}
fn apply_mark(sim:&mut StableSim<'_>,k:EntityKey,t:usize) {
    if markable(sim,t)&&sim.get_entity(t).is_some_and(|e|e.is_alive()) {
        let n=marker(k);sim.entity_remove_buff(t,&n);sim.add_buff(t,&BuffV1::timed(&n,MARK_TICKS));
    }
}
fn magic(sim:&mut StableSim<'_>,c:usize,t:usize,n:usize) {sim.deal_damage_typed(c,t,n,DamageTypeV1::Ap,AttackTypeV1::Skill);}
fn offset(from: Pos, to: Pos) -> (f64, f64) {
    (to.0 as f64 - from.0 as f64, to.1 as f64 - from.1 as f64)
}

fn fork_hits(from: Pos, to: Pos, direction: (f64, f64), radius: f64) -> bool {
    let (x, y) = offset(from, to);
    let forward = x * direction.0 + y * direction.1;
    let across = (y * direction.0 - x * direction.1).abs();
    let (sin, cos) = S1_HALF_ANGLE_DEGREES.to_radians().sin_cos();

    if across * cos <= forward * sin {
        return x.hypot(y) <= S1_RANGE + radius;
    }

    // Outside the angular span, the closest point is on the nearer radial edge.
    let edge = (forward * cos + across * sin).clamp(0., S1_RANGE);
    (forward - edge * cos).hypot(across - edge * sin) <= radius
}

fn fork_direction(sim: &StableSim<'_>, caster: usize, from: Pos, input: InputTargetV1) -> Option<(f64, f64)> {
    let (x, y) = match InputTargetKindV1::from_code(input.kind)? {
        InputTargetKindV1::Target => offset(from, pos(sim, input.target_id)?),
        InputTargetKindV1::Pos => offset(from, (input.x, input.y)),
        InputTargetKindV1::Dir => (input.dir_x as f64, input.dir_y as f64),
        _ => return None,
    };
    let length = x.hypot(y);
    if length > 0. {
        return Some((x / length, y / length));
    }

    // An enemy at the cone's origin can be hit in any direction. Look for a
    // direction within its hit circle before accepting a zero-length cast.
    for id in foes(sim, caster) {
        let Some(enemy) = sim.get_entity(id) else { continue };
        let (dx, dy) = offset(from, enemy.pos());
        let distance = dx.hypot(dy);
        if distance <= fork_target_radius(&enemy) {
            return Some(if distance > 0. { (dx / distance, dy / distance) } else { (1., 0.) });
        }
    }
    None
}

fn fork_target_radius(target: &StableEntity<'_, '_>) -> f64 {
    if target.is_minion() { target.radius() as f64 } else { 0. }
}

fn fork_ready_target(from: Pos, target: &StableEntity<'_, '_>) -> bool {
    let (dx, dy) = offset(from, target.pos());
    let runout = if target.is_minion() { target.stat().move_speed * S1_WINDUP_TICKS } else { 0 };
    let distance = dx.hypot(dy);
    distance <= S1_RANGE && distance + runout as f64 <= S1_RANGE + fork_target_radius(target)
}

fn fork_fallback(sim: &StableSim<'_>, caster: usize, from: Pos, input: InputV1) -> Option<InputV1> {
    let direction = fork_direction(sim, caster, from, input.target);
    let mut nearest_minion = None;
    for id in foes(sim, caster) {
        let Some(enemy) = sim.get_entity(id) else { continue };
        if !fork_ready_target(from, &enemy) {
            continue;
        }
        let point = enemy.pos();
        if direction.is_some_and(|direction| fork_hits(from, point, direction, fork_target_radius(&enemy))) {
            return Some(input);
        }
        let distance = d2(from, point);
        if enemy.is_minion() && nearest_minion.is_none_or(|(_, best)| distance < best) {
            nearest_minion = Some((point, distance));
        }
    }
    nearest_minion.map(|((x, y), _)| InputV1::action(InputKindV1::Skill, InputTargetV1::pos(x, y)))
}
fn aim(sim:&StableSim<'_>,c:usize,i:InputTargetV1)->Option<(Pos,(f64,f64))> {
    let from=pos(sim,c)?;
    let target=match InputTargetKindV1::from_code(i.kind)? {
        InputTargetKindV1::Target=>pos(sim,i.target_id)?,
        InputTargetKindV1::Pos=>(i.x,i.y),
        InputTargetKindV1::Dir=>((from.0 as i128+i.dir_x as i128).clamp(0,u64::MAX as i128) as u64,(from.1 as i128+i.dir_y as i128).clamp(0,u64::MAX as i128) as u64),
        _=>return None,
    };
    let x=target.0 as f64-from.0 as f64;let y=target.1 as f64-from.1 as f64;let r=x.hypot(y);
    (r>=1.).then_some((target,(x/r,y/r)))
}
fn angle_bucket(v:(f64,f64),n:usize)->usize {((v.1.atan2(v.0).rem_euclid(std::f64::consts::TAU)/std::f64::consts::TAU*n as f64).round() as usize)%n}

#[derive(Clone)]
struct Link { target:EntityKey, start:usize, paid:usize }
#[derive(Clone)]
struct LinkCast { ap:usize, links:Vec<Link>, refunded:bool }
#[derive(Clone)]
struct Burst { at:usize, ap:usize, seeds:Vec<Pos> }
#[derive(Clone,Default)]
struct Runtime { updated:Option<usize>, links:Vec<LinkCast>, bursts:Vec<Burst> }
static STATES:LazyLock<Mutex<HashMap<EntityKey,Runtime>>>=LazyLock::new(||Mutex::new(HashMap::new()));
fn take(sim:&mut StableSim<'_>,c:usize)->(EntityKey,Runtime) {
    let k=crate::entity_key(sim,c);let mut all=STATES.lock().unwrap_or_else(|e|e.into_inner());
    // Bound abandoned pre-sim entries. Current identity cannot alias an older copy.
    if all.len()>4096 {all.retain(|key,_|key.scope==k.scope);}
    let mut state=all.remove(&k).unwrap_or_default();
    if state.updated.is_some_and(|last|last>sim.tick()){state=Runtime::default();}
    (k,state)
}
fn put(k:EntityKey,s:Runtime) {STATES.lock().unwrap_or_else(|e|e.into_inner()).insert(k,s);}
fn valid_link(sim:&StableSim<'_>,k:EntityKey,l:&Link)->bool {
    enemy(sim,k.entity_id,l.target.entity_id) && crate::sim_copy_of(sim,l.target.entity_id)==Some(l.target.copy) &&
    pos(sim,k.entity_id).zip(pos(sim,l.target.entity_id)).is_some_and(|(a,b)|within(a,b,BREAK_RANGE))
}
fn wire(sim:&mut StableSim<'_>,c:usize,t:usize) {
    if sim.tick()%3!=0{return}
    let Some((a,b))=pos(sim,c).zip(pos(sim,t)) else{return};
    let x=b.0 as f64-a.0 as f64;let y=b.1 as f64-a.1 as f64;let r=x.hypot(y);
    let count=(r/12_000.).ceil().clamp(1.,16.) as usize;let angle=angle_bucket((x,y),16);
    for i in 0..count {let q=(i as f64+0.5)/count as f64;
        fx(sim,&format!("eudora_link_{angle}_{}",(sim.tick()/3)%9),c,((a.0 as f64+x*q).max(0.) as u64,(a.1 as f64+y*q).max(0.) as u64),4);
    }
}
fn fork(sim:&mut StableSim<'_>,c:usize,input:InputTargetV1) {
    if !sim.get_entity(c).is_some_and(|e|e.is_alive()&&e.is_targetable()){return}
    let Some(from)=pos(sim,c) else{return};
    let Some(dir)=fork_direction(sim,c,from,input) else{return};
    let (k,mut s)=take(sim,c);let ap=power(sim,c);let now=sim.tick();let n=marker(k);let mut links=vec![];
    fx(sim,&format!("eudora_fan_{}",angle_bucket(dir,16)),c,from,18);
    for t in foes(sim,c) {
        let Some(entity) = sim.get_entity(t) else { continue };
        if !fork_hits(from, entity.pos(), dir, fork_target_radius(&entity)) { continue }
        let damage = if entity.is_champion() { spell(FORK_HERO, ap) }
            else { spell(FORK_HIT, ap) * if entity.is_minion() { 2 } else { 1 } };
        let enhanced=markable(sim,t)&&named(sim,t,&n);
        let target_key=enhanced.then(||crate::entity_key(sim,t));
        magic(sim,c,t,damage);
        apply_mark(sim,k,t);
        if let Some(target)=target_key {if enemy(sim,c,t){links.push(Link{target,start:now,paid:0});}}
    }
    if !links.is_empty() {
        s.links.push(LinkCast{ap,links,refunded:false});
        sim.entity_remove_buff(c,SPEED);sim.add_buff(c,&BuffV1{move_speed_mult:20,..BuffV1::timed(SPEED,LINK_TICKS+1)});
    }
    put(k,s);
}
fn ball_hit(sim:&mut StableSim<'_>,c:usize,t:usize) {
    if !enemy(sim,c,t){return}
    let Some(center)=pos(sim,t) else{return};let (k,s)=take(sim,c);let ap=power(sim,c);
    let enhanced=markable(sim,t)&&named(sim,t,&marker(k));
    let targets:Vec<_>=if enhanced {foes(sim,c).into_iter().filter(|i|pos(sim,*i).is_some_and(|p|within(center,p,S2_RADIUS))).collect()} else {vec![t]};
    for id in targets {
        let name=format!("{MR}_{c}");sim.entity_remove_buff(id,&name);
        sim.add_buff(id,&BuffV1{magic_resistance:-8,..BuffV1::timed(&name,108)});
        magic(sim,c,id,spell(BALL_HIT,ap));sim.apply_cc(id,&CcV1::stun(48));apply_mark(sim,k,id);
    }
    fx(sim,if enhanced{"eudora_ball_burst"}else{"eudora_impact"},c,center,20);
    put(k,s);
}
fn ult(sim:&mut StableSim<'_>,c:usize,input:InputTargetV1) {
    if !sim.get_entity(c).is_some_and(|e|e.is_alive()&&e.is_targetable()){return}
    let center=if input.kind==InputTargetKindV1::Pos.code(){(input.x,input.y)}else{let Some((p,_))=aim(sim,c,input)else{return};p};
    let Some(from)=pos(sim,c) else{return};
    if !within(from,center,ULT_RANGE){return}
    let (k,mut s)=take(sim,c);let ap=power(sim,c);let n=marker(k);let mut seeds=vec![];
    fx(sim,"eudora_strike",c,center,24);fx(sim,"eudora_discharge",c,center,24);
    for t in foes(sim,c) {
        let Some(p)=pos(sim,t) else{continue};let r=d2(center,p);
        if r>OUTER_RADIUS*OUTER_RADIUS{continue}
        if markable(sim,t)&&named(sim,t,&n){seeds.push(p);}
        let profile=if r<=INNER_RADIUS*INNER_RADIUS{ULT_CENTER}else{ULT_OUTER};
        magic(sim,c,t,spell(profile,ap));apply_mark(sim,k,t);
    }
    if !seeds.is_empty(){for p in &seeds{fx(sim,"eudora_warning",c,*p,BURST_DELAY);}
        s.bursts.push(Burst{at:sim.tick()+BURST_DELAY,ap,seeds});}
    put(k,s);
}
fn projectile(sim:&mut StableSim<'_>,c:usize,t:usize,basic:bool) {
    if !(if basic {basic_target(sim,c,t)} else {enemy(sim,c,t)})||!sim.get_entity(c).is_some_and(|e|e.is_alive()&&e.is_targetable()){return}
    let Some((from,to))=pos(sim,c).zip(pos(sim,t)) else{return};
    if !within(from,to,if basic{62_000.}else{S2_RANGE}){return}
    let name=if basic {format!("eudora_bolt_{}",angle_bucket((to.0 as f64-from.0 as f64,to.1 as f64-from.1 as f64),16))}else{"eudora_ball".into()};
    let team=sim.get_entity(c).map_or(0,|e|e.team());
    let spec=ProjectileSpawnV1 {caster_id:c,team,x:from.0,y:from.1,radius:3_000,speed:6_000,
        move_kind:ProjectileMoveKindV1::Target.code(),target_id:t,target_x:to.0,target_y:to.1,
        attack_type:if basic{AttackTypeV1::BaseAttack}else{AttackTypeV1::Skill}.code(),
        casting_type:CastingTypeV1::Targeting.code(),casting_target:if basic{CastingTargetV1::Enemy}else{CastingTargetV1::EnemyWithoutTower}.code(),..Default::default()};
    sim.spawn_projectile(&name,if basic{"mlbb_heroes:eudora_basic_hit"}else{"mlbb_heroes:eudora_ball_hit"},&spec);
}
fn tick_state(sim:&mut StableSim<'_>,k:EntityKey,mut s:Runtime) {
    let now=sim.tick();if s.updated.is_some_and(|last|last>now){s=Runtime::default();} if s.updated==Some(now){put(k,s);return} s.updated=Some(now);
    let c=k.entity_id;let alive=sim.get_entity(c).is_some_and(|e|e.is_alive()&&e.is_targetable());
    if !alive {s.links.clear();sim.entity_remove_buff(c,SPEED);}
    for cast in &mut s.links {
        cast.links.retain(|l|valid_link(sim,k,l));
        for l in &mut cast.links {
            let elapsed=now.saturating_sub(l.start).min(LINK_TICKS);
            let due=elapsed/LINK_PERIOD;
            while l.paid<due {magic(sim,c,l.target.entity_id,spell(LINK_PULSE,cast.ap));apply_mark(sim,k,l.target.entity_id);l.paid+=1;}
            if elapsed==LINK_TICKS {
                if enemy(sim,c,l.target.entity_id){magic(sim,c,l.target.entity_id,spell(FORK_HIT,cast.ap));apply_mark(sim,k,l.target.entity_id);
                    if let Some(p)=pos(sim,l.target.entity_id){fx(sim,"eudora_impact",c,p,18);}
                    if !cast.refunded {cast.refunded=true;let ok=sim.entity_reduce_cooldown(c,SkillSlotV1::Skill,REFUND);
                        if !ok {crate::wlog("Eudora: host rejected S1 cooldown refund");}}
                }
            }else{wire(sim,c,l.target.entity_id);}
        }
        cast.links.retain(|l|now.saturating_sub(l.start)<LINK_TICKS && valid_link(sim,k,l));
    }
    s.links.retain(|b|!b.links.is_empty());
    if s.links.is_empty(){sim.entity_remove_buff(c,SPEED);}
    // Follow-up hits refresh/apply marks but never schedule further explosions. Overlap is capped
    // with one hit per victim per cast, not one hit per seed.
    for burst in &s.bursts {if now>=burst.at {
        for p in &burst.seeds {fx(sim,"eudora_strike",c,*p,24);fx(sim,"eudora_discharge",c,*p,24);}
        for t in foes(sim,c){if pos(sim,t).is_some_and(|p|burst.seeds.iter().any(|seed|within(*seed,p,BURST_RADIUS))){magic(sim,c,t,spell(THUNDERBURST,burst.ap));apply_mark(sim,k,t);}}
    }}
    s.bursts.retain(|b|now<b.at);
    // Render the owner's mark only for this caster's marked victims; no shared
    // view-buff removal can erase another Eudora's independent mark.
    if now%6==0 {for t in foes(sim,c){if named(sim,t,&marker(k)){if let Some(p)=pos(sim,t){fx(sim,&format!("eudora_mark_{}",(now/6)%9),c,p,7);}}}}
    put(k,s);
}
pub(crate) fn tick_all(sim:&mut StableSim<'_>) {
    let scope=crate::SimScope::of(sim);
    let mut keys:Vec<_>=STATES.lock().unwrap_or_else(|e|e.into_inner()).keys().copied()
        .filter(|k|k.scope==scope&&crate::sim_copy_of(sim,k.entity_id)==Some(k.copy)).collect();
    keys.sort_by_key(|k|k.entity_id);
    for k in keys {let s=STATES.lock().unwrap_or_else(|e|e.into_inner()).remove(&k);if let Some(s)=s{tick_state(sim,k,s);}}
}

#[derive(Clone,Copy)]
enum Op {Basic,BasicHit,Fork,Ball,BallHit,Ult}
impl StableEffectType for Op {
    fn apply(&self,sim:&mut StableSim<'_>,_:u64,c:usize,i:InputTargetV1) {
        match self {
            Self::Basic=>{if i.kind==InputTargetKindV1::Target.code(){projectile(sim,c,i.target_id,true)}},
            Self::BasicHit=>{if i.kind==InputTargetKindV1::Target.code()&&basic_target(sim,c,i.target_id){
                let a=sim.get_entity(c).map_or(0,|e|e.stat().attack);sim.deal_damage_typed(c,i.target_id,a,DamageTypeV1::Ad,AttackTypeV1::BaseAttack);
            }},
            Self::Fork=>fork(sim,c,i),
            Self::Ball=>{if i.kind==InputTargetKindV1::Target.code(){projectile(sim,c,i.target_id,false)}},
            Self::BallHit=>{if i.kind==InputTargetKindV1::Target.code(){ball_hit(sim,c,i.target_id)}},
            Self::Ult=>ult(sim,c,i),
        }
    }
    fn expected_damage(&self,s:&StatV1)->(usize,usize) {match self {Self::Basic|Self::BasicHit=>(s.attack,0),Self::Fork=>(0,spell(FORK_HERO,s.magic_power)),Self::Ball|Self::BallHit=>(0,spell(BALL_HIT,s.magic_power)),Self::Ult=>(0,spell(ULT_CENTER,s.magic_power))}}
}
#[derive(Clone,Default)]
struct Passive {voice:crate::audio::VoiceLifecycle}
impl StablePassive for Passive {
    fn clone_box(&self)->Box<dyn StablePassive>{Box::new(self.clone())}
    fn on_spawn(&mut self,sim:&mut StableSim<'_>,_:usize,c:usize) {
        self.voice.on_spawn(sim,c,Some("mlbb_eudora_voice_select"));
        let k=crate::entity_key(sim,c);
        STATES.lock().unwrap_or_else(|e|e.into_inner()).insert(k,Runtime::default());
        let n=marker(k);for t in foes(sim,c){sim.entity_remove_buff(t,&n);}sim.entity_remove_buff(c,SPEED);
    }
    fn on_update(&mut self,sim:&mut StableSim<'_>,_:u64,_:usize,c:usize) {
        self.voice.on_update(sim,c,&[],&["mlbb_eudora_voice_move01","mlbb_eudora_voice_move02","mlbb_eudora_voice_move03","mlbb_eudora_voice_move04","mlbb_eudora_voice_move05","mlbb_eudora_voice_move06","mlbb_eudora_voice_move08","mlbb_eudora_voice_move09"]);
        let (k,s)=take(sim,c);tick_state(sim,k,s);
    }
    fn on_dead(&mut self,sim:&mut StableSim<'_>,p:usize){self.voice.on_dead(sim,p,&["mlbb_eudora_voice_death"]);}
}

struct EudoraAi;
impl StablePlayerAi for EudoraAi {
    fn clone_box(&self)->Box<dyn StablePlayerAi>{Box::new(Self)}
    fn id(&self)->String{"mlbb_eudora_combo".into()}
    fn priority(&self)->i32{160}
    fn matches(&self,i:&StableAiInit)->bool{i.champion_name.to_ascii_lowercase().contains("eudora")}
    fn think(&mut self,ctx:&mut StableAiContext<'_>,base:Option<InputV1>)->Option<InputV1>{
        let player=ctx.player_id();
        let (proposals,fallback)={let sim=ctx.sim()?;let c=sim.get_player(player)?.champion()?;let id=c.id();let from=c.pos();
            if !c.is_alive()||!c.is_targetable(){return base}
            let k=crate::entity_key_ro(&sim,id);let n=marker(k);
            let mut targets:Vec<_>=foes(&sim,id).into_iter().filter(|t|sim.get_entity(*t).is_some_and(|e|e.is_champion())&&pos(&sim,*t).is_some_and(|p|within(from,p,ULT_RANGE))).collect();
            targets.sort_by_key(|t|{let e=sim.get_entity(*t).unwrap();let(h,m)=e.hp();(h.saturating_mul(1000)/m.max(1),*t)});
            let ready=|slot|c.cooldown(slot).is_some_and(|x|x.unlocked&&x.charges>0&&x.remaining==0);
            let mut out=vec![];
            if let Some(t)=targets.first().copied(){let p=pos(&sim,t)?;let marked=named(&sim,t,&n);
                let group=targets.iter().filter(|x|pos(&sim,**x).is_some_and(|q|within(p,q,S2_RADIUS))).count()>1;
                if group&&!marked&&ready(SkillSlotV1::Skill)&&within(from,p,S1_RANGE){out.push(InputV1::action(InputKindV1::Skill,InputTargetV1::pos(p.0,p.1)));}
                if ready(SkillSlotV1::Skill2)&&within(from,p,S2_RANGE){out.push(InputV1::action(InputKindV1::Skill2,InputTargetV1::target(t)));}
                if ready(SkillSlotV1::Ult){out.push(InputV1::action(InputKindV1::Ult,InputTargetV1::pos(p.0,p.1)));}
                if ready(SkillSlotV1::Skill)&&within(from,p,S1_RANGE){out.push(InputV1::action(InputKindV1::Skill,InputTargetV1::pos(p.0,p.1)));}
            }
            let fallback = match base {
                Some(input) if input.kind == InputKindV1::Skill.code() => fork_fallback(&sim, id, from, input),
                _ => base,
            };
            (out,fallback)
        };
        for i in proposals {if ctx.is_valid_input(&i){return Some(i)}}
        fallback.filter(|input| Some(*input) == base || ctx.is_valid_input(input))
    }
}
pub fn register(m:&mut StableMod){
    for (n,op) in [("basic",Op::Basic),("basic_hit",Op::BasicHit),("fork",Op::Fork),("ball",Op::Ball),("ball_hit",Op::BallHit),("ult",Op::Ult)] {m.add_native_effect(&format!("mlbb_heroes:eudora_{n}"),op);}
    m.add_native_passive("mlbb_heroes:eudora_superconductor",Passive::default());m.add_player_input_ai(EudoraAi);
}

#[cfg(test)]
#[path="eudora_tests.rs"]
mod tests;
