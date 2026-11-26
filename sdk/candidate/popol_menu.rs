//! Card-only preview in starting selection and draft. Gameplay carriers stay transparent.
use crate::draft_preview::Hero;
use mod_api_stable::*;
use std::sync::{LazyLock, Mutex};

const FIRST_SCAN: u64 = 500_000;
const SCAN_INTERVAL: u64 = 1_500_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Card {
    path: String,
    identity: String,
    hero: Hero,
}
#[derive(Default)]
struct Cache {
    scene: Option<(SceneKindV1, Option<ClientSceneKindV1>)>,
    elapsed: u64,
    next_scan: u64,
    cards: Vec<Card>,
    draw_logged: bool,
}
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));
fn card_hero(text: &str) -> Option<Hero> {
    if text.contains("mlbb_popol_kupa") || (text.contains("Popol") && text.contains("Kupa")) {
        Some(Hero::Popol)
    } else if text.contains("mlbb_xborg") || text.trim() == "X.Borg" || text.contains(r#""X.Borg""#)
    {
        Some(Hero::Xborg)
    } else {
        None
    }
}
fn child(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}.{name}")
    }
}
trait ReadUi {
    fn children(&self, path: &str) -> Vec<String>;
    fn kind(&self, path: &str) -> Option<String>;
    fn label(&self, path: &str) -> Option<String>;
    fn state(&self, path: &str) -> Option<String>;
    fn rect(&self, path: &str) -> Option<(f32, f32, f32, f32)>;
    fn shown(&self, path: &str) -> Option<bool>;
}
impl ReadUi for StableClient<'_> {
    fn children(&self, p: &str) -> Vec<String> {
        self.ui_child_names(p)
    }
    fn kind(&self, p: &str) -> Option<String> {
        self.ui_runner_name(p)
    }
    fn label(&self, p: &str) -> Option<String> {
        self.ui_text(p)
    }
    fn state(&self, p: &str) -> Option<String> {
        self.ui_state_json(p)
    }
    fn rect(&self, p: &str) -> Option<(f32, f32, f32, f32)> {
        self.ui_node_rect(p)
    }
    fn shown(&self, p: &str) -> Option<bool> {
        self.ui_visible(p)
    }
}
fn identity(ui: &impl ReadUi, path: &str) -> Option<Hero> {
    // Template item IDs can identify a card even if its caption is custom-drawn.
    if let Some(id) = path.rsplit('.').next() {
        match id.strip_prefix("champion_").unwrap_or(id) {
            "mlbb_popol_kupa" => return Some(Hero::Popol),
            "mlbb_xborg" => return Some(Hero::Xborg),
            _ => {}
        }
    }
    match ui.kind(path).as_deref() {
        Some("label") => ui.label(path).as_deref().and_then(card_hero),
        Some("selectable" | "color_selectable" | "button") => {
            ui.state(path).as_deref().and_then(card_hero)
        }
        _ => None,
    }
}
fn visible(ui: &impl ReadUi, path: &str) -> bool {
    let mut p = path;
    loop {
        if ui.shown(p) == Some(false) {
            return false;
        }
        match p.rsplit_once('.') {
            Some((parent, _)) => p = parent,
            None => break,
        }
    }
    true
}
fn image_rect(card: (f32, f32, f32, f32)) -> Option<(f32, f32, f32, f32)> {
    let (x, y, w, h) = card;
    if !(90. ..=320.).contains(&w) || !(100. ..=360.).contains(&h) {
        return None;
    }
    let side = (w - 24.).min(h - 36.).min(96.);
    Some((
        x + (w - side) / 2.,
        y + 4. + (h - 32. - side) / 2.,
        side,
        side,
    ))
}
fn scan(ui: &impl ReadUi) -> Vec<Card> {
    let mut pending = vec![(String::new(), 0usize)];
    let mut visited = 0;
    let mut result: Vec<Card> = Vec::new();
    while let Some((path, depth)) = pending.pop() {
        visited += 1;
        if visited > 8192 {
            break;
        }
        // An invisible branch cannot provide a visible card.
        if !path.is_empty() && ui.shown(&path) == Some(false) {
            continue;
        }
        if let Some(hero) = identity(ui, &path) {
            // Selectable cards own their caption. Labels instead identify an ancestor.
            let mut parent = path.as_str();
            for _ in 0..5 {
                if ui.rect(parent).and_then(image_rect).is_some() {
                    if !result.iter().any(|c| c.path == parent) {
                        result.push(Card {
                            path: parent.to_string(),
                            identity: path.clone(),
                            hero,
                        });
                    }
                    break;
                }
                match parent.rsplit_once('.') {
                    Some((p, _)) => parent = p,
                    None => break,
                }
            }
        }
        if depth < 32 {
            for name in ui.children(&path) {
                pending.push((child(&path, &name), depth + 1));
            }
        }
    }
    result
}
fn live_card(ui: &impl ReadUi, card: &Card) -> bool {
    visible(ui, &card.path) && identity(ui, &card.identity) == Some(card.hero)
}
fn clipped(ui: &impl ReadUi, path: &str, rect: (f32, f32, f32, f32)) -> bool {
    let mut p = path;
    while let Some((parent, _)) = p.rsplit_once('.') {
        p = parent;
        if ui.kind(p).as_deref() == Some("scroll_view") {
            if let Some((x, y, w, h)) = ui.rect(p) {
                if rect.0 < x || rect.1 < y || rect.0 + rect.2 > x + w || rect.1 + rect.3 > y + h {
                    return true;
                }
            }
        }
    }
    false
}
fn eligible(ctx: &StableClient<'_>) -> Option<(SceneKindV1, Option<ClientSceneKindV1>)> {
    let scene = ctx.scene_kind()?;
    let screen = ctx.client_scene_kind();
    match (scene, screen) {
        (SceneKindV1::NewGame | SceneKindV1::DatabaseEdit, _)
        | (
            SceneKindV1::InGame,
            Some(ClientSceneKindV1::Lineup | ClientSceneKindV1::StadiumEntrance),
        ) => Some((scene, screen)),
        _ => None,
    }
}

