# rBuilder website

Static marketing + docs hub (Next.js App Router, `output: "export"`).

## Stack

- Next.js (static export)
- Tailwind CSS v4
- shadcn-style primitives (Button, Tabs, Badge)
- Warp-inspired tokens from `DESIGN.md`

## Develop

```bash
cd website
pnpm install
pnpm dev
```

Open http://localhost:3000

## Build

```bash
pnpm build   # writes ./out
```

For GitHub Pages project site locally:

```bash
NEXT_PUBLIC_BASE_PATH=/rBuilder pnpm build
```

## Deploy

GitHub Actions workflow: `.github/workflows/website.yml`

1. Repo **Settings → Pages → Build and deployment → Source: GitHub Actions**
2. Push changes under `website/` (or run the workflow manually)
3. Site: `https://sshaaf.github.io/rBuilder/`

## Routes

| Path | Purpose |
|------|---------|
| `/` | Landing |
| `/install/` | Install + first hour |
| `/docs/` | Docs hub (links to repo markdown) |
| `/agents/` | Agent loop |
| `/demo/` | Interactive demo scenarios |
| `/community/` | Stars, discussions, contribute |
