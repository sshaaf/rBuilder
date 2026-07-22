# ecommerce-go

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/go.md) · [HTML](../rbuilder-reports/languages/go.html) (2026-07-22).

```bash
rbuilder -f json discover . --cfg -e vendor
rbuilder -f json blast-radius 'internal/service/order.go::Checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 46 |
| Nodes | 495 |
| Edges | 1099 |
| Discover ms | 308 |
| Cache MB | 1.16 |

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
| `handleError` | 40.80 | 16 | 16 |
| `MapRepoError` | 40.40 | 8 | 8 |
| `toProductResponse` | 40.15 | 3 | 3 |
| `toCategoryResponse` | 40.15 | 3 | 3 |
| `NewUnauthorized` | 40.10 | 2 | 2 |

Partial Go indexing possible; verify file coverage in discover metrics.

Raw: [`../rbuilder-reports/go-summary.json`](../rbuilder-reports/go-summary.json)
