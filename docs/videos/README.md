# rBuilder demo video

**`rbuilder-feature-demo.mp4`** — ~28s montage (25–30s) of all dashboard features, **card-focused** (stat cards, blast metrics, migration tuning, taint detail, etc.).

## Regenerate

```bash
rbuilder -r /path/to/gbuilder discover . --all
rbuilder -r /path/to/gbuilder serve --port 8080

cd dashboard
DASHBOARD_URL=http://127.0.0.1:8080/ DEMO_MAX_SECS=28 node scripts/record-feature-demo.mjs
```

Requires **ffmpeg** and **Playwright** (dashboard devDependency). Output: `docs/videos/rbuilder-feature-demo.mp4`.

Optional env: `CAPTURE_FN_DATAFLOW`, `CAPTURE_FN_BLAST`, `CAPTURE_FN_TAINT`, `CAPTURE_SLICE_LINE`, `CAPTURE_SLICE_VAR` (same as screenshot script).

Encoding keeps **all** tab segments: if the raw capture exceeds `DEMO_MAX_SECS`, ffmpeg **speed-compresses** the full recording (no head-trim), so blast radius and later tabs stay in the final cut.
