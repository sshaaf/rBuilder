# ecommerce-typescript

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/typescript.md) · [HTML](../rbuilder-reports/languages/typescript.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e node_modules,dist
rbuilder -f json blast-radius 'src/services/orderService.ts::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 45 |
| Nodes | 1437 |
| Edges | 2782 |
| Discover ms | 117 |
| Cache MB | 2.94 |

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

High AST node count; compare with JavaScript sibling.

Raw: [`../rbuilder-reports/typescript-summary.json`](../rbuilder-reports/typescript-summary.json)
