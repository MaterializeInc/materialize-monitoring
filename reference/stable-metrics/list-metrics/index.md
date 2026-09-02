# List of Metrics




# List of Metrics

This has the list of metrics which are available for usage.
In many systems, only a subset of metrics get stored based
on how they would be used.

## Metric Tiers

Importance is about **storage**, not stability: it says which metrics are worth keeping when capacity is limited, not what we promise about their names.
For that, see [Stability and Deprecations](../../stability/) — in short, `mz_*` names come from Materialize itself and we disclose rather than freeze them.


Metrics are grouped by "metricImportance" levels (mzmon-specific).
These levels guide which metrics are prioritized in
metric stores which have limited capacity.

The **essential** metrics are the set of metrics that
are critical and you would always want to have available.
These are used in alerting.

The **recommended** metrics are the set of metrics that
are used in dashboards and are generally desirable for
troubleshooting.

The **extended** set of metrics are used for optional/experimental
dashboards.

The **diagnostic** set of metrics are used for in-depth
troubleshooting and analysis.

In our `materialize-monitoring` configuration, we also
provide an **all** min-importance for including
absolutely everything.
This is recommended if you have cheaper metric storage
like our bundled Thanos provider.

## Essential Metrics

> [!WARNING]
> FIXME: Some links for alerts mistakenly point at the common-queries page.


