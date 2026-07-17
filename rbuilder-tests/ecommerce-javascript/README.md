# ecommerce-javascript

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/javascript.md) · [HTML](../rbuilder-reports/languages/javascript.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e node_modules
rbuilder -f json blast-radius 'src/services/orderService.js::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 44 |
| Nodes | 1267 |
| Edges | 2444 |
| Discover ms | 98 |
| Cache MB | 2.72 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✓ |
| check | ✓ |
| slice / taint | — / — |

### Top symbols

_No functions with score > 0._

Mirror of TypeScript graph without types.

Raw: [`../rbuilder-reports/javascript-summary.json`](../rbuilder-reports/javascript-summary.json)
