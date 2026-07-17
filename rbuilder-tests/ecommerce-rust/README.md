# ecommerce-rust

Axum + SQLx e-commerce API for rBuilder test indexing.

## Run

```bash
cargo run
# http://localhost:8080/health
```

## Environment

| Variable | Default |
|----------|---------|
| `DATABASE_URL` | `sqlite:ecommerce.db` |
| `JWT_SECRET` | `dev-secret-change-me` |
| `BIND_ADDR` | `0.0.0.0:8080` |

## Demo data

Seeds **Electronics** category with **Wireless Headphones** and **USB-C Hub** on first run.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/rust.md) · [HTML](../rbuilder-reports/languages/rust.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e target
rbuilder -f json blast-radius 'src/services/order.rs::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 51 |
| Nodes | 195 |
| Edges | 293 |
| Discover ms | 118 |
| Cache MB | 2.08 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✓ |
| check | ✓ |
| slice / taint | ◐ / ✓ |

### Top symbols

_No functions with score > 0._

Best deep-analysis reference (CFG/PDG/inspect/taint).

Raw: [`../rbuilder-reports/rust-summary.json`](../rbuilder-reports/rust-summary.json)
