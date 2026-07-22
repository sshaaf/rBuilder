# ecommerce-c

C reference fixture for rBuilder Tier 1 language support.

Layered REST-style ecommerce API (SQLite + service/repository pattern) used by
`rbuilder discover --all -l c` dashboard gates.

## Layout

- `include/ecommerce/` — headers (models, repositories, services, handlers)
- `src/` — implementations

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/c.md) · [HTML](../rbuilder-reports/languages/c.html) (2026-07-22).

```bash
rbuilder -f json discover . --cfg -e build,cmake-build-debug,.rbuilder
rbuilder -f json blast-radius 'src/coolstore/services/shopping_cart_service.c::price_shopping_cart'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 84 |
| Nodes | 486 |
| Edges | 838 |
| Discover ms | 309 |
| Cache MB | 0.92 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✗ |
| check | ✓ |
| slice / taint | — / ✓ |

### Top symbols

| Symbol | Score | Callers | Impact |
|--------|------:|--------:|-------:|
| `seed` | 25.15 | 1 | 3 |
| `correctness_leaf` | 25.10 | 1 | 2 |
| `init_shopping_cart_for_pricing` | 25.10 | 1 | 2 |
| `round2` | 25.05 | 1 | 1 |
| `is_post` | 25.05 | 1 | 1 |

C fixture with CoolStore /services cart pricing mutations.

Raw: [`../rbuilder-reports/c-summary.json`](../rbuilder-reports/c-summary.json)
