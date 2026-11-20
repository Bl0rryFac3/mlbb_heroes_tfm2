# Aldous refresh — 2026-10-23

- Skill 1 primary damage is `40 + ((level - 1) * 80 / 11) + 100% AD + 2.5 * Soul Steal stacks`, capped at 1,000 stacks. The stack term contributes +2,500 raw damage at cap; nearby splash remains 40% of primary damage.
- At level 15, Aldous has 121 AD. At 1,000 stacks, Skill 1 is 2,762 raw damage before defense. This clears the conservative full-health Fanny benchmark (2,092 HP / 28 defense) under the standard 100/(100+DEF) mitigation relation; in-game damage is not verified.
- Aldous basic-attack frames and action timing were restored to the prior four-pose version: 0.065 s per pose, 14-tick duration, hit tick 2, 60-tick cooldown. The temporary alternate version is preserved under `source/aldous/attack_animation_intermediate_2026-10-23/`.
- English, Korean, Japanese, Simplified Chinese, and Traditional Chinese Skill 1 descriptions match the new stack coefficient.
- Initial pre-change backups are preserved under `source/aldous/refresh_2026-10-23_before/`.
