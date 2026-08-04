# Extra metrics destinations.
#
# The gateway always remote-writes to Thanos; these fan out in addition to it.
# Each one turns on `destination.otel`, which is the chart's switch for the whole
# OTLP path — shared by every OTLP exporter, so enabling a second one later does
# not change this.

locals {
  google_cloud_metrics_document = var.google_cloud_metrics == null ? [] : [yamlencode({
    pipeline = {
      metrics = {
        gateway = {
          destination = {
            otel = {
              enabled = true
              googleCloudExporter = merge(
                {
                  enabled             = true
                  minMetricImportance = var.google_cloud_metrics.min_importance
                },
                var.google_cloud_metrics.prefix == null ? {} : {
                  prefix = var.google_cloud_metrics.prefix
                },
              )
            }
          }
        }
      }
    }
  })]
}
