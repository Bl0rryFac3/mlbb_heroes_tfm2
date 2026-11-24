//! Shared voice and repeatable-SFX gating for the roster.
//! Voice events are serialized across the sim and throttled per champion; short
//! per-clip SFX gates only collapse duplicate bursts, not ordinary combat audio.
use mod_api_stable::*;

const RUN_REPEAT_TICKS: usize = 480; // 8 s
const IDLE_REPEAT_TICKS: usize = 1_200; // 20 s
const IDLE_AFTER_TICKS: usize = 720; // 12 s stationary
const MOVE_THRESHOLD_SQ: u128 = 250_000; // ignore sub-500-unit position jitter
const VOICE_ACTOR_LOCK: &str = "mlbb_heroes_voice_actor_lock";
const VOICE_GLOBAL_LOCK: &str = "mlbb_heroes_voice_global_lock";
const VOICE_ACTOR_COOLDOWN_TICKS: usize = 300; // 5 s per champion
const SFX_DUPLICATE_WINDOW_TICKS: usize = 15; // 0.25 s per clip and champion

// Julian's assets currently need the client path API. Unlike named simulation sounds,
// that API is independent of pause/replay. Only carry requests across ONE client update,
// and require forward progress from a positively identified viewer in that same update.
// Never keep a backlog for a later frame or infer playback from the wall clock alone.
const PATH_VOLUME: f32 = 0.68;
const VOICE_RETRY_MARK_TICKS: usize = 600;
const US_PER_TICK: u64 = 16_667;
const MAX_AUDIO_FRAME_TICKS: usize = 120; // larger jumps are seeks/catch-up, not live playback

#[derive(Clone, Debug)]
struct PathRequest {
    scope: crate::SimScope,
    tick: usize,
    entity: usize,
    check_actor_gate: bool,
    line: String,
}

#[derive(Default)]
struct PathPlayback {
    frame_open: bool,
    now_us: u64,
    scope: Option<crate::SimScope>,
    // High-water marks survive backwards scrubbing, so old events cannot speak again.
    frontiers: std::collections::HashMap<crate::SimScope, usize>,
    frame_start: Option<usize>,
    frame_end: Option<usize>,
    pending: Option<PathRequest>,
    global_until_us: u64,
    actor_until_us: std::collections::HashMap<(crate::SimScope, usize), u64>,
}

impl PathPlayback {
    fn begin_frame(&mut self, visible: bool, dt: u64) {
        self.now_us = self.now_us.saturating_add(dt);
        self.frame_open = visible;
        self.frame_start = self
            .scope
            .and_then(|scope| self.frontiers.get(&scope).copied());
        self.frame_end = None;
        self.pending = None;
        if !visible {
            self.scope = None;
            // Frontiers intentionally remain: leaving and reopening the same replay must not
            // release a queue of already-heard events. A different match has a different scope.
        }
    }

    fn observe(&mut self, scope: crate::SimScope, tick: usize) {
        if !self.frame_open {
            return;
        }
        if self.scope != Some(scope) {
            self.scope = Some(scope);
            self.frame_start = self.frontiers.get(&scope).copied().or(Some(tick));
            self.frame_end = None;
            self.pending = None;
        }
        // Bound storage without deleting the active match's replay frontier.
        if self.frontiers.len() >= 64 && !self.frontiers.contains_key(&scope) {
            self.frontiers.clear();
        }
        let frontier = self.frontiers.entry(scope).or_insert(tick);
        if tick > *frontier {
            *frontier = tick;
            self.frame_end = Some(tick);
        }
    }

    fn offer(&mut self, request: PathRequest) -> bool {
        let Some(start) = self.frame_start else {
            return false;
        };
        if !self.frame_open
            || self.scope != Some(request.scope)
            || request.tick <= start
            || request.tick.saturating_sub(start) > MAX_AUDIO_FRAME_TICKS
            || self.now_us < self.global_until_us
            || (request.check_actor_gate
                && self
                    .actor_until_us
                    .get(&(request.scope, request.entity))
                    .is_some_and(|until| self.now_us < *until))
            || self.pending.is_some()
        {
            return false;
        }
        self.pending = Some(request);
        true
    }

    fn finish_frame(&mut self) -> Option<PathRequest> {
        self.frame_open = false;
        let request = self.pending.take()?;
        let (Some(start), Some(end)) = (self.frame_start, self.frame_end) else {
            return None;
        };
        if end <= start
            || end - start > MAX_AUDIO_FRAME_TICKS
            || request.scope != self.scope?
            || request.tick > end
            || request.tick <= start
        {
            return None;
        }
        Some(request)
    }

    fn played(&mut self, request: &PathRequest) {
        self.global_until_us = self
            .now_us
            .saturating_add(voice_lock_ticks(&request.line) as u64 * US_PER_TICK);
        self.actor_until_us.retain(|_, until| *until > self.now_us);
        self.actor_until_us.insert(
            (request.scope, request.entity),
            self.now_us.saturating_add(
                VOICE_ACTOR_COOLDOWN_TICKS.max(voice_lock_ticks(&request.line)) as u64
                    * US_PER_TICK,
            ),
        );
    }
}

static PATH_PLAYBACK: std::sync::LazyLock<std::sync::Mutex<PathPlayback>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(PathPlayback::default()));

fn viewer_origin(kind: Option<SimOriginKindV1>) -> bool {
    matches!(
        kind,
        Some(
            SimOriginKindV1::ClientMatchView
                | SimOriginKindV1::ClientSpectate
                | SimOriginKindV1::ClientReplay
        )
    )
}

pub(crate) fn voice_in_match_view(sim: &StableSim<'_>) -> bool {
    viewer_origin(sim.sim_origin().and_then(|o| SimOriginKindV1::from_code(o.kind)))
}

fn path_scene_visible(ctx: &StableClient<'_>) -> bool {
    ctx.scene_kind() == Some(SceneKindV1::InGame)
        && matches!(
            ctx.client_scene_kind(),
            Some(
                ClientSceneKindV1::Match
                    | ClientSceneKindV1::InGame
                    | ClientSceneKindV1::SetFeedback
                    | ClientSceneKindV1::TutorialMorgad
                    | ClientSceneKindV1::TutorialSerpen
                    | ClientSceneKindV1::Tutorial5v5
            )
        )
}

pub(crate) fn observe_voice_tick(sim: &StableSim<'_>) {
    if !viewer_origin(
        sim.sim_origin()
            .and_then(|o| SimOriginKindV1::from_code(o.kind)),
    ) {
        return;
    }
    if let Ok(mut playback) = PATH_PLAYBACK.lock() {
        playback.observe(crate::SimScope::of(sim), sim.tick());
    }
}

fn voice_stamp(sim: &StableSim<'_>) -> String {
    format!(
        "origin={:?} t={}",
        sim.sim_origin().map(|o| o.kind),
        sim.tick()
    )
}

// Preserve the shared native-sound policy for the rest of the roster.
fn voice_sim_is_audible(sim: &StableSim<'_>) -> bool {
    !matches!(
        sim.sim_origin()
            .and_then(|o| SimOriginKindV1::from_code(o.kind)),
        Some(SimOriginKindV1::ServerPresim | SimOriginKindV1::Tool)
    )
}

fn arm_voice_locks(sim: &mut StableSim<'_>, entity_id: usize, line: &str) {
    let _ = sim.entity_remove_buff(entity_id, VOICE_ACTOR_LOCK);
    sim.add_buff(
        entity_id,
        &BuffV1::timed(VOICE_ACTOR_LOCK, VOICE_ACTOR_COOLDOWN_TICKS),
    );
    sim.add_buff(
        entity_id,
        &BuffV1::timed(VOICE_GLOBAL_LOCK, voice_lock_ticks(line)),
    );
}

fn is_path_voice(line: &str) -> bool {
    line.starts_with("mlbb_julian_voice") || line.starts_with("mlbb_popol_voice") || line.starts_with("mlbb_granger_voice")
        || line.starts_with("mlbb_atlas_voice")
}

fn play_julian_voice_path(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    line: &str,
    check_actor_gate: bool,
) -> bool {
    if !viewer_origin(
        sim.sim_origin()
            .and_then(|o| SimOriginKindV1::from_code(o.kind)),
    ) {
        return false;
    }
    let request = PathRequest {
        scope: crate::SimScope::of(sim),
        tick: sim.tick(),
        entity: entity_id,
        check_actor_gate,
        line: line.into(),
    };
    let accepted = PATH_PLAYBACK
        .lock()
        .map(|mut playback| playback.offer(request))
        .unwrap_or(false);
    if accepted {
        arm_voice_locks(sim, entity_id, line);
    }
    crate::wlog(format!(
        "[VOICE frame-request] {line} entity={entity_id} accepted={accepted} {}",
        voice_stamp(sim)
    ));
    accepted
}

pub struct VoicePathExtension;
impl StableExtension for VoicePathExtension {
    fn pre_update(&self, ctx: &mut StableClient<'_>, dt_micros: u64) {
        if let Ok(mut playback) = PATH_PLAYBACK.lock() {
            playback.begin_frame(path_scene_visible(ctx), dt_micros);
        }
    }

    fn post_update(&self, ctx: &mut StableClient<'_>, _dt_micros: u64) {
        let next = PATH_PLAYBACK
            .lock()
            .ok()
            .and_then(|mut playback| playback.finish_frame());
        if !path_scene_visible(ctx) {
            return;
        }
        if let Some(request) = next {
            let path = format!("asset/mlbb_heroes/sound/sfx/{}", request.line);
            let ok = ctx.play_sound(&path, PATH_VOLUME);
            if ok {
                if let Ok(mut playback) = PATH_PLAYBACK.lock() {
                    playback.played(&request);
                }
            }
            crate::wlog(format!(
                "[VOICE frame-play] {} entity={} tick={} ok={ok} volume={PATH_VOLUME}",
                request.line, request.entity, request.tick
            ));
        }
    }

    fn on_end(&self) {
        if let Ok(mut playback) = PATH_PLAYBACK.lock() {
            *playback = PathPlayback::default();
        }
    }
}

