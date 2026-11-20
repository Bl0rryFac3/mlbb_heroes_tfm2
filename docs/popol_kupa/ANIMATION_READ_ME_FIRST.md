# Popol & Kupa — animation handoff, batch 1

Start with the four safe clips: Popol idle/run and normal Kupa idle/run. The attached PNGs are padded copies of your originals, with all source pixels preserved. Do not animate the diagnostic MOUNT TEST composition from the technical build.

Use `references/popol_reference_72x72.png` for Popol and `references/kupa_reference_96x72.png` for Kupa. Original tight masters are retained separately for provenance. Keep the reference at native size/1× output. The old divide-by-ten instruction is for enlarged source art and does not apply here. Export RGBA PNG with real transparent alpha, hard pixel edges, no texture filtering or smoothing.

Produce one motion at a time, nine frames per motion, preferably a 3×3 grid. Popol cell: 72×72 (3×3 output 216×216). Kupa cell: 96×72 (3×3 output 288×216). Feet/paws are grounded around cell row 55. The cell dimensions are part of this handoff: do not auto-crop each frame independently. If your tool insists on a different layout, retain every returned frame and report its cell size. Do not resize the characters to fit a forced square.

Return files named `popol_idle.png`, `popol_run.png`, `kupa_idle.png`, `kupa_run.png`, plus the remaining approved motions when ready. A clear repeated first/last pose is fine; do not remove frames to satisfy a grid. Actual usable frame counts will be measured before compiling.

`MOTION_PROMPTS.md` includes later combat/body clips for planning. The current implementation is a diagnostic test, not a finished hero. Mounted and Alpha poses need their own correct source designs; those paid/creative assets have not been generated. Finish the mount/targetability render test before spending time on those motions. Separate world-space VFX (spears, marks, traps, shields, frost, transformation bursts) will be a later pack tied to final event timings.

No SpriteCook/PixelLab credits were spent to prepare this handoff. SpriteCook masters are on the reter connection; their asset IDs and original hashes are in spritecook-assets.json. Animate the supplied padded PNG in your existing PixelLab workflow.
