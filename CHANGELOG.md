# materialize-monitoring Changelog

<!-- This repo uses different versioning streams for its artifacts.
Artifacts are mapped out in packages/components.yaml.
Unreleased sections are placeholders ("_Changes Pending_") until a
version-update/<component> PR populates and releases them; that PR also bumps
the component's version_paths. See reference/internal/versioning.md and
reference/internal/releasing.md.
-->

## Pipelines v0.9.0 (Unreleased)

_Changes Pending_

## Pipelines v0.8.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Update Rust crate regex to v1.13.1
        * [materialize-monitoring#133](https://github.com/MaterializeInc/materialize-monitoring/pull/133)
    * CLO-152 Support importance axis for extracted metrics
        * [materialize-monitoring#132](https://github.com/MaterializeInc/materialize-monitoring/pull/132)
    * Update Rust crate tokio to v1.52.4
        * [materialize-monitoring#131](https://github.com/MaterializeInc/materialize-monitoring/pull/131)
    * Port metric registry to rust
        * [materialize-monitoring#129](https://github.com/MaterializeInc/materialize-monitoring/pull/129)
    * Implement a Query Registry for reducing total metric set
        * [materialize-monitoring#125](https://github.com/MaterializeInc/materialize-monitoring/pull/125)
    * Update Rust crate clap to v4.6.2
        * [materialize-monitoring#124](https://github.com/MaterializeInc/materialize-monitoring/pull/124)

## Prometheus Scrapers v0.4.0 (Unreleased)

_Changes Pending_

## materialize-monitoring Helm Chart v0.7.0 (Unreleased)

_Changes Pending_

## materialize-monitoring Helm Chart v0.6.0

* Allow configuring otlpExporter, googleCloudExporter, datadogExporter
    * [materialize-monitoring#106](https://github.com/MaterializeInc/materialize-monitoring/pull/106)

## materialize-monitoring Helm Chart v0.5.0

* CLO-112 Harden Long-Term storage in GCP
    * [materialize-monitoring#103](https://github.com/MaterializeInc/materialize-monitoring/pull/103)

## Pipelines v0.7.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* CLO-152 Replace prometheus-style pipeline with otelcol for processing
    * [materialize-monitoring#115](https://github.com/MaterializeInc/materialize-monitoring/pull/115)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)

## mzmon-lib (shared library) v0.9.0 (Unreleased)

_Changes Pending_

## Pipelines v0.6.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)

## Pipelines v0.5.0

* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Split loki.write out of main processing pipeline
    * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0

## Prometheus Scrapers v0.3.0

* Enable MZ podmonitors by default
    * [materialize-monitoring#116](https://github.com/MaterializeInc/materialize-monitoring/pull/116)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)

### Dependencies

* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * CLO-152 add schema support for otelcol pipeline blocks
        * [materialize-monitoring#110](https://github.com/MaterializeInc/materialize-monitoring/pull/110)
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Enable alloy pipelines in materialize-monitoring
        * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

## Container Images v0.2.0 (Unreleased)

_Changes Pending_

## mzmon-lib (shared library) v0.8.0

* Update Rust crate jsonschema to 0.47.0
    * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Update Rust crate jsonschema to v0.46.10
    * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)
* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Update dependency grafana-foundation-sdk to v0.0.18
    * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Update Rust crate reqwest to 0.13
    * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)

## materialize-monitoring Helm Chart v0.4.0

* Implement alloy metrics pipelines
    * [materialize-monitoring#96](https://github.com/MaterializeInc/materialize-monitoring/pull/96)
* Pin ghcr.io/materializeinc/mzmon-alloy Docker tag to c47e937
    * [materialize-monitoring#90](https://github.com/MaterializeInc/materialize-monitoring/pull/90)
* Enable alloy pipelines in materialize-monitoring
    * [materialize-monitoring#89](https://github.com/MaterializeInc/materialize-monitoring/pull/89)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
* Included Pipelines @ v0.6.0..v0.7.0
    * Split loki.write out of main processing pipeline
        * [materialize-monitoring#81](https://github.com/MaterializeInc/materialize-monitoring/pull/81)
    * Implement Gateway Pipeline for Logs
        * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)
* Included Prometheus Scrapers @ v0.2.0..v0.3.0
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Included mzmon-lib (shared library) @ v0.8.0..v0.9.0
    * Release mzmon-lib (shared library) v0.8.0
        * [materialize-monitoring#75](https://github.com/MaterializeInc/materialize-monitoring/pull/75)
    * Update Rust crate jsonschema to 0.47.0
        * [materialize-monitoring#98](https://github.com/MaterializeInc/materialize-monitoring/pull/98)
    * Update Rust crate jsonschema to v0.46.10
        * [materialize-monitoring#91](https://github.com/MaterializeInc/materialize-monitoring/pull/91)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)

## Dashboards v0.12.0 (Unreleased)

_Changes Pending_

## Dashboards v0.11.0

* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0

## Prometheus Scrapers v0.2.0

* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

## materialize-monitoring Optional CRDs v0.3.0 (Unreleased)

_Changes Pending_

## Container Images v0.1.1

* Update debian Docker tag to trixie-20260623
    * [materialize-monitoring#73](https://github.com/MaterializeInc/materialize-monitoring/pull/73)
* Create distroless images for alloy
    * [materialize-monitoring#72](https://github.com/MaterializeInc/materialize-monitoring/pull/72)

## Container Images v0.1.0

* Bootstrapped

## Dashboards v0.10.0

* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Prometheus Scrapers v0.1.1

* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Add PodMonitors for prometheus.operator
    * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## Prometheus Scrapers v0.1.0

* Bootstrapped

## Dashboards v0.9.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)

## mzmon-lib (shared library) v0.7.0

* Update dependency pydantic-settings to v2.14.2 [SECURITY]
    * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
* Update python Docker tag to v3.14
    * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
* Update Rust crate jsonschema to v0.46.9
    * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
* Update Rust crate anyhow to v1.0.103
    * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
* Update Rust crate itertools to 0.15.0
    * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
* MaterializeInc/jun/add-auth-to-compute-sql-endpoint
    * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
* Add annotations to distinguish dashboards; roadmapping
    * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
* Release Dashboards v0.11.0
    * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
* Release Dashboards v0.10.0
    * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
* Update PR description on bump updates
    * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
* Improvements to better support GCP/GKE/GMP Dashboards/Datasources
    * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
* Attach explicit pod labels to scrapers in GCP
    * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
* Generate PodMonitoring resources for GCP
    * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
* Upgrade to rust 1.96
    * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release Dashboards v0.9.0
    * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
* Only upload artifacts while in a draft state
    * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
* Release Dashboards v0.8.0
    * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)

## Pipelines v0.4.0

* Implement Gateway Pipeline for Logs
    * [materialize-monitoring#79](https://github.com/MaterializeInc/materialize-monitoring/pull/79)

### Dependencies

* Included mzmon-lib (shared library) @ v0.7.0..v0.8.0
    * Update dependency grafana-foundation-sdk to v0.0.18
        * [materialize-monitoring#51](https://github.com/MaterializeInc/materialize-monitoring/pull/51)
    * Update Rust crate reqwest to 0.13
        * [materialize-monitoring#61](https://github.com/MaterializeInc/materialize-monitoring/pull/61)
    * Release mzmon-lib (shared library) v0.7.0
        * [materialize-monitoring#28](https://github.com/MaterializeInc/materialize-monitoring/pull/28)
    * Update dependency pydantic-settings to v2.14.2 [SECURITY]
        * [materialize-monitoring#64](https://github.com/MaterializeInc/materialize-monitoring/pull/64)
    * Update python Docker tag to v3.14
        * [materialize-monitoring#59](https://github.com/MaterializeInc/materialize-monitoring/pull/59)
    * Update Rust crate jsonschema to v0.46.9
        * [materialize-monitoring#56](https://github.com/MaterializeInc/materialize-monitoring/pull/56)
    * Update Rust crate anyhow to v1.0.103
        * [materialize-monitoring#55](https://github.com/MaterializeInc/materialize-monitoring/pull/55)
    * Update Rust crate itertools to 0.15.0
        * [materialize-monitoring#60](https://github.com/MaterializeInc/materialize-monitoring/pull/60)
    * MaterializeInc/jun/add-auth-to-compute-sql-endpoint
        * [materialize-monitoring#47](https://github.com/MaterializeInc/materialize-monitoring/pull/47)
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Expose classic scrapeconfigs
        * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)

## materialize-monitoring Helm Chart v0.3.0

* Implement Loki with Production Configuration
    * [materialize-monitoring#48](https://github.com/MaterializeInc/materialize-monitoring/pull/48)
* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)
* Release materialize-monitoring Helm Chart v0.3.0
    * [materialize-monitoring#17](https://github.com/MaterializeInc/materialize-monitoring/pull/17)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included Dashboards @ v0.11.0..v0.12.0
    * Add annotations to distinguish dashboards; roadmapping
        * [materialize-monitoring#45](https://github.com/MaterializeInc/materialize-monitoring/pull/45)
    * Release Dashboards v0.11.0
        * [materialize-monitoring#44](https://github.com/MaterializeInc/materialize-monitoring/pull/44)
    * Support optimizing for clouds; add GCP specific variation
        * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)
    * Release Dashboards v0.10.0
        * [materialize-monitoring#36](https://github.com/MaterializeInc/materialize-monitoring/pull/36)
    * Improvements to better support GCP/GKE/GMP Dashboards/Datasources
        * [materialize-monitoring#40](https://github.com/MaterializeInc/materialize-monitoring/pull/40)
    * Release Dashboards v0.9.0
        * [materialize-monitoring#30](https://github.com/MaterializeInc/materialize-monitoring/pull/30)
    * Release Dashboards v0.8.0
        * [materialize-monitoring#18](https://github.com/MaterializeInc/materialize-monitoring/pull/18)
    * Use global_id to not run into errors on right join
        * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
    * Coalesce object names into dashboards
        * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Included Pipelines @ v0.3.0..v0.4.0
* Included Prometheus Scrapers @ v0.1.1..v0.2.0
    * Attach explicit pod labels to scrapers in GCP
        * [materialize-monitoring#39](https://github.com/MaterializeInc/materialize-monitoring/pull/39)
    * Add PodMonitors for prometheus.operator
        * [materialize-monitoring#31](https://github.com/MaterializeInc/materialize-monitoring/pull/31)
* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Update PR description on bump updates
        * [materialize-monitoring#42](https://github.com/MaterializeInc/materialize-monitoring/pull/42)
    * Generate PodMonitoring resources for GCP
        * [materialize-monitoring#38](https://github.com/MaterializeInc/materialize-monitoring/pull/38)
    * Upgrade to rust 1.96
        * [materialize-monitoring#37](https://github.com/MaterializeInc/materialize-monitoring/pull/37)
    * Only upload artifacts while in a draft state
        * [materialize-monitoring#29](https://github.com/MaterializeInc/materialize-monitoring/pull/29)
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## materialize-monitoring Optional CRDs v0.2.0

* Expose classic scrapeconfigs
    * [materialize-monitoring#34](https://github.com/MaterializeInc/materialize-monitoring/pull/34)

## Dashboards v0.8.0

* Use global_id to not run into errors on right join
    * [materialize-monitoring#24](https://github.com/MaterializeInc/materialize-monitoring/pull/24)
* Coalesce object names into dashboards
    * [materialize-monitoring#23](https://github.com/MaterializeInc/materialize-monitoring/pull/23)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0
    * Release mzmon-lib (shared library) v0.6.0
        * [materialize-monitoring#20](https://github.com/MaterializeInc/materialize-monitoring/pull/20)
    * Include artifacts when creating github releases
        * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)

## Pipelines v0.3.0

### Dependencies

* Included mzmon-lib (shared library) @ v0.5.0..v0.6.0
    * Support generating a release when version bump PRs are merged
        * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
    * Support auto-formatting based on labels
        * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
    * Generated automated versioning PRs
        * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
    * Monitoring Roadmap and Version/Changelog Management
        * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## mzmon-lib (shared library) v0.6.0

* Include artifacts when creating github releases
    * [materialize-monitoring#26](https://github.com/MaterializeInc/materialize-monitoring/pull/26)
* Support generating a release when version bump PRs are merged
    * [materialize-monitoring#25](https://github.com/MaterializeInc/materialize-monitoring/pull/25)
* Support auto-formatting based on labels
    * [materialize-monitoring#22](https://github.com/MaterializeInc/materialize-monitoring/pull/22)
* Generated automated versioning PRs
    * [materialize-monitoring#21](https://github.com/MaterializeInc/materialize-monitoring/pull/21)
* Monitoring Roadmap and Version/Changelog Management
    * [materialize-monitoring#16](https://github.com/MaterializeInc/materialize-monitoring/pull/16)

## materialize-monitoring Helm Chart v0.2.0

### Dependencies

* Included Dashboards @ v0.6.0..v0.7.0
    * Fix cloud compatibility with Environment Monitoring dashboards
        * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
    * Update for self-managed workloads
        * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)
* Included Pipelines @ v0.1.0..v0.2.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)
* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## Dashboards v0.7.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Generate agent logging pipeline
        * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
    * Alloy Pipeline Generation
        * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Pipelines v0.2.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

### Dependencies

* Included mzmon-lib (shared library) @ v0.4.0..v0.5.0
    * Implement capsules and targets for alloy pipelines
        * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)

## mzmon-lib (shared library) v0.5.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## Dashboards v0.6.0

* Fix cloud compatibility with Environment Monitoring dashboards
    * [materialize-monitoring#15](https://github.com/MaterializeInc/materialize-monitoring/pull/15)
* Update for self-managed workloads
    * [materialize-monitoring#14](https://github.com/MaterializeInc/materialize-monitoring/pull/14)

## Pipelines v0.1.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)

## mzmon-lib (shared library) v0.4.0

* Generate agent logging pipeline
    * [materialize-monitoring#13](https://github.com/MaterializeInc/materialize-monitoring/pull/13)
* Implement capsules and targets for alloy pipelines
    * [materialize-monitoring#12](https://github.com/MaterializeInc/materialize-monitoring/pull/12)
* Alloy Pipeline Generation
    * [materialize-monitoring#11](https://github.com/MaterializeInc/materialize-monitoring/pull/11)

## materialize-monitoring Helm Chart v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Provide helm reference documentation for materialize-monitoring
    * [materialize-monitoring#9](https://github.com/MaterializeInc/materialize-monitoring/pull/9)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)

## materialize-monitoring Optional CRDs v0.1.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)

## mzmon-lib (shared library) v0.3.0

* Linting in CI and with pre-commit; Contributing
    * [materialize-monitoring#10](https://github.com/MaterializeInc/materialize-monitoring/pull/10)
* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)

## Dashboards v0.5.0

* Add table of grafana dashboards that can be downloaded
    * [materialize-monitoring#7](https://github.com/MaterializeInc/materialize-monitoring/pull/7)
* WIP Monitoring charts for self managed
    * [materialize-monitoring#6](https://github.com/MaterializeInc/materialize-monitoring/pull/6)
* Update contributor documentation around dashboards
    * [materialize-monitoring#5](https://github.com/MaterializeInc/materialize-monitoring/pull/5)