pub(crate) const VOICE_CUES: &[(&str, &str, bool, bool)] = &[
    // SUN_CUES_BEGIN
    ("sun_select", "mlbb_sun_voice_select", false, false),
    ("sun_move01", "mlbb_sun_voice_move01", false, false),
    ("sun_move02", "mlbb_sun_voice_move02", false, false),
    ("sun_move03", "mlbb_sun_voice_move03", false, false),
    ("sun_move04", "mlbb_sun_voice_move04", false, false),
    ("sun_move05", "mlbb_sun_voice_move05", false, false),
    ("sun_move06", "mlbb_sun_voice_move06", false, false),
    ("sun_move07", "mlbb_sun_voice_move07", false, false),
    ("sun_move08", "mlbb_sun_voice_move08", false, false),
    ("sun_move09", "mlbb_sun_voice_move09", false, false),
    ("sun_move10", "mlbb_sun_voice_move10", false, false),
    ("sun_move11", "mlbb_sun_voice_move11", false, false),
    ("sun_basic01", "mlbb_sun_voice_basic01", false, false),
    ("sun_basic02", "mlbb_sun_voice_basic02", false, false),
    ("sun_basic03", "mlbb_sun_voice_basic03", false, false),
    ("sun_kill01", "mlbb_sun_voice_kill01", false, false),
    ("sun_kill02", "mlbb_sun_voice_kill02", false, false),
    ("sun_kill03", "mlbb_sun_voice_kill03", false, false),
    ("sun_skill01", "mlbb_sun_voice_skill01", false, false),
    ("sun_skill03", "mlbb_sun_voice_skill03", false, false),
    ("sun_ult01", "mlbb_sun_voice_ult01", true, false),
    ("sun_ult02", "mlbb_sun_voice_ult02", true, false),
    ("sun_death01", "mlbb_sun_voice_death01", true, false),
    ("sun_death02", "mlbb_sun_voice_death02", true, false),
    ("sun_respawn01", "mlbb_sun_voice_respawn01", false, false),
    ("sun_respawn02", "mlbb_sun_voice_respawn02", false, false),
    // SUN_CUES_END
    // LESLEY_CUES_BEGIN
    ("lesley_select", "mlbb_lesley_voice_select", false, false),
    ("lesley_move01", "mlbb_lesley_voice_move01", false, false),
    ("lesley_move02", "mlbb_lesley_voice_move02", false, false),
    ("lesley_move03", "mlbb_lesley_voice_move03", false, false),
    ("lesley_move04", "mlbb_lesley_voice_move04", false, false),
    ("lesley_move05", "mlbb_lesley_voice_move05", false, false),
    ("lesley_move06", "mlbb_lesley_voice_move06", false, false),
    ("lesley_move07", "mlbb_lesley_voice_move07", false, false),
    ("lesley_move08", "mlbb_lesley_voice_move08", false, false),
    ("lesley_move09", "mlbb_lesley_voice_move09", false, false),
    ("lesley_move10", "mlbb_lesley_voice_move10", false, false),
    ("lesley_move11", "mlbb_lesley_voice_move11", false, false),
    ("lesley_move12", "mlbb_lesley_voice_move12", false, false),
    ("lesley_basic01", "mlbb_lesley_voice_basic01", false, false),
    ("lesley_basic02", "mlbb_lesley_voice_basic02", false, false),
    ("lesley_basic03", "mlbb_lesley_voice_basic03", false, false),
    ("lesley_kill01", "mlbb_lesley_voice_kill01", false, false),
    ("lesley_kill02", "mlbb_lesley_voice_kill02", false, false),
    ("lesley_kill03", "mlbb_lesley_voice_kill03", false, false),
    ("lesley_skill01", "mlbb_lesley_voice_skill01", false, false),
    ("lesley_skill02", "mlbb_lesley_voice_skill02", false, false),
    ("lesley_ult01", "mlbb_lesley_voice_ult01", true, false),
    ("lesley_ult02", "mlbb_lesley_voice_ult02", true, false),
    ("lesley_death01", "mlbb_lesley_voice_death01", true, false),
    ("lesley_death02", "mlbb_lesley_voice_death02", true, false),
    ("lesley_respawn01", "mlbb_lesley_voice_respawn01", false, false),
    ("lesley_respawn02", "mlbb_lesley_voice_respawn02", false, false),
    // LESLEY_CUES_END
    // VALIR_CUES_BEGIN
    ("valir_select", "mlbb_valir_voice_select", false, false),
    ("valir_move01", "mlbb_valir_voice_move01", false, false),
    ("valir_move02", "mlbb_valir_voice_move02", false, false),
    ("valir_move03", "mlbb_valir_voice_move03", false, false),
    ("valir_move04", "mlbb_valir_voice_move04", false, false),
    ("valir_move05", "mlbb_valir_voice_move05", false, false),
    ("valir_move06", "mlbb_valir_voice_move06", false, false),
    ("valir_move07", "mlbb_valir_voice_move07", false, false),
    ("valir_skill01", "mlbb_valir_voice_skill01", false, false),
    ("valir_skill02", "mlbb_valir_voice_skill02", false, false),
    ("valir_skill03", "mlbb_valir_voice_skill03", false, false),
    ("valir_skill04", "mlbb_valir_voice_skill04", false, false),
    ("valir_ult", "mlbb_valir_voice_ult", true, false),
    ("valir_death01", "mlbb_valir_voice_death01", true, false),
    ("valir_death02", "mlbb_valir_voice_death02", true, false),
    ("valir_death03", "mlbb_valir_voice_death03", true, false),
    // VALIR_CUES_END
    // GRANGER_CUES_BEGIN
    ("granger_select", "mlbb_granger_voice_select", false, false),
    ("granger_move01", "mlbb_granger_voice_move01", false, false),
    ("granger_move02", "mlbb_granger_voice_move02", false, false),
    ("granger_move03", "mlbb_granger_voice_move03", false, false),
    ("granger_move04", "mlbb_granger_voice_move04", false, false),
    ("granger_move05", "mlbb_granger_voice_move05", false, false),
    ("granger_move06", "mlbb_granger_voice_move06", false, false),
    ("granger_move08", "mlbb_granger_voice_move08", false, false),
    ("granger_move09", "mlbb_granger_voice_move09", false, false),
    ("granger_move10", "mlbb_granger_voice_move10", false, false),
    ("granger_skill01", "mlbb_granger_voice_skill01", false, false),
    ("granger_ult01", "mlbb_granger_voice_ult01", false, false),
    ("granger_ult02", "mlbb_granger_voice_ult02", false, false),
    ("granger_death01", "mlbb_granger_voice_death01", false, false),
    ("granger_respawn01", "mlbb_granger_voice_respawn01", false, false),
    ("granger_respawn02", "mlbb_granger_voice_respawn02", false, false),
    ("granger_kill01", "mlbb_granger_voice_kill01", false, false),
    ("granger_kill02", "mlbb_granger_voice_kill02", false, false),
    ("granger_kill03", "mlbb_granger_voice_kill03", false, false),
    // GRANGER_CUES_END

    // Eudora default voice set; no basic-attack voice spam.
    ("eudora_select", "mlbb_eudora_voice_select", false, false),
    ("eudora_move01", "mlbb_eudora_voice_move01", false, false),
    ("eudora_move02", "mlbb_eudora_voice_move02", false, false),
    ("eudora_move03", "mlbb_eudora_voice_move03", false, false),
    ("eudora_move04", "mlbb_eudora_voice_move04", false, false),
    ("eudora_move05", "mlbb_eudora_voice_move05", false, false),
    ("eudora_move06", "mlbb_eudora_voice_move06", false, false),
    ("eudora_move08", "mlbb_eudora_voice_move08", false, false),
    ("eudora_move09", "mlbb_eudora_voice_move09", false, false),
    ("eudora_skill01", "mlbb_eudora_voice_skill01", false, true),
    ("eudora_skill02", "mlbb_eudora_voice_skill02", false, true),
    ("eudora_ult01", "mlbb_eudora_voice_ult01", true, true),
    ("eudora_ult02", "mlbb_eudora_voice_ult02", true, true),
    ("eudora_death", "mlbb_eudora_voice_death", true, true),
    ("popol_intro", "mlbb_popol_voice_intro", false, false),
    // Julian default voice set. Real MLBB clip durations drive the voice gate.
    ("julian_spawn", "mlbb_julian_voice_spawn", false, false),
    ("julian_move01", "mlbb_julian_voice_move01", false, false),
    ("julian_move03", "mlbb_julian_voice_move03", false, false),
    ("julian_move04", "mlbb_julian_voice_move04", false, false),
    ("julian_move05", "mlbb_julian_voice_move05", false, false),
    ("julian_move07", "mlbb_julian_voice_move07", false, false),
    ("julian_move08", "mlbb_julian_voice_move08", false, false),
    ("julian_move09", "mlbb_julian_voice_move09", false, false),
    ("julian_move10", "mlbb_julian_voice_move10", false, false),
    ("julian_move11", "mlbb_julian_voice_move11", false, false),
    ("julian_skill01", "mlbb_julian_voice_skill01", false, true),
    ("julian_skill02", "mlbb_julian_voice_skill02", false, true),
    ("julian_ult01", "mlbb_julian_voice_ult01", true, true),
    ("julian_death", "mlbb_julian_voice_death", true, true),
    ("julian_select", "mlbb_julian_voice_select", true, false),
    ("fanny_skill2", "mlbb_fanny_voice_skill2", false, false),
    ("fanny_ult", "mlbb_fanny_voice_ult", true, false),
    ("aldous_ult", "mlbb_aldous_voice_ult", true, false),
    (
        "aldous_ult_phase2",
        "mlbb_aldous_voice_ult_phase2",
        true,
        true,
    ),
    ("estes_skill", "mlbb_estes_voice_skill", false, false),
    // Angela. S1 has two recorded takes (the wiki's skill01 / skill02); the
    // random-pick cue below covers the data reference, so only the ult needs a
    // plain line here.
    ("angela_ult", "mlbb_angela_voice_ult", true, false),
    // Arlott. Priority so the ult line is not silently dropped when it lands
    // during another clip; the mod's own per-actor + global gates still apply.
    ("arlott_skill1", "mlbb_arlott_skill_cast", false, false),
    ("arlott_skill2", "mlbb_arlott_skill2_cast", false, false),
    ("arlott_ult", "mlbb_arlott_ult_cast", true, false),
    ("estes_ult", "mlbb_estes_voice_ult", true, false),
    // Angela's S1 carries up to 5 charges recharging every 1 s, so casts 2-5
    // always land inside her own 300-tick actor lock. A queued non-priority line
    // re-dispatches through `play_voice_with_actor_gate(.., true)` and is then
    // dropped outright -- there is no retry. Bypassing the per-actor gate makes
    // these behave like her (working) ult: the shared global channel lock below
    // still prevents overlap, so she cannot spam, but she is no longer silent.
    ("angela_skill01", "mlbb_angela_voice_skill01", false, true),
    ("angela_skill02", "mlbb_angela_voice_skill02", false, true),
    (
        "angela_voice_select",
        "mlbb_angela_voice_select",
        false,
        false,
    ),
    (
        "angela_voice_move01",
        "mlbb_angela_voice_move01",
        false,
        false,
    ),
    (
        "angela_voice_move02",
        "mlbb_angela_voice_move02",
        false,
        false,
    ),
    (
        "angela_voice_move03",
        "mlbb_angela_voice_move03",
        false,
        false,
    ),
    (
        "angela_voice_move04",
        "mlbb_angela_voice_move04",
        false,
        false,
    ),
    (
        "angela_voice_move05",
        "mlbb_angela_voice_move05",
        false,
        false,
    ),
    (
        "angela_voice_move06",
        "mlbb_angela_voice_move06",
        false,
        false,
    ),
    (
        "angela_voice_move07",
        "mlbb_angela_voice_move07",
        false,
        false,
    ),
    (
        "angela_voice_move08",
        "mlbb_angela_voice_move08",
        false,
        false,
    ),
    (
        "angela_voice_move09",
        "mlbb_angela_voice_move09",
        false,
        false,
    ),
    (
        "angela_voice_death01",
        "mlbb_angela_voice_death01",
        true,
        true,
    ),
    (
        "angela_voice_death02",
        "mlbb_angela_voice_death02",
        true,
        true,
    ),
    (
        "angela_voice_death03",
        "mlbb_angela_voice_death03",
        true,
        true,
    ),
    ("gusion_skill1", "mlbb_gusion_voice_skill1", false, false),
    ("gusion_skill2", "mlbb_gusion_voice_skill2", false, false),
    ("gusion_ult", "mlbb_gusion_voice_ult", true, false),
    (
        "gusion_ult_recast",
        "mlbb_gusion_voice_ult_recast",
        true,
        false,
    ),
    ("karrie_skill01", "mlbb_karrie_voice_skill01", false, false),
    ("karrie_skill02", "mlbb_karrie_voice_skill02", false, false),
    ("karrie_ult", "mlbb_karrie_voice_ult", true, false),
    ("chou_skill1_a", "mlbb_chou_voice_skill1_a", false, false),
    ("chou_skill1_b", "mlbb_chou_voice_skill1_b", false, false),
    ("chou_skill2", "mlbb_chou_voice_skill2", false, false),
    ("chou_ult", "mlbb_chou_voice_ult", true, false),
    ("lancelot_basic", "mlbb_lancelot_voice_basic", false, false),
    (
        "lancelot_skill1",
        "mlbb_lancelot_voice_skill1",
        false,
        false,
    ),
    (
        "lancelot_skill2",
        "mlbb_lancelot_voice_skill2",
        false,
        false,
    ),
    ("lancelot_ult", "mlbb_lancelot_voice_ult", true, false),
    ("aldous_voice_idle", "mlbb_aldous_voice_idle", false, false),
    ("aldous_voice_run", "mlbb_aldous_voice_run", false, false),
    (
        "estes_voice_death01",
        "mlbb_estes_voice_death01",
        true,
        true,
    ),
    (
        "estes_voice_death02",
        "mlbb_estes_voice_death02",
        true,
        true,
    ),
    (
        "estes_voice_death03",
        "mlbb_estes_voice_death03",
        true,
        true,
    ),
    (
        "estes_voice_move01",
        "mlbb_estes_voice_move01",
        false,
        false,
    ),
    (
        "estes_voice_move02",
        "mlbb_estes_voice_move02",
        false,
        false,
    ),
    (
        "estes_voice_move03",
        "mlbb_estes_voice_move03",
        false,
        false,
    ),
    (
        "estes_voice_move04",
        "mlbb_estes_voice_move04",
        false,
        false,
    ),
    (
        "estes_voice_move05",
        "mlbb_estes_voice_move05",
        false,
        false,
    ),
    (
        "estes_voice_move06",
        "mlbb_estes_voice_move06",
        false,
        false,
    ),
    (
        "estes_voice_select",
        "mlbb_estes_voice_select",
        false,
        false,
    ),
    ("fanny_voice_idle", "mlbb_fanny_voice_idle", false, false),
    ("fanny_voice_run", "mlbb_fanny_voice_run", false, false),
    (
        "gusion_voice_skill1",
        "mlbb_gusion_voice_skill1",
        false,
        false,
    ),
    (
        "gusion_voice_skill2",
        "mlbb_gusion_voice_skill2",
        false,
        false,
    ),
    ("gusion_voice_ult", "mlbb_gusion_voice_ult", true, false),
    (
        "gusion_voice_ult_recast",
        "mlbb_gusion_voice_ult_recast",
        true,
        false,
    ),
    (
        "karrie_voice_death01",
        "mlbb_karrie_voice_death01",
        true,
        true,
    ),
    (
        "karrie_voice_death02",
        "mlbb_karrie_voice_death02",
        true,
        true,
    ),
    (
        "karrie_voice_move01",
        "mlbb_karrie_voice_move01",
        false,
        false,
    ),
    (
        "karrie_voice_move02",
        "mlbb_karrie_voice_move02",
        false,
        false,
    ),
    (
        "karrie_voice_move03",
        "mlbb_karrie_voice_move03",
        false,
        false,
    ),
    (
        "karrie_voice_move04",
        "mlbb_karrie_voice_move04",
        false,
        false,
    ),
    (
        "karrie_voice_move05",
        "mlbb_karrie_voice_move05",
        false,
        false,
    ),
    (
        "karrie_voice_move06",
        "mlbb_karrie_voice_move06",
        false,
        false,
    ),
    (
        "karrie_voice_move07",
        "mlbb_karrie_voice_move07",
        false,
        false,
    ),
    (
        "karrie_voice_move08",
        "mlbb_karrie_voice_move08",
        false,
        false,
    ),
    (
        "karrie_voice_select",
        "mlbb_karrie_voice_select",
        false,
        false,
    ),
    (
        "lancelot_voice_idle",
        "mlbb_lancelot_voice_idle",
        false,
        false,
    ),
    (
        "lancelot_voice_run",
        "mlbb_lancelot_voice_run",
        false,
        false,
    ),
    ("uranus_skill1", "mlbb_uranus_voice_skill1", false, false),
    ("uranus_ult", "mlbb_uranus_voice_ult", true, false),
    (
        "uranus_voice_death01",
        "mlbb_uranus_voice_death01",
        true,
        true,
    ),
    (
        "uranus_voice_death02",
        "mlbb_uranus_voice_death02",
        true,
        true,
    ),
    (
        "uranus_voice_death03",
        "mlbb_uranus_voice_death03",
        true,
        true,
    ),
    ("zhask_voice_ult01", "mlbb_zhask_voice_ult01", true, true),
    ("zhask_voice_ult02", "mlbb_zhask_voice_ult02", true, true),
    ("zhask_voice_ult03", "mlbb_zhask_voice_ult03", true, true),
    (
        "zhask_voice_death01",
        "mlbb_zhask_voice_death01",
        true,
        true,
    ),
    (
        "zhask_voice_death02",
        "mlbb_zhask_voice_death02",
        true,
        true,
    ),
    (
        "zhask_voice_death03",
        "mlbb_zhask_voice_death03",
        true,
        true,
    ),
    (
        "zhask_voice_death04",
        "mlbb_zhask_voice_death04",
        true,
        true,
    ),
    (
        "balmond_voice_select",
        "mlbb_balmond_voice_select",
        false,
        false,
    ),
    (
        "balmond_voice_move01",
        "mlbb_balmond_voice_move01",
        false,
        false,
    ),
    (
        "balmond_voice_move02",
        "mlbb_balmond_voice_move02",
        false,
        false,
    ),
    (
        "balmond_voice_move03",
        "mlbb_balmond_voice_move03",
        false,
        false,
    ),
    (
        "balmond_voice_move04",
        "mlbb_balmond_voice_move04",
        false,
        false,
    ),
    (
        "balmond_voice_move05",
        "mlbb_balmond_voice_move05",
        false,
        false,
    ),
    (
        "balmond_voice_move06",
        "mlbb_balmond_voice_move06",
        false,
        false,
    ),
    (
        "balmond_voice_move07",
        "mlbb_balmond_voice_move07",
        false,
        false,
    ),
    (
        "balmond_voice_move08",
        "mlbb_balmond_voice_move08",
        false,
        false,
    ),
    (
        "balmond_voice_move09",
        "mlbb_balmond_voice_move09",
        false,
        false,
    ),
    (
        "balmond_voice_move10",
        "mlbb_balmond_voice_move10",
        false,
        false,
    ),
    (
        "balmond_voice_move11",
        "mlbb_balmond_voice_move11",
        false,
        false,
    ),
    (
        "balmond_voice_move12",
        "mlbb_balmond_voice_move12",
        false,
        false,
    ),
    (
        "balmond_voice_move13",
        "mlbb_balmond_voice_move13",
        false,
        false,
    ),
    (
        "balmond_voice_move14",
        "mlbb_balmond_voice_move14",
        false,
        false,
    ),
    (
        "balmond_voice_move15",
        "mlbb_balmond_voice_move15",
        false,
        false,
    ),
    (
        "balmond_voice_move16",
        "mlbb_balmond_voice_move16",
        false,
        false,
    ),
    (
        "balmond_voice_move17",
        "mlbb_balmond_voice_move17",
        false,
        false,
    ),
    (
        "balmond_voice_move18",
        "mlbb_balmond_voice_move18",
        false,
        false,
    ),
    (
        "balmond_voice_move19",
        "mlbb_balmond_voice_move19",
        false,
        false,
    ),
    (
        "balmond_voice_basic01",
        "mlbb_balmond_voice_basic01",
        false,
        false,
    ),
    (
        "balmond_voice_basic02",
        "mlbb_balmond_voice_basic02",
        false,
        false,
    ),
    (
        "balmond_voice_basic03",
        "mlbb_balmond_voice_basic03",
        false,
        false,
    ),
    (
        "balmond_voice_basic04",
        "mlbb_balmond_voice_basic04",
        false,
        false,
    ),
    (
        "balmond_voice_ult01",
        "mlbb_balmond_voice_ult01",
        true,
        false,
    ),
    (
        "balmond_voice_ult02",
        "mlbb_balmond_voice_ult02",
        true,
        false,
    ),
    (
        "balmond_voice_death01",
        "mlbb_balmond_voice_death01",
        true,
        false,
    ),
    (
        "balmond_voice_death02",
        "mlbb_balmond_voice_death02",
        true,
        false,
    ),
    // Gord. Conqueror set: S1 "Dark Void!", S2 "Energy Vortex!", ult "Galaxy
    // Beam!". Priority on the ult so the line is not dropped when the cast
    // lands while another clip is still finishing; the shared per-actor and
    // global gates still apply.
    ("gord_skill1", "mlbb_gord_skill1_cast", false, false),
    ("gord_skill2", "mlbb_gord_skill2_cast", false, false),
    ("gord_ult", "mlbb_gord_ult_cast", true, false),
    // Belerick. Only the ult is a cast line — the Fandom audio page carries no
    // skill1/skill2 voice for him. Deaths are priority + bypass the actor gate
    // the way Estes' are, so the line is not eaten by the cooldown the kill
    // itself just started. Select/move are routine and drop while occupied.
    ("belerick_ult", "mlbb_belerick_ult_cast", true, false),
    (
        "belerick_voice_select",
        "mlbb_belerick_voice_select",
        false,
        false,
    ),
    (
        "belerick_voice_move01",
        "mlbb_belerick_voice_move01",
        false,
        false,
    ),
    (
        "belerick_voice_move02",
        "mlbb_belerick_voice_move02",
        false,
        false,
    ),
    (
        "belerick_voice_move03",
        "mlbb_belerick_voice_move03",
        false,
        false,
    ),
    (
        "belerick_voice_move04",
        "mlbb_belerick_voice_move04",
        false,
        false,
    ),
    (
        "belerick_voice_move05",
        "mlbb_belerick_voice_move05",
        false,
        false,
    ),
    (
        "belerick_voice_move06",
        "mlbb_belerick_voice_move06",
        false,
        false,
    ),
    (
        "belerick_voice_move07",
        "mlbb_belerick_voice_move07",
        false,
        false,
    ),
    (
        "belerick_voice_move08",
        "mlbb_belerick_voice_move08",
        false,
        false,
    ),
    (
        "belerick_voice_death01",
        "mlbb_belerick_voice_death01",
        true,
        true,
    ),
    (
        "belerick_voice_death02",
        "mlbb_belerick_voice_death02",
        true,
        true,
    ),
    (
        "belerick_voice_death03",
        "mlbb_belerick_voice_death03",
        true,
        true,
    ),
    // Cecilion. Bat Impact / Sanguine Claws / Bats Feast, from the default
    // (non-skin) set on the Fandom audio page. Priority on the ult so the line
    // is not dropped when the cast lands while another clip is still finishing;
    // the shared per-actor and global gates still apply.
    //
    // The second field is the EVENT, not the clip: the engine reads
    // `<event>.sound_info`, which in turn names `mlbb_cecilion_sfx_vo_<slot>`.
    // Both the event and the clip need a route in `mod.override_info` or
    // nothing plays. Clip durations for the gate: skill1 1.41s, skill2 1.00s,
    // ult 2.39s.
    //
    // NOT fetched on purpose: `Cecilion.skill03` (Moonlit Waltz) and
    // `Cecilion.talk01-04` (Carmilla interactions) — Moonlit Waltz is dropped
    // from this port and the Carmilla lines need a hero we do not have.
    ("cecilion_skill1", "mlbb_cecilion_skill1_cast", false, false),
    ("cecilion_skill2", "mlbb_cecilion_skill2_cast", false, false),
    ("cecilion_ult", "mlbb_cecilion_ult_cast", true, false),
    // Johnson. Lines from the DEFAULT (non-skin) set on the Fandom audio page,
    // hero id 321. The ult gets priority and bypasses the actor gate so both
    // ult lines are audible; the global gate still applies.
    //
    // The second field is the EVENT, not the clip: the engine reads
    // `<event>.sound_info`, which in turn names `mlbb_heroes_clip_<event>`.
    // BOTH the event and the clip need a `mod.override_info` route or nothing
    // plays (see the Cecilion note above — same trap).
    //
    // Skill 1 is intentionally absent: the wiki lists only a grunt
    // (`Audio321-skill01.ogg`), and Johnson's S1 already has hit SFX.
    ("johnson_skill2", "mlbb_johnson_voice_skill2", false, false),
    ("johnson_ult01", "mlbb_johnson_voice_ult01", true, true),
    ("johnson_ult02", "mlbb_johnson_voice_ult02", true, true),
    (
        "johnson_voice_select",
        "mlbb_johnson_voice_select",
        false,
        false,
    ),
    (
        "johnson_voice_move01",
        "mlbb_johnson_voice_move01",
        false,
        false,
    ),
    (
        "johnson_voice_move02",
        "mlbb_johnson_voice_move02",
        false,
        false,
    ),
    (
        "johnson_voice_move03",
        "mlbb_johnson_voice_move03",
        false,
        false,
    ),
    (
        "johnson_voice_move04",
        "mlbb_johnson_voice_move04",
        false,
        false,
    ),
    (
        "johnson_voice_move05",
        "mlbb_johnson_voice_move05",
        false,
        false,
    ),
    (
        "johnson_voice_death01",
        "mlbb_johnson_voice_death01",
        true,
        true,
    ),
    (
        "johnson_voice_death02",
        "mlbb_johnson_voice_death02",
        true,
        true,
    ),
    (
        "johnson_voice_recall01",
        "mlbb_johnson_voice_recall01",
        false,
        false,
    ),
    (
        "johnson_voice_attack01",
        "mlbb_johnson_voice_attack01",
        false,
        false,
    ),
];

