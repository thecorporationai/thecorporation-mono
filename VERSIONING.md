# Versioning

This project uses **CalVer** (Calendar Versioning), pip-style.

## Format

```
YY.M.MICRO
```

| Segment | Meaning                              | Example |
|---------|--------------------------------------|---------|
| `YY`    | 2-digit year                         | `26`    |
| `M`     | Month, no leading zero               | `3`     |
| `MICRO` | Incrementing release number per month | `1`     |

## Examples

| Tag      | Meaning                        |
|----------|--------------------------------|
| `26.3.1` | First release of March 2026    |
| `26.3.2` | Second release of March 2026   |
| `26.4.1` | First release of April 2026    |

## Rules

1. Tags are applied at the monorepo root and apply to the whole repo.
2. The `MICRO` number resets to `1` at the start of each new month.
3. Tags are created on the default branch only.
4. Use `git tag -a YY.M.MICRO -m "description"` for annotated tags.

## Reference

- [CalVer specification](https://calver.org)
- This convention follows the pip-style variant (`YY.M.MICRO`).
