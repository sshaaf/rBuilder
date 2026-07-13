/**
 * Record a 25–30s rBuilder feature montage — card-focused, all features covered.
 *
 * Prereq:
 *   rbuilder -r /path/to/gbuilder serve --port 8080
 *
 * Usage:
 *   DASHBOARD_URL=http://127.0.0.1:8080/ node dashboard/scripts/record-feature-demo.mjs
 *
 * Output: docs/videos/rbuilder-feature-demo.mp4
 */

import { chromium } from "playwright";
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";

const BASE = process.env.DASHBOARD_URL ?? "http://127.0.0.1:8080/";
const ROOT = path.resolve(import.meta.dirname, "../..");
const OUT_DIR = path.join(ROOT, "docs/videos");
const RAW_WEBM = path.join(OUT_DIR, "rbuilder-feature-demo.raw.webm");
const OUT_MP4 = path.join(OUT_DIR, "rbuilder-feature-demo.mp4");
const TARGET_SECS = Number(process.env.DEMO_MAX_SECS ?? "28");

const FN = process.env.CAPTURE_FN_DATAFLOW ?? "addEmbeddingSimilarityEdges";
const FN_BLAST = process.env.CAPTURE_FN_BLAST ?? "addEmbeddingSimilarityEdges";
const FN_TAINT = process.env.CAPTURE_FN_TAINT ?? "clearFileGraph";
const SLICE_LINE = process.env.CAPTURE_SLICE_LINE ?? "45";
const SLICE_VAR = process.env.CAPTURE_SLICE_VAR ?? "threshold";

fs.mkdirSync(OUT_DIR, { recursive: true });

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function clickTab(page, label) {
  await page.getByRole("button", { name: label, exact: true }).click();
  await sleep(400);
}

async function selectFunction(page, name) {
  const search = page.locator('.function-list-sidebar input[type="search"]');
  if (await search.count()) {
    await search.fill("");
    await sleep(150);
    await search.fill(name);
    await sleep(450);
  }
  const item = page.locator(".function-list-item", {
    has: page.locator(".function-list-item-name", { hasText: name }),
  });
  await item.first().click();
  await sleep(550);
}

async function waitWasm(page) {
  await page.waitForSelector(".rb-app", { timeout: 60000 });
  await page.waitForFunction(
    () => {
      const msg = document.body.textContent ?? "";
      if (msg.includes("WASM engine required for blast-radius")) return false;
      if (msg.includes("Waiting for WASM engine")) return false;
      return true;
    },
    { timeout: 90000 },
  );
  await sleep(1800);
}

async function waitForBlastResults(page) {
  await page.getByText("Callers of", { exact: false }).waitFor({ state: "visible", timeout: 25000 });
  await page.waitForFunction(
    () => {
      const el = document.querySelector(".blast-view .card-body .fs-4.fw-semibold.text-primary");
      return el && el.textContent && el.textContent.trim().length > 0;
    },
    { timeout: 25000 },
  );
  await sleep(600);
}

async function setCaption(page, text) {
  await page.evaluate((caption) => {
    let el = document.getElementById("rb-demo-caption");
    if (!el) {
      el = document.createElement("div");
      el.id = "rb-demo-caption";
      Object.assign(el.style, {
        position: "fixed",
        left: "50%",
        bottom: "24px",
        transform: "translateX(-50%)",
        zIndex: "99999",
        background: "rgba(13, 110, 253, 0.94)",
        color: "#fff",
        padding: "10px 20px",
        borderRadius: "8px",
        font: "600 17px/1.25 system-ui, sans-serif",
        boxShadow: "0 4px 18px rgba(0,0,0,0.28)",
        pointerEvents: "none",
        maxWidth: "92vw",
        textAlign: "center",
      });
      document.body.appendChild(el);
    }
    el.textContent = caption;
    el.style.opacity = "1";
  }, text);
}

async function clearHighlights(page) {
  await page.evaluate(() => {
    document.querySelectorAll("[data-rb-demo-highlight]").forEach((el) => {
      el.style.outline = "";
      el.style.outlineOffset = "";
      el.style.boxShadow = "";
      el.removeAttribute("data-rb-demo-highlight");
    });
  });
}

/** Highlight one dashboard stat card by label text. */
async function focusStatCard(page, label, ms, caption) {
  await setCaption(page, caption);
  await page.evaluate((lbl) => {
    for (const card of document.querySelectorAll(".stat-card")) {
      if (!card.textContent?.includes(lbl)) continue;
      card.scrollIntoView({ block: "center", behavior: "instant" });
      card.setAttribute("data-rb-demo-highlight", "1");
      card.style.outline = "3px solid #0d6efd";
      card.style.outlineOffset = "3px";
      card.style.boxShadow = "0 0 0 6px rgba(13, 110, 253, 0.15)";
    }
  }, label);
  await sleep(ms);
  await clearHighlights(page);
}