fn has_named_buff(sim: &StableSim<'_>, entity_id: usize, name: &str) -> bool {
    sim.get_entity(entity_id).is_some_and(|entity| {
        (0..entity.buff_count())
            .filter_map(|index| entity.buff_at(index))
            .any(|buff| buff.name() == name)
    })
}

fn global_voice_remaining(sim: &StableSim<'_>) -> Option<usize> {
    for index in 0..sim.entity_count() {
        let Some(entity) = sim.entity_at(index) else {
            continue;
        };
        for buff_index in 0..entity.buff_count() {
            if let Some(buff) = entity.buff_at(buff_index) {
                if buff.name() == VOICE_GLOBAL_LOCK {
                    return Some(buff.duration_tick.max(1));
                }
            }
        }
    }
    None
}

/// Approximate playback duration in sim ticks, derived from bundled WAV lengths
/// plus any sequential delays in their .sound_info files.
pub(crate) fn voice_lock_ticks(line: &str) -> usize {
    match line {
        // SUN_DURATIONS_BEGIN
        "mlbb_sun_voice_select" => 125,
        "mlbb_sun_voice_move01" => 134,
        "mlbb_sun_voice_move02" => 221,
        "mlbb_sun_voice_move03" => 317,
        "mlbb_sun_voice_move04" => 284,
        "mlbb_sun_voice_move05" => 264,
        "mlbb_sun_voice_move06" => 242,
        "mlbb_sun_voice_move07" => 216,
        "mlbb_sun_voice_move08" => 211,
        "mlbb_sun_voice_move09" => 211,
        "mlbb_sun_voice_move10" => 194,
        "mlbb_sun_voice_move11" => 192,
        "mlbb_sun_voice_basic01" => 77,
        "mlbb_sun_voice_basic02" => 57,
        "mlbb_sun_voice_basic03" => 56,
        "mlbb_sun_voice_kill01" => 185,
        "mlbb_sun_voice_kill02" => 151,
        "mlbb_sun_voice_kill03" => 143,
        "mlbb_sun_voice_skill01" => 63,
        "mlbb_sun_voice_skill03" => 97,
        "mlbb_sun_voice_ult01" => 105,
        "mlbb_sun_voice_ult02" => 91,
        "mlbb_sun_voice_death01" => 126,
        "mlbb_sun_voice_death02" => 99,
        "mlbb_sun_voice_respawn01" => 166,
        "mlbb_sun_voice_respawn02" => 100,
        // SUN_DURATIONS_END
        // LESLEY_DURATIONS_BEGIN
        "mlbb_lesley_voice_select" => 138,
        "mlbb_lesley_voice_move01" => 212,
        "mlbb_lesley_voice_move02" => 329,
        "mlbb_lesley_voice_move03" => 177,
        "mlbb_lesley_voice_move04" => 93,
        "mlbb_lesley_voice_move05" => 169,
        "mlbb_lesley_voice_move06" => 114,
        "mlbb_lesley_voice_move07" => 536,
        "mlbb_lesley_voice_move08" => 127,
        "mlbb_lesley_voice_move09" => 79,
        "mlbb_lesley_voice_move10" => 121,
        "mlbb_lesley_voice_move11" => 166,
        "mlbb_lesley_voice_move12" => 128,
        "mlbb_lesley_voice_basic01" => 33,
        "mlbb_lesley_voice_basic02" => 29,
        "mlbb_lesley_voice_basic03" => 31,
        "mlbb_lesley_voice_kill01" => 63,
        "mlbb_lesley_voice_kill02" => 101,
        "mlbb_lesley_voice_kill03" => 127,
        "mlbb_lesley_voice_skill01" => 136,
        "mlbb_lesley_voice_skill02" => 45,
        "mlbb_lesley_voice_ult01" => 60,
        "mlbb_lesley_voice_ult02" => 79,
        "mlbb_lesley_voice_death01" => 131,
        "mlbb_lesley_voice_death02" => 61,
        "mlbb_lesley_voice_respawn01" => 157,
        "mlbb_lesley_voice_respawn02" => 125,
        // LESLEY_DURATIONS_END
        // VALIR_DURATIONS_BEGIN
        "mlbb_valir_voice_select" => 251,
        "mlbb_valir_voice_move01" => 210,
        "mlbb_valir_voice_move02" => 350,
        "mlbb_valir_voice_move03" => 349,
        "mlbb_valir_voice_move04" => 214,
        "mlbb_valir_voice_move05" => 180,
        "mlbb_valir_voice_move06" => 263,
        "mlbb_valir_voice_move07" => 198,
        "mlbb_valir_voice_skill01" => 112,
        "mlbb_valir_voice_skill02" => 130,
        "mlbb_valir_voice_skill03" => 115,
        "mlbb_valir_voice_skill04" => 165,
        "mlbb_valir_voice_ult" => 287,
        "mlbb_valir_voice_death01" => 197,
        "mlbb_valir_voice_death02" => 290,
        "mlbb_valir_voice_death03" => 113,
        // VALIR_DURATIONS_END
        // GRANGER_DURATIONS_BEGIN
        "mlbb_granger_voice_select" => 224,
        "mlbb_granger_voice_move01" => 222,
        "mlbb_granger_voice_move02" => 328,
        "mlbb_granger_voice_move03" => 276,
        "mlbb_granger_voice_move04" => 255,
        "mlbb_granger_voice_move05" => 244,
        "mlbb_granger_voice_move06" => 216,
        "mlbb_granger_voice_move08" => 211,
        "mlbb_granger_voice_move09" => 176,
        "mlbb_granger_voice_move10" => 166,
        "mlbb_atlas_voice_skill" => 121,
        "mlbb_atlas_voice_ult" => 232,
        "mlbb_granger_voice_skill01" => 77,
        "mlbb_granger_voice_ult01" => 121,
        "mlbb_granger_voice_ult02" => 76,
        "mlbb_granger_voice_death01" => 147,
        "mlbb_granger_voice_respawn01" => 263,
        "mlbb_granger_voice_respawn02" => 143,
        "mlbb_granger_voice_kill01" => 206,
        "mlbb_granger_voice_kill02" => 219,
        "mlbb_granger_voice_kill03" => 136,
        // GRANGER_DURATIONS_END

        "mlbb_popol_voice_intro" => 218,
        "mlbb_eudora_voice_select" => 217,
        "mlbb_eudora_voice_move01" => 278,
        "mlbb_eudora_voice_move02" => 260,
        "mlbb_eudora_voice_move03" => 130,
        "mlbb_eudora_voice_move04" => 312,
        "mlbb_eudora_voice_move05" => 184,
        "mlbb_eudora_voice_move06" => 345,
        "mlbb_eudora_voice_move08" => 264,
        "mlbb_eudora_voice_move09" => 213,
        "mlbb_eudora_voice_skill01" => 87,
        "mlbb_eudora_voice_skill02" => 72,
        "mlbb_eudora_voice_ult01" => 80,
        "mlbb_eudora_voice_ult02" => 72,
        "mlbb_eudora_voice_death" => 303,

        "mlbb_aldous_voice_idle" => 226,
        "mlbb_aldous_voice_run" => 250,
        "mlbb_aldous_voice_ult" => 162,
        "mlbb_aldous_voice_ult_phase2" => 152,
        "mlbb_estes_voice_death01" => 164,
        "mlbb_estes_voice_death02" => 78,
        "mlbb_estes_voice_death03" => 80,
        "mlbb_estes_voice_move01" => 203,
        "mlbb_estes_voice_move02" => 179,
        "mlbb_estes_voice_move03" => 130,
        "mlbb_estes_voice_move04" => 154,
        "mlbb_estes_voice_move05" => 133,
        "mlbb_estes_voice_move06" => 182,
        "mlbb_estes_voice_select" => 110,
        "mlbb_estes_voice_skill" => 39,
        "mlbb_estes_voice_ult" => 170,
        "mlbb_fanny_voice_idle" => 144,
        "mlbb_fanny_voice_run" => 198,
        "mlbb_fanny_voice_skill2" => 182,
        "mlbb_fanny_voice_ult" => 126,
        "mlbb_gusion_voice_skill1" => 61,
        "mlbb_gusion_voice_skill2" => 80,
        "mlbb_gusion_voice_ult" => 144,
        "mlbb_gusion_voice_ult_recast" => 80,
        "mlbb_karrie_voice_death01" => 133,
        "mlbb_karrie_voice_death02" => 136,
        "mlbb_karrie_voice_move01" => 218,
        "mlbb_karrie_voice_move02" => 115,
        "mlbb_karrie_voice_move03" => 177,
        "mlbb_karrie_voice_move04" => 221,
        "mlbb_karrie_voice_move05" => 236,
        "mlbb_karrie_voice_move06" => 115,
        "mlbb_karrie_voice_move07" => 211,
        "mlbb_karrie_voice_move08" => 159,
        "mlbb_karrie_voice_select" => 166,
        "mlbb_karrie_voice_skill01" => 62,
        "mlbb_karrie_voice_skill02" => 61,
        "mlbb_karrie_voice_ult" => 161,
        "mlbb_lancelot_voice_basic" => 53,
        "mlbb_lancelot_voice_idle" => 203,
        "mlbb_lancelot_voice_run" => 83,
        "mlbb_lancelot_voice_skill1" => 50,
        "mlbb_lancelot_voice_skill2" => 52,
        "mlbb_lancelot_voice_ult" => 112,
        // Original Uranus clips: duration at 60 ticks/s plus the 0.2s tail margin.
        "mlbb_uranus_voice_skill1" => 112,
        "mlbb_uranus_voice_ult" => 238,
        "mlbb_uranus_voice_death01" => 267,
        "mlbb_uranus_voice_death02" => 304,
        "mlbb_uranus_voice_death03" => 214,
        // Zhask default English clips: measured WAV duration at 60 ticks/s + 0.2s tail.
        "mlbb_angela_voice_death01" => 268,
        "mlbb_angela_voice_death02" => 269,
        "mlbb_angela_voice_death03" => 228,
        "mlbb_angela_voice_move01" => 244,
        "mlbb_angela_voice_move02" => 413,
        "mlbb_angela_voice_move03" => 222,
        "mlbb_angela_voice_move04" => 263,
        "mlbb_angela_voice_move05" => 362,
        "mlbb_angela_voice_move06" => 437,
        "mlbb_angela_voice_move07" => 195,
        "mlbb_angela_voice_move08" => 386,
        "mlbb_angela_voice_move09" => 256,
        "mlbb_angela_voice_select" => 312,
        "mlbb_angela_voice_skill01" => 87,
        "mlbb_angela_voice_skill02" => 75,
        "mlbb_angela_voice_ult" => 99,
        "mlbb_zhask_voice_ult01" => 160,
        "mlbb_zhask_voice_ult02" => 171,
        "mlbb_zhask_voice_ult03" => 150,
        "mlbb_zhask_voice_death01" => 421,
        "mlbb_zhask_voice_death02" => 259,
        "mlbb_zhask_voice_death03" => 95,
        "mlbb_zhask_voice_death04" => 70,
        "mlbb_balmond_voice_basic01" => 64,
        "mlbb_balmond_voice_basic02" => 79,
        "mlbb_balmond_voice_basic03" => 78,
        "mlbb_balmond_voice_basic04" => 87,
        "mlbb_balmond_voice_death01" => 255,
        "mlbb_balmond_voice_death02" => 103,
        "mlbb_balmond_voice_move01" => 208,
        "mlbb_balmond_voice_move02" => 201,
        "mlbb_balmond_voice_move03" => 254,
        "mlbb_balmond_voice_move04" => 298,
        "mlbb_balmond_voice_move05" => 341,
        "mlbb_balmond_voice_move06" => 196,
        "mlbb_balmond_voice_move07" => 366,
        "mlbb_balmond_voice_move08" => 266,
        "mlbb_balmond_voice_move09" => 173,
        "mlbb_balmond_voice_move10" => 136,
        "mlbb_balmond_voice_move11" => 248,
        "mlbb_balmond_voice_move12" => 157,
        "mlbb_balmond_voice_move13" => 170,
        "mlbb_balmond_voice_move14" => 362,
        "mlbb_balmond_voice_move15" => 328,
        "mlbb_balmond_voice_move16" => 338,
        "mlbb_balmond_voice_move17" => 309,
        "mlbb_balmond_voice_move18" => 231,
        "mlbb_balmond_voice_move19" => 324,
        "mlbb_balmond_voice_select" => 258,
        "mlbb_balmond_voice_ult01" => 189,
        "mlbb_balmond_voice_ult02" => 175,
        // Gord (Conqueror set) — measured from the shipped clips, so the global
        // gate releases as each line ends instead of holding a flat 3 s.
        "mlbb_gord_skill1_cast" => 89,
        "mlbb_gord_skill2_cast" => 140,
        "mlbb_gord_ult_cast" => 148,
        // Belerick — measured from the installed clips by
        // belerick_work/install_belerick_audio.py (round(seconds * 60) + 12
        // tail), not from the wiki .ogg lengths, which carry long silence.
        "mlbb_belerick_ult_cast" => 185,
        "mlbb_belerick_voice_select" => 224,
        "mlbb_belerick_voice_move01" => 298,
        "mlbb_belerick_voice_move02" => 237,
        "mlbb_belerick_voice_move03" => 210,
        "mlbb_belerick_voice_move04" => 370,
        "mlbb_belerick_voice_move05" => 305,
        "mlbb_belerick_voice_move06" => 397,
        "mlbb_belerick_voice_move07" => 327,
        "mlbb_belerick_voice_move08" => 466,
        "mlbb_belerick_voice_death01" => 423,
        "mlbb_belerick_voice_death02" => 101,
        "mlbb_belerick_voice_death03" => 87,
        "mlbb_julian_voice_select" => 185,
        "mlbb_julian_voice_spawn" => 129,
        "mlbb_julian_voice_move01" => 120,
        "mlbb_julian_voice_move03" => 126,
        "mlbb_julian_voice_move04" => 73,
        "mlbb_julian_voice_move05" => 100,
        "mlbb_julian_voice_move07" => 152,
        "mlbb_julian_voice_move08" => 55,
        "mlbb_julian_voice_move09" => 146,
        "mlbb_julian_voice_move10" => 187,
        "mlbb_julian_voice_move11" => 145,
        "mlbb_julian_voice_skill01" => 65,
        "mlbb_julian_voice_skill02" => 92,
        "mlbb_julian_voice_ult01" => 81,
        "mlbb_julian_voice_death" => 139,
        _ => 180,
    }
}

