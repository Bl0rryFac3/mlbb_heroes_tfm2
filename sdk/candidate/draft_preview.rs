//! Idle preview in the live ban/pick champion grid.
use mod_api_stable::{ClientSceneKindV1, SceneKindV1, StableClient};
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

type Rect = (f32, f32, f32, f32);
const GRID: &str = "main.champions";
const CONTENTS: &str = "main.champions.contents";
const DRAFT_STEP: &str = "main.header.step";
const SCAN_INTERVAL: u64 = 250_000;
const NODE_LIMIT: usize = 640;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hero {
    Popol,
    Granger,
    Hanzo,
    Atlas,
    YuZhong,
}

impl Hero {
    fn from_name(s: &str) -> Option<Self> {
        match s.trim() {
            "Popol & Kupa" | "mlbb_popol_kupa" => Some(Self::Popol),
            "Granger" | "mlbb_granger" => Some(Self::Granger),
            "Hanzo" | "mlbb_hanzo" => Some(Self::Hanzo),
            "Atlas" | "mlbb_atlas" => Some(Self::Atlas),
            "Yu Zhong" | "mlbb_yu_zhong" => Some(Self::YuZhong),
            _ => None,
        }
    }

    fn art(self, elapsed_us: u64) -> (String, f32) {
        let elapsed_us = elapsed_us % 1_200_000;
        let frame = elapsed_us * 9 / 1_200_000;
        let (name, frame, aspect) = match self {
            Self::Popol => ("popol_menu_idle", frame, 120. / 72.),
            Self::Granger => ("granger_draft_idle", frame, 72. / 72.),
            Self::Hanzo => ("hanzo_draft_idle", elapsed_us / 240_000, 96. / 72.),
            Self::Atlas => ("atlas_draft_idle", frame, 96. / 88.),
            Self::YuZhong => ("yu_zhong_draft_idle", frame, 1.),
        };
        (format!("asset/mlbb_heroes/icons/{name}_{frame}"), aspect)
    }
}

trait Ui {
    fn children(&self, path: &str) -> Vec<String>;
    fn visible(&self, path: &str) -> bool;
    fn label(&self, path: &str) -> Option<String>;
    fn state(&self, path: &str) -> Option<String>;
    fn rect(&self, path: &str) -> Option<Rect>;
}

impl Ui for StableClient<'_> {
    fn children(&self, path: &str) -> Vec<String> {
        self.ui_child_names(path)
    }
    fn visible(&self, path: &str) -> bool {
        self.ui_visible(path) == Some(true)
    }
    fn label(&self, path: &str) -> Option<String> {
        self.ui_text(path)
    }
    fn state(&self, path: &str) -> Option<String> {
        self.ui_state_json(path)
    }
    fn rect(&self, path: &str) -> Option<Rect> {
        self.ui_node_rect(path)
    }
}

fn is_match(ctx: &StableClient<'_>) -> bool {
    ctx.scene_kind() == Some(SceneKindV1::InGame)
        && ctx.client_scene_kind() == Some(ClientSceneKindV1::Match)
}

fn active(ui: &impl Ui) -> bool {
    ui.visible("main") && ui.visible(DRAFT_STEP) && ui.visible(GRID) && ui.visible(CONTENTS)
}

fn node_id(path: &str) -> &str {
    path.rsplit('.').next().unwrap_or(path)
}

fn hero_at(ui: &impl Ui, path: &str) -> Option<Hero> {
    let id = node_id(path);
    Hero::from_name(id)
        .or_else(|| id.strip_prefix("champion_").and_then(Hero::from_name))
        .or_else(|| ui.label(path).as_deref().and_then(Hero::from_name))
        .or_else(|| {
            // The selectable runner exposes its caption in a flat JSON state object.
            let state = ui.state(path)?;
            let (_, text) = state.split_once("\"text\"")?;
            let text = text
                .trim_start()
                .strip_prefix(':')?
                .trim_start()
                .strip_prefix('"')?;
            Hero::from_name(text.split_once('"')?.0)
        })
}

fn valid_tile((x, y, w, h): Rect) -> bool {
    x.is_finite()
        && y.is_finite()
        && (76. ..=256.).contains(&w)
        && (75. ..=240.).contains(&h)
        && (0.65..=1.7).contains(&(w / h))
}

fn contains((x, y, w, h): Rect, (mx, my): (f32, f32)) -> bool {
    mx >= x && my >= y && mx < x + w && my < y + h
}

fn encloses(outer: Rect, inner: Rect) -> bool {
    let (x, y, w, h) = inner;
    x >= outer.0 && y >= outer.1 && x + w <= outer.0 + outer.2 && y + h <= outer.1 + outer.3
}

#[derive(Debug, PartialEq, Eq)]
struct Tile {
    card: String,
    identity: String,
    hero: Hero,
}