pub(crate) fn update(ctx: &StableClient<'_>, dt: u64) {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let scene = eligible(ctx);
    if cache.scene != scene {
        *cache = Cache {
            scene,
            ..Cache::default()
        };
    }
    if scene.is_none() {
        return;
    }
    cache.elapsed = cache.elapsed.saturating_add(dt.min(250_000));
    let probe = ctx.key_pressed("F8");
    if cache.elapsed >= cache.next_scan.max(FIRST_SCAN) || probe {
        let mut cards = scan(ctx);
        if scene.is_some_and(|(kind, _)| kind == SceneKindV1::DatabaseEdit) {
            cards.retain(|card| card.hero == Hero::Xborg);
        }
        if cache.next_scan == 0 || cache.cards != cards || probe {
            cache.draw_logged = false;
            crate::wlog(format!(
                "[champion preview] scene={scene:?} cards={cards:?}"
            ));
        }
        cache.cards = cards;
        cache.next_scan = cache.elapsed + SCAN_INTERVAL;
    }
}

pub(crate) fn render(ctx: &mut StableClient<'_>) {
    let Some(scene) = eligible(ctx) else { return };
    if !ctx.can_draw() {
        return;
    }
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.scene != Some(scene) {
        return;
    }
    let mut submitted = false;
    for card in &cache.cards {
        if !live_card(ctx, card) {
            continue;
        }
        let Some((x, y, w, h)) = ctx.ui_node_rect(&card.path).and_then(image_rect) else {
            continue;
        };
        let (asset, aspect) = card.hero.art(cache.elapsed);
        let height = w / aspect;
        let rect = (x, y + (h - height) / 2., w, height);
        if clipped(ctx, &card.path, rect) {
            continue;
        }
        submitted = true;
        let ok = ctx.draw_svg("UI", &asset, rect, 100, 0xffffffff);
        if !cache.draw_logged {
            crate::wlog(format!(
                "[champion preview] draw hero={:?} path={} rect={rect:?} submitted={ok}",
                card.hero, card.path
            ));
        }
    }
    cache.draw_logged |= submitted;
}

