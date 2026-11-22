# Lesley and Yu Zhong Windows candidate 1.1.173

Close Teamfight Manager 2 and move the previous MLBB Heroes folder out of `mods`. Extract the ZIP into `mods`. The Mods menu should show **1.1.173**. Candidate **1.1.172** is the previous private build.

## Lesley

- Check an ordinary shot, Lethal Shot and all four Ultimate Snipe rounds. Each fired round should render one bullet. The three source sheets used a two-row grid, and the former frame height pulled two bullets into one animation frame.
- Watch Tactical Grenade from either facing. One fan should start at the cast position. Lesley should step backward along a clear route while surviving enemy champions struck by the cone are knocked away. Repeat at a wall and confirm that her backstep stops on the near side.
- Watch Master of Camouflage while entering, idling and moving. The cast sprite fades and the native invisibility timer is renewed during the active state. Confirm enemy direct basic targeting drops, an area skill can still hit, and she reveals on her own shot or skill, on expiry, or on damage following the first 0.5s. The opaque body could reappear between the cast clip and normal movement if the installed game’s renderer draws invisible moving champions fully opaque. Record that behavior if it occurs.
- Spend Lethal Shot once, then test each skill independently. Each successful cast readies one empowered basic. After it is spent, taking damage restarts the five-second natural recharge. A shot already ready can still fire after she takes a hit.

## Yu Zhong

- Land Soul Grip on an enemy champion and watch the empowered basic attack on the same target. It should close a short distance on a clear route. If another enemy is selected, watch for the normal empowered hit without a target switch.
- Watch Furious Dive’s leap, landing hit, and directional follow-up. The brief second dash follows his movement after landing when a safe route is available. The delayed airborne pulse should stay centered on the landing area. Repeat with a wall or enemy tower near the route, and try both human and Dragonoid forms.

The Windows build and automated checks verify the code and assets. Enemy targeting, camouflage rendering, and the movement feel should be checked in a live match.