/// Test-only: returns the lock duration for a cue, or None when the cue is unregistered.
#[cfg(test)]
pub(crate) fn lock_duration_for_test(line: &str) -> Option<usize> {
    if VOICE_CUES.iter().any(|(_, event, _, _)| *event == line) {
        // the fallback arm is 180; treat that as "registered" here because the
        // regression we guard is a cue with NO arm at all and NO VOICE_CUES row.
        Some(voice_lock_ticks(line))
    } else {
        None
    }
}

fn play_voice_with_actor_gate(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    line: &str,
    check_actor_gate: bool,
) -> bool {
    // 1.1.133: audio belongs to a sim the player can see. See `voice_sim_is_audible` for why this
    // was provably not the case, and `VOICE_SIM_FILTER` to switch it back off. Checked before
    // anything else, so a headless step cannot arm a lock, spend a retry slot, or push to the queue.
    if !voice_sim_is_audible(sim) {
        crate::wlog(&format!(
            "[VOICE drop] {line} entity={entity_id} reason=headless_sim {}",
            voice_stamp(sim)
        ));
        return false;
    }
    // Diagnostic: a dropped line used to be completely silent in the log, which made a
    // "cannot hear him" report impossible to diagnose. Every request now records its
    // outcome so `mlbb_heroes.log` can distinguish gate-drop from asset failure.
    let actor_locked = has_named_buff(sim, entity_id, VOICE_ACTOR_LOCK);
    if check_actor_gate && actor_locked {
        crate::wlog(&format!(
            "[VOICE drop] {line} entity={entity_id} reason=actor_lock"
        ));
        return false;
    }
    if let Some(remaining) = global_voice_remaining(sim) {
        crate::wlog(&format!(
            "[VOICE drop] {line} entity={entity_id} reason=global_lock remaining={remaining}"
        ));
        return false;
    }
    // 1.1.128: the OUTCOME is recorded here rather than in the `play_voice` wrapper, because
    // the data-file cue path (`VoiceCue::apply`) calls this function directly - in 1.1.127 a
    // *successful* skill line left no row at all, so "I can't hear him" could be neither proven
    // nor disproven from the log. Every request now ends in exactly one row.
    // 1.1.131: decided BEFORE the locks are armed. Arming them first (1.1.130) meant a repeat
    // request held the voice channel for a line that was never going to play, which kept the
    // re-queue loop in `VoiceCue::apply` alive and stacked copies of the same clip.
    if is_path_voice(line) {
        return play_julian_voice_path(sim, entity_id, line, check_actor_gate);
    }
    arm_voice_locks(sim, entity_id, line);
    let played = sim.play_sfx(line, entity_id, &InputTargetV1::target(entity_id));
    if played {
        crate::wlog(&format!("[VOICE play] {line} entity={entity_id} ok=true"));
    } else {
        // The engine refused the clip, so the locks armed above protect a playback that will
        // never happen. Releasing them lets the next request take the channel immediately
        // instead of waiting out a phantom duration (1.1.127 playtest: 582 refusals match-wide,
        // 66 of them Julian's = 25% of his engine-level attempts).
        let _ = sim.entity_remove_buff(entity_id, VOICE_ACTOR_LOCK);
        let _ = sim.entity_remove_buff(entity_id, VOICE_GLOBAL_LOCK);
        crate::wlog(&format!(
            "[VOICE refuse] {line} entity={entity_id} reason=engine_refused locks_released"
        ));
    }
    played
}

