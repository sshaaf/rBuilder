# ecommerce-csharp

E-commerce reference app (ASP.NET Core 8 + EF Core SQLite + JWT).

## Run

```bash
cd src/Ecommerce
dotnet run
# or from repo root:
dotnet run --project src/Ecommerce
```

Listens on `http://localhost:5000` (or ports in `launchSettings.json`).

## rBuilder

See [summary report](../rbuilder-reports/REPORT.md) · language report (after `./scripts/run_rbuilder_report.py`).

```bash
rbuilder -f json discover . --cfg -e bin,obj,data
rbuilder -f json blast-radius 'src/Ecommerce/Services/OrderService.cs::CheckoutAsync'
rbuilder -f json metrics --communities --pagerank
rbuilder -f json check --policy-file ../rbuilder-policy.json
```

| Metric | Value |
|--------|------:|
| Files indexed | 46 |
| Nodes | 272 |
| Edges | 488 |
| Discover ms | 142 |

| Feature | Status |
|---------|:------:|
| discover | ✓ |
| blast-radius | ✓ (expected) |
| metrics | ✓ |
| export | ✓ |
| check | ✓ |
| slice / taint / CFG | ✓ |

Regenerate suite reports: [`../scripts/run_rbuilder_report.py`](../scripts/run_rbuilder_report.py)
