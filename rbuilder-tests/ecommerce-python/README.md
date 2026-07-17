# ecommerce-python

E-commerce reference app.

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · [language report](../rbuilder-reports/languages/python.md) · [HTML](../rbuilder-reports/languages/python.html) (2026-07-07).

```bash
rbuilder -f json discover . --cfg -e .venv,__pycache__
rbuilder -f json blast-radius 'app/services/order.py::checkout'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 54 |
| Nodes | 189 |
| Edges | 270 |
| Discover ms | 120 |
| Cache MB | 2.41 |

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

Second full CFG/PDG language; rich class nodes.

Raw: [`../rbuilder-reports/python-summary.json`](../rbuilder-reports/python-summary.json)
