# materialize-monitoring Changelog

<!-- This repo uses different versioning streams for its artifacts.
Artifacts are mapped out in packages/components.yaml.
Unreleased sections are placeholders ("_Changes Pending_") until a
version-update/<component> PR populates and releases them; that PR also bumps
the component's version_paths. See reference/internal/versioning.md and
reference/internal/releasing.md.
-->

## mzmon-lib (shared library) v0.8.0 (Unreleased)

_Changes Pending_

## materialize-monitoring Helm Chart v0.4.0 (Unreleased)

_Changes Pending_

## Dashboards v0.12.0 (Unreleased)

_Changes Pending_

## Dashboards v0.11.0

* Support optimizing for clouds; add GCP specific variation
    * [materialize-monitoring#43](https://github.com/MaterializeInc/materialize-monitoring/pull/43)

### Dependencies

* Included mzmon-lib (shared library) @ v0.6.0..v0.7.0

## Prometheus Scrapers v0.2.0 (Unreleased)

_Changes Pending_

## materialize-monitoring Optional CRDs v0.3.0 (Unreleased)

_Changes Pending_

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

## Pipelines v0.4.0 (Unreleased)

_Changes Pending_

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
