//! Card-only preview in starting selection and draft. Gameplay carriers stay transparent.
use mod_api_stable::*;
use std::sync::{LazyLock, Mutex};
#[derive(Clone, Debug, PartialEq, Eq)]
struct Card {
    path: String,
    identity: String,
}
#[derive(Default)]
struct Cache {
    scene: Option<(SceneKindV1, Option<ClientSceneKindV1>)>,
    elapsed: u64,
    scanned: bool,
    cards: Vec<Card>,
    draw_logged: bool,
}
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| Mutex::new(Cache::default()));
fn is_name(text: &str) -> bool {
    text.contains("mlbb_popol_kupa") || (text.contains("Popol") && text.contains("Kupa"))
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
fn identity(ui: &impl ReadUi, path: &str) -> bool {
    // Template item IDs can identify a card even if its caption is custom-drawn.
    if path
        .rsplit('.')
        .next()
        .is_some_and(|id| id == "mlbb_popol_kupa")
    {
        return true;
    }
    match ui.kind(path).as_deref() {
        Some("label") => ui.label(path).is_some_and(|s| is_name(&s)),
        Some("selectable" | "color_selectable" | "button") => {
            ui.state(path).is_some_and(|s| is_name(&s))
        }
        _ => false,
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
        if identity(ui, &path) {
            // Selectable cards own their caption. Labels instead identify an ancestor.
            let mut parent = path.as_str();
            for _ in 0..5 {
                if ui.rect(parent).and_then(image_rect).is_some() {
                    if !result.iter().any(|c| c.path == parent) {
                        result.push(Card {
                            path: parent.to_string(),
                            identity: path.clone(),
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
    visible(ui, &card.path) && identity(ui, &card.identity)
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
        (SceneKindV1::NewGame, _)
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
    let started = cache.elapsed;
    cache.elapsed = cache.elapsed.saturating_add(dt.min(250_000));
    let ready = started < 500_000 && cache.elapsed >= 500_000;
    // F8 requests one new search after navigation or filtering inside the same screen.
    if (!cache.scanned && ready) || ctx.key_pressed("F8") {
        cache.cards = scan(ctx);
        cache.scanned = true;
        cache.draw_logged = false;
        crate::wlog(format!(
            "[POPOL preview] scene={scene:?} cards={:?}",
            cache.cards
        ));
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
    let frame = (cache.elapsed.saturating_mul(15) / 2_000_000) % 9;
    let mut submitted = false;
    for card in &cache.cards {
        if !live_card(ctx, card) {
            continue;
        }
        let Some((x, y, w, h)) = ctx.ui_node_rect(&card.path).and_then(image_rect) else {
            continue;
        };
        let height = w * 72. / 120.;
        let rect = (x, y + (h - height) / 2., w, height);
        if clipped(ctx, &card.path, rect) {
            continue;
        }
        let asset = format!("asset/mlbb_heroes/icons/popol_menu_idle_{frame}");
        submitted = true;
        let ok = ctx.draw_svg("UI", &asset, rect, 100, 0xffffffff);
        if !cache.draw_logged {
            crate::wlog(format!(
                "[POPOL preview] draw path={} rect={rect:?} submitted={ok}",
                card.path
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
                identity: "draft.pool.item_14".into()
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
