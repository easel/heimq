# Heimq Release Runbook

This runbook covers the release process for `easel/heimq`.

## Tag naming

Create release tags as `vX.Y.Z`.

- `v` is the Git tag prefix.
- `X.Y.Z` is the version used by the binary, Helm chart, and OCI image tags.
- Treat the tag as immutable once it has been published externally.

## Pre-release verification

Before creating a release tag, run the local validation surface:

```bash
just release-check
```

The release check runs:

- `cargo build -p heimq --release`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `helm lint charts/heimq`
- `helm template heimq charts/heimq`

If you also want to smoke-test the image locally, run:

```bash
just docker-build
./scripts/docker-image-smoke.sh heimq:local
```

## Workflow behavior

Release automation is split across three workflows:

- [.github/workflows/release-binaries.yml](../../.github/workflows/release-binaries.yml) runs on `v*` tag pushes and manual dispatch. It validates format, clippy, and workspace tests before building `heimq --release`, packaging the binary, and creating a GitHub Release.
- [.github/workflows/docker-image.yml](../../.github/workflows/docker-image.yml) runs on `v*` tag pushes and manual dispatch. It builds a local smoke image, runs the image smoke test, and then pushes release tags to GHCR.
- [.github/workflows/helm-chart.yml](../../.github/workflows/helm-chart.yml) validates the chart on pull requests, `main`, tags, and manual dispatch. On `v*` tag pushes it packages the chart and publishes it to GHCR.

## Artifact verification

Verify the release assets after publication:

- GitHub Release assets should include `heimq-linux-x86_64` and `heimq-linux-x86_64.sha256`.
- Validate the checksum with `sha256sum -c heimq-linux-x86_64.sha256`.
- Confirm the release notes correspond to the tagged commit and version.
- Confirm the Helm chart version and `appVersion` match the release tag without the leading `v`.

## Publish locations

- Docker images publish to `ghcr.io/easel/heimq`.
- Helm charts publish to `oci://ghcr.io/easel/charts/heimq`.
- GitHub Release binaries are attached to the release on `easel/heimq`.

## Rollback and recovery

If a release is caught before it is broadly consumed:

1. Delete or supersede the bad GitHub Release.
2. Remove the corresponding package version if the registry entry must be cleared.
3. Retag and republish only if the bad tag has not already been consumed by users or automation.

If the bad release is already in use:

1. Leave the published tag in place.
2. Publish a new patch release tag such as `vX.Y.(Z+1)`.
3. Point users back to the prior known-good Docker image, Helm chart, or GitHub Release asset until the patch release is available.

Do not rewrite an already-published release tag as the normal recovery path.