pub(crate) fn clear() {
    *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Cache::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[derive(Default)]
    struct Node {
        kind: &'static str,
        text: Option<&'static str>,
        state: Option<&'static str>,
        rect: Option<(f32, f32, f32, f32)>,
        hidden: bool,
    }
    #[derive(Default)]
    struct Ui(HashMap<String, Node>);
    impl ReadUi for Ui {
        fn children(&self, p: &str) -> Vec<String> {
            self.0
                .keys()
                .filter_map(|k| {
                    let (parent, last) = k.rsplit_once('.').unwrap_or(("", k));
                    (parent == p).then(|| last.to_string())
                })
                .collect()
        }
        fn kind(&self, p: &str) -> Option<String> {
            self.0.get(p).map(|n| n.kind.to_string())
        }
        fn label(&self, p: &str) -> Option<String> {
            self.0.get(p).and_then(|n| n.text.map(str::to_string))
        }
        fn state(&self, p: &str) -> Option<String> {
            self.0.get(p).and_then(|n| n.state.map(str::to_string))
        }
        fn rect(&self, p: &str) -> Option<(f32, f32, f32, f32)> {
            self.0.get(p).and_then(|n| n.rect)
        }
        fn shown(&self, p: &str) -> Option<bool> {
            self.0.get(p).map(|n| !n.hidden)
        }
    }
    fn fixture() -> Ui {
        let mut ui = Ui::default();
        ui.0.insert("draft".into(), Node::default());
        ui.0.insert("draft.pool".into(), Node::default());
        ui.0.insert(
            "draft.pool.item_14".into(),
            Node {
                kind: "selectable",
                state: Some(r#"{"selected":false,"text":"Popol & Kupa [ANIM TEST]"}"#),
                rect: Some((320., 540., 134., 130.)),
                ..Default::default()
            },
        );
        ui
    }
    #[test]
    fn selectable_caption_without_label_children_is_recognized_in_draft() {
        let ui = fixture();
        let cards = scan(&ui);
        assert_eq!(
            cards,
            vec![Card {
                path: "draft.pool.item_14".into(),
                identity: "draft.pool.item_14".into(),
                hero: Hero::Popol,
            }]
        );
    }
    #[test]
    fn starting_champion_label_resolves_to_its_own_card() {
        let mut ui = fixture();
        ui.0.get_mut("draft.pool.item_14").unwrap().state = None;
        ui.0.insert(
            "draft.pool.item_14.name".into(),
            Node {
                kind: "label",
                text: Some("Popol & Kupa [ANIM TEST]"),
                rect: Some((320., 680., 150., 20.)),
                ..Default::default()
            },
        );
        let cards = scan(&ui);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].path, "draft.pool.item_14");
    }
    #[test]
    fn recycled_card_path_does_not_draw_popol_over_another_hero() {
        let mut ui = fixture();
        let card = scan(&ui).pop().unwrap();
        ui.0.get_mut(&card.path).unwrap().state = Some(r#"{"text":"Hanabi"}"#);
        assert!(!live_card(&ui, &card));
    }
    #[test]
    fn hidden_pool_is_skipped() {
        let mut ui = fixture();
        ui.0.get_mut("draft.pool").unwrap().hidden = true;
        assert!(scan(&ui).is_empty());
    }
    #[test]
    fn search_text_is_not_a_champion_card() {
        let mut ui = Ui::default();
        ui.0.insert(
            "search".into(),
            Node {
                kind: "text_edit",
                state: Some(r#"{"text":"Popol & Kupa"}"#),
                rect: Some((0., 0., 160., 130.)),
                ..Default::default()
            },
        );
        assert!(scan(&ui).is_empty());
    }
    #[test]
    fn xborg_caption_resolves_to_matching_idle_art_and_rejects_reused_cards() {
        let mut ui = fixture();
        let path = "draft.pool.item_15";
        ui.0.insert(
            path.into(),
            Node {
                kind: "color_selectable",
                state: Some(r#"{"text":"X.Borg"}"#),
                rect: Some((460., 540., 134., 130.)),
                ..Default::default()
            },
        );
        let cards = scan(&ui);
        let card = cards.iter().find(|card| card.hero == Hero::Xborg).unwrap();
        assert_eq!(card.path, path);
        assert_eq!(
            card.hero.art(300_000).0,
            "asset/mlbb_heroes/icons/xborg_draft_idle_2"
        );
        assert!(live_card(&ui, card));
        ui.0.get_mut(path).unwrap().state = Some(r#"{"text":"Popol & Kupa"}"#);
        assert!(!live_card(&ui, card));
    }
    #[test]
    fn template_item_id_works_with_a_custom_drawn_caption() {
        let mut ui = Ui::default();
        ui.0.insert(
            "mlbb_popol_kupa".into(),
            Node {
                kind: "",
                rect: Some((0., 0., 160., 180.)),
                ..Default::default()
            },
        );
        assert_eq!(scan(&ui).len(), 1);
    }
    #[test]
    fn small_draft_card_keeps_art_above_caption() {
        let (x, y, w, h) = image_rect((320., 540., 134., 130.)).unwrap();
        assert!(x >= 320. && x + w <= 454. && y >= 540. && y + h <= 650.);
    }
    #[test]
    fn large_panels_are_not_cards() {
        assert!(image_rect((0., 0., 1920., 1080.)).is_none());
    }
}
