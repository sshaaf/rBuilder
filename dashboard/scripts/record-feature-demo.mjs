/**
 * Record an rBuilder dashboard feature montage — 5 seconds per feature.
 *
 * Each segment highlights the active tab in `.rb-main-tabs` and the tab's
 * main content panel so viewers can see which area belongs to which feature.
 *
 * Prereq:
 *   rbuilder -r /path/to/gbuilder discover . --all
 *   rbuilder -r /path/to/gbuilder semantic index   # Search tab
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
const SEC_PER_FEATURE = Number(process.env.DEMO_SEC_PER_FEATURE ?? "5");
const HOLD_MS = Math.round(SEC_PER_FEATURE * 1000);

const FN = process.env.CAPTURE_FN_DATAFLOW ?? "addEmbeddingSimilarityEdges";
const FN_BLAST = process.env.CAPTURE_FN_BLAST ?? "addEmbeddingSimilarityEdges";
const FN_TAINT = process.env.CAPTURE_FN_TAINT ?? "clearFileGraph";
const SEMANTIC_QUERY = process.env.CAPTURE_SEMANTIC_QUERY ?? "embedding similarity graph";
const SLICE_LINE = process.env.CAPTURE_SLICE_LINE ?? "45";
const SLICE_VAR = process.env.CAPTURE_SLICE_VAR ?? "threshold";

/** One segment per dashboard feature (order matches README feature list). */
const FEATURE_SEGMENTS = [
  { key: "discover", tab: null, panel: ".rb-stats-row", caption: "discover · graph snapshot & index metrics" },
  { key: "gql", tab: "Graph Visualization", panel: ".graph-panel.h-100", caption: "GQL · package call graph" },
  { key: "semantic-search", tab: "Search", panel: ".search-view", caption: "Semantic search · code-daemon · fusion ranking" },
  { key: "graph-metrics", tab: "Functions", panel: ".functions-view, .functions-table", caption: "Graph metrics · PageRank · betweenness · blast" },
  { key: "cfg", tab: "CFG / PDG Analysis", panel: ".cfg-detail, .cfg-graph-panel", caption: "CFG · control-flow blocks & dominators" },
  { key: "pdg", tab: "Dataflow", panel: ".dataflow-graph-panel", caption: "PDG · data & control dependencies" },
  { key: "dominance", tab: "Dataflow", panel: ".dataflow-graph-panel", caption: "Dominance · dominator tree & frontiers" },
  { key: "program-slicing", tab: "Program Slicing", panel: ".slice-view", caption: "Program slicing · criterion & highlighted lines" },
  { key: "blast-radius", tab: "Blast Radius", panel: ".blast-view", caption: "Blast radius · impact score & caller table" },
  { key: "taint", tab: "Taint Analysis", panel: ".taint-view", caption: "Taint analysis · source → sink flows" },
  { key: "migration", tab: "Migration", panel: ".migration-view, .migration-tuning", caption: "Migration planner · presets & package roadmap" },
  { key: "ci-policy", tab: "Query Guide", panel: ".guide-view", caption: "CI policy · check · blast-radius gates" },
  { key: "export", tab: "Query Guide", panel: ".guide-view", caption: "Export · GraphML · Mermaid · JSON subgraphs" },
];

const TARGET_SECS = FEATURE_SEGMENTS.length * SEC_PER_FEATURE;

fs.mkdirSync(OUT_DIR, { recursive: true });

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function clickTab(page, label) {
  const tab = page.locator(".rb-main-tabs").getByRole("button", { name: label, exact: true });
  await tab.scrollIntoViewIfNeeded();
  await tab.click();
  await sleep(350);
}

