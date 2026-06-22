"""Local build context.

These allow passing some state to dashboards that control how things
are rendered.
"""

from __future__ import annotations

import dataclasses
import enum

from .version import DashboardAPI


class CloudHint(enum.StrEnum):
    """Hints about the cloud environment the dashboard is being built for."""

    GENERIC = "generic"
    """Best effort to render something that works in most places."""
    MZ = "mz"
    """Metrics come from Materialize's managed service via customer endpoints."""
    MZ_K8S = "mz_k8s"
    """Metrics come from Materialize SaaS via prometheus scrapers internally.

    Materialize Cloud runs on top of AWS.
    This should not be used by external users.
    """
    AWS = "aws"
    """Self-managed AWS."""
    AZURE = "azure"
    """Self-managed Azure."""
    GCP = "gcp"
    """Self-managed GCP.

    GKE exposes less information than is desired to cAdvisor/KSM.
    Cloud Monitoring Dashboards (which can import Grafana dashboards) have a lot of limitations.
    """


class ExportHint(enum.StrEnum):
    """Hints about the intended export target for the dashboard."""

    GENERIC = "generic"
    """Normal Grafana dashboard."""
    CLOUD_MONITORING = "cloud_monitoring"
    """Google Cloud Monitoring dashboard."""
    DASHBOARD_GALLERY = "dashboard_gallery"
    """Public Grafana Dashboard Gallery."""
    HELM_CHART = "helm_chart"
    """Dashboard intended to be rendered as part of a Helm chart."""
    DOCS = "docs"
    """Dashboard intended to be parsed for documentation.

    Queries in documentation should be slightly more readable and have less
    indirection (variables, templating).
    """


@dataclasses.dataclass(frozen=True)
class BuildContext:
    """Context for building dashboards.

    This can be used to pass state to dashboards that control how things
    are rendered.
    """

    api_hint: DashboardAPI = DashboardAPI.DASHBOARD_V2
    cloud_hint: CloudHint = CloudHint.GENERIC
    export_hint: ExportHint = ExportHint.GENERIC


DEFAULT_BUILD_CONTEXT = BuildContext()
