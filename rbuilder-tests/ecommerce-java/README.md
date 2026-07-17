# ecommerce-java

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/java.md) · [HTML](../rbuilder-reports/languages/java.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e target,data
rbuilder -f json blast-radius 'src/main/java/com/example/ecommerce/service/OrderService.java::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 52 |
| Nodes | 528 |
| Edges | 1134 |
| Discover ms | 169 |
| Cache MB | 3.83 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ |
| metrics | ✓ |
| export | ✓ |
| check | ✓ |
| slice / taint | ◐ / ✓ |

### Top symbols

| Symbol | Score | Callers | Impact |
|--------|------:|--------:|-------:|
| `findByEmail` | 40.85 | 3 | 17 |
| `currentUser` | 40.70 | 7 | 14 |
| `getRole` | 40.35 | 6 | 7 |
| `getUserCart` | 40.30 | 5 | 6 |
| `parseClaims` | 40.20 | 2 | 4 |

Strongest CALLS graph and community modularity in this suite.

Raw: [`../rbuilder-reports/java-summary.json`](../rbuilder-reports/java-summary.json)
