# ecommerce-cpp

C++ reference fixture for rBuilder Tier 1 language support.

Layered ecommerce API (SQLite + service/repository pattern) mirroring ecommerce-c.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/cpp.md) · [HTML](../rbuilder-reports/languages/cpp.html) (2026-07-22).

```bash
rbuilder -f json discover . --cfg -e build,cmake-build-debug,.rbuilder
rbuilder -f json blast-radius 'src/coolstore/services/shopping_cart_service.cpp::priceShoppingCart'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 81 |
| Nodes | 638 |
| Edges | 1224 |
| Discover ms | 308 |
| Cache MB | 0.96 |

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
| `correctnessLeaf` | 25.10 | 1 | 2 |
| `initShoppingCartForPricing` | 25.10 | 1 | 2 |
| `correctnessMid` | 25.05 | 1 | 1 |
| `cart_delete` | 25.05 | 1 | 1 |
| `getShoppingCart` | 25.05 | 1 | 1 |

C++ fixture with CoolStore /services cart pricing mutations.

Raw: [`../rbuilder-reports/cpp-summary.json`](../rbuilder-reports/cpp-summary.json)
