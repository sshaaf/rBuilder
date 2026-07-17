# rBuilder Test Applications

In-tree Tier‑1 fixtures and graph **correctness** suite for [rBuilder](https://github.com/sshaaf/rBuilder) (copied into this monorepo; not a git submodule).

Reference **e-commerce store** applications for indexing, blast-radius, and migration tests.

Each Tier 1 language project implements the same domain:

| Feature | Description |
|---------|-------------|
| **Users** | Register, login, JWT/session auth |
| **Categories** | Product taxonomy |
| **Products** | Catalog with stock and pricing |
| **Cart** | Per-user shopping cart |
| **Orders** | Checkout and order history |
| **Reviews** | Product ratings and comments |
| **Inventory** | Stock adjustments on checkout |

## Projects

| Directory | Stack | Database | Run |
|-----------|-------|----------|-----|
| [`ecommerce-rust/`](ecommerce-rust/) | Axum + SQLx | SQLite | `cargo run` |
| [`ecommerce-python/`](ecommerce-python/) | FastAPI + SQLAlchemy | SQLite | `uvicorn app.main:app --reload` |
| [`ecommerce-go/`](ecommerce-go/) | Gin + GORM | SQLite | `go run ./cmd/server` |
| [`ecommerce-java/`](ecommerce-java/) | Spring Boot + JPA | H2 (file) | `./mvnw spring-boot:run` |
| [`ecommerce-csharp/`](ecommerce-csharp/) | ASP.NET Core + EF Core | SQLite | `dotnet run --project src/Ecommerce` |
| [`ecommerce-c/`](ecommerce-c/) | C + SQLite (layered services/repos) | SQLite | `make` (optional) |
| [`ecommerce-cpp/`](ecommerce-cpp/) | C++ + SQLite (classes/namespaces) | SQLite | `cmake --build build` (optional) |
| [`ecommerce-typescript/`](ecommerce-typescript/) | Express + better-sqlite3 | SQLite | `npm run build && npm start` |
| [`ecommerce-javascript/`](ecommerce-javascript/) | Express + better-sqlite3 | SQLite | `npm start` |

## Shared REST API (conceptual)

```
GET    /health
POST   /api/auth/register
POST   /api/auth/login
GET    /api/categories
POST   /api/categories
GET    /api/products
GET    /api/products/:id
POST   /api/products
GET    /api/cart
POST   /api/cart/items
DELETE /api/cart/items/:productId
POST   /api/orders
GET    /api/orders
GET    /api/orders/:id
GET    /api/products/:id/reviews
POST   /api/products/:id/reviews
```

Use these repos with `rbuilder discover .` to compare graph structure across languages.

## Graph data correctness (expected-facts)

Beyond smoke reports, each `ecommerce-*` app ships hand-labeled facts under
`ecommerce-*/correctness/expected-facts.json` (schema: [`correctness/SCHEMA.md`](correctness/SCHEMA.md)).

Checked by the standard Rust test suite (from the rBuilder repo root):

```bash
cargo test --test graph_correctness
cargo test --test graph_correctness java   # filter by project id
```

Required failures fail the test. Domain edges that extractors still miss (e.g. some checkout→clearCart paths) are `best_effort` warnings. See [rBuilder#26](https://github.com/sshaaf/rBuilder/issues/26).

Regenerate analysis reports:

```bash
./scripts/run_rbuilder_report.sh
# or: RBUILDER=/path/to/rbuilder ./scripts/run_rbuilder_report.py --update-readmes
```

See [`scripts/README.md`](scripts/README.md) for options.

## rBuilder analysis results

Summary: **[rbuilder-reports/REPORT.md](rbuilder-reports/REPORT.md)** · [HTML](rbuilder-reports/REPORT.html) (run 2026-07-07)

**Language reports:** [Rust](rbuilder-reports/languages/rust.md) · [Python](rbuilder-reports/languages/python.md) · [Go](rbuilder-reports/languages/go.md) · [Java](rbuilder-reports/languages/java.md) · [C#](rbuilder-reports/languages/csharp.md) · [TypeScript](rbuilder-reports/languages/typescript.md) · [JavaScript](rbuilder-reports/languages/javascript.md)

### Feature coverage (✓ ok · ◐ partial · — unsupported/n/a)

| Feature | Rust | Py | Go | Java | C# | TS | JS |
|---------|:----:|:--:|:--:|:----:|:--:|:--:|:--:|
| discover (`--cfg`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Dashboard | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| GQL queries | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Metrics (communities + PageRank) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Blast radius | ✓ | ✓ | — | ✓ | ✓ | ✓ | ✓ |
| Export (JSON subgraph) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| CI check (`--policy-file`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Program slice | ◐ | ◐ | — | ◐ | ◐ | — | — |
| Taint analysis | ✓ | ✓ | — | ✓ | ✓ | — | — |
| Inspect CFG | ✓ | ✓ | — | ✓ | ✓ | — | — |
| Inspect PDG | ✓ | ✓ | — | ✓ | ✓ | — | — |
| Inspect dominators | ✓ | ✓ | — | ✓ | ✓ | — | — |
| Serve daemon | — | — | — | — | — | — | — |

### Index size

| Project | Files | Nodes | Edges | Discover ms |
|---------|------:|------:|------:|------------:|
| Rust | 51 | 195 | 293 | 118 |
| Python | 54 | 189 | 270 | 120 |
| Go | 13 | 34 | 42 | 54 |
| Java | 52 | 528 | 1134 | 169 |
| C# | 46 | 272 | 488 | 142 |
| TypeScript | 45 | 1437 | 2782 | 117 |
| JavaScript | 44 | 1267 | 2444 | 98 |

### Blast radius (max score per project)

Full function scan (`--blast-top N`); checkout leaf symbols often score 0.

| Project | Scanned | Score > 0 | Max score | Top symbol |
|---------|--------:|----------:|----------:|------------|
| Rust | 34 | 0 | 0.00 | `—` |
| Python | 50 | 0 | 0.00 | `—` |
| Go | 13 | 0 | 0.00 | `—` |
| Java | 98 | 36 | 40.85 | `findByEmail` |
| TypeScript | 54 | 0 | 0.00 | `—` |
| JavaScript | 54 | 0 | 0.00 | `—` |

Per-project details: [`rbuilder-reports/languages/`](rbuilder-reports/languages/) · each `ecommerce-*/README.md` § **rBuilder**.

Regenerate: `./scripts/run_rbuilder_report.py`
