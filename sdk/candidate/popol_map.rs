//! Reuse Fanny's map-hook wall data, without Fanny's synthetic cable borders.
use super::*;
use std::collections::VecDeque;
use std::sync::Arc;
static GROUND: LazyLock<Mutex<HashMap<u32, Arc<crate::Grid>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn shared_ground(mode: GameModeKindV1) -> Arc<crate::Grid> {
    let source = crate::MAP_GRIDS.lock().unwrap_or_else(|e| e.into_inner());
    let mut cached = GROUND.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(grid) = cached.get(&mode.code()) {
        if source
            .get(&mode.code())
            .is_none_or(|s| s.cells == grid.cells)
        {
            return Arc::clone(grid);
        }
    }
    let grid = Arc::new(ground_grid(
        source
            .get(&mode.code())
            .cloned()
            .unwrap_or_else(|| crate::Grid::builtin(mode)),
    ));
    cached.insert(mode.code(), Arc::clone(&grid));
    grid
}
trait GridRef {
    fn grid(&self) -> &crate::Grid;
}
impl GridRef for crate::Grid {
    fn grid(&self) -> &crate::Grid {
        self
    }
}
impl GridRef for Arc<crate::Grid> {
    fn grid(&self) -> &crate::Grid {
        self
    }
}

#[derive(Clone, Default)]
pub(super) struct Layout {
    mode: Option<u32>,
    observed: VecDeque<Pos>,
    last: Option<Pos>,
    cached: Option<Arc<crate::Grid>>,
    next_select: usize,
    next_early: usize,
}
fn ground_grid(mut grid: crate::Grid) -> crate::Grid {
    // MOBA_BOUNDS is a Fanny cable collision surface, not a unit movement wall.
    grid.bounds = None;
    grid
}
impl Layout {
    pub(super) fn observe(&mut self, p: Pos) {
        if self.last.is_none_or(|q| dist(q, p) > 2000.) {
            self.observed.push_back(p);
            if self.observed.len() > 128 {
                self.observed.pop_front();
            }
            self.last = Some(p);
        }
    }
    pub(super) fn select(
        &mut self,
        sim: &StableSim<'_>,
        from: Pos,
        to: Pos,
    ) -> (u32, Arc<crate::Grid>, bool) {
        let now = sim.tick();
        if let (Some(mode), Some(grid)) = (self.mode, self.cached.as_ref()) {
            if now < self.next_select
                && (now < self.next_early
                    || (!grid.is_wall(from.0 as f64, from.1 as f64)
                        && !grid.is_wall(to.0 as f64, to.1 as f64)))
            {
                return (mode, Arc::clone(grid), false);
            }
        }
        let live: Vec<Pos> = (0..sim.entity_count())
            .filter_map(|i| sim.entity_at(i))
            .filter(|e| e.is_alive() && e.is_champion())
            .map(|e| e.pos())
            .collect();
        let candidates: Vec<_> = crate::MODES
            .iter()
            .copied()
            .map(|mode| (mode.code(), shared_ground(mode)))
            .collect();
        let old = self.mode;
        let (mode, grid) = pick(candidates, &self.observed, &live, from, to, old);
        let changed = old != Some(mode)
            || self
                .cached
                .as_ref()
                .is_some_and(|old| old.cells != grid.cells);
        self.cached = Some(Arc::clone(&grid));
        self.next_select = now + 120;
        self.next_early = now + 15;
        self.mode = Some(mode);
        (mode, grid, changed)
    }
}
fn pick<G: GridRef>(
    candidates: Vec<(u32, G)>,
    history: &VecDeque<Pos>,
    live: &[Pos],
    from: Pos,
    to: Pos,
    old: Option<u32>,
) -> (u32, G) {
    candidates
        .into_iter()
        .min_by_key(|(mode, g)| {
            let g = g.grid();
            let blocked = |p: &Pos| g.is_wall(p.0 as f64, p.1 as f64);
            // All champions plus actual owner movement disambiguate maps better
            // than two moving endpoints. A tie retains this owner's prior layout.
            (
                history.iter().filter(|p| blocked(p)).count()
                    + live.iter().filter(|p| blocked(p)).count(),
                usize::from(blocked(&from)) + usize::from(blocked(&to)),
                usize::from(old != Some(*mode)),
                *mode,
            )
        })
        .expect("the game always has three supported map modes")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_units_do_not_inherit_fanny_virtual_cable_edges() {
        let mut g = crate::Grid::from_rows(&[".....", ".....", "....."]);
        g.bounds = Some((56000., 56000., 904000., 904000.));
        assert!(g.solid(48000., 48000.));
        let g = ground_grid(g);
        assert!(!g.solid(48000., 48000.));
        let mut n = Navigation::default();
        let q = n.step(&g, (48000, 48000), (100000, 48000), 1);
        assert!(q.0 > 48000);
        assert!(dist(q, (48000, 48000)) <= SPEED + 2.);
    }
    #[test]
    fn live_champions_disambiguate_endpoints_shared_by_two_maps() {
        let a = crate::Grid::from_rows(&[".....", "..#..", "....."]);
        let b = crate::Grid::from_rows(&[".....", ".....", "....."]);
        let (mode, _) = pick(
            vec![(0, a), (1, b)],
            &VecDeque::new(),
            &[(80000, 48000)],
            (16000, 16000),
            (112000, 16000),
            Some(0),
        );
        assert_eq!(mode, 1);
    }
    #[test]
    fn actual_walked_cells_override_stale_layout_selection() {
        let a = crate::Grid::from_rows(&[".....", "..#..", "....."]);
        let b = crate::Grid::from_rows(&[".....", ".....", "....."]);
        let (mode, _) = pick(
            vec![(0, a), (1, b)],
            &VecDeque::from([(80000, 48000)]),
            &[],
            (16000, 16000),
            (112000, 16000),
            Some(0),
        );
        assert_eq!(mode, 1);
    }
    #[test]
    fn equal_scores_keep_layout_stable() {
        let a = crate::Grid::from_rows(&[".....", ".....", "....."]);
        let (mode, _) = pick(
            vec![(0, a.clone()), (1, a)],
            &VecDeque::new(),
            &[],
            (16000, 16000),
            (112000, 16000),
            Some(1),
        );
        assert_eq!(mode, 1);
    }
}
