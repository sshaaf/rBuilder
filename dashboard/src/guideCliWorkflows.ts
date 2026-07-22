import type { TabId } from "./tabDocs";

export interface CliCommandBlock {
  comment?: string;
  commands: string[];
}

export interface CliWorkflowSection {
  /** Dashboard tab this mirrors (guide uses multiple). */
  tabId: TabId;
  tabLabel: string;
  summary: string;
  prerequisite?: string;
  blocks: CliCommandBlock[];
  notes?: string[];
}

/** Shown at the top of the Query Guide. Prefer `--with-*` (there is no `--all`). */
export const GUIDE_PREREQUISITES = `export REPO="$PWD"   # repository root after discover

# Fast index (graph, blast scores, metrics)
rbuilder discover .

# Dashboard UI + CFG/PDG (Dataflow, CFG, Slice overlays, CPG mutations)
rbuilder discover . --with-cfg --with-dashboard

# Richer pass used by the feature demo / migration tab
rbuilder discover . -l java -e target \\
  --with-cfg --with-security --with-taint --with-dashboard --with-harmonic \\
  --export-migration-hints

# Optional: semantic Search tab (after discover)
rbuilder semantic index --embedder vocab --dimensions 256`;

/**
 * Examples target rbuilder-tests/ecommerce-java (JWT /api/* + CoolStore /services/*).
 * Substitute symbols/paths for other repos.
 */
