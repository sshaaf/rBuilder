# ecommerce-go

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/go.md) · [HTML](../rbuilder-reports/languages/go.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e vendor
rbuilder -f json blast-radius 'internal/service/order.go::Checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 13 |
| Nodes | 34 |
| Edges | 42 |
| Discover ms | 54 |
| Cache MB | 1.07 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | — |
| metrics | ✓ |
| export | ✓ |
| check | ✓ |
| slice / taint | — / — |

### Top symbols

_No functions with score > 0._

Partial Go indexing possible; verify file coverage in discover metrics.

Raw: [`../rbuilder-reports/go-summary.json`](../rbuilder-reports/go-summary.json)