/// Low-priority routine lines are dropped while either voice gate is occupied.
pub(crate) fn play_voice(sim: &mut StableSim<'_>, entity_id: usize, line: &str) -> bool {
    // 1.1.128: logging moved inside `play_voice_with_actor_gate` so the cue path is covered too.
    play_voice_with_actor_gate(sim, entity_id, line, true)
}

fn voice_effect_suffix(line: &str) -> Option<&'static str> {
    VOICE_CUES
        .iter()
        .find(|(_, event, _, _)| *event == line)
        .map(|(suffix, _, _, _)| *suffix)
}

fn queue_voice_suffix(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    suffix: &str,
    delay: usize,
) -> bool {
    let effect_ref = format!("mlbb_heroes:audio_voice_{suffix}");
    sim.queue_effect(
        &effect_ref,
        AttackTypeV1::Skill,
        entity_id,
        &InputTargetV1::target(entity_id),
        delay.max(1),
    )
}

fn queue_priority_voice(sim: &mut StableSim<'_>, entity_id: usize, line: &str) -> bool {
    let Some(suffix) = voice_effect_suffix(line) else {
        return false;
    };
    let delay = global_voice_remaining(sim).unwrap_or(0).saturating_add(1);
    queue_voice_suffix(sim, entity_id, suffix, delay)
}