fn scan(ui: &impl Ui, cursor: (f32, f32)) -> Option<Tile> {
    let mut pending = VecDeque::from([(CONTENTS.to_owned(), 0, None::<String>)]);
    let mut examined = 0;
    while let Some((node, depth, inherited_card)) = pending.pop_front() {
        examined += 1;
        let rect = (node != CONTENTS)
            .then(|| ui.rect(&node))
            .flatten()
            .filter(|r| valid_tile(*r));
        let card = if let Some(rect) = rect {
            if !contains(rect, cursor) {
                continue;
            }
            Some(node.clone())
        } else {
            inherited_card
        };
        if !ui.visible(&node) {
            continue;
        }
        if let Some((card, hero)) = card.as_ref().zip(hero_at(ui, &node)) {
            return Some(Tile {
                card: card.clone(),
                identity: node,
                hero,
            });
        }
        if depth < 4 {
            let remaining = NODE_LIMIT.saturating_sub(examined + pending.len());
            for child in ui.children(&node).into_iter().take(remaining) {
                pending.push_back((format!("{node}.{child}"), depth + 1, card.clone()));
            }
        }
    }
    None
}

fn hovered(ui: &impl Ui, tiles: &[Tile], grid: Rect, cursor: (f32, f32)) -> Option<(Hero, Rect)> {
    if !contains(grid, cursor) {
        return None;
    }
    tiles.iter().find_map(|tile| {
        let rect = ui
            .rect(&tile.card)
            .filter(|r| valid_tile(*r) && contains(*r, cursor))?;
        let mut path = tile.identity.as_str();
        while path != CONTENTS {
            if !ui.visible(path) {
                return None;
            }
            path = path.rsplit_once('.')?.0;
        }
        (hero_at(ui, &tile.identity) == Some(tile.hero)).then_some((tile.hero, rect))
    })
}

fn art_rect((x, y, w, h): Rect, aspect: f32) -> Rect {
    // Keep the animation above the caption and away from the role badges.
    let max_h = h * 0.66;
    let width = (w * 0.82).min(max_h * aspect);
    let height = width / aspect;
    (x + (w - width) / 2., y + h * 0.7 - height, width, height)
}

#[cfg(windows)]
fn cursor((w, h): (f32, f32)) -> Option<(f32, f32)> {
    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }
    #[repr(C)]
    struct ScreenRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowThreadProcessId(window: *mut c_void, process: *mut u32) -> u32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn ScreenToClient(window: *mut c_void, point: *mut Point) -> i32;
        fn GetClientRect(window: *mut c_void, rect: *mut ScreenRect) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetCurrentProcessId() -> u32;
    }
    let mut process = 0;
    let mut point = Point { x: 0, y: 0 };
    let mut rect = ScreenRect {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // Only read cursor coordinates relative to this game's foreground client window.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null()
            || GetWindowThreadProcessId(window, &mut process) == 0
            || process != GetCurrentProcessId()
            || GetCursorPos(&mut point) == 0
            || ScreenToClient(window, &mut point) == 0
            || GetClientRect(window, &mut rect) == 0
        {
            return None;
        }
    }
    let width = (rect.right - rect.left) as f32;
    let height = (rect.bottom - rect.top) as f32;
    (width > 0. && height > 0.).then(|| (point.x as f32 * w / width, point.y as f32 * h / height))
}

#[cfg(not(windows))]
fn cursor(_: (f32, f32)) -> Option<(f32, f32)> {
    None
}

#[derive(Default)]
struct State {
    elapsed: u64,
    next_scan: u64,
    tile: Option<Tile>,
    active: bool,
    probe: bool,
}
static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| Mutex::new(State::default()));

