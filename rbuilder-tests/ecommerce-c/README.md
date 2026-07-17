# ecommerce-c

C reference fixture for rBuilder Tier 1 language support.

Layered REST-style ecommerce API (SQLite + service/repository pattern) used by
`rbuilder discover --all -l c` dashboard gates.

## Layout

- `include/ecommerce/` — headers (models, repositories, services, handlers)
- `src/` — implementations

## rBuilder

```bash
rbuilder discover --all -r . -l c -v
rbuilder serve -r . --host 127.0.0.1 --port 8080
```
