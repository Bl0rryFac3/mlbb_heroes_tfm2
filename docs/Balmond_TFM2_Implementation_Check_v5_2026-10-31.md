# Balmond v5 implementation check — 31 October 2026

## V5 S1 change

At the user's selection, the existing supplied S1 trail asset was uniformly resized to 354×118, approximately 65.2% (closest integer-pixel representation of the selected 65%). Its six frame crops are now 59×118 instead of 90.5×181. Frame durations and the caster-follow VFX trigger are unchanged. No art was regenerated.

V5 carries forward the v4 changes: caster-anchored S1 and Ultimate visuals; explicit S2 `skill2` body animation; `EnemyWithoutTower` S2 targeting at 35,000 range; explicit Ultimate body animation using an approximate 30-tick timing-map conversion. Rust damage mechanics and formulas are unchanged.

## Validation and remaining limitations

The v5 static audit passes **80 checks, 0 failures**, including the new atlas-size and frame-crop checks. The v5 delta patch applies cleanly to a v4 tree, and the binary atlas/FAnim replacement passes the audit after copying. This does not establish how the trail looks in TFM2. No full source/SDK build or in-game test was run; no runtime ZIP is included.

S2's target enum can still select jungle units and does not guarantee hero/minion priority. Ultimate fan orientation remains unverified.

## Files

- [V5 source/data/asset patch ZIP](https://drive.google.com/file/d/18EzucowYjLBCw3LZsK-2q7CGuMrjpuX5/view?usp=drivesdk)
- [Rescaled S1 trail sheet](https://drive.google.com/file/d/1j6sGNhA7BzWOcBoZBgElk85PrFZKT7kF/view?usp=drivesdk)
- [Updated S1 FAnim crop data](https://drive.google.com/file/d/1QqMr2LXQL0o--9G3RwZ1bbf2_-OHN_6X/view?usp=drivesdk)

The archive contains no SDK or runtime DLL. It applies after v4, updating the two S1 atlas files and adding v5 audit/notes.
