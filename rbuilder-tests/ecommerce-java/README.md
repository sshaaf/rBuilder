# ecommerce-java

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/java.md) · [HTML](../rbuilder-reports/languages/java.html) (2026-07-22).

```bash
rbuilder -f json discover . --cfg -e target,data
rbuilder -f json blast-radius 'src/main/java/com/example/ecommerce/service/OrderService.java::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 66 |
| Nodes | 993 |
| Edges | 2211 |
| Discover ms | 307 |
| Cache MB | 1.89 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✗ |
| check | ✓ |
| slice / taint | ◐ / ✓ |

### Top symbols

| Symbol | Score | Callers | Impact |
|--------|------:|--------:|-------:|
| `findByEmail` | 40.85 | 3 | 17 |
| `currentUser` | 40.70 | 7 | 14 |
| `getProductByItemId` | 40.45 | 2 | 9 |
| `getRole` | 40.35 | 6 | 7 |
| `getUserCart` | 40.30 | 5 | 6 |

Strongest CALLS graph and community modularity in this suite.

Raw: [`../rbuilder-reports/java-summary.json`](../rbuilder-reports/java-summary.json)
