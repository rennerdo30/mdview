# Dependency Audit

This note records the current duplicate-transitive-dependency review. It is meant to make dependency update decisions repeatable without turning routine lockfile maintenance into a risky migration project.

## Current Duplicate Groups

Run:

```bash
cargo tree -d
```

As of this audit, the duplicate groups are primarily upstream stack splits:

- `image` `0.24.x` and `0.25.x`
  - `printpdf` depends on `image 0.24`.
  - `eframe` and mdview use `image 0.25`.
- `png`, `gif`, and `tiff`
  - These duplicate because the two `image` major lines pull different codec versions.
- macOS/windowing crates: `objc2`, `objc2-app-kit`, `objc2-foundation`, `block2`, `bitflags`, and `core-foundation`
  - These come from `eframe`/`winit`/`glutin`/`rfd`/`muda` version combinations.
- `thiserror` `1.x` and `2.x`
  - `muda` still pulls `thiserror 1.x`; `syntect` and AVIF-related image dependencies pull `2.x`.
- `owned_ttf_parser`/`ttf-parser`
  - `printpdf` and egui text rendering depend on different compatible lines.
- `getrandom` and `webpki-roots`
  - These are pulled by unrelated networking/tempfile/TLS paths.

## Decision

No direct dependency change is justified solely for deduplication right now. The large duplicate groups are caused by ecosystem boundaries between GUI/windowing, PDF export, image codecs, and platform integrations. Forcing upgrades or dependency replacement here would have more regression risk than expected performance or binary-size benefit.

The best low-risk path is:

- Keep using `cargo update` for compatible lockfile refreshes.
- Re-check `cargo tree -d` after upgrading top-level GUI, image, PDF, dialog, or menu crates.
- Prefer deduplication only when it follows naturally from a planned top-level dependency upgrade.
- Revisit `printpdf` image-stack duplication when evaluating PDF image support improvements.

## Validation

After dependency changes, run the workflow in [DEPENDENCY_UPDATES.md](DEPENDENCY_UPDATES.md).
