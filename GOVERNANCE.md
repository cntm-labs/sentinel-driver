# sentinel-driver Governance

## 1. API Stability Contract (v1.x)

`sentinel-driver` is in the 1.x line. **All changes must be additive** until v2.

### Allowed in a minor release
- New methods on existing structs or traits (with default implementations for traits).
- New `pub` structs, traits, enums — provided they carry `#[non_exhaustive]`.
- New cargo features, **default-off**.
- New associated type defaults.
- A symbol may be renamed via `#[deprecated(since, note)]` alias. The old symbol persists until the next major version.

### Forbidden in a minor release
- Changing the signature of any `pub` item.
- Removing or renaming a `pub` symbol without a deprecated alias.
- Adding a required method to a `pub` trait (default-method bodies are fine).
- Changing the default cargo feature set.
- Adding a new generic bound to an existing generic.

## 2. Deprecation Flow

A symbol marked `#[deprecated]` must remain for **at least one minor version** before removal, which can only happen at the next major version.

## 3. Release Cadence

- **Monthly** minor release if there are unreleased commits.
- **Hotfix SLA: 48 hours** for security-impacting issues.
- Releases are orchestrated by `release-please`.

## 4. Breaking-Change Process (v2)

1. Open an RFC issue describing the break and migration path.
2. Comment window of **14 days minimum**.
3. Maintainer super-majority approval.
4. Work proceeds on a `v2` branch; `main` remains v1.x.
