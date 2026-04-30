# Release Process

`pg-logstats` uses Buildkite for normal CI and GitHub Actions only for tagged
release publishing.

## First Publish

The first crates.io publish must be done manually with a crates.io API token.
Do not rely on `.github/workflows/release.yml` for the first publish because
Trusted Publishing can only be configured after the crate exists on crates.io.

```bash
cargo login
make check
cargo publish --locked
```

## Install Smoke

`make install-smoke` is not part of the normal development loop. It exists for
release readiness: it installs the crate into `target/install-smoke` with
`cargo install --path .`, then checks that the installed `pg-logstats` binary can
print `--version` and `--help`.

Run it manually before the first publish, or when changing packaging,
dependencies, binary names, or CLI startup behavior:

```bash
make install-smoke
```

After the crate exists on crates.io, configure Trusted Publishing for:

- owner: `vrajat`
- repository: `pg-logstats`
- workflow: `release.yml`
- environment: `crates-io`

## Tagged Release

Later releases use `cargo-release` to create the version bump commit, tag, and
push. Install it before running the release targets:

```bash
cargo install cargo-release
```

Preview the release without pushing a tag or publishing:

```bash
make release-dry-run LEVEL=patch
```

Execute the release from `main`:

```bash
make release LEVEL=patch
```

`make release` does not publish directly. It runs `cargo release ... --no-publish`
and pushes the release tag. The tag starts `.github/workflows/release.yml`,
which verifies the release candidate, runs an install smoke check in GitHub
Actions, publishes the crate, and creates a GitHub release.

Homebrew formula work should wait until the crates.io release path is stable.
