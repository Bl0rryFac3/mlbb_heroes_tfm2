//! Identifies the map from the tower placements the game supplied at creation.
use mod_api_stable::{GameModeKindV1, StableSim};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

type Position = (u64, u64);
type TowerLayout = Vec<Position>;

static LAYOUTS: LazyLock<Mutex<HashMap<u32, TowerLayout>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Capture the host's towers array. Empty or malformed input clears stale data
/// for this mode so an older layout cannot identify a different match.
pub(super) fn capture(mode: GameModeKindV1, raw: &str) -> bool {
    let layout = parse_layout(raw);
    let mut layouts = LAYOUTS.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(layout) = layout {
        layouts.insert(mode.code(), layout);
        true
    } else {
        layouts.remove(&mode.code());
        false
    }
}

pub(super) fn identify(sim: &StableSim<'_>) -> Option<GameModeKindV1> {
    let mut positions = (0..sim.tower_count())
        .map(|index| {
            sim.get_entity(sim.tower_id_at(index))
                .map(|tower| tower.pos())
        })
        .collect::<Option<TowerLayout>>()?;
    positions.sort_unstable();
    let layouts = LAYOUTS.lock().unwrap_or_else(|error| error.into_inner());
    identify_layout(&layouts, &positions)
}

fn identify_layout(
    layouts: &HashMap<u32, TowerLayout>,
    positions: &[Position],
) -> Option<GameModeKindV1> {
    if positions.is_empty() {
        return None;
    }
    let mut matching = layouts
        .iter()
        .filter(|(_, layout)| layout.as_slice() == positions);
    let mode = GameModeKindV1::from_code(*matching.next()?.0)?;
    matching.next().is_none().then_some(mode)
}

// The stable map document stores each tower type as an object whose pos field
// contains one [x, y] pair per team. Other fields describe its gameplay.
fn parse_layout(raw: &str) -> Option<TowerLayout> {
    let mut json = TowerJson(raw);
    json.take('[')?;
    let mut positions = Vec::new();
    loop {
        json.take('{')?;
        let mut pair = None;
        loop {
            let field = json.string()?;
            json.take(':')?;
            if field == "pos" {
                if pair.is_some() {
                    return None;
                }
                json.take('[')?;
                let blue = json.position()?;
                json.take(',')?;
                let red = json.position()?;
                json.take(']')?;
                pair = Some([blue, red]);
            } else {
                json.skip_field_value()?;
            }
            if json.take('}').is_some() {
                break;
            }
            json.take(',')?;
        }
        positions.extend(pair?);
        if json.take(']').is_some() {
            break;
        }
        json.take(',')?;
    }
    if !json.0.trim().is_empty() {
        return None;
    }
    positions.sort_unstable();
    Some(positions)
}

struct TowerJson<'a>(&'a str);

impl<'a> TowerJson<'a> {
    fn take(&mut self, token: char) -> Option<()> {
        self.0 = self.0.trim_start().strip_prefix(token)?;
        Some(())
    }

    fn string(&mut self) -> Option<&'a str> {
        self.take('"')?;
        let mut escaped = false;
        for (index, ch) in self.0.char_indices() {
            if ch == '"' && !escaped {
                let value = &self.0[..index];
                self.0 = &self.0[index + 1..];
                return Some(value);
            }
            escaped = ch == '\\' && !escaped;
        }
        None
    }

    fn coordinate(&mut self) -> Option<u64> {
        self.0 = self.0.trim_start();
        let digits = self.0.bytes().take_while(u8::is_ascii_digit).count();
        let value = self.0[..digits].parse().ok()?;
        self.0 = &self.0[digits..];
        Some(value)
    }

    fn position(&mut self) -> Option<Position> {
        self.take('[')?;
        let x = self.coordinate()?;
        self.take(',')?;
        let y = self.coordinate()?;
        self.take(']')?;
        Some((x, y))
    }

    // Skip the host's other fields, respecting nested containers and strings.
    // The host has already serialized and validated the enclosing JSON.
    fn skip_field_value(&mut self) -> Option<()> {
        self.0 = self.0.trim_start();
        let mut depth = 0usize;
        let mut string = false;
        let mut escaped = false;
        for (index, ch) in self.0.char_indices() {
            if string {
                match ch {
                    '"' if !escaped => string = false,
                    _ => escaped = ch == '\\' && !escaped,
                }
                continue;
            }
            match ch {
                '"' => string = true,
                '[' | '{' => depth += 1,
                '}' | ',' if depth == 0 && index > 0 => {
                    self.0 = &self.0[index..];
                    return Some(());
                }
                ']' | '}' => depth = depth.checked_sub(1)?,
                _ => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sorted_positions_from_both_teams_and_each_tower() {
        let raw = r#"[{"tier":"Twin","pos":[[90,80],[15,70]],"metadata":{"flags":[true,"x,}"]}},
                      {"pos":[[30,10],[2,60]],"tier":1}]"#;
        assert_eq!(
            parse_layout(raw),
            Some(vec![(2, 60), (15, 70), (30, 10), (90, 80)])
        );
    }

    #[test]
    fn malformed_or_empty_layout_supplies_no_map_evidence() {
        for raw in [
            "[]",
            r#"[{"pos":[[1,2]]}]"#,
            r#"[{"pos":[[1,2],[3,4],[5,6]]}]"#,
            r#"[{"pos":[[1,2],[3,4]]},{"tier":1}]"#,
        ] {
            assert!(parse_layout(raw).is_none(), "{raw}");
        }
    }

    #[test]
    fn requires_one_mode_with_the_complete_exact_tower_inventory() {
        let moba = GameModeKindV1::Moba.code();
        let single_lane = GameModeKindV1::SingleLane.code();
        let positions = vec![(2, 60), (15, 70)];
        let mut layouts = HashMap::from([(moba, positions.clone())]);
        assert_eq!(
            identify_layout(&layouts, &positions).map(|mode| mode.code()),
            Some(moba)
        );
        assert!(identify_layout(&layouts, &positions[..1]).is_none());
        assert!(identify_layout(&layouts, &[(2, 60), (15, 71)]).is_none());
        assert!(identify_layout(&layouts, &[]).is_none());
        layouts.insert(single_lane, positions.clone());
        assert!(identify_layout(&layouts, &positions).is_none());
    }
}