/** Scroll cards into view, highlight, hold with caption. */
async function focusCards(page, selector, ms, label) {
  await setCaption(page, label);
  const loc = page.locator(selector);
  if ((await loc.count()) > 0) {
    await loc.first().scrollIntoViewIfNeeded();
    await sleep(300);
    await page.evaluate((sel) => {
      document.querySelectorAll(sel).forEach((el) => {
        el.setAttribute("data-rb-demo-highlight", "1");
        el.style.outline = "3px solid #0d6efd";
        el.style.outlineOffset = "3px";
        el.style.boxShadow = "0 0 0 6px rgba(13, 110, 253, 0.15)";
      });
    }, selector);
  }
  await sleep(ms);
  await clearHighlights(page);
}

async function hold(page, ms, label) {
  await setCaption(page, label);
  await sleep(ms);
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 720 },
  recordVideo: { dir: OUT_DIR, size: { width: 1280, height: 720 } },
});
const page = await context.newPage();

await page.goto(BASE, { waitUntil: "networkidle", timeout: 120000 });
await waitWasm(page);

// 1. discover — dashboard stat cards + blast summary
await focusCards(page, ".rb-stats-row .stat-card", 2200, "discover · graph snapshot & index metrics");
await focusStatCard(page, "High Blast Radius", 2000, "discover · pre-computed blast scores at index time");

// 2. GQL — graph metagraph
await clickTab(page, "Graph Visualization");
await page.waitForSelector(".graph-panel.h-100", { timeout: 20000 });
await focusCards(page, ".graph-legend, .graph-toolbar", 2400, "GQL · explore package call graph");

// 3. Graph metrics — Functions table metric columns
await clickTab(page, "Functions");
await page.waitForSelector(".functions-view table thead, .functions-table thead", { timeout: 15000 });
const prBtn = page.getByRole("button", { name: /Sort by PR/i });
if (await prBtn.count()) await prBtn.click();
await sleep(400);
await focusCards(page, ".functions-view table thead, .functions-table thead", 2600, "Graph metrics · PageRank · betweenness · harmonic · blast");

// 4. CFG
await clickTab(page, "CFG / PDG Analysis");
await selectFunction(page, FN);
const loadCfg = page.getByRole("button", { name: /Load CFG graph/i });
if (await loadCfg.count()) await loadCfg.click();
await page.locator(".cfg-detail").first().waitFor({ state: "visible", timeout: 25000 }).catch(() => {});
await sleep(1000);
await focusCards(page, ".cfg-dom-col table, .dominance-panel", 2400, "CFG · control-flow blocks & dominators");

// 5. PDG
await clickTab(page, "Dataflow");
await selectFunction(page, FN);
await page.locator("#df-view").selectOption("dataflow");
await page.locator(".dataflow-graph-panel").waitFor({ state: "visible", timeout: 20000 }).catch(() => {});
await sleep(800);
await focusCards(page, ".dataflow-graph-panel .border-bottom, .view-legend", 2300, "PDG · data & control dependencies");

// 6. Dominance
await page.locator("#df-view").selectOption("dominator");
await sleep(1200);
await focusCards(page, ".dataflow-graph-panel", 2200, "Dominance · dominator tree & frontiers");

// 7. Program slicing — stats + editor
await clickTab(page, "Program Slicing");
await selectFunction(page, FN);
await page.locator("#slice-line").fill(String(SLICE_LINE));
await page.locator("#slice-var").fill(SLICE_VAR);
await page.getByRole("button", { name: "Compute slice" }).click();
await page.getByText(/slice:/i).waitFor({ state: "visible", timeout: 15000 }).catch(() => {});
await sleep(500);
await focusCards(page, ".slice-view .small.text-muted, .slice-view .btn-primary", 2400, "Program slicing · criterion & highlighted lines");

// 8. Blast radius — impact score cards (main focus)
await clickTab(page, "Blast Radius");
await page.waitForSelector(".blast-view", { timeout: 20000 });
await waitWasm(page);
await selectFunction(page, FN_BLAST);
await waitForBlastResults(page);
await page.locator(".blast-view .row.g-2").first().scrollIntoViewIfNeeded();
await focusCards(
  page,
  ".blast-view .row.g-2 .card",
  3800,
  "Blast radius · impact score · direct callers · impact zone",
);
await focusCards(
  page,
  ".blast-view .card .table-responsive",
  2400,
  "Blast radius · transitive caller table",
);