export const CLI_WORKFLOWS: CliWorkflowSection[] = [
  {
    tabId: "graph",
    tabLabel: "Graph visualization",
    summary:
      "Explore package/community structure, drill into call neighborhoods, and export subgraphs for external tools.",
    prerequisite: "rbuilder discover .",
    blocks: [
      {
        comment: "List functions and orient by name",
        commands: [
          'rbuilder -r "$REPO" gql --macro-name all_functions unused',
          'rbuilder -r "$REPO" gql "MATCH (n:Function) WHERE n.name LIKE \'*Service*\' RETURN n LIMIT 20"',
        ],
      },
      {
        comment: "Named communities (dashboard metagraph labels)",
        commands: [
          'rbuilder -r "$REPO" -f json gql --macro-name all_communities unused | jq ".rows[:5]"',
          'rbuilder -r "$REPO" communities list | head -15',
        ],
      },
      {
        comment: "Call chains (1–3 hops) — same edges the metagraph summarizes",
        commands: [
          'rbuilder -r "$REPO" gql "MATCH (a:Function)-[:CALLS*1..3]->(b:Function) RETURN a,b LIMIT 50"',
          'rbuilder -r "$REPO" gql --macro-name call_chain unused',
          'rbuilder -r "$REPO" gql "MATCH (a:Function)-[:CALLS]->(b:Function) WHERE b.name = \'clearCart\' RETURN a,b"',
        ],
      },
      {
        comment: "Export a subgraph (GraphML, Mermaid) for offline layout",
        commands: [
          'rbuilder -r "$REPO" export --export-format graphml --export-output subgraph.graphml \\',
          '  --query "name:priceShoppingCart"',
          'rbuilder -r "$REPO" export --export-format mermaid --export-output clearCart.mmd \\',
          "  --query 'name:clearCart'",
        ],
      },
      {
        comment: "Many queries in one session — HTTP dashboard + GQL API",
        commands: [
          'rbuilder -r "$REPO" serve --open',
          'curl -sS -X POST http://127.0.0.1:8080/api/query \\',
          "  -H 'Content-Type: application/json' \\",
          "  -d '{\"macro\":\"all_functions\"}' | jq '.count'",
        ],
      },
    ],
    notes: [
      "Dashboard metagraph drill-down has no single CLI command; combine GQL + export, or use `serve` for repeated queries.",
      "`export --query` uses filter syntax (`name:`, `type:`, `functions`, `all`) — not full GQL MATCH.",
      "Add `-f json` on any command for scripts and CI.",
    ],
  },
  {
    tabId: "search",
    tabLabel: "Semantic search",
    summary:
      "Natural-language function search (and community-scoped search) — same as the Search tab after `semantic index`.",
    prerequisite: "rbuilder discover . && rbuilder semantic index --embedder vocab",
    blocks: [
      {
        comment: "Build the Hamming index (offline vocab embedder)",
        commands: ['rbuilder -r "$REPO" semantic index --embedder vocab --dimensions 256'],
      },
      {
        comment: "Query functions",
        commands: [
          'rbuilder -r "$REPO" -f json semantic query "shopping cart checkout" --limit 5 \\',
          "  | jq '.hits[:3] | map({name, score})'",
        ],
      },
      {
        comment: "Scope to communities (Search tab community mode)",
        commands: [
          'rbuilder -r "$REPO" -f json semantic query "shopping cart" --scope community --limit 3 \\',
          "  | jq '.hits | map({name, ranking, score})'",
        ],
      },
    ],
    notes: [
      "With `rbuilder serve`, the Search tab hits `POST /api/semantic/query` on the same index.",
    ],
  },
  {
    tabId: "functions",
    tabLabel: "Function inventory",
    summary:
      "Browse symbols with PageRank, betweenness, harmonic centrality, and blast scores — same columns as the Functions table.",
    prerequisite: "rbuilder discover .",
    blocks: [
      {
        comment: "Inventory all functions",
        commands: [
          'rbuilder -r "$REPO" gql --macro-name all_functions unused',
          'rbuilder -r "$REPO" -f json gql --macro-name all_functions unused | jq ".count"',
        ],
      },
      {
        comment: "Centrality reports (Functions table PR / BC columns)",
        commands: [
          'rbuilder -r "$REPO" metrics --pagerank',
          'rbuilder -r "$REPO" metrics --betweenness',
          'rbuilder -r "$REPO" -f json metrics --pagerank | jq ".pagerank.top[:10]"',
        ],
      },
      {
        comment: "Filter by type or file path",
        commands: [
          'rbuilder -r "$REPO" gql "MATCH (n:Class) RETURN n LIMIT 30"',
          'rbuilder -r "$REPO" gql "MATCH (n:Function) WHERE n.file_path LIKE \'*coolstore*\' RETURN n"',
        ],
      },
    ],
    notes: [
      "Blast scores in the table come from discover; re-run `blast-radius <symbol>` for caller lists at a chosen depth.",
      "JSON shape is `.pagerank.top` / `.betweenness.top` — not `.rows`.",
    ],
  },
  {
    tabId: "cfg",
    tabLabel: "CFG / PDG analysis",
    summary:
      "Inspect control-flow blocks, branches, and dominance inside one function — equivalent to the CFG graph + dominance panel.",
    prerequisite: "rbuilder discover . --with-cfg",
    blocks: [
      {
        comment: "CFG for CoolStore pricing (ecommerce-java)",
        commands: [
          'rbuilder -r "$REPO" inspect priceShoppingCart cfg',
          'rbuilder -r "$REPO" -f mermaid inspect priceShoppingCart cfg',
          'rbuilder -r "$REPO" inspect priceShoppingCart cfg --prune',
        ],
      },
      {
        comment: "JWT /api checkout method",
        commands: [
          'rbuilder -r "$REPO" inspect checkout cfg',
          'rbuilder -r "$REPO" -f json inspect checkout cfg | jq "{blocks: (.nodes|length), edges: (.edges|length)}"',
        ],
      },
      {
        comment: "Dominator tree + frontiers",
        commands: ['rbuilder -r "$REPO" inspect priceShoppingCart dom --frontiers'],
      },
    ],
    notes: [
      "`inspect` takes a **function** symbol only (no `--class`). Prefer unique names (`priceShoppingCart`) or `Class::method`.",
      "Large repos: dashboard loads one function on demand from the CFG archive; CLI `inspect` reads `.rbuilder/analysis/`.",
    ],
  },
  {
    tabId: "dataflow",
    tabLabel: "Dataflow",
    summary:
      "Statement-level PDG / dominator views, plus the Field mutations (CPG) panel for typed writes such as ShoppingCart.",
    prerequisite: "rbuilder discover . --with-cfg --with-dashboard",
    blocks: [
      {
        comment: "CPG status + field mutations (Dataflow → Field mutations panel)",
        commands: [
          'rbuilder -r "$REPO" cpg status',
          'rbuilder -r "$REPO" cpg mutations --type ShoppingCart --exclude-ctors',
          'rbuilder -r "$REPO" -f json cpg mutations --type ShoppingCart --exclude-ctors',
        ],
      },
      {
        comment: "PDG / dataflow edges for pricing",
        commands: [
          'rbuilder -r "$REPO" inspect priceShoppingCart pdg --edge-layer data',
          'rbuilder -r "$REPO" inspect priceShoppingCart pdg --def-use',
          'rbuilder -r "$REPO" -f mermaid inspect priceShoppingCart pdg --edge-layer data',
        ],
      },
      {
        comment: "Dominator tree (Dataflow → Dominator Tree view)",
        commands: ['rbuilder -r "$REPO" inspect priceShoppingCart dom --frontiers'],
      },
      {
        comment: "CALL neighborhood via CPG façade",
        commands: [
          'rbuilder -r "$REPO" cpg function priceShoppingCart',
          'rbuilder -r "$REPO" cpg calls \'ShoppingCartService::priceShoppingCart\'',
        ],
      },
    ],
    notes: [
      "C fixtures: query the struct typedef (`shopping_cart_t`), not `ShoppingCart`.",
      "Empty mutations ⇒ no typed non-ctor writes recovered (try `--include-unresolved`).",
    ],
  },
  {
    tabId: "slice",
    tabLabel: "Program slicing",
    summary:
      "Backward or forward line-level slice for a variable at a line — same as Compute slice in the dashboard.",
    prerequisite: "rbuilder discover . --with-cfg",
    blocks: [
      {
        comment: "Backward slice — JWT cart addItem (ecommerce-java)",
        commands: [
          'rbuilder -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem",
        ],
      },
      {
        comment: "Forward slice",
        commands: [
          'rbuilder -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem --direction forward",
        ],
      },
      {
        comment: "JSON for automation",
        commands: [
          'rbuilder -r "$REPO" -f json slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem | jq .",
        ],
      },
    ],
    notes: [
      "`--function` is the **method** name (`addItem`), not the class name.",
    ],
  },
  {
    tabId: "blast",
    tabLabel: "Blast radius",
    summary:
      "Upstream impact if you change a symbol — impact score, direct callers, and impact zone (dashboard table is depth-limited).",
    prerequisite: "rbuilder discover .",
    blocks: [
      {
        comment: "JWT /api cart clear",
        commands: [
          'rbuilder -r "$REPO" blast-radius \'CartService::clearCart\'',
          'rbuilder -r "$REPO" -f json blast-radius \'CartService::clearCart\' | jq "{score: .metrics.score, callers: .topology.direct_callers}"',
        ],
      },
      {
        comment: "CoolStore /services pricing",
        commands: [
          'rbuilder -r "$REPO" blast-radius \'ShoppingCartService::priceShoppingCart\'',
          'rbuilder -r "$REPO" -f json blast-radius \'ShoppingCartService::priceShoppingCart\' | jq ".metrics"',
        ],
      },
      {
        comment: "Limit caller depth (matches dashboard depth slider)",
        commands: [
          'rbuilder -r "$REPO" blast-radius \'CartService::clearCart\' --depth 1',
          'rbuilder -r "$REPO" blast-radius \'CartService::clearCart\' --depth 5',
        ],
      },
      {
        comment: "CI policy gate on changed functions",
        commands: [
          'rbuilder -r "$REPO" -f json check --policy-file "$REPO/../rbuilder-policy.json" \\',
          "  | jq '{schema_version, violations: (.violations|length)}'",
        ],
      },
    ],
    notes: [
      "Sidebar scores are full-graph metrics from discover; the caller table respects the depth slider.",
      "Prefer `Class::method` when simple names collide.",
    ],
  },
  {
    tabId: "taint",
    tabLabel: "Taint analysis",
    summary:
      "Source-to-sink flows and sanitizer checks per function — requires CFG/PDG from discover.",
    prerequisite: "rbuilder discover . --with-cfg --with-taint --with-dashboard",
    blocks: [
      {
        comment: "On-demand taint at a program point",
        commands: [
          'rbuilder -r "$REPO" slice \\',
          "  src/main/java/com/example/ecommerce/service/CartService.java \\",
          "  --line 53 --variable item --function addItem --taint",
        ],
      },
      {
        comment: "Find CoolStore / JWT endpoints, then trace",
        commands: [
          'rbuilder -r "$REPO" gql "MATCH (n:Function) WHERE n.name LIKE \'*checkout*\' OR n.name LIKE \'*Endpoint*\' RETURN n LIMIT 20"',
          'rbuilder -r "$REPO" slice <file> --line <N> --variable <VAR> --function <method> --taint',
        ],
      },
    ],
    notes: [
      "Dashboard Taint tab lists flows exported at discover time (`--with-taint`); CLI `slice --taint` re-runs analysis on demand.",
    ],
  },
  {
    tabId: "migration",
    tabLabel: "Migration planner",
    summary:
      "Package-level extraction roadmap from communities, centrality, and blast — export plan JSON for agents or CI.",
    prerequisite:
      "rbuilder discover . --with-cfg --with-dashboard --with-harmonic --export-migration-hints",
    blocks: [
      {
        comment: "Default hybrid strategy",
        commands: [
          'rbuilder discover . --with-cfg --with-dashboard --with-harmonic --export-migration-hints',
          'jq ".packages[:5]" .rbuilder/dashboard/migration_plan.json',
        ],
      },
      {
        comment: "Strategy presets (dashboard α/β/γ presets)",
        commands: [
          'rbuilder discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-preset risk_mitigation",
          'rbuilder discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-preset hotspot_first",
          'rbuilder discover . --with-cfg --with-harmonic --export-migration-hints \\',
          "  --migration-order priority",
        ],
      },
    ],
    notes: [
      "There is no `discover --all` — compose `--with-cfg`, `--with-taint`, `--with-dashboard`, `--with-harmonic`, etc.",
      "`--export-migration-plan` remains a deprecated alias of `--export-migration-hints`.",
      "Interactive weight tuning is dashboard-only; re-run discover with presets to refresh CLI exports.",
    ],
  },
  {
    tabId: "guide",
    tabLabel: "GQL reference",
    summary: "Core graph queries used across tabs — patterns, macros, and JSON output.",
    prerequisite: "rbuilder discover .",
    blocks: [
      {
        comment: "Macros (shortcuts)",
        commands: [
          'rbuilder -r "$REPO" gql --macro-name all_functions unused',
          'rbuilder -r "$REPO" gql --macro-name all_communities unused',
          'rbuilder -r "$REPO" gql --macro-name direct_calls unused',
          'rbuilder -r "$REPO" gql --macro-name call_chain unused',
        ],
      },
      {
        comment: "Patterns",
        commands: [
          'rbuilder -r "$REPO" -f json gql "MATCH (n:Function) RETURN n LIMIT 5" | jq ".count"',
          'rbuilder -r "$REPO" gql "MATCH (a:Function)-[:CALLS]->(b:Function) RETURN a,b LIMIT 25"',
          'rbuilder -r "$REPO" gql --explain "MATCH (n:Function) WHERE n.name = \'clearCart\' RETURN n"',
        ],
      },
      {
        comment: "HTTP (same as serve /api/query — GraphQL alias at /graphql)",
        commands: [
          'curl -sS -X POST http://127.0.0.1:8080/api/query \\',
          "  -H 'Content-Type: application/json' \\",
          "  -d '{\"query\":\"MATCH (n:Function) WHERE n.name LIKE \\\"*Cart*\\\" RETURN n LIMIT 10\"}' | jq .",
        ],
      },
    ],
    notes: [
      "`POST /graphql` is an alias of `/api/query`; body is JSON (`query` or `macro`), not a GraphQL schema document.",
    ],
  },
];
