# Dependency Update Workflow

Use this checklist for routine dependency refreshes.

1. Update dependencies:

```bash
cargo update
```

2. Verify the locked dependency set:

```bash
cargo check --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

3. Review duplicate transitive dependencies when dependency churn is large:

```bash
cargo tree -d
```

4. Commit `Cargo.toml` and `Cargo.lock` together when direct dependency
   requirements change. Commit `Cargo.lock` by itself when only transitive
   versions changed.

5. Avoid major framework migrations during routine updates unless they fix a
   security issue, correctness issue, or clear performance problem.