pub(crate) fn play_priority_voice(sim: &mut StableSim<'_>, entity_id: usize, line: &str) -> bool {
    if is_path_voice(line) {
        return play_voice_with_actor_gate(sim, entity_id, line, false);
    }
    if global_voice_remaining(sim).is_some() {
        return queue_priority_voice(sim, entity_id, line);
    }
    play_voice_with_actor_gate(sim, entity_id, line, false)
}

pub(crate) fn play_throttled_sfx(
    sim: &mut StableSim<'_>,
    entity_id: usize,
    line: &str,
    cue_id: usize,
    target: &InputTargetV1,
) -> bool {
    let lock_name = format!("mlbb_heroes_sfx_gate_{cue_id}");
    if has_named_buff(sim, entity_id, &lock_name) {
        return false;
    }
    sim.add_buff(
        entity_id,
        &BuffV1::timed(&lock_name, SFX_DUPLICATE_WINDOW_TICKS),
    );
    sim.play_sfx(line, entity_id, target)
}

#[derive(Clone, Copy)]
struct VoiceCue {
    suffix: &'static str,
    line: &'static str,
    priority: bool,
    bypass_actor_gate: bool,
}

/// Take the one allowed retry for this (champion, cue), or refuse if it is already spent. A buff is
/// the only durable per-entity state a cue can reach, so the marker lives there; the name embeds the
/// suffix so two heroes can never share a slot.
fn voice_retry_slot(sim: &mut StableSim<'_>, entity_id: usize, suffix: &str) -> bool {
    let name = format!("mlbb_heroes_voice_retry_{suffix}");
    if has_named_buff(sim, entity_id, &name) {
        return false;
    }
    sim.add_buff(entity_id, &BuffV1::timed(&name, VOICE_RETRY_MARK_TICKS));
    true
}

impl StableEffectType for VoiceCue {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, _input: InputTargetV1) {
        if is_path_voice(self.line) {
            let _ = play_voice_with_actor_gate(sim, caster_id, self.line, !self.bypass_actor_gate);
            return;
        }
        if self.priority {
            if let Some(remaining) = global_voice_remaining(sim) {
                // 1.1.133: bound the wait-and-retry. This branch used to reschedule itself with no
                // attempt counter at all, and the cost is measurable in the owner's own match:
                // `mlbb_julian_voice_ult01` appears 207 times across 885 s, where his ult cooldown
                // floor of 6 s caps the number of POSSIBLE casts at 147, at a median spacing of 161
                // ticks (2.68 s) - at least 60 of those lines belonged to no cast. One retry per
                // (champion, cue) per 10 s now, and the refusal is logged instead of silent.
                if voice_retry_slot(sim, caster_id, self.suffix) {
                    let _ = queue_voice_suffix(
                        sim,
                        caster_id,
                        self.suffix,
                        remaining.saturating_add(1),
                    );
                } else {
                    crate::wlog(&format!(
                        "[VOICE drop] {} entity={caster_id} reason=retry_cap {}",
                        self.line,
                        voice_stamp(sim)
                    ));
                }
            } else {
                // Urgent lines can supersede the per-actor cooldown, but never
                // overlap a clip already occupying the shared channel.
                let _ = play_voice_with_actor_gate(sim, caster_id, self.line, false);
            }
        } else {
            let _ = play_voice_with_actor_gate(sim, caster_id, self.line, !self.bypass_actor_gate);
        }
    }
}

/// Angela's two recorded S1 takes. Her data references `audio_voice_angela_skill`
/// from both S1 and S2 (she has no separate S2 line), so one random cue serves
/// both rather than pinning each skill to a fixed take.
const ANGELA_SKILL_LINES: &[&str] = &["mlbb_angela_voice_skill01", "mlbb_angela_voice_skill02"];

const ZHASK_ULT_LINES: &[&str] = &[
    "mlbb_zhask_voice_ult01",
    "mlbb_zhask_voice_ult02",
    "mlbb_zhask_voice_ult03",
];

const BALMOND_ULT_LINES: &[&str] = &["mlbb_balmond_voice_ult01", "mlbb_balmond_voice_ult02"];

/// Johnson's two ult lines are alternatives, not two halves of one line, so a
/// random pick is correct — chaining them would talk over himself. Same shape
/// as Zhask and Balmond.
const JOHNSON_ULT_LINES: &[&str] = &["mlbb_johnson_voice_ult01", "mlbb_johnson_voice_ult02"];

#[derive(Clone, Copy)]
struct RandomVoiceCue {
    lines: &'static [&'static str],
}

impl StableEffectType for RandomVoiceCue {
    fn apply(&self, sim: &mut StableSim<'_>, rng: u64, caster_id: usize, _input: InputTargetV1) {
        if self.lines.is_empty() {
            return;
        }
        // Mix the host-provided effect seed with match state so repeated casts
        // do not simply cycle through the same fixed line order.
        let mixed = rng ^ (caster_id as u64).rotate_left(19) ^ (sim.tick() as u64).rotate_left(41);
        let mut value = mixed.wrapping_add(0x9E3779B97F4A7C15);
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^= value >> 31;
        let line = self.lines[(value as usize) % self.lines.len()];
        let _ = play_priority_voice(sim, caster_id, line);
    }
}

#[derive(Clone, Copy)]
struct SoundCue {
    line: &'static str,
    cue_id: usize,
}

impl StableEffectType for SoundCue {
    fn apply(&self, sim: &mut StableSim<'_>, _seed: u64, caster_id: usize, input: InputTargetV1) {
        let _ = play_throttled_sfx(sim, caster_id, self.line, self.cue_id, &input);
    }
}

