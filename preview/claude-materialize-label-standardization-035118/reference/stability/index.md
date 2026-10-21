# Stability Guarantees and Deprecation Policy




# Stability Guarantees and Deprecation Policy

This page describes which parts of `materialize-monitoring` you can rely on, and how the project retires an interface before it changes.

<!-- more -->

<blockquote class="book-hint note">
The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT",
"SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this
document are to be interpreted as described in
<a href="https://datatracker.ietf.org/doc/html/rfc2119" rel="external" class="external-link">RFC 2119</a>.
</blockquote>


## Semantic artifact versioning

This repository publishes several release artifacts, and each one carries its own [SemVer](https://semver.org/) version.
An artifact bumps its version for the following kinds of change:

Major
: A breaking change to a [public interface](#public-interfaces), after its [deprecation](#deprecations) cycle.

Minor
: New functionality, or another non-breaking change.

Patch
: A backward-compatible bug fix, or an inconsequential change.

<!-- TODO: create a reference/components.md page to link to -->
Most consumers depend on the `materialize-monitoring` Helm chart and Terraform module, which share one version.
The guarantees on this page apply from v1.0.0 of the chart and module.

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
* Artifact and OCI names (chart name and container image repository; not tags)
* Documented Loki stream labels
* Documented Alert names
* Documented Recording Rules

> [!NOTE]
> Any undocumented breakage of above MUST be considered a bug and can be filed to
> our [issue tracker](https://github.com/MaterializeInc/materialize-monitoring/issues).

## Deprecations

`materialize-monitoring` MUST give notice before it removes a public interface, and retires one in the following order:

1. It MUST announce the deprecation in the [changelog](../changelog/), in a `**Deprecated:**` bullet that names the replacement.
2. It MUST keep the deprecated interface working for at least 30 days, and MUST NOT remove it before the next major version.
3. It MUST record the removal in the changelog, in a `**Removed:**` bullet.

> [!WARNING]
> An interface known to have no consumers MAY be treated as unstable retroactively, and removed without a full deprecation cycle.

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

