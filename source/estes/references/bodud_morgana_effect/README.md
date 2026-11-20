# BoDUD Morgana tether reference

Source: [BoDUD/TFM2-League-Heroes](https://github.com/BoDUD/TFM2-League-Heroes), especially [Morgana PR #30](https://github.com/BoDUD/TFM2-League-Heroes/pull/30) and the [Morgana champion data](https://raw.githubusercontent.com/BoDUD/TFM2-League-Heroes/main/league/champion/league_morgana.data_champion).

The copied `league_morgana_fx#anim.fanim` has an `r_chain` animation (four 55×29 frames). Morgana binds that visible animation to `league_morgana_r_chain` in `view_buffs`. The repeated `TargetProjectile`s in Soul Shackles have no `view_projectiles` binding; they are used to re-check tether range, not to draw the visible chain. Therefore the Estes v1.1.14 projectile-pulse addition was not a faithful implementation of Morgana's visual technique.

For Estes, retain the explicit requirement: draw the supplied Estes tether art continuously between the live caster and selected ally. The active native renderer is in `src/lib.rs` / `src/estes.rs`; before another release, instrument the link count and `draw_svg` return value, then make the game-recognized draw asset visibly render. Do not substitute a debug line or claim runtime success without an in-game check.