/// Register .data_champion wrappers so voice and custom action SFX pass through
/// the shared gates instead of firing directly on every trigger.
pub(crate) fn register_native_cues(module: &mut StableMod) {
    for (suffix, line, priority, bypass_actor_gate) in VOICE_CUES {
        let effect_ref = format!("mlbb_heroes:audio_voice_{suffix}");
        module.add_native_effect(
            &effect_ref,
            VoiceCue {
                suffix: *suffix,
                line: *line,
                priority: *priority,
                bypass_actor_gate: *bypass_actor_gate,
            },
        );
    }

    module.add_native_effect(
        "mlbb_heroes:audio_voice_angela_skill",
        RandomVoiceCue {
            lines: ANGELA_SKILL_LINES,
        },
    );

    module.add_native_effect(
        "mlbb_heroes:audio_voice_zhask_ult",
        RandomVoiceCue {
            lines: ZHASK_ULT_LINES,
        },
    );

    module.add_native_effect(
        "mlbb_heroes:audio_voice_balmond_ult",
        RandomVoiceCue {
            lines: BALMOND_ULT_LINES,
        },
    );

    module.add_native_effect(
        "mlbb_heroes:audio_voice_johnson_ult",
        RandomVoiceCue {
            lines: JOHNSON_ULT_LINES,
        },
    );

    const SOUNDS: &[(&str, &str, usize)] = &[
        ("fighter_attack", "fighter_attack", 12),
        ("fighter_skill", "fighter_skill", 13),
        ("aldous_attack", "mlbb_aldous_atk_hit", 1),
        ("aldous_skill1", "mlbb_aldous_s1_cast", 2),
        ("aldous_skill2", "mlbb_aldous_s2_cast", 3),
        ("aldous_ult", "mlbb_aldous_ult_cast", 4),
        ("gusion_attack", "mlbb_gusion_sfx_attack", 5),
        ("gusion_skill1_throw", "mlbb_gusion_sfx_s1_throw", 6),
        ("gusion_skill1_recast", "mlbb_gusion_sfx_s1_recast", 7),
        ("gusion_skill2_throw", "mlbb_gusion_sfx_s2_throw", 8),
        ("gusion_skill2_recall", "mlbb_gusion_sfx_s2_recall", 9),
        ("gusion_ult", "mlbb_gusion_sfx_ult", 10),
        ("gusion_ult_recast", "mlbb_gusion_sfx_ult_recast", 11),
    ];
    for (suffix, line, cue_id) in SOUNDS {
        let effect_ref = format!("mlbb_heroes:audio_sfx_{suffix}");
        module.add_native_effect(
            &effect_ref,
            SoundCue {
                line: *line,
                cue_id: *cue_id,
            },
        );
    }
}

#[derive(Clone, Default)]
pub(crate) struct VoiceLifecycle {
    last_pos: Option<(u64, u64)>,
    was_moving: bool,
    idle_ticks: usize,
    next_run_tick: usize,
    next_idle_tick: usize,
    run_cursor: usize,
    idle_cursor: usize,
}

impl VoiceLifecycle {
    pub(crate) fn on_spawn(
        &mut self,
        sim: &mut StableSim<'_>,
        entity_id: usize,
        spawn_line: Option<&str>,
    ) {
        self.last_pos = sim.get_entity(entity_id).map(|entity| entity.pos());
        self.was_moving = false;
        self.idle_ticks = 0;
        self.next_run_tick = sim.tick();
        self.next_idle_tick = sim.tick().saturating_add(IDLE_AFTER_TICKS);
        self.run_cursor = 0;
        self.idle_cursor = 0;
        if let Some(line) = spawn_line {
            let _ = play_voice(sim, entity_id, line);
        }
    }

    pub(crate) fn on_update(
        &mut self,
        sim: &mut StableSim<'_>,
        entity_id: usize,
        idle_lines: &[&str],
        run_lines: &[&str],
    ) {
        let Some((position, alive)) = sim
            .get_entity(entity_id)
            .map(|entity| (entity.pos(), entity.is_alive()))
        else {
            return;
        };
        if !alive {
            self.was_moving = false;
            self.last_pos = Some(position);
            return;
        }

        let now = sim.tick();
        let moving = self.last_pos.is_some_and(|previous| {
            let dx = i128::from(position.0) - i128::from(previous.0);
            let dy = i128::from(position.1) - i128::from(previous.1);
            let dx = dx.unsigned_abs();
            let dy = dy.unsigned_abs();
            dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)) > MOVE_THRESHOLD_SQ
        });

        if moving {
            self.idle_ticks = 0;
            if !run_lines.is_empty() && (!self.was_moving || now >= self.next_run_tick) {
                let line = run_lines[self.run_cursor % run_lines.len()];
                let _ = play_voice(sim, entity_id, line);
                self.run_cursor = self.run_cursor.wrapping_add(1);
                self.next_run_tick = now.saturating_add(RUN_REPEAT_TICKS);
            }
        } else {
            self.idle_ticks = self.idle_ticks.saturating_add(1);
            if !idle_lines.is_empty()
                && self.idle_ticks >= IDLE_AFTER_TICKS
                && now >= self.next_idle_tick
            {
                let line = idle_lines[self.idle_cursor % idle_lines.len()];
                let _ = play_voice(sim, entity_id, line);
                self.idle_cursor = self.idle_cursor.wrapping_add(1);
                self.next_idle_tick = now.saturating_add(IDLE_REPEAT_TICKS);
                self.idle_ticks = 0;
            }
        }

        self.was_moving = moving;
        self.last_pos = Some(position);
    }

    pub(crate) fn on_dead(&mut self, sim: &mut StableSim<'_>, player_id: usize, lines: &[&str]) {
        if lines.is_empty() {
            return;
        }
        let Some((entity_id, deaths)) = sim
            .get_player(player_id)
            .and_then(|player| Some((player.champion()?.id(), player.deaths())))
        else {
            return;
        };
        let line = lines[deaths % lines.len()];
        let _ = play_priority_voice(sim, entity_id, line);
    }

    pub(crate) fn on_dead_random(
        &mut self,
        sim: &mut StableSim<'_>,
        player_id: usize,
        lines: &[&str],
    ) {
        if lines.is_empty() {
            return;
        }
        let Some((entity_id, deaths)) = sim
            .get_player(player_id)
            .and_then(|player| Some((player.champion()?.id(), player.deaths())))
        else {
            return;
        };
        let seed = (sim.tick() as u64)
            ^ (entity_id as u64).rotate_left(19)
            ^ (deaths as u64).rotate_left(37)
            ^ (player_id as u64).rotate_left(7);
        let mut value = seed.wrapping_add(0x9E3779B97F4A7C15);
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^= value >> 31;
        let line = lines[(value as usize) % lines.len()];
        let _ = play_priority_voice(sim, entity_id, line);
    }
}

#[cfg(test)]
mod path_audio_tests {
    use super::*;