// 9. Taint — flows table + detail card
await clickTab(page, "Taint Analysis");
await selectFunction(page, FN_TAINT);
await page.locator(".taint-view table tbody tr").first().waitFor({ state: "visible", timeout: 15000 }).catch(() => {});
await page.locator(".taint-view table tbody tr").first().click();
await sleep(500);
await focusCards(page, ".taint-view .col-lg-5 .border.rounded, .taint-view table", 2600, "Taint analysis · source → sink flows & severity");

// 10. Migration planner — tuning cards + package table
await clickTab(page, "Migration");
await page.waitForSelector(".migration-tuning", { timeout: 20000 }).catch(() => {});
await focusCards(page, ".migration-tuning", 2600, "Migration planner · α/β/γ presets & roadmap sort");
await page.locator(".migration-table-section").scrollIntoViewIfNeeded();
await sleep(400);
await focusCards(page, ".migration-table-section table thead", 2400, "Migration planner · scheduled steps & package priority");

// 11. CI policy — Query Guide check workflow
await clickTab(page, "Query Guide");
await page.waitForSelector(".guide-view", { timeout: 15000 });
const checkSection = page.locator(".guide-view section", { hasText: "Blast radius" });
if (await checkSection.count()) {
  await checkSection.first().scrollIntoViewIfNeeded();
  await sleep(400);
}
await focusCards(page, ".guide-view pre.guide-cli-pre, .guide-view .card", 2400, "CI policy · check · blast-radius gates");

// 12. Export — Query Guide graph export block
const graphGuide = page.locator(".guide-view section", { hasText: "Graph visualization" });
if (await graphGuide.count()) {
  await graphGuide.first().scrollIntoViewIfNeeded();
  await sleep(400);
}
await focusCards(page, "#cli-graph pre, .guide-view section#cli-graph", 2200, "Export · GraphML · Mermaid · JSON subgraphs");

// Outro — stat cards again
await page.locator(".rb-stats-row").first().scrollIntoViewIfNeeded();
await focusCards(page, ".rb-stats-row .stat-card", 2000, "rBuilder · one index · every structural question");

await page.evaluate(() => {
  document.getElementById("rb-demo-caption")?.remove();
});
await clearHighlights(page);

const video = page.video();
await context.close();
await browser.close();

if (!video) throw new Error("Playwright did not return a video handle");

const saved = await video.path();
fs.renameSync(saved, RAW_WEBM);

const probe = spawnSync(
  "ffprobe",
  ["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", RAW_WEBM],
  { encoding: "utf8" },
);
const rawDur = parseFloat(probe.stdout.trim() || "0");
// Fit 25–30s: use native length when already in range; otherwise speed-compress
// (never truncate — that dropped blast radius and later tabs from prior runs).
let vf = "fps=30,scale=1280:720:flags=lanczos";
let encodeMode = "native";
if (rawDur > TARGET_SECS + 0.5) {
  const factor = rawDur / TARGET_SECS;
  vf = `setpts=PTS/${factor},fps=30,scale=1280:720:flags=lanczos`;
  encodeMode = `speedup_${factor.toFixed(2)}x`;
} else if (rawDur < 25) {
  encodeMode = "short_native";
}

const ffArgs = [
  "-y",
  "-i",
  RAW_WEBM,
  "-vf",
  vf,
  "-c:v",
  "libx264",
  "-preset",
  "fast",
  "-crf",
  "22",
  "-pix_fmt",
  "yuv420p",
  "-movflags",
  "+faststart",
];
if (rawDur > TARGET_SECS + 0.5) {
  ffArgs.push("-t", String(TARGET_SECS));
} else if (rawDur >= 25 && rawDur <= 30) {
  ffArgs.push("-t", String(rawDur));
}

const ff = spawnSync("ffmpeg", [...ffArgs, OUT_MP4], { encoding: "utf8" });

if (ff.status !== 0) {
  console.error(ff.stderr);
  throw new Error("ffmpeg encode failed");
}

const finalProbe = spawnSync(
  "ffprobe",
  ["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", OUT_MP4],
  { encoding: "utf8" },
);

console.log(
  JSON.stringify(
    {
      dashboard: BASE,
      output: OUT_MP4,
      raw_duration_s: rawDur,
      final_duration_s: parseFloat(finalProbe.stdout.trim() || "0"),
      target_secs: TARGET_SECS,
      encode_mode: encodeMode,
      functions: { FN, FN_BLAST, FN_TAINT },
      features: [
        "discover (stat cards + High Blast Radius)",
        "GQL",
        "graph metrics",
        "CFG",
        "PDG",
        "dominance",
        "program slicing",
        "blast radius (metric cards)",
        "taint analysis",
        "migration planner",
        "CI policy (check)",
        "export",
      ],
    },
    null,
    2,
  ),
);
