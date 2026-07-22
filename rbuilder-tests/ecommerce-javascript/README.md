# ecommerce-javascript

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/javascript.md) · [HTML](../rbuilder-reports/languages/javascript.html) (2026-07-22).

```bash
rbuilder -f json discover . --cfg -e node_modules
rbuilder -f json blast-radius 'src/services/orderService.js::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 60 |
| Nodes | 1440 |
| Edges | 2843 |
| Discover ms | 303 |
| Cache MB | 1.74 |

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
| `getDb` | 40.80 | 16 | 16 |
| `asyncHandler` | 40.45 | 9 | 9 |
| `nowIso` | 40.35 | 6 | 7 |
| `createShoppingCartItem` | 40.10 | 2 | 2 |
| `correctnessLeaf` | 25.10 | 1 | 2 |

Mirror of TypeScript graph without types.

Raw: [`../rbuilder-reports/javascript-summary.json`](../rbuilder-reports/javascript-summary.json)