    fn scope(id: u64) -> crate::SimScope {
        crate::SimScope {
            kind: SimOriginKindV1::ClientMatchView.code(),
            match_id: id,
            ..Default::default()
        }
    }
    fn request(id: u64, tick: usize, entity: usize, line: &str) -> PathRequest {
        PathRequest {
            scope: scope(id),
            tick,
            entity,
            check_actor_gate: true,
            line: line.into(),
        }
    }
    fn prime(p: &mut PathPlayback) {
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 0);
        assert!(p.finish_frame().is_none());
    }

    #[test]
    fn paused_render_frames_cannot_start_or_save_a_line() {
        let mut p = PathPlayback::default();
        prime(&mut p);
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 1);
        assert!(p.offer(request(1, 1, 7, "mlbb_julian_voice_move01")));
        assert!(p.finish_frame().is_some());
        for _ in 0..600 {
            p.begin_frame(true, 16_667);
            p.observe(scope(1), 1); // rendering and wall time continue, the simulation does not
            assert!(!p.offer(request(1, 1, 7, "mlbb_julian_voice_move03")));
            assert!(p.finish_frame().is_none());
        }
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 2);
        assert!(
            p.finish_frame().is_none(),
            "resume cannot drain a paused backlog"
        );
    }

    #[test]
    fn precomputed_match_and_seek_requests_are_discarded() {
        let mut p = PathPlayback::default();
        prime(&mut p);
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 1);
        assert!(p.offer(request(1, 1, 7, "mlbb_julian_voice_skill01")));
        p.observe(scope(1), 20_000);
        assert!(
            p.finish_frame().is_none(),
            "catch-up invalidates even an earlier accepted line"
        );
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 3);
        assert!(!p.offer(request(1, 3, 7, "mlbb_julian_voice_skill01")));
        assert!(
            p.finish_frame().is_none(),
            "rewind must not speak old events again"
        );
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 20_001);
        assert!(p.offer(request(1, 20_001, 7, "mlbb_julian_voice_skill02")));
        assert!(
            p.finish_frame().is_some(),
            "fresh progress after a seek must recover"
        );
    }

    #[test]
    fn hidden_scene_and_match_change_cannot_replay_a_pending_line() {
        let mut p = PathPlayback::default();
        prime(&mut p);
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 1);
        assert!(p.offer(request(1, 1, 7, "mlbb_julian_voice_spawn")));
        p.begin_frame(false, 16_667);
        p.observe(scope(1), 500);
        assert!(!p.offer(request(1, 500, 7, "mlbb_julian_voice_spawn")));
        assert!(p.finish_frame().is_none());
        p.begin_frame(true, 16_667);
        p.observe(scope(2), 0);
        assert!(!p.offer(request(1, 501, 7, "mlbb_julian_voice_spawn")));
        assert!(p.finish_frame().is_none());
        p.begin_frame(true, 16_667);
        p.observe(scope(2), 1);
        assert!(p.offer(request(2, 1, 7, "mlbb_julian_voice_skill01")));
        assert!(p.finish_frame().is_some());
    }

    #[test]
    fn full_clip_and_actor_cooldown_prevent_rotation_overlap_at_high_sim_speed() {
        let mut p = PathPlayback::default();
        prime(&mut p);
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 1);
        assert!(p.offer(request(1, 1, 7, "mlbb_julian_voice_move10")));
        let line = p.finish_frame().unwrap();
        p.played(&line);
        for tick in 2..100 {
            p.begin_frame(true, 16_667);
            p.observe(scope(1), tick);
            assert!(!p.offer(request(1, tick, 7, "mlbb_julian_voice_skill01")));
            assert!(
                !p.offer(request(1, tick, 8, "mlbb_julian_voice_ult01")),
                "global channel holds the full clip"
            );
            assert!(p.finish_frame().is_none());
        }
        p.begin_frame(true, 4_000_000);
        p.observe(scope(1), 100);
        assert!(p.offer(request(1, 100, 7, "mlbb_julian_voice_skill01")));
        assert!(p.finish_frame().is_some());
    }

    #[test]
    fn frame_queue_never_bursts_or_carries_a_request_forward() {
        let mut p = PathPlayback::default();
        prime(&mut p);
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 1);
        assert!(p.offer(request(1, 1, 7, "mlbb_julian_voice_skill01")));
        assert!(!p.offer(request(1, 1, 7, "mlbb_julian_voice_skill02")));
        assert!(p.finish_frame().is_some());
        assert!(p.finish_frame().is_none());
        p.begin_frame(true, 16_667);
        p.observe(scope(1), 2);
        assert!(p.finish_frame().is_none());
    }

    #[test]
    fn only_positive_viewer_origins_can_feed_client_audio() {
        for kind in [
            SimOriginKindV1::ServerPresim,
            SimOriginKindV1::Tool,
            SimOriginKindV1::Unknown,
        ] {
            assert!(!viewer_origin(Some(kind)));
        }
        assert!(!viewer_origin(None));
        for kind in [
            SimOriginKindV1::ClientMatchView,
            SimOriginKindV1::ClientSpectate,
            SimOriginKindV1::ClientReplay,
        ] {
            assert!(viewer_origin(Some(kind)));
        }
    }
    #[test]
    fn client_hooks_and_sim_callbacks_do_not_play_during_pause_or_management() {
        use std::{ffi::c_void, mem::size_of};
        struct Host {
            screen: u32,
            origin: u32,
            tick: usize,
            played: Vec<(String, f32)>,
        }
        unsafe extern "C" fn scene(_: *const c_void) -> u32 {
            SceneKindV1::InGame.code()
        }
        unsafe extern "C" fn screen(p: *const c_void) -> u32 {
            (*(p as *const Host)).screen
        }
        unsafe extern "C" fn tick(p: *const c_void) -> usize {
            (*(p as *const Host)).tick
        }
        unsafe extern "C" fn origin(p: *const c_void, out: *mut SimOriginV1) -> bool {
            *out = SimOriginV1 {
                kind: (*(p as *const Host)).origin,
                match_id: 834,
                ..Default::default()
            };
            true
        }
        unsafe extern "C" fn play(
            p: *mut c_void,
            path: *const u8,
            len: usize,
            volume: f32,
        ) -> bool {
            (*(p as *mut Host)).played.push((
                String::from_utf8_lossy(std::slice::from_raw_parts(path, len)).into(),
                volume,
            ));
            true
        }
        let mut host = Host {
            screen: ClientSceneKindV1::Main.code(),
            origin: SimOriginKindV1::ClientMatchView.code(),
            tick: 0,
            played: vec![],
        };
        let mut scene_table: SceneVtableV1 = unsafe { std::mem::zeroed() };
        scene_table.size = size_of::<SceneVtableV1>();
        scene_table.kind = Some(scene);
        let mut data: DataVtableV1 = unsafe { std::mem::zeroed() };
        data.size = size_of::<DataVtableV1>();
        data.client_scene_kind = Some(screen);
        let mut asset: AssetVtableV1 = unsafe { std::mem::zeroed() };
        asset.size = size_of::<AssetVtableV1>();
        asset.play_sound = Some(play);
        let mut sim_table: SimVtableV1 = unsafe { std::mem::zeroed() };
        sim_table.size = size_of::<SimVtableV1>();
        sim_table.tick = Some(tick);
        sim_table.sim_origin = Some(origin);
        let state = &mut host as *mut Host as *mut c_void;
        let mut client_raw: ClientCtxV1 = unsafe { std::mem::zeroed() };
        client_raw.size = size_of::<ClientCtxV1>();
        client_raw.state = state;
        client_raw.scene = &scene_table;
        client_raw.data = &data;
        client_raw.asset = &asset;
        let mut sim_raw = SimCtxV1 {
            size: size_of::<SimCtxV1>(),
            sim: &sim_table,
            frame: std::ptr::null(),
            state,
        };
        let extension = VoicePathExtension;
        extension.on_end();
        let mut frame = |h: &mut Host, screen_code: u32, origin_code: u32, t: usize| {
            h.screen = screen_code;
            h.origin = origin_code;
            h.tick = t;
            let mut ctx = unsafe { StableClient::from_raw(&mut client_raw).unwrap() };
            extension.pre_update(&mut ctx, 16_667);
            let mut sim = unsafe { StableSim::from_raw(&mut sim_raw).unwrap() };
            observe_voice_tick(&sim);
            VoiceCue {
                suffix: "julian_ult01",
                line: "mlbb_julian_voice_ult01",
                priority: true,
                bypass_actor_gate: true,
            }
            .apply(&mut sim, 0, 7, InputTargetV1::target(7));
            extension.post_update(&mut ctx, 16_667);
        };
        let view = SimOriginKindV1::ClientMatchView.code();
        let live = ClientSceneKindV1::InGame.code();
        frame(&mut host, ClientSceneKindV1::Main.code(), view, 500);
        assert!(
            host.played.is_empty(),
            "broad InGame scene is NOT sufficient"
        );
        frame(&mut host, live, view, 0);
        assert!(
            host.played.is_empty(),
            "initialization must not dump spawn audio"
        );
        frame(&mut host, live, view, 1);
        assert_eq!(
            host.played,
            vec![(
                "asset/mlbb_heroes/sound/sfx/mlbb_julian_voice_ult01".into(),
                0.68
            )]
        );
        for _ in 0..600 {
            frame(&mut host, live, view, 1);
        }
        assert_eq!(
            host.played.len(),
            1,
            "ten seconds of paused rendering cannot release lines"
        );
        frame(
            &mut host,
            live,
            SimOriginKindV1::ServerPresim.code(),
            100_000,
        );
        frame(&mut host, live, SimOriginKindV1::Unknown.code(), 100_000);
        assert_eq!(
            host.played.len(),
            1,
            "headless and unknown sims cannot bypass the client gate"
        );
        frame(&mut host, live, view, 2);
        assert_eq!(
            host.played.len(),
            2,
            "a fresh cast on resume must still be audible"
        );
        extension.on_end();
    }
}

#[cfg(test)]
mod popol_path_audio_tests {
    use super::*;
    #[test]
    fn popol_and_julian_use_the_same_frame_gated_path() {
        assert!(is_path_voice("mlbb_popol_voice_intro"));
        assert!(is_path_voice("mlbb_julian_voice_spawn"));
        assert!(!is_path_voice("mlbb_eudora_voice_select"));
        assert_eq!(voice_lock_ticks("mlbb_popol_voice_intro"), 218);
        assert!(VOICE_CUES
            .iter()
            .any(|(_, name, _, _)| *name == "mlbb_popol_voice_intro"));
    }
    #[test]
    fn popol_intro_has_no_paused_or_presimulation_backlog() {
        let scope = crate::SimScope {
            kind: SimOriginKindV1::ClientMatchView.code(),
            match_id: 991,
            ..Default::default()
        };
        let req = |tick| PathRequest {
            scope,
            tick,
            entity: 1,
            check_actor_gate: true,
            line: "mlbb_popol_voice_intro".into(),
        };
        let mut p = PathPlayback::default();
        p.begin_frame(true, 16667);
        p.observe(scope, 60);
        assert!(p.finish_frame().is_none());
        p.begin_frame(true, 16667);
        p.observe(scope, 60);
        assert!(!p.offer(req(60)));
        assert!(p.finish_frame().is_none());
        p.begin_frame(true, 16667);
        p.observe(scope, 61);
        assert!(p.offer(req(61)));
        let r = p.finish_frame().unwrap();
        p.played(&r);
        p.begin_frame(true, 16667);
        p.observe(scope, 62);
        assert!(!p.offer(req(62)));
        assert!(p.finish_frame().is_none());
        assert!(!viewer_origin(Some(SimOriginKindV1::ServerPresim)));
    }
}

#[cfg(test)]
mod granger_path_audio_tests {
    use super::*;
    fn scope() -> crate::SimScope {
        crate::SimScope {kind:SimOriginKindV1::ClientMatchView.code(),match_id:147,..Default::default()}
    }
    fn request(tick:usize,entity:usize,line:&str)->PathRequest {
        PathRequest {scope:scope(),tick,entity,check_actor_gate:true,line:line.into()}
    }
    #[test]
    fn every_granger_event_uses_guarded_path_and_a_measured_lock() {
        let cues:Vec<_>=VOICE_CUES.iter().filter(|(_,n,_,_)|n.starts_with("mlbb_granger_voice_")).collect();
        assert_eq!(cues.len(),19);
        for (_,name,_,_) in cues {
            assert!(is_path_voice(name));assert!((60..=400).contains(&voice_lock_ticks(name)));
        }
        assert_eq!(voice_lock_ticks("mlbb_granger_voice_move02"),328);
        assert_eq!(voice_lock_ticks("mlbb_granger_voice_skill01"),77);
        assert_eq!(voice_lock_ticks("mlbb_granger_voice_death01"),147);
    }
    #[test]
    fn granger_pause_seek_and_hidden_scene_cannot_accumulate_audio() {
        let mut p=PathPlayback::default();
        p.begin_frame(true,16667);p.observe(scope(),60);assert!(p.finish_frame().is_none());
        for _ in 0..300 {
            p.begin_frame(true,16667);p.observe(scope(),60);
            assert!(!p.offer(request(60,1,"mlbb_granger_voice_select")));assert!(p.finish_frame().is_none());
        }
        p.begin_frame(true,16667);p.observe(scope(),61);
        assert!(p.offer(request(61,1,"mlbb_granger_voice_skill01")));assert!(p.finish_frame().is_some());
        p.begin_frame(true,16667);p.observe(scope(),20000);
        assert!(!p.offer(request(20000,1,"mlbb_granger_voice_ult01")));assert!(p.finish_frame().is_none());
        p.begin_frame(false,16667);p.observe(scope(),20001);
        assert!(!p.offer(request(20001,1,"mlbb_granger_voice_death01")));assert!(p.finish_frame().is_none());
        p.begin_frame(true,16667);p.observe(scope(),20001);assert!(p.finish_frame().is_none());
    }
    #[test]
    fn granger_and_julian_share_one_measured_voice_channel() {
        let mut p=PathPlayback::default();
        p.begin_frame(true,16667);p.observe(scope(),0);p.finish_frame();
        p.begin_frame(true,16667);p.observe(scope(),1);
        assert!(p.offer(request(1,1,"mlbb_granger_voice_move02")));
        let r=p.finish_frame().unwrap();p.played(&r);
        p.begin_frame(true,16667);p.observe(scope(),2);
        assert!(!p.offer(request(2,2,"mlbb_julian_voice_skill01")));
        assert!(!p.offer(request(2,1,"mlbb_granger_voice_ult01")));assert!(p.finish_frame().is_none());
        p.begin_frame(true,6_000_000);p.observe(scope(),3);
        assert!(p.offer(request(3,2,"mlbb_julian_voice_skill01")));assert!(p.finish_frame().is_some());
    }
}