pub(crate) fn update(ctx: &StableClient<'_>, dt: u64) {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !is_match(ctx) {
        if state.active {
            *state = State::default();
        }
        return;
    }
    state.elapsed = state.elapsed.saturating_add(dt.min(SCAN_INTERVAL));
    let probe = ctx.key_pressed("F8");
    state.active = active(ctx);
    if !state.active {
        state.tile = None;
        state.next_scan = 0;
    }
    if probe {
        let children = ctx.ui_child_names(CONTENTS);
        let examples: Vec<_> = children
            .iter()
            .take(4)
            .map(|name| {
                let path = format!("{CONTENTS}.{name}");
                (
                    name,
                    ctx.ui_runner_name(&path),
                    ctx.ui_node_rect(&path),
                    ctx.ui_child_names(&path)
                        .into_iter()
                        .take(8)
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        crate::wlog(format!(
            "[draft grid] F8 active={} children={} examples={examples:?} cached={:?}",
            state.active,
            children.len(),
            state.tile
        ));
        state.probe = true;
    }
}

pub(crate) fn render(ctx: &mut StableClient<'_>) {
    if !is_match(ctx) || !ctx.can_draw() {
        return;
    }
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let probe = std::mem::take(&mut state.probe);
    if !state.active && !probe {
        return;
    }
    let Some(size) = ctx.draw_map_size("UI") else {
        return;
    };
    let mouse = cursor(size);
    let grid = ctx.rect(GRID).filter(|_| state.active);
    let current = grid
        .zip(mouse)
        .filter(|(grid, mouse)| contains(*grid, *mouse))
        .and_then(|(grid, mouse)| {
            let mut hit = hovered(ctx, state.tile.as_slice(), grid, mouse);
            if probe || (hit.is_none() && state.elapsed >= state.next_scan) {
                state.tile = scan(ctx, mouse);
                state.next_scan = state.elapsed + SCAN_INTERVAL;
                hit = hovered(ctx, state.tile.as_slice(), grid, mouse);
            }
            hit
        });
    let submitted = current.is_some_and(|(hero, tile)| {
        let (asset, aspect) = hero.art(state.elapsed);
        let rect = art_rect(tile, aspect);
        grid.is_some_and(|bounds| encloses(bounds, rect))
            && ctx.draw_svg("UI", &asset, rect, 5000, 0xffffffff)
    });
    if probe {
        crate::wlog(format!("[draft grid] F8 render ui={size:?} viewport={grid:?} mouse={mouse:?} target={current:?} submitted={submitted}"));
    }
}

pub(crate) fn clear() {
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = State::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Default)]
    struct Node {
        rect: Option<Rect>,
        label: Option<&'static str>,
        state: Option<&'static str>,
        hidden: bool,
    }

    #[derive(Default)]
    struct Tree(BTreeMap<String, Node>);

    impl Ui for Tree {
        fn children(&self, path: &str) -> Vec<String> {
            self.0
                .keys()
                .filter_map(|node| {
                    let (parent, child) = node.rsplit_once('.')?;
                    (path == parent).then(|| child.to_owned())
                })
                .collect()
        }
        fn visible(&self, path: &str) -> bool {
            self.0.get(path).is_some_and(|n| !n.hidden)
        }
        fn label(&self, path: &str) -> Option<String> {
            self.0.get(path)?.label.map(str::to_owned)
        }
        fn state(&self, path: &str) -> Option<String> {
            self.0.get(path)?.state.map(str::to_owned)
        }
        fn rect(&self, path: &str) -> Option<Rect> {
            self.0.get(path)?.rect
        }
    }

    const VIEW: Rect = (305., 108., 1305., 575.);
    const ATLAS: &str = "main.champions.contents.item_08";
    const HANZO: &str = "main.champions.contents.item_41";
    fn fixture() -> Tree {
        let mut tree = Tree::default();
        for path in ["main", DRAFT_STEP, GRID, CONTENTS] {
            tree.0.insert(path.into(), Node::default());
        }
        tree.0.insert(
            ATLAS.into(),
            Node {
                rect: Some((1456., 119., 133., 132.)),
                state: Some(r#"{"selected": false, "text": "Atlas"}"#),
                ..Node::default()
            },
        );
        tree.0.insert(
            HANZO.into(),
            Node {
                rect: Some((575., 399., 133., 132.)),
                ..Node::default()
            },
        );
        tree.0.insert(
            format!("{HANZO}.name"),
            Node {
                label: Some("Hanzo"),
                ..Node::default()
            },
        );
        tree
    }
    #[test]
    fn observed_grid_scopes_the_hover_and_resolves_caption_or_label() {
        let tree = fixture();
        assert!(active(&tree));
        assert_eq!(scan(&tree, (1520., 169.)).unwrap().hero, Hero::Atlas);
        assert_eq!(scan(&tree, (612., 410.)).unwrap().hero, Hero::Hanzo);
        assert!(scan(&tree, (850., 400.)).is_none());
    }
    #[test]
    fn stale_hidden_or_scrolled_tiles_do_not_receive_an_overlay() {
        let mut tree = fixture();
        let point = (1520., 169.);
        let tile = scan(&tree, point).unwrap();
        assert_eq!(
            hovered(&tree, std::slice::from_ref(&tile), VIEW, point)
                .unwrap()
                .0,
            Hero::Atlas
        );
        tree.0.get_mut(ATLAS).unwrap().state = Some(r#"{"text":"Fanny"}"#);
        assert!(hovered(&tree, std::slice::from_ref(&tile), VIEW, point).is_none());
        tree.0.get_mut(ATLAS).unwrap().state = Some(r#"{"text":"Atlas"}"#);
        tree.0.get_mut(ATLAS).unwrap().hidden = true;
        assert!(hovered(&tree, std::slice::from_ref(&tile), VIEW, point).is_none());
        tree.0.get_mut(DRAFT_STEP).unwrap().hidden = true;
        assert!(!active(&tree));
    }
    #[test]
    fn cropped_art_is_kept_inside_the_grid_and_above_the_caption() {
        let tile = (1456., 119., 133., 132.);
        for hero in [Hero::Atlas, Hero::Hanzo, Hero::Granger, Hero::Popol] {
            let (first, aspect) = hero.art(0);
            assert_ne!(first, hero.art(300_000).0);
            assert_eq!(first, hero.art(1_200_000).0);
            let rect = art_rect(tile, aspect);
            assert!(encloses(tile, rect) && encloses(VIEW, rect));
            assert!(rect.1 + rect.3 < 219.);
        }
        assert!(!encloses(VIEW, art_rect((1456., 70., 133., 132.), 1.)));
    }
}
