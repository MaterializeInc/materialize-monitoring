---
title: "Alloy"
weight: 10
---

# Materialize Monitoring Alloy

`mzmon-alloy` is a minimal, distroless repackaging of [Grafana Alloy](https://github.com/grafana/alloy) hardened for use in Materialize Monitoring.
It is published to `ghcr.io/materializeinc/mzmon-alloy` and is a drop-in replacement for the upstream `grafana/alloy` image.

Rather than build Alloy from source, the image repackages the signed upstream release binary (verified against the release `SHA256SUMS`) onto a [distroless](https://github.com/GoogleContainerTools/distroless) base.
This keeps us in lockstep with upstream releases while giving us a smaller, non-root, FIPS-capable image.

<!-- Source: packages/alloy/. Built via `make alloy-image`; published by the publish-images GitHub Actions job. -->

## Tags and versioning

Images are tagged `<alloy-version>-<suffix>`, for example `v1.17.0-mz1`.

- `<alloy-version>` is the upstream Alloy release the binary comes from.
- `<suffix>` is the Materialize revision of that repackaging.
  Release images use `mzN` (`mz1`, `mz2`, …); the counter increments when we change the image at a fixed Alloy version and resets to `mz1` on an Alloy upgrade.
- Pull requests that touch the image publish a throwaway build tagged `<alloy-version>-dev.0--pr.g<short-sha>` for pre-release testing.

## Compliance and security posture

The table below summarizes how the image maps to the frameworks we track.
Status legend: ✅ enforced in the image · ⚙️ enforced at deploy/CI (image is compatible) · ⚠️ operator/config-dependent.

| Framework | Control | Status | How it is met |
|---|---|---|---|
| FIPS 140-3 | Validated cryptographic module | ✅ | `alloy-boringcrypto` routes Go crypto through the CMVP-validated BoringCrypto module; build-asserted via `grep boringcrypto`. |
| FIPS 140-3 | Approved-mode operation | ⚠️ | Requires FIPS-approved ciphers / TLS 1.2+ in the overlaid pipeline config. |
| CIS Docker 4.1 | Run as non-root user | ✅ | `USER 473:473` (numeric). |
| CIS Docker 4.2 | Trusted, pinned base image | ✅ | Distroless base, pinned by digest and tracked by Renovate. |
| CIS Docker 4.3 | No unnecessary packages | ✅ | Distroless final stage — no shell or package manager. |
| CIS Docker 4.5 | Image signing | ⚙️ | `cosign` signing in CI (planned). |
| CIS Docker 4.6 | Health check | ⚙️ | No `HEALTHCHECK` (distroless); use a Kubernetes `httpGet` probe on `/-/ready:12345`. |
| CIS Docker 4.7 | No standalone `apt-get update` | ✅ | Combined update+install layer in the builder. |
| CIS Docker 4.8 | No setuid/setgid binaries | ✅ | Binary installed `0755`; distroless base has none. |
| CIS Docker 4.9 | Prefer `COPY` over `ADD` | ✅ | `ADD` used only to fetch the release artifact, then checksum-verified. |
| CIS Docker 4.10 | No secrets in the image | ✅ | None present. |
| CIS Docker 4.11 | Verify downloaded packages | ✅ | Alloy binary verified against upstream `SHA256SUMS`. |
| CIS Kubernetes §5 | Restricted pod `securityContext` | ⚙️ | Image supports the [securityContext](#kubernetes-securitycontext) below (non-root, read-only rootfs, drop all caps). |
| NIST SP 800-190 §4.1.1 | Image vulnerabilities | ⚙️ | Distroless minimizes surface; Trivy/Grype scan gate in CI (planned). |
| NIST SP 800-190 §4.1.2 | Image configuration defects | ✅ | Non-root, minimal, no secrets, no embedded services. |
| NIST SP 800-190 §4.1.3 | Embedded malware / integrity | ✅ | Checksum-verified binary + build provenance + SBOM (signing planned). |
| NIST SP 800-190 §4.1.5 | Use of untrusted images | ✅ | Digest-pinned distroless base; official verified Alloy release. |

### FIPS 140-3

The image ships the `alloy-boringcrypto` release variant, so Alloy's Go cryptography is routed through the CMVP-validated BoringCrypto module.
FIPS mode is engaged automatically at process start (power-on self-tests run at init); no runtime environment variable is required.
The build asserts the FIPS backend is present (`alloy --version | grep boringcrypto`), so a wrong or downgraded release asset fails the build.

The FIPS boundary is Alloy's in-process Go cryptography only.
The distroless base ships Debian's (non-validated) OpenSSL libraries, but Alloy is a pure-Go binary and never calls them, so they are outside the boundary — a FIPS-validated base OS is not required for this workload.
Operating in an approved mode still requires configuration: TLS on `remote_write` and receivers must be constrained to FIPS-approved cipher suites and TLS 1.2+ in the overlaid pipeline config.

### Image hardening

- **Non-root by default.** Runs as uid/gid `473:473` (a numeric user, which satisfies Kubernetes `runAsNonRoot`).
- **Distroless base.** No shell, package manager, or unnecessary packages, minimizing CVE and attack surface (NIST SP 800-190 §4.1).
- **Verified provenance.** The Alloy binary is checksum-verified against the upstream `SHA256SUMS` at build time; base images are pinned by digest and tracked by Renovate.
- **Supply-chain attestations.** Published images carry build provenance (`provenance: mode=max`) and an SBOM.

### Kubernetes securityContext

The image is compatible with a locked-down `securityContext`; set this in the deployment (CIS Kubernetes Benchmark §5):

```yaml
securityContext:
  runAsNonRoot: true
  runAsUser: 473
  allowPrivilegeEscalation: false
  readOnlyRootFilesystem: true
  capabilities:
    drop: ["ALL"]
  seccompProfile:
    type: RuntimeDefault
```

With `readOnlyRootFilesystem: true`, provide writable `emptyDir` volumes for the two paths Alloy writes to:

- `/var/lib/alloy` — the storage path (WAL / remote-config state). Declared as a `VOLUME` in the image.
- `/tmp` — scratch space.

A host-monitoring DaemonSet that scrapes the node (host `/proc`, `/sys`, journal) is a deliberate exception that needs additional mounts and relaxed settings.
The Materialize scraping use case does not, and should run fully unprivileged as above.

## Health checks

The image has no `HEALTHCHECK` — distroless has no shell and Alloy has no self-probe subcommand.
Use a Kubernetes `httpGet` readiness/liveness probe against `/-/ready` on the Alloy HTTP port (`12345`) instead.

## Building locally

```console
make alloy-image
```

This builds the multi-arch image (`linux/amd64`, `linux/arm64`) and smoke-tests `alloy --version` on both.
Publishing to GHCR happens in CI, not from the Makefile.