<ul>
    <li id="certmanager_certificate_ready_status">certmanager_certificate_ready_status
        <details>
            Used labels: condition
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.certificate_not_ready">materialize.environmentd.certificate_not_ready</a></li>
            </ul>
        </details>
    </li>
    <li id="cilium_bpf_map_pressure">cilium_bpf_map_pressure
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cilium.bpf_map_pressure">infra.cilium.bpf_map_pressure</a></li>
            </ul>
        </details>
    </li>
    <li id="cilium_drop_count_total">cilium_drop_count_total
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cilium.drop_rate_elevated">infra.cilium.drop_rate_elevated</a></li>
            </ul>
        </details>
    </li>
    <li id="container_cpu_cfs_periods_total">container_cpu_cfs_periods_total
        <details>
            Used labels: container
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.cpu_throttled">materialize.environmentd.cpu_throttled</a></li>
            </ul>
        </details>
    </li>
    <li id="container_cpu_cfs_throttled_periods_total">container_cpu_cfs_throttled_periods_total
        <details>
            Used labels: container
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.cpu_throttled">materialize.environmentd.cpu_throttled</a></li>
            </ul>
        </details>
    </li>
    <li id="container_cpu_usage_seconds_total">container_cpu_usage_seconds_total
        <details>
            Used labels: container, namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.pods_high_cpu_ratio">infra.kubernetes.pods_high_cpu_ratio</a></li>
                <li><a href="../common-queries#infra.kubernetes.vector_high_cpu_ratio">infra.kubernetes.vector_high_cpu_ratio</a></li>
                <li><a href="../common-queries#materialize.environmentd.high_cpu">materialize.environmentd.high_cpu</a></li>
                <li><a href="../common-queries#materialize.generations.cpu">materialize.generations.cpu</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.usage.absolute">materialize.kubernetes.cpu.usage.absolute</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.usage.percent">materialize.kubernetes.cpu.usage.percent</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.cpu_usage">materialize.kubernetes.pods.cpu_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="container_file_descriptors">container_file_descriptors
        <details>
            Used labels: container
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_critical">infra.kubernetes.file_descriptors_critical</a></li>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_elevated">infra.kubernetes.file_descriptors_elevated</a></li>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_warning">infra.kubernetes.file_descriptors_warning</a></li>
            </ul>
        </details>
    </li>
    <li id="container_fs_limit_bytes">container_fs_limit_bytes
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.container_disk_usage">infra.kubernetes.container_disk_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="container_fs_usage_bytes">container_fs_usage_bytes
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.container_disk_usage">infra.kubernetes.container_disk_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="container_last_seen">container_last_seen
        <details>
            Used labels: materialize_cloud_swap
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.swap_cluster_oom">materialize.clusterd.swap_cluster_oom</a></li>
            </ul>
        </details>
    </li>
    <li id="container_memory_swap">container_memory_swap
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.swap_cluster_oom">materialize.clusterd.swap_cluster_oom</a></li>
            </ul>
        </details>
    </li>
    <li id="container_memory_working_set_bytes">container_memory_working_set_bytes
        <details>
            Used labels: container, namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.memory_elevated">infra.kubernetes.memory_elevated</a></li>
                <li><a href="../common-queries#infra.kubernetes.memory_high">infra.kubernetes.memory_high</a></li>
                <li><a href="../common-queries#materialize.environmentd.high_memory">materialize.environmentd.high_memory</a></li>
                <li><a href="../common-queries#materialize.environmentd.memory_elevated">materialize.environmentd.memory_elevated</a></li>
                <li><a href="../common-queries#materialize.generations.memory">materialize.generations.memory</a></li>
                <li><a href="../common-queries#materialize.generations.pods">materialize.generations.pods</a></li>
                <li><a href="../common-queries#materialize.kubernetes.memory.usage.absolute">materialize.kubernetes.memory.usage.absolute</a></li>
                <li><a href="../common-queries#materialize.kubernetes.memory.usage.percent">materialize.kubernetes.memory.usage.percent</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.memory_usage">materialize.kubernetes.pods.memory_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_receive_bytes_total">container_network_receive_bytes_total
        <details>
            Used labels: namespace, pod, workload
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.egress_gateway.excessive_traffic">infra.egress_gateway.excessive_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.high_traffic">infra.egress_gateway.high_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.low_traffic">infra.egress_gateway.low_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.traffic_missing_metrics">infra.egress_gateway.traffic_missing_metrics</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_rx">materialize.kubernetes.pods.network_rx</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_receive_errors_total">container_network_receive_errors_total
        <details>
            Used labels: namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_errors">materialize.kubernetes.pods.network_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_receive_packets_dropped_total">container_network_receive_packets_dropped_total
        <details>
            Used labels: namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_drops">materialize.kubernetes.pods.network_drops</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_transmit_bytes_total">container_network_transmit_bytes_total
        <details>
            Used labels: namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_tx">materialize.kubernetes.pods.network_tx</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_transmit_errors_total">container_network_transmit_errors_total
        <details>
            Used labels: namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_errors">materialize.kubernetes.pods.network_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="container_network_transmit_packets_dropped_total">container_network_transmit_packets_dropped_total
        <details>
            Used labels: namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.pods.network_drops">materialize.kubernetes.pods.network_drops</a></li>
            </ul>
        </details>
    </li>
    <li id="container_spec_cpu_period">container_spec_cpu_period
        <details>
            Used labels: container, namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.high_cpu">materialize.environmentd.high_cpu</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.capacity">materialize.kubernetes.cpu.capacity</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.capacity.all_containers">materialize.kubernetes.cpu.capacity.all_containers</a></li>
            </ul>
        </details>
    </li>
    <li id="container_spec_cpu_quota">container_spec_cpu_quota
        <details>
            Used labels: container, namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.high_cpu">materialize.environmentd.high_cpu</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.capacity">materialize.kubernetes.cpu.capacity</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.capacity.all_containers">materialize.kubernetes.cpu.capacity.all_containers</a></li>
            </ul>
        </details>
    </li>
    <li id="container_spec_memory_limit_bytes">container_spec_memory_limit_bytes
        <details>
            Used labels: container, namespace, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.memory_elevated">infra.kubernetes.memory_elevated</a></li>
                <li><a href="../common-queries#infra.kubernetes.memory_high">infra.kubernetes.memory_high</a></li>
                <li><a href="../common-queries#materialize.environmentd.high_memory">materialize.environmentd.high_memory</a></li>
                <li><a href="../common-queries#materialize.environmentd.memory_elevated">materialize.environmentd.memory_elevated</a></li>
                <li><a href="../common-queries#materialize.kubernetes.memory.capacity">materialize.kubernetes.memory.capacity</a></li>
                <li><a href="../common-queries#materialize.kubernetes.memory.capacity.all_containers">materialize.kubernetes.memory.capacity.all_containers</a></li>
                <li><a href="../common-queries#materialize.kubernetes.memory.usage.percent">materialize.kubernetes.memory.usage.percent</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.memory_usage">materialize.kubernetes.pods.memory_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="container_spec_memory_swap_limit_bytes">container_spec_memory_swap_limit_bytes
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.swap_cluster_oom">materialize.clusterd.swap_cluster_oom</a></li>
            </ul>
        </details>
    </li>
    <li id="container_start_time_seconds">container_start_time_seconds
        <details>
            Used labels: container, namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.last_restart">materialize.kubernetes.last_restart</a></li>
            </ul>
        </details>
    </li>
    <li id="container_ulimits_soft">container_ulimits_soft
        <details>
            Used labels: ulimit
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_critical">infra.kubernetes.file_descriptors_critical</a></li>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_elevated">infra.kubernetes.file_descriptors_elevated</a></li>
                <li><a href="../common-queries#infra.kubernetes.file_descriptors_warning">infra.kubernetes.file_descriptors_warning</a></li>
            </ul>
        </details>
    </li>
    <li id="coredns_dns_request_duration_seconds_bucket">coredns_dns_request_duration_seconds_bucket
        <details>
            Used labels: zone
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.coredns.slow_queries">infra.coredns.slow_queries</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_capacity">crdb_dedicated_capacity
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_critical">infra.cockroachdb.disk_usage_critical</a></li>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_elevated">infra.cockroachdb.disk_usage_elevated</a></li>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_high">infra.cockroachdb.disk_usage_high</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_capacity_used">crdb_dedicated_capacity_used
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_critical">infra.cockroachdb.disk_usage_critical</a></li>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_elevated">infra.cockroachdb.disk_usage_elevated</a></li>
                <li><a href="../common-queries#infra.cockroachdb.disk_usage_high">infra.cockroachdb.disk_usage_high</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_intentcount">crdb_dedicated_intentcount
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.write_intent_accumulation_critical">infra.cockroachdb.write_intent_accumulation_critical</a></li>
                <li><a href="../common-queries#infra.cockroachdb.write_intent_accumulation_high">infra.cockroachdb.write_intent_accumulation_high</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_ranges_unavailable">crdb_dedicated_ranges_unavailable
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.ranges_unavailable">infra.cockroachdb.ranges_unavailable</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_ranges_underreplicated">crdb_dedicated_ranges_underreplicated
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.ranges_underreplicated">infra.cockroachdb.ranges_underreplicated</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_rocksdb_read_amplification">crdb_dedicated_rocksdb_read_amplification
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.lsm_read_amplification_critical">infra.cockroachdb.lsm_read_amplification_critical</a></li>
                <li><a href="../common-queries#infra.cockroachdb.lsm_read_amplification_high">infra.cockroachdb.lsm_read_amplification_high</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_schedules_backup_last_completed_time">crdb_dedicated_schedules_backup_last_completed_time
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.backup_missing">infra.cockroachdb.backup_missing</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_sql_mem_distsql_current">crdb_dedicated_sql_mem_distsql_current
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.sql_memory_pressure_high">infra.cockroachdb.sql_memory_pressure_high</a></li>
                <li><a href="../common-queries#infra.cockroachdb.sql_memory_rapid_growth">infra.cockroachdb.sql_memory_rapid_growth</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_sql_service_latency_bucket">crdb_dedicated_sql_service_latency_bucket
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.query_latency">infra.cockroachdb.query_latency</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_sys_cpu_combined_percent_normalized">crdb_dedicated_sys_cpu_combined_percent_normalized
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.cpu_usage_critical">infra.cockroachdb.cpu_usage_critical</a></li>
                <li><a href="../common-queries#infra.cockroachdb.cpu_usage_high">infra.cockroachdb.cpu_usage_high</a></li>
            </ul>
        </details>
    </li>
    <li id="crdb_dedicated_sys_totalmem">crdb_dedicated_sys_totalmem
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.cockroachdb.sql_memory_pressure_high">infra.cockroachdb.sql_memory_pressure_high</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_deployment_status_condition">kube_deployment_status_condition
        <details>
            Used labels: condition, deployment, status
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.deployment_unavailable_critical">infra.kubernetes.deployment_unavailable_critical</a></li>
                <li><a href="../common-queries#infra.kubernetes.deployment_unavailable_notice">infra.kubernetes.deployment_unavailable_notice</a></li>
                <li><a href="../common-queries#infra.kubernetes.deployment_unavailable_warning">infra.kubernetes.deployment_unavailable_warning</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_deployment_status_replicas_ready">kube_deployment_status_replicas_ready
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.deployments.readiness">materialize.kubernetes.deployments.readiness</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_deployment_status_replicas_unavailable">kube_deployment_status_replicas_unavailable
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.deployments.readiness">materialize.kubernetes.deployments.readiness</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_horizontalpodautoscaler_spec_max_replicas">kube_horizontalpodautoscaler_spec_max_replicas
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.hpa_replicas_critical">infra.kubernetes.hpa_replicas_critical</a></li>
                <li><a href="../common-queries#infra.kubernetes.hpa_replicas_high">infra.kubernetes.hpa_replicas_high</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_horizontalpodautoscaler_status_current_replicas">kube_horizontalpodautoscaler_status_current_replicas
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.hpa_replicas_critical">infra.kubernetes.hpa_replicas_critical</a></li>
                <li><a href="../common-queries#infra.kubernetes.hpa_replicas_high">infra.kubernetes.hpa_replicas_high</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_created">kube_node_created
        <details>
            Used labels: node
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.created">infra.nodes.created</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_info">kube_node_info
        <details>
            Used labels: node
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.info.address">infra.nodes.info.address</a></li>
                <li><a href="../common-queries#infra.nodes.info.kernel">infra.nodes.info.kernel</a></li>
                <li><a href="../common-queries#infra.nodes.info.kubelet">infra.nodes.info.kubelet</a></li>
                <li><a href="../common-queries#infra.nodes.info.os">infra.nodes.info.os</a></li>
                <li><a href="../common-queries#infra.nodes.info.runtime">infra.nodes.info.runtime</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_spec_taint">kube_node_spec_taint
        <details>
            Used labels: key, node
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.node_unreachable">infra.kubernetes.node_unreachable</a></li>
                <li><a href="../common-queries#infra.nodes.taints">infra.nodes.taints</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_spec_unschedulable">kube_node_spec_unschedulable
        <details>
            Used labels: node
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.unschedulable">infra.nodes.unschedulable</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_status_allocatable">kube_node_status_allocatable
        <details>
            Used labels: node, resource
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.allocation.cpu">infra.nodes.allocation.cpu</a></li>
                <li><a href="../common-queries#infra.nodes.allocation.memory">infra.nodes.allocation.memory</a></li>
                <li><a href="../common-queries#infra.nodes.allocation.pods">infra.nodes.allocation.pods</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_status_capacity">kube_node_status_capacity
        <details>
            Used labels: node, resource
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.capacity.cpu">infra.nodes.capacity.cpu</a></li>
                <li><a href="../common-queries#infra.nodes.capacity.ephemeral_storage">infra.nodes.capacity.ephemeral_storage</a></li>
                <li><a href="../common-queries#infra.nodes.capacity.memory">infra.nodes.capacity.memory</a></li>
                <li><a href="../common-queries#infra.nodes.capacity.pods">infra.nodes.capacity.pods</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_node_status_condition">kube_node_status_condition
        <details>
            Used labels: condition, node, status
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.node_disk_pressure">infra.kubernetes.node_disk_pressure</a></li>
                <li><a href="../common-queries#infra.nodes.condition.ready">infra.nodes.condition.ready</a></li>
                <li><a href="../common-queries#infra.nodes.conditions">infra.nodes.conditions</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_resource_limits">kube_pod_container_resource_limits
        <details>
            Used labels: namespace, node, pod, resource
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.pods.budgets">infra.nodes.pods.budgets</a></li>
                <li><a href="../common-queries#materialize.kubernetes.cpu.usage.percent">materialize.kubernetes.cpu.usage.percent</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.cpu_usage">materialize.kubernetes.pods.cpu_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_resource_requests">kube_pod_container_resource_requests
        <details>
            Used labels: container, namespace, node, pod, resource, unit
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.daemonset_high_cpu">infra.kubernetes.daemonset_high_cpu</a></li>
                <li><a href="../common-queries#infra.kubernetes.daemonset_saturating_cpu">infra.kubernetes.daemonset_saturating_cpu</a></li>
                <li><a href="../common-queries#infra.kubernetes.daemonset_saturating_mem">infra.kubernetes.daemonset_saturating_mem</a></li>
                <li><a href="../common-queries#infra.kubernetes.pods_high_cpu_ratio">infra.kubernetes.pods_high_cpu_ratio</a></li>
                <li><a href="../common-queries#infra.kubernetes.vector_high_cpu_ratio">infra.kubernetes.vector_high_cpu_ratio</a></li>
                <li><a href="../common-queries#infra.nodes.allocation.cpu">infra.nodes.allocation.cpu</a></li>
                <li><a href="../common-queries#infra.nodes.allocation.memory">infra.nodes.allocation.memory</a></li>
                <li><a href="../common-queries#infra.nodes.pods.budgets">infra.nodes.pods.budgets</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_status_last_terminated_exitcode">kube_pod_container_status_last_terminated_exitcode
        <details>
            Used labels: container, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.error_kill">materialize.clusterd.error_kill</a></li>
                <li><a href="../common-queries#materialize.clusterd.system_cluster_terminated">materialize.clusterd.system_cluster_terminated</a></li>
                <li><a href="../common-queries#materialize.environmentd.terminated">materialize.environmentd.terminated</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_status_last_terminated_reason">kube_pod_container_status_last_terminated_reason
        <details>
            Used labels: container, namespace, reason
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.oomkill_core_systems">infra.kubernetes.oomkill_core_systems</a></li>
                <li><a href="../common-queries#infra.kubernetes.oomkill_important_systems">infra.kubernetes.oomkill_important_systems</a></li>
                <li><a href="../common-queries#infra.kubernetes.oomkill_nonessential_systems">infra.kubernetes.oomkill_nonessential_systems</a></li>
                <li><a href="../common-queries#materialize.clusterd.swap_cluster_oom">materialize.clusterd.swap_cluster_oom</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_status_restarts_total">kube_pod_container_status_restarts_total
        <details>
            Used labels: container, namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.oomkill_core_systems">infra.kubernetes.oomkill_core_systems</a></li>
                <li><a href="../common-queries#infra.kubernetes.oomkill_important_systems">infra.kubernetes.oomkill_important_systems</a></li>
                <li><a href="../common-queries#infra.kubernetes.oomkill_nonessential_systems">infra.kubernetes.oomkill_nonessential_systems</a></li>
                <li><a href="../common-queries#infra.kubernetes.pod_restart_rate_high">infra.kubernetes.pod_restart_rate_high</a></li>
                <li><a href="../common-queries#infra.kubernetes.pod_restart_rate_high_nonessential">infra.kubernetes.pod_restart_rate_high_nonessential</a></li>
                <li><a href="../common-queries#infra.nodes.pods.restarts">infra.nodes.pods.restarts</a></li>
                <li><a href="../common-queries#materialize.clusterd.error_kill">materialize.clusterd.error_kill</a></li>
                <li><a href="../common-queries#materialize.clusterd.new_restarts_during_release">materialize.clusterd.new_restarts_during_release</a></li>
                <li><a href="../common-queries#materialize.clusterd.swap_cluster_oom">materialize.clusterd.swap_cluster_oom</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_container_status_waiting">kube_pod_container_status_waiting
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.pods_stuck_in_waiting">infra.kubernetes.pods_stuck_in_waiting</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_created">kube_pod_created
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.new_restarts_during_release">materialize.clusterd.new_restarts_during_release</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_info">kube_pod_info
        <details>
            Used labels: node
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.allocation.pods">infra.nodes.allocation.pods</a></li>
                <li><a href="../common-queries#infra.nodes.pods.by_namespace">infra.nodes.pods.by_namespace</a></li>
                <li><a href="../common-queries#infra.nodes.pods.by_phase">infra.nodes.pods.by_phase</a></li>
                <li><a href="../common-queries#infra.nodes.pods.not_ready">infra.nodes.pods.not_ready</a></li>
                <li><a href="../common-queries#infra.nodes.pods.restarts">infra.nodes.pods.restarts</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_start_time">kube_pod_start_time
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.pods_stuck_in_waiting">infra.kubernetes.pods_stuck_in_waiting</a></li>
                <li><a href="../common-queries#materialize.clusterd.not_receiving_commands">materialize.clusterd.not_receiving_commands</a></li>
                <li><a href="../common-queries#materialize.environmentd.uptime_sla">materialize.environmentd.uptime_sla</a></li>
                <li><a href="../common-queries#materialize.environmentd.uptime_slo">materialize.environmentd.uptime_slo</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_status_phase">kube_pod_status_phase
        <details>
            Used labels: namespace, phase
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.infra_pod_pending">infra.kubernetes.infra_pod_pending</a></li>
                <li><a href="../common-queries#infra.nodes.allocation.pods">infra.nodes.allocation.pods</a></li>
                <li><a href="../common-queries#infra.nodes.pods.by_namespace">infra.nodes.pods.by_namespace</a></li>
                <li><a href="../common-queries#infra.nodes.pods.by_phase">infra.nodes.pods.by_phase</a></li>
                <li><a href="../common-queries#infra.nodes.pods.not_ready">infra.nodes.pods.not_ready</a></li>
                <li><a href="../common-queries#materialize.environmentd.pod_pending">materialize.environmentd.pod_pending</a></li>
                <li><a href="../common-queries#materialize.environmentd.pod_pending_critical">materialize.environmentd.pod_pending_critical</a></li>
                <li><a href="../common-queries#materialize.kubernetes.pods.readiness">materialize.kubernetes.pods.readiness</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_pod_status_ready">kube_pod_status_ready
        <details>
            Used labels: condition
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.nodes.pods.not_ready">infra.nodes.pods.not_ready</a></li>
            </ul>
        </details>
    </li>
    <li id="kube_statefulset_status_replicas_ready">kube_statefulset_status_replicas_ready
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.kubernetes.statefulsets.ready">materialize.kubernetes.statefulsets.ready</a></li>
            </ul>
        </details>
    </li>
    <li id="kubelet_node_name">kubelet_node_name
        <details>
            Used labels: workload
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.egress_gateway.no_traffic">infra.egress_gateway.no_traffic</a></li>
            </ul>
        </details>
    </li>
    <li id="kubelet_volume_stats_capacity_bytes">kubelet_volume_stats_capacity_bytes
        <details>
            Used labels: persistentvolumeclaim
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.volume_usage">infra.kubernetes.volume_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="kubelet_volume_stats_used_bytes">kubelet_volume_stats_used_bytes
        <details>
            Used labels: persistentvolumeclaim
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.kubernetes.volume_usage">infra.kubernetes.volume_usage</a></li>
            </ul>
        </details>
    </li>
    <li id="loki_distributor_bytes_received_total">loki_distributor_bytes_received_total
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.monitoring.logging_collection_down">infra.monitoring.logging_collection_down</a></li>
            </ul>
        </details>
    </li>
    <li id="loki_panic_total">loki_panic_total
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.loki.panics">infra.loki.panics</a></li>
            </ul>
        </details>
    </li>
    <li id="loki_request_duration_seconds_bucket">loki_request_duration_seconds_bucket
        <details>
            Used labels: route
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.loki.req_duration_high">infra.loki.req_duration_high</a></li>
            </ul>
        </details>
    </li>
    <li id="loki_request_duration_seconds_count">loki_request_duration_seconds_count
        <details>
            Used labels: route, status_code
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.loki.push_err_high">infra.loki.push_err_high</a></li>
                <li><a href="../common-queries#infra.loki.req_err_high">infra.loki.req_err_high</a></li>
                <li><a href="../common-queries#infra.loki.writer_err_high">infra.loki.writer_err_high</a></li>
            </ul>
        </details>
    </li>
    <li id="loki_write_dropped_entries_total">loki_write_dropped_entries_total
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.loki.alloy_log_drops">infra.loki.alloy_log_drops</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_adapter_commands">mz_adapter_commands
        <details>
            Used labels: application_name, materialize_cloud_organization_name, status
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.connections.adapter.command_rate">materialize.connections.adapter.command_rate</a></li>
                <li><a href="../common-queries#materialize.connections.adapter.commands_by_application">materialize.connections.adapter.commands_by_application</a></li>
                <li><a href="../common-queries#materialize.console.errors">materialize.console.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_auth_request_count">mz_auth_request_count
        <details>
            Used labels: path, status
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.auth.errors">materialize.auth.errors</a></li>
                <li><a href="../common-queries#materialize.auth.refresh_failures">materialize.auth.refresh_failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_cluster_server_last_command_received">mz_cluster_server_last_command_received
        <details>
            Used labels: pod, server_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.not_receiving_commands">materialize.clusterd.not_receiving_commands</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_dataflow_replica_expiration_remaining_seconds">mz_dataflow_replica_expiration_remaining_seconds
        <details>
            Used labels: pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.expiration_7d">materialize.clusterd.expiration_7d</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_external_calls_count">mz_external_calls_count
        <details>
            Used labels: status
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.external_uptime.checker_not_calling">materialize.external_uptime.checker_not_calling</a></li>
                <li><a href="../common-queries#materialize.external_uptime.connections_failing">materialize.external_uptime.connections_failing</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_external_envd_up">mz_external_envd_up
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.external_uptime.envd_unreachable">materialize.external_uptime.envd_unreachable</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_memory_limiter_memory_limit_bytes">mz_memory_limiter_memory_limit_bytes
        <details>
            Used labels: pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.system_cluster_high_memory">materialize.clusterd.system_cluster_high_memory</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_memory_limiter_memory_usage_bytes">mz_memory_limiter_memory_usage_bytes
        <details>
            Used labels: pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusterd.system_cluster_high_memory">materialize.clusterd.system_cluster_high_memory</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_parameter_frontend_last_cse_time_seconds">mz_parameter_frontend_last_cse_time_seconds
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.launchdarkly.stale_cse">materialize.launchdarkly.stale_cse</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_parameter_frontend_last_sse_time_seconds">mz_parameter_frontend_last_sse_time_seconds
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.launchdarkly.stale_sse">materialize.launchdarkly.stale_sse</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_blob_failures">mz_persist_blob_failures
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_cmd_failed_count">mz_persist_cmd_failed_count
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_columnar_op_count">mz_persist_columnar_op_count
        <details>
            Used labels: op, result
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_columnar_validation_count">mz_persist_columnar_validation_count
        <details>
            Used labels: result
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_compaction_dropped">mz_persist_compaction_dropped
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_compaction_failed">mz_persist_compaction_failed
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_compaction_noop">mz_persist_compaction_noop
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_consensus_failures">mz_persist_consensus_failures
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_external_blob_delete_noop_count">mz_persist_external_blob_delete_noop_count
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_external_failed_count">mz_persist_external_failed_count
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_lease_timeout_read">mz_persist_lease_timeout_read
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_pushdown_parts_mismatched_stats_count">mz_persist_pushdown_parts_mismatched_stats_count
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_schema_cache_fetch_state_count">mz_persist_schema_cache_fetch_state_count
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_shard_unconsolidated_snapshot">mz_persist_shard_unconsolidated_snapshot
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_persist_state_update_state_slow_path">mz_persist_state_update_state_slow_path
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_shard_finalization_outstanding">mz_shard_finalization_outstanding
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.collection_finalization_stuck">materialize.storage.collection_finalization_stuck</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_time_to_first_row_seconds_bucket">mz_time_to_first_row_seconds_bucket
        <details>
            Used labels: application_name, instance_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.console.query_latency">materialize.console.query_latency</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_txn_placeholder_schema_apply">mz_txn_placeholder_schema_apply
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.persist.failures">materialize.persist.failures</a></li>
            </ul>
        </details>
    </li>
    <li id="node_boot_time_seconds">node_boot_time_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.uptime">node.uptime</a></li>
            </ul>
        </details>
    </li>
    <li id="node_cpu_seconds_total">node_cpu_seconds_total
        <details>
            Used labels: instance, mode
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.cpu.utilization">node.cpu.utilization</a></li>
                <li><a href="../common-queries#node.debug.cpu.by_mode">node.debug.cpu.by_mode</a></li>
                <li><a href="../common-queries#node.debug.cpu.per_core">node.debug.cpu.per_core</a></li>
                <li><a href="../common-queries#node.load.normalized">node.load.normalized</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_io_time_seconds_total">node_disk_io_time_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.disk.io_utilization">node.disk.io_utilization</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filefd_allocated">node_filefd_allocated
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.filefd.utilization">node.filefd.utilization</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filefd_maximum">node_filefd_maximum
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.filefd.utilization">node.filefd.utilization</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filesystem_avail_bytes">node_filesystem_avail_bytes
        <details>
            Used labels: fstype, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.filesystem.available.ratio">node.filesystem.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filesystem_readonly">node_filesystem_readonly
        <details>
            Used labels: fstype, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.filesystem.readonly">node.filesystem.readonly</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filesystem_size_bytes">node_filesystem_size_bytes
        <details>
            Used labels: fstype, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.filesystem.available.ratio">node.filesystem.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_load1">node_load1
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.load.normalized">node.load.normalized</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_MemAvailable_bytes">node_memory_MemAvailable_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.memory.available.ratio">node.memory.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_MemTotal_bytes">node_memory_MemTotal_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.breakdown">node.debug.memory.breakdown</a></li>
                <li><a href="../common-queries#node.memory.available.ratio">node.memory.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_SwapFree_bytes">node_memory_SwapFree_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.swap.used.ratio">node.swap.used.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_SwapTotal_bytes">node_memory_SwapTotal_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.swap.used.ratio">node.swap.used.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_receive_bytes_total">node_network_receive_bytes_total
        <details>
            Used labels: device, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.egress_gateway.excessive_traffic">infra.egress_gateway.excessive_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.high_traffic">infra.egress_gateway.high_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.low_traffic">infra.egress_gateway.low_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.no_traffic">infra.egress_gateway.no_traffic</a></li>
                <li><a href="../common-queries#infra.egress_gateway.traffic_missing_metrics">infra.egress_gateway.traffic_missing_metrics</a></li>
                <li><a href="../common-queries#node.debug.network.saturation">node.debug.network.saturation</a></li>
                <li><a href="../common-queries#node.debug.network.throughput">node.debug.network.throughput</a></li>
                <li><a href="../common-queries#node.network.rx.total">node.network.rx.total</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_receive_drop_total">node_network_receive_drop_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.network.drops">node.network.drops</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_receive_errs_total">node_network_receive_errs_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.network.errors">node.network.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_transmit_bytes_total">node_network_transmit_bytes_total
        <details>
            Used labels: device, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.egress_gateway.no_traffic">infra.egress_gateway.no_traffic</a></li>
                <li><a href="../common-queries#node.debug.network.saturation">node.debug.network.saturation</a></li>
                <li><a href="../common-queries#node.debug.network.throughput">node.debug.network.throughput</a></li>
                <li><a href="../common-queries#node.network.tx.total">node.network.tx.total</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_transmit_drop_total">node_network_transmit_drop_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.network.drops">node.network.drops</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_transmit_errs_total">node_network_transmit_errs_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.network.errors">node.network.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_nf_conntrack_entries">node_nf_conntrack_entries
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.conntrack.utilization">node.conntrack.utilization</a></li>
            </ul>
        </details>
    </li>
    <li id="node_nf_conntrack_entries_limit">node_nf_conntrack_entries_limit
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.conntrack.utilization">node.conntrack.utilization</a></li>
            </ul>
        </details>
    </li>
    <li id="node_pressure_cpu_waiting_seconds_total">node_pressure_cpu_waiting_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.cpu.pressure">node.cpu.pressure</a></li>
            </ul>
        </details>
    </li>
    <li id="node_pressure_memory_stalled_seconds_total">node_pressure_memory_stalled_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.memory.pressure">node.memory.pressure</a></li>
            </ul>
        </details>
    </li>
    <li id="node_pressure_memory_waiting_seconds_total">node_pressure_memory_waiting_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.memory.pressure">node.memory.pressure</a></li>
            </ul>
        </details>
    </li>
    <li id="node_scrape_collector_success">node_scrape_collector_success
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.collector.success">node.collector.success</a></li>
            </ul>
        </details>
    </li>
    <li id="node_time_seconds">node_time_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.uptime">node.uptime</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_oom_kill">node_vmstat_oom_kill
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.memory.oom_kills">node.memory.oom_kills</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pswpin">node_vmstat_pswpin
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.swap.activity">node.swap.activity</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pswpout">node_vmstat_pswpout
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.swap.activity">node.swap.activity</a></li>
            </ul>
        </details>
    </li>
    <li id="up">up
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, job, namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#infra.monitoring.clusterd_metrics_missing">infra.monitoring.clusterd_metrics_missing</a></li>
                <li><a href="../common-queries#infra.monitoring.critical_metrics_missing">infra.monitoring.critical_metrics_missing</a></li>
                <li><a href="../common-queries#materialize.scraper.mzmon.clusterd">materialize.scraper.mzmon.clusterd</a></li>
                <li><a href="../common-queries#materialize.scraper.mzmon.environmentd">materialize.scraper.mzmon.environmentd</a></li>
                <li><a href="../common-queries#materialize.scraper.mzmon.orchestratord">materialize.scraper.mzmon.orchestratord</a></li>
            </ul>
        </details>
    </li>
    <li id="v2_mz_can_connect">v2_mz_can_connect
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.uptime_sla">materialize.environmentd.uptime_sla</a></li>
                <li><a href="../common-queries#materialize.environmentd.uptime_slo">materialize.environmentd.uptime_slo</a></li>
            </ul>
        </details>
    </li>
    <li id="v2_mz_envd_up">v2_mz_envd_up
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.simplest_query">materialize.environmentd.simplest_query</a></li>
            </ul>
        </details>
    </li>
    <li id="v2_mz_views_query_successful">v2_mz_views_query_successful
        <details>
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.environmentd.query_views_critical">materialize.environmentd.query_views_critical</a></li>
                <li><a href="../common-queries#materialize.environmentd.query_views_warning">materialize.environmentd.query_views_warning</a></li>
            </ul>
        </details>
    </li>
