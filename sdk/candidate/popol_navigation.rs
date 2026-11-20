//! Four-neighbor grid routing for the companion. No world-position warp.
#[cfg(test)]
use super::SPEED;
use super::{dist, legal_step, Pos};
use crate::Grid;
use std::collections::VecDeque;
const CELL: u64 = 32_000;

#[derive(Clone, Default)]
pub(super) struct Navigation {
    pub layout: super::maplayout::Layout,
    pub blocked_count: usize,
    path: VecDeque<Pos>,
    goal: Option<(u64, u64)>,
    next_plan: usize,
    stalled: usize,
}
fn center(x: usize, y: usize) -> Pos {
    (x as u64 * CELL + CELL / 2, y as u64 * CELL + CELL / 2)
}
fn clear(g: &Grid, p: Pos) -> bool {
    [
        (0., 0.),
        (-4_000., 0.),
        (4_000., 0.),
        (0., -4_000.),
        (0., 4_000.),
    ]
    .into_iter()
    .all(|(x, y)| !g.solid(p.0 as f64 + x, p.1 as f64 + y))
}
fn line_clear(g: &Grid, a: Pos, b: Pos) -> bool {
    let steps = (dist(a, b) / 2_000.).ceil().max(1.) as usize;
    (0..=steps).all(|i| {
        clear(
            g,
            (
                (a.0 as f64 + (b.0 as f64 - a.0 as f64) * i as f64 / steps as f64) as u64,
                (a.1 as f64 + (b.1 as f64 - a.1 as f64) * i as f64 / steps as f64) as u64,
            ),
        )
    })
}
fn route(g: &Grid, from: Pos, to: Pos) -> Option<VecDeque<Pos>> {
    let h = g.cells.len();
    let w = g.cells.first()?.len();
    if w == 0 {
        return None;
    }
    let start = ((from.0 / CELL) as usize, (from.1 / CELL) as usize);
    let goal = ((to.0 / CELL) as usize, (to.1 / CELL) as usize);
    if start.0 >= w || start.1 >= h || goal.0 >= w || goal.1 >= h {
        return None;
    }
    let mut parent = vec![None; w * h];
    let start_idx = start.1 * w + start.0;
    parent[start_idx] = Some(start_idx);
    let mut q = VecDeque::from([start]);
    while let Some((x, y)) = q.pop_front() {
        if (x, y) == goal {
            break;
        }
        for (dx, dy) in [(1isize, 0isize), (0, 1), (-1, 0), (0, -1)] {
            let (nx, ny) = (x as isize + dx, y as isize + dy);
            if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                continue;
            }
            let (nx, ny) = (nx as usize, ny as usize);
            let idx = ny * w + nx;
            if parent[idx].is_none() && clear(g, center(nx, ny)) {
                parent[idx] = Some(y * w + x);
                q.push_back((nx, ny));
            }
        }
    }
    let mut index = goal.1 * w + goal.0;
    if parent[index].is_none() {
        return None;
    }
    let mut result = VecDeque::new();
    while index != start_idx {
        result.push_front(center(index % w, index / w));
        index = parent[index]?;
    }
    if clear(g, to) {
        result.push_back(to);
    }
    Some(result)
}
impl Navigation {
    pub(super) fn clear_route(&mut self) {
        self.path.clear();
        self.goal = None;
        self.next_plan = 0;
        self.stalled = 0;
    }
    #[cfg(test)]
    pub(super) fn step(&mut self, g: &Grid, from: Pos, to: Pos, now: usize) -> Pos {
        self.step_with_speed(g, from, to, now, SPEED)
    }
    pub(super) fn step_with_speed(
        &mut self,
        g: &Grid,
        from: Pos,
        to: Pos,
        now: usize,
        speed: f64,
    ) -> Pos {
        // A spawned entity may be on the edge of a legal cell after host collision
        // resolution. Move boundedly inward before enforcing full body clearance.
        if !clear(g, from) && !g.solid(from.0 as f64, from.1 as f64) {
            let c = center((from.0 / CELL) as usize, (from.1 / CELL) as usize);
            let n = dist(from, c);
            if n > 1. {
                let p = (
                    (from.0 as f64 + (c.0 as f64 - from.0 as f64) / n * n.min(speed)) as u64,
                    (from.1 as f64 + (c.1 as f64 - from.1 as f64) / n * n.min(speed)) as u64,
                );
                if !g.solid(p.0 as f64, p.1 as f64)
                    && p.0 / CELL == from.0 / CELL
                    && p.1 / CELL == from.1 / CELL
                {
                    return p;
                }
            }
        }
        if line_clear(g, from, to) {
            self.path.clear();
            self.stalled = 0;
            return legal_step(g, from, to, speed);
        }
        let cell = (to.0 / CELL, to.1 / CELL);
        if self.goal != Some(cell)
            || (self.path.is_empty() || self.stalled >= 15) && now >= self.next_plan
        {
            self.goal = Some(cell);
            self.path = route(g, from, to).unwrap_or_default();
            self.next_plan = now + 15;
            self.stalled = 0;
        }
        while self.path.front().is_some_and(|p| dist(from, *p) < 1_200.) {
            self.path.pop_front();
        }
        // Pull the route taut only across clear space, never across a blocked cell.
        let aim = self
            .path
            .iter()
            .rev()
            .find(|p| line_clear(g, from, **p))
            .copied()
            .or_else(|| self.path.front().copied());
        let next = aim.map_or(from, |p| legal_step(g, from, p, speed));
        if next == from {
            self.stalled += 1
        } else {
            self.stalled = 0
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_around_the_end_of_a_base_wall() {
        let g = Grid::from_rows(&[
            ".......", "..#....", "..#....", "..#....", ".......", ".......",
        ]);
        let mut n = Navigation::default();
        let mut p = center(1, 2);
        let goal = center(4, 2);
        let mut detoured = false;
        for t in 0..400 {
            let q = n.step(&g, p, goal, t);
            assert!(clear(&g, q));
            assert!(dist(p, q) <= SPEED + 2.);
            detoured |= q.1 < 32_000 || q.1 >= 128_000;
            p = q;
            if dist(p, goal) < 2_000. {
                break;
            }
        }
        assert!(dist(p, goal) < 2_000., "failed to rejoin {p:?}");
        assert!(detoured);
    }
    #[test]
    fn enters_legal_cell_interior_after_spawn_collision_without_a_warp() {
        let g = Grid::from_rows(&["....", ".#..", "....", "...."]);
        let mut n = Navigation::default();
        let p = (30_000, 48_000);
        let q = n.step(&g, p, center(0, 2), 0);
        assert!(dist(p, q) <= SPEED + 2.);
        assert!(q.0 < p.0);
        assert!(!g.solid(q.0 as f64, q.1 as f64));
    }
    #[test]
    fn unreachable_goal_does_not_walk_through_walls() {
        let g = Grid::from_rows(&[".....", ".###.", ".#.#.", ".###.", "....."]);
        let mut n = Navigation::default();
        let mut p = center(0, 2);
        for t in 0..60 {
            p = n.step(&g, p, center(2, 2), t);
        }
        assert_eq!(p, center(0, 2));
    }
    #[test]
    fn goal_change_discards_a_stale_route() {
        let g = Grid::from_rows(&[".....", "..#..", "..#..", "....."]);
        let mut n = Navigation::default();
        let p = center(1, 1);
        let _ = n.step(&g, p, center(4, 1), 0);
        let q = n.step(&g, p, center(0, 1), 1);
        assert!(q.0 < p.0);
    }
}