async function selectFunction(page, name) {
  const search = page.locator('.function-list-sidebar input[type="search"]');
  if (await search.count()) {
    await search.fill("");
    await sleep(120);
    await search.fill(name);
    await sleep(400);
  }
  const item = page.locator(".function-list-item", {
    has: page.locator(".function-list-item-name", { hasText: name }),
  });
  if ((await item.count()) > 0) {
    await item.first().click();
    await sleep(450);
    return;
  }
  const fallback = page.locator(".function-list-item").first();
  if (await fallback.count()) {
    await fallback.click();
    await sleep(450);
  }
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
  await sleep(1200);
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
  await sleep(400);
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

/** Highlight active tab button + main panel for HOLD_MS. */
async function focusTabAndPanel(page, tabLabel, panelSelector, caption) {
  await setCaption(page, caption);

  if (tabLabel) {
    await clickTab(page, tabLabel);
  }

  await page.evaluate(
    ({ tabLabel, panelSelector }) => {
      const styleHighlight = (el) => {
        el.setAttribute("data-rb-demo-highlight", "1");
        el.style.outline = "3px solid #0d6efd";
        el.style.outlineOffset = "3px";
        el.style.boxShadow = "0 0 0 6px rgba(13, 110, 253, 0.15)";
      };

      const tabBar = document.querySelector(".rb-main-tabs");
      if (tabBar) {
        styleHighlight(tabBar);
        tabBar.scrollIntoView({ block: "nearest", behavior: "instant" });
      }

      if (tabLabel) {
        for (const btn of document.querySelectorAll(".rb-main-tabs .nav-link")) {
          const label = btn.querySelector("span")?.textContent?.trim() ?? btn.textContent?.trim();
          if (label === tabLabel) {
            styleHighlight(btn);
          }
        }
      }

      const workspace = document.querySelector(".rb-tab-workspace");
      if (workspace) styleHighlight(workspace);

      const panelCard = document.querySelector(".rb-tab-panel-card");
      if (panelCard) styleHighlight(panelCard);

      for (const sel of panelSelector.split(",").map((s) => s.trim())) {
        const panel = document.querySelector(sel);
        if (panel) {
          panel.scrollIntoView({ block: "nearest", behavior: "instant" });
          styleHighlight(panel);
          break;
        }
      }
    },
    { tabLabel, panelSelector },
  );

  await sleep(HOLD_MS);
  await clearHighlights(page);
}

async function prepareSegment(page, key) {
  try {
    switch (key) {
      case "graph-metrics": {
        const prBtn = page.getByRole("button", { name: /Sort by PR/i });
        if (await prBtn.count()) await prBtn.click();
        await sleep(300);
        break;
      }
      case "cfg": {
        await selectFunction(page, FN);
        const loadCfg = page.getByRole("button", { name: /Load CFG graph/i });
        if (await loadCfg.count()) await loadCfg.click();
        await page.locator(".cfg-detail").first().waitFor({ state: "visible", timeout: 25000 }).catch(() => {});
        await sleep(600);
        break;
      }
      case "pdg": {
        await selectFunction(page, FN);
        const dfView = page.locator("#df-view");
        if (await dfView.count()) {
          await dfView.selectOption("dataflow");
          await page.locator(".dataflow-graph-panel").waitFor({ state: "visible", timeout: 20000 }).catch(() => {});
        }
        await sleep(500);
        break;
      }
      case "dominance": {
        const dfView = page.locator("#df-view");
        if (await dfView.count()) {
          await dfView.selectOption("dominator");
          await sleep(700);
        }
        break;
      }
      case "program-slicing": {
        await selectFunction(page, FN);
        await page.locator("#slice-line").fill(String(SLICE_LINE));
        await page.locator("#slice-var").fill(SLICE_VAR);
        await page.getByRole("button", { name: "Compute slice" }).click();
        await page.getByText(/slice:/i).waitFor({ state: "visible", timeout: 15000 }).catch(() => {});
        await sleep(400);
        break;
      }
      case "blast-radius": {
        await waitWasm(page);
        await selectFunction(page, FN_BLAST);
        await waitForBlastResults(page);
        break;
      }
      case "taint": {
        await selectFunction(page, FN_TAINT);
        await page.locator(".taint-view table tbody tr").first().waitFor({ state: "visible", timeout: 15000 }).catch(() => {});
        await page.locator(".taint-view table tbody tr").first().click().catch(() => {});
        await sleep(350);
        break;
      }
      case "migration": {
        await page.waitForSelector(".migration-tuning, .migration-view", { timeout: 20000 }).catch(() => {});
        await sleep(400);
        break;
      }
      case "semantic-search": {
        const input = page.locator('.search-view input[type="search"]');
        await input.waitFor({ state: "visible", timeout: 15000 });
        if (await input.isEnabled()) {
          await input.fill(SEMANTIC_QUERY);
          await page.locator('.search-view button[type="submit"]').click();
          await page.locator(".search-results tbody tr").first().waitFor({ state: "visible", timeout: 30000 }).catch(() => {});
          await sleep(400);
        }
        break;
      }
      case "ci-policy": {
        const section = page.locator(".guide-view section", { hasText: "Blast radius" });
        if (await section.count()) await section.first().scrollIntoViewIfNeeded();
        await sleep(300);
        break;
      }
      case "export": {
        const section = page.locator(".guide-view section", { hasText: "Graph visualization" });
        if (await section.count()) await section.first().scrollIntoViewIfNeeded();
        await sleep(300);
        break;
      }
      default:
        break;
    }
  } catch (err) {
    console.warn(`prepareSegment(${key}) skipped:`, err.message ?? err);
  }
}

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 720 },
  recordVideo: { dir: OUT_DIR, size: { width: 1280, height: 720 } },
});
const page = await context.newPage();

await page.goto(BASE, { waitUntil: "networkidle", timeout: 120000 });
await waitWasm(page);

for (const segment of FEATURE_SEGMENTS) {
  if (segment.tab) {
    await clickTab(page, segment.tab);
  }
  await prepareSegment(page, segment.key);
  await focusTabAndPanel(page, segment.tab, segment.panel, segment.caption);
}

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

let vf = "fps=30,scale=1280:720:flags=lanczos";
let encodeMode = "native";
if (rawDur > TARGET_SECS + 1) {
  const factor = rawDur / TARGET_SECS;
  vf = `setpts=PTS/${factor},fps=30,scale=1280:720:flags=lanczos`;
  encodeMode = `speedup_${factor.toFixed(2)}x`;
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
if (rawDur > TARGET_SECS + 1) {
  ffArgs.push("-t", String(TARGET_SECS));
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
      sec_per_feature: SEC_PER_FEATURE,
      target_secs: TARGET_SECS,
      encode_mode: encodeMode,
      features: FEATURE_SEGMENTS.map((s) => s.key),
    },
    null,
    2,
  ),
);
