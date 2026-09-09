# Stability Guarantees and Deprecation Policy




# Stability Guarantees and Deprecation Policy

This page describes what functionality provided by `materialize-monitoring`
a user can reasonably rely on.

<!-- more -->

<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


## Semantic Artifact Versioning

This repository is broken up into multiple different release artifacts all
with their own discrete [SemVer](https://semver.org/) versions.
Generally:
* Breaking changes are tracked by major version bumps
* New functionality / non-breaking changes are tracked by minor version bumps
* Backwards-compatible bug fixes / inconsequential changes are tracked by patch version bumps

<!-- TODO: create a reference/components.md page to link to -->
Most consumers will be concerned specifically with the `materialize-monitoring` Helm chart and Terraform modules.

> [!WARNING]
> Before the Helm/TF 1.0 release, minor versions carry both breaking and non-breaking features.
> `materialize-monitoring` SHOULD try to adhere to notices of breaking changes and
> deprecations, but can only provide best-effort guarantees before 1.0.

## Public Interfaces

Public interfaces are the pieces of functionality that a direct consumer would use
to perform a task, without a layer of further indirection.
Traditionally, these are things like APIs, but in the context of observability
there's a lot of other ways that a consumer would interact with the monitoring
systems.

This list describes some of the interfaces (non-comprehensive) a consumer may build upon reliably:
* Documented Terraform module inputs and outputs
* Documented `canonical` Prometheus queries (actual PromQL expressions)
    * `best-effort` queries SHOULD provide release notes but are more free to change
* Dashboard identities (Grafana UIDs, which show up in URLs)
* Annotation namespace (`monitoring.materialize.cloud/*`)
* Artifact and OCI names (Chart name, container image repo, etc.)
* Documented Loki stream labels
* Documented Alert names
* Documented Recording Rules

> [!NOTE]
> Any undocumented breakage of above MUST be considered a bug and can be filed to
> our [issue tracker](https://github.com/MaterializeInc/materialize-monitoring/issues).

## Deprecations

`materialize-monitoring` MUST adequately provide notice for any such deprecations
that will make room for breakages in behavior.
Each Release in [Changelog](../changelog/) may carry a `**Deprecated**:` notation
indicating the slated removal of a public interface.

Our policy is as follows (post 1.0):
1. Deprecated interfaces MUST be clearly documented in the release notes using a `**Deprecated**:` notation.
2. At least one major version and at least 30 days MUST pass before a deprecated interface is removed.
3. A removed interface MUST use the `**Removed**:` notation in our release notes after being successfully deprecated.

> [!WARNING]
> Interfaces which are known to not have any consumers MAY be treated as unstable
> retroactively and removed without a full deprecation cycle.

## Unstable Interfaces

Some interfaces may be annotated with Unstable, Experimental, or Internal.
These interfaces are not subject to the same stricter guarantees about stability.

These interfaces MAY have accompanying documentation.

The following interfaces (non-exhaustively) may be considered unstable:
* Helm values
* Container Image tags
* Extended and Diagnostic Metric tiers
* Upstream Metric names and labels
    * Particularly `mz_*` metrics from Materialize have their own release cadence
    * Changes to these are RECOMMENDED to be documented
* Undocumented Alerts
* Undocumented Recording Rules
* Dashboard variables
* Loki structured metadata
    * Changes that affect labels MUST be considered a breaking change
* Styles/theming/positioning of panels within dashboards

## Intermediate Artifacts

Some components in `materialize-monitoring` are subsumed into other components.
Breaking changes in those artifacts are not necessarily breaking changes
in the component that contains them.
For example, a breaking change within the query registry may not be represented
as a fully breaking change to the downstream helm charts as the dashboards fully
abstract such changes.
This example would still have an entry within the changelog, however, as
dependent components are shown within a release.