</ul>


## Recommended Metrics


<ul>
    <li id="environmentd_needs_update">environmentd_needs_update
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.environments.needing_update">materialize.operator.environments.needing_update</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_active_sessions">mz_active_sessions
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.connections.sessions.active">materialize.connections.sessions.active</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_active_subscribes">mz_active_subscribes
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.subscribes.active">materialize.compute.subscribes.active</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_arrangement_maintenance_seconds_total">mz_arrangement_maintenance_seconds_total
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.arrangements.maintenance_rate">materialize.compute.arrangements.maintenance_rate</a></li>
                <li><a href="../common-queries#materialize.compute.arrangements.maintenance_rate_by_worker">materialize.compute.arrangements.maintenance_rate_by_worker</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_arrangement_record_count">mz_arrangement_record_count
        <details>
            Used labels: collection_id, instance_id, materialize_cloud_organization_name, replica_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.arrangements.records.system">materialize.compute.arrangements.records.system</a></li>
                <li><a href="../common-queries#materialize.compute.arrangements.records.transient">materialize.compute.arrangements.records.transient</a></li>
                <li><a href="../common-queries#materialize.compute.arrangements.records.user">materialize.compute.arrangements.records.user</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_cluster_status">mz_compute_cluster_status
        <details>
            Used labels: compute_cluster_id, compute_replica_id, compute_replica_name, materialize_cloud_organization_name, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.clusters.count">materialize.clusters.count</a></li>
                <li><a href="../common-queries#materialize.clusters.info">materialize.clusters.info</a></li>
                <li><a href="../common-queries#materialize.clusters.replicas.count">materialize.clusters.replicas.count</a></li>
                <li><a href="../common-queries#materialize.clusters.replicas.sizes">materialize.clusters.replicas.sizes</a></li>
                <li><a href="../common-queries#materialize.generations.version">materialize.generations.version</a></li>
                <li><a href="../common-queries#materialize.health.clusters.status.percentage">materialize.health.clusters.status.percentage</a></li>
                <li><a href="../common-queries#materialize.health.environment.availability.percentage">materialize.health.environment.availability.percentage</a></li>
                <li><a href="../common-queries#materialize.info.version">materialize.info.version</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_commands_total">mz_compute_commands_total
        <details>
            Used labels: materialize_cloud_organization_name, pod
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.generations.active">materialize.generations.active</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_controller_hydration_queue_size">mz_compute_controller_hydration_queue_size
        <details>
            Used labels: instance_id, materialize_cloud_organization_name, replica_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.hydration.queue_size">materialize.compute.hydration.queue_size</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_hydration_time_seconds">mz_compute_hydration_time_seconds
        <details>
            Used labels: hydrated, instance_id, materialize_cloud_organization_name, replica_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.hydration.slowest_collections">materialize.compute.hydration.slowest_collections</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_peek_duration_seconds_bucket">mz_compute_peek_duration_seconds_bucket
        <details>
            Used labels: instance_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.connections.peek_latency.p50">materialize.connections.peek_latency.p50</a></li>
                <li><a href="../common-queries#materialize.connections.peek_latency.p90">materialize.connections.peek_latency.p90</a></li>
                <li><a href="../common-queries#materialize.connections.peek_latency.p99">materialize.connections.peek_latency.p99</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_compute_replica_history_dataflow_count">mz_compute_replica_history_dataflow_count
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.dataflows.count">materialize.compute.dataflows.count</a></li>
                <li><a href="../common-queries#materialize.compute.dataflows.count_by_worker">materialize.compute.dataflows.count_by_worker</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_dataflow_elapsed_seconds_total">mz_dataflow_elapsed_seconds_total
        <details>
            Used labels: instance_id, materialize_cloud_organization_name, replica_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.dataflows.elapsed_rate">materialize.compute.dataflows.elapsed_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_dataflow_wallclock_lag_seconds">mz_dataflow_wallclock_lag_seconds
        <details>
            Used labels: instance_id, materialize_cloud_organization_name, pod, quantile, replica_id
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.freshness.lag_by_cluster">materialize.compute.freshness.lag_by_cluster</a></li>
                <li><a href="../common-queries#materialize.compute.freshness.lag_total_by_cluster">materialize.compute.freshness.lag_total_by_cluster</a></li>
                <li><a href="../common-queries#materialize.compute.freshness.top_collections">materialize.compute.freshness.top_collections</a></li>
                <li><a href="../common-queries#materialize.compute.hydration.currently_hydrating">materialize.compute.hydration.currently_hydrating</a></li>
                <li><a href="../common-queries#materialize.generations.collections">materialize.generations.collections</a></li>
                <li><a href="../common-queries#materialize.generations.hydrating">materialize.generations.hydrating</a></li>
                <li><a href="../common-queries#materialize.generations.lag.max">materialize.generations.lag.max</a></li>
                <li><a href="../common-queries#materialize.generations.lag.total">materialize.generations.lag.total</a></li>
                <li><a href="../common-queries#materialize.generations.lag.total_by_cluster">materialize.generations.lag.total_by_cluster</a></li>
                <li><a href="../common-queries#materialize.info.max_lag">materialize.info.max_lag</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_indexes_count">mz_indexes_count
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.indexes.by_type">materialize.compute.indexes.by_type</a></li>
                <li><a href="../common-queries#materialize.compute.indexes.count">materialize.compute.indexes.count</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_mzd_views_count">mz_mzd_views_count
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.materialized_views.count">materialize.compute.materialized_views.count</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_query_total">mz_query_total
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.connections.queries.distribution">materialize.connections.queries.distribution</a></li>
                <li><a href="../common-queries#materialize.connections.queries.rate">materialize.connections.queries.rate</a></li>
                <li><a href="../common-queries#materialize.connections.queries.rate_by_statement">materialize.connections.queries.rate_by_statement</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_bytes_committed">mz_sink_bytes_committed
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.lag">materialize.storage.sinks.lag</a></li>
                <li><a href="../common-queries#materialize.storage.sinks.throughput">materialize.storage.sinks.throughput</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_bytes_staged">mz_sink_bytes_staged
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.lag">materialize.storage.sinks.lag</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_commit_conflicts">mz_sink_iceberg_commit_conflicts
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.commit_failures">materialize.storage.sinks.iceberg.commit_failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_commit_duration_seconds_bucket">mz_sink_iceberg_commit_duration_seconds_bucket
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.commit_latency">materialize.storage.sinks.iceberg.commit_latency</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_commit_failures">mz_sink_iceberg_commit_failures
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.commit_failures">materialize.storage.sinks.iceberg.commit_failures</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_data_files_written">mz_sink_iceberg_data_files_written
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.file_rate">materialize.storage.sinks.iceberg.file_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_delete_files_written">mz_sink_iceberg_delete_files_written
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.file_rate">materialize.storage.sinks.iceberg.file_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_iceberg_snapshots_committed">mz_sink_iceberg_snapshots_committed
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.iceberg.file_rate">materialize.storage.sinks.iceberg.file_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_rdkafka_connects">mz_sink_rdkafka_connects
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.kafka.connect_rate">materialize.storage.sinks.kafka.connect_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_rdkafka_disconnects">mz_sink_rdkafka_disconnects
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.kafka.connect_rate">materialize.storage.sinks.kafka.connect_rate</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_rdkafka_outbuf_msg_cnt">mz_sink_rdkafka_outbuf_msg_cnt
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.kafka.output_buffer">materialize.storage.sinks.kafka.output_buffer</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_sink_rdkafka_txerrs">mz_sink_rdkafka_txerrs
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.kafka.tx_errors">materialize.storage.sinks.kafka.tx_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_source_bytes_received">mz_source_bytes_received
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sources.bytes_received">materialize.storage.sources.bytes_received</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_source_messages_received">mz_source_messages_received
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sources.ingestion_by_replica">materialize.storage.sources.ingestion_by_replica</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_source_offset_commit_failures">mz_source_offset_commit_failures
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sources.upstream_errors">materialize.storage.sources.upstream_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_source_offset_committed">mz_source_offset_committed
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sources.upstream_errors">materialize.storage.sources.upstream_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_source_offset_known">mz_source_offset_known
        <details>
            Used labels: cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sources.upstream_errors">materialize.storage.sources.upstream_errors</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_storage_objects">mz_storage_objects
        <details>
            Used labels: materialize_cloud_organization_name, type
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.sinks.by_type">materialize.storage.sinks.by_type</a></li>
                <li><a href="../common-queries#materialize.storage.sinks.count">materialize.storage.sinks.count</a></li>
                <li><a href="../common-queries#materialize.storage.sources.by_type">materialize.storage.sources.by_type</a></li>
                <li><a href="../common-queries#materialize.storage.sources.catalog">materialize.storage.sources.catalog</a></li>
                <li><a href="../common-queries#materialize.storage.sources.count">materialize.storage.sources.count</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_tables_count">mz_tables_count
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.storage.tables.count">materialize.storage.tables.count</a></li>
            </ul>
        </details>
    </li>
    <li id="mz_views_count">mz_views_count
        <details>
            Used labels: materialize_cloud_organization_name
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.compute.views.count">materialize.compute.views.count</a></li>
            </ul>
        </details>
    </li>
    <li id="node_arp_entries">node_arp_entries
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.arp.entries">node.debug.arp.entries</a></li>
            </ul>
        </details>
    </li>
    <li id="node_context_switches_total">node_context_switches_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.context_switches">node.debug.context_switches</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_io_time_weighted_seconds_total">node_disk_io_time_weighted_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.queue_depth">node.debug.disk.queue_depth</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_read_bytes_total">node_disk_read_bytes_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.throughput">node.debug.disk.throughput</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_read_time_seconds_total">node_disk_read_time_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.latency">node.debug.disk.latency</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_reads_completed_total">node_disk_reads_completed_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.iops">node.debug.disk.iops</a></li>
                <li><a href="../common-queries#node.debug.disk.latency">node.debug.disk.latency</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_write_time_seconds_total">node_disk_write_time_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.latency">node.debug.disk.latency</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_writes_completed_total">node_disk_writes_completed_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.iops">node.debug.disk.iops</a></li>
                <li><a href="../common-queries#node.debug.disk.latency">node.debug.disk.latency</a></li>
            </ul>
        </details>
    </li>
    <li id="node_disk_written_bytes_total">node_disk_written_bytes_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.disk.throughput">node.debug.disk.throughput</a></li>
            </ul>
        </details>
    </li>
    <li id="node_entropy_available_bits">node_entropy_available_bits
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.entropy.available">node.debug.entropy.available</a></li>
            </ul>
        </details>
    </li>
    <li id="node_entropy_pool_size_bits">node_entropy_pool_size_bits
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.entropy.available">node.debug.entropy.available</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filesystem_files">node_filesystem_files
        <details>
            Used labels: fstype, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.filesystem.inodes.available.ratio">node.debug.filesystem.inodes.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_filesystem_files_free">node_filesystem_files_free
        <details>
            Used labels: fstype, instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.filesystem.inodes.available.ratio">node.debug.filesystem.inodes.available.ratio</a></li>
            </ul>
        </details>
    </li>
    <li id="node_intr_total">node_intr_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.context_switches">node.debug.context_switches</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_Buffers_bytes">node_memory_Buffers_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.breakdown">node.debug.memory.breakdown</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_Cached_bytes">node_memory_Cached_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.breakdown">node.debug.memory.breakdown</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_Committed_AS_bytes">node_memory_Committed_AS_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.kernel">node.debug.memory.kernel</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_MemFree_bytes">node_memory_MemFree_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.breakdown">node.debug.memory.breakdown</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_SReclaimable_bytes">node_memory_SReclaimable_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.breakdown">node.debug.memory.breakdown</a></li>
                <li><a href="../common-queries#node.debug.memory.kernel">node.debug.memory.kernel</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_SUnreclaim_bytes">node_memory_SUnreclaim_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.kernel">node.debug.memory.kernel</a></li>
            </ul>
        </details>
    </li>
    <li id="node_memory_Slab_bytes">node_memory_Slab_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.kernel">node.debug.memory.kernel</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_TcpExt_ListenDrops">node_netstat_TcpExt_ListenDrops
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.errors">node.debug.tcp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_TcpExt_ListenOverflows">node_netstat_TcpExt_ListenOverflows
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.errors">node.debug.tcp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_TcpExt_TCPRcvQDrop">node_netstat_TcpExt_TCPRcvQDrop
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.errors">node.debug.tcp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_TcpExt_TCPSynRetrans">node_netstat_TcpExt_TCPSynRetrans
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.retransmits">node.debug.tcp.retransmits</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_TcpExt_TCPTimeouts">node_netstat_TcpExt_TCPTimeouts
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.errors">node.debug.tcp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_Tcp_OutSegs">node_netstat_Tcp_OutSegs
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.retransmits">node.debug.tcp.retransmits</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_Tcp_RetransSegs">node_netstat_Tcp_RetransSegs
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.tcp.retransmits">node.debug.tcp.retransmits</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_Udp_InErrors">node_netstat_Udp_InErrors
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.udp.errors">node.debug.udp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_Udp_NoPorts">node_netstat_Udp_NoPorts
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.udp.errors">node.debug.udp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_netstat_Udp_RcvbufErrors">node_netstat_Udp_RcvbufErrors
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.udp.errors">node.debug.udp.errors</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_carrier">node_network_carrier
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.network.operstate">node.debug.network.operstate</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_speed_bytes">node_network_speed_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.network.saturation">node.debug.network.saturation</a></li>
            </ul>
        </details>
    </li>
    <li id="node_network_up">node_network_up
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.network.operstate">node.debug.network.operstate</a></li>
            </ul>
        </details>
    </li>
    <li id="node_schedstat_waiting_seconds_total">node_schedstat_waiting_seconds_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.schedstat.waiting">node.debug.schedstat.waiting</a></li>
            </ul>
        </details>
    </li>
    <li id="node_scrape_collector_duration_seconds">node_scrape_collector_duration_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.exporter.scrape_duration">node.debug.exporter.scrape_duration</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_TCP_alloc">node_sockstat_TCP_alloc
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.tcp">node.debug.sockets.tcp</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_TCP_inuse">node_sockstat_TCP_inuse
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.tcp">node.debug.sockets.tcp</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_TCP_mem_bytes">node_sockstat_TCP_mem_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.memory">node.debug.sockets.memory</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_TCP_orphan">node_sockstat_TCP_orphan
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.tcp">node.debug.sockets.tcp</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_TCP_tw">node_sockstat_TCP_tw
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.tcp">node.debug.sockets.tcp</a></li>
            </ul>
        </details>
    </li>
    <li id="node_sockstat_UDP_mem_bytes">node_sockstat_UDP_mem_bytes
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.sockets.memory">node.debug.sockets.memory</a></li>
            </ul>
        </details>
    </li>
    <li id="node_softnet_dropped_total">node_softnet_dropped_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.softnet.dropped">node.debug.softnet.dropped</a></li>
            </ul>
        </details>
    </li>
    <li id="node_softnet_processed_total">node_softnet_processed_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.softnet.processed">node.debug.softnet.processed</a></li>
            </ul>
        </details>
    </li>
    <li id="node_softnet_times_squeezed_total">node_softnet_times_squeezed_total
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.softnet.squeezed">node.debug.softnet.squeezed</a></li>
            </ul>
        </details>
    </li>
    <li id="node_timex_estimated_error_seconds">node_timex_estimated_error_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.time.drift">node.debug.time.drift</a></li>
            </ul>
        </details>
    </li>
    <li id="node_timex_maxerror_seconds">node_timex_maxerror_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.time.drift">node.debug.time.drift</a></li>
            </ul>
        </details>
    </li>
    <li id="node_timex_offset_seconds">node_timex_offset_seconds
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.time.drift">node.debug.time.drift</a></li>
            </ul>
        </details>
    </li>
    <li id="node_timex_sync_status">node_timex_sync_status
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.time.sync_status">node.debug.time.sync_status</a></li>
            </ul>
        </details>
    </li>
    <li id="node_udp_queues">node_udp_queues
        <details>
            Used labels: instance, ip, queue
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.udp.queues">node.debug.udp.queues</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgfault">node_vmstat_pgfault
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.page_faults">node.debug.memory.page_faults</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgmajfault">node_vmstat_pgmajfault
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.page_faults">node.debug.memory.page_faults</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgscan_direct">node_vmstat_pgscan_direct
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.reclaim">node.debug.memory.reclaim</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgscan_kswapd">node_vmstat_pgscan_kswapd
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.reclaim">node.debug.memory.reclaim</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgsteal_direct">node_vmstat_pgsteal_direct
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.reclaim">node.debug.memory.reclaim</a></li>
            </ul>
        </details>
    </li>
    <li id="node_vmstat_pgsteal_kswapd">node_vmstat_pgsteal_kswapd
        <details>
            Used labels: instance
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#node.debug.memory.reclaim">node.debug.memory.reclaim</a></li>
            </ul>
        </details>
    </li>
    <li id="orchestratord_is_leader">orchestratord_is_leader
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.reconciling.replicas">materialize.operator.reconciling.replicas</a></li>
            </ul>
        </details>
    </li>
    <li id="orchestratord_reconciliation_duration_seconds_bucket">orchestratord_reconciliation_duration_seconds_bucket
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.reconciliation.duration">materialize.operator.reconciliation.duration</a></li>
            </ul>
        </details>
    </li>
    <li id="orchestratord_reconciliation_step_duration_seconds_bucket">orchestratord_reconciliation_step_duration_seconds_bucket
        <details>
            Used labels: namespace
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.reconciliation.step.duration.p99">materialize.operator.reconciliation.step.duration.p99</a></li>
            </ul>
        </details>
    </li>
    <li id="orchestratord_reconciliation_steps_total">orchestratord_reconciliation_steps_total
        <details>
            Used labels: namespace, outcome
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.reconciliation.steps.incomplete">materialize.operator.reconciliation.steps.incomplete</a></li>
                <li><a href="../common-queries#materialize.operator.reconciliation.steps.rate">materialize.operator.reconciliation.steps.rate</a></li>
            </ul>
        </details>
    </li>
    <li id="orchestratord_reconciliations_total">orchestratord_reconciliations_total
        <details>
            Used labels: namespace, outcome
            <br />
            Example queries:
            <ul>
                <li><a href="../common-queries#materialize.operator.reconciliation.failures.by_controller">materialize.operator.reconciliation.failures.by_controller</a></li>
                <li><a href="../common-queries#materialize.operator.reconciliation.failures.total">materialize.operator.reconciliation.failures.total</a></li>
                <li><a href="../common-queries#materialize.operator.reconciliation.outcomes">materialize.operator.reconciliation.outcomes</a></li>
                <li><a href="../common-queries#materialize.operator.reconciliation.rate">materialize.operator.reconciliation.rate</a></li>
            </ul>
        </details>
    </li>
</ul>


