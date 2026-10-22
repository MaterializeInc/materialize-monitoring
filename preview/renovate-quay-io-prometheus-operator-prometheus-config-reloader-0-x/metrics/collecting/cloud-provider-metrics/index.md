# Cloud Provider Metrics




# Cloud Provider Metrics

The gateway can pull what a cloud provider's monitoring API publishes about the metadata database and the buckets a Materialize deployment depends on,
about whether the provider can supply the nodes its cluster asks for,
and about the load balancers in front of the cluster and the NAT gateways its traffic leaves through.
The result is written beside every other metric, with the same retention, the same PromQL, and the same destinations.
A provider series is therefore joinable with `mz_persist_*` in a single expression.
A Grafana CloudWatch, Cloud Monitoring or Azure Monitor datasource cannot offer that.

The **Infrastructure Cloud Provider** dashboard (`infra-cloud`) draws the database and bucket pulls,
**Infrastructure Autoscaling** (`infra-autoscaling`) the node and quota pulls on its Cloud Capacity tab,
and **Infrastructure Networking** (`infra-net`) the load balancer and NAT gateway pulls on its Cloud Networking tab,
each with rows for whichever provider or pull it finds; see [Available Dashboards](/materialize-monitoring/preview/renovate-quay-io-prometheus-operator-prometheus-config-reloader-0-x/dashboards/all/).

Provider collection is off by default and adds to what the clients already report about the same dependencies.
Persist, Loki and Thanos measure every request they make against the database and the bucket, at full resolution and at no cost.
The provider adds what no client can see: CPU, memory and storage headroom, the burst credits that throttle a volume, transaction-ID consumption, and bucket growth.
For the cluster's nodes it adds the provider's own view: host and instance health, node groups that cannot reach their desired size, compute quota,
and on AKS the managed cluster autoscaler, which runs where nothing in the cluster can scrape it.
For the network around the cluster it adds whether a load balancer has a healthy backend and what it carries,
and whether a NAT gateway is running out of source ports.
The [external-dependency design](../../../reference/development/design-docs/20260920-external-dependency-monitoring/#pulling-provider-metrics-into-the-pipeline) records the reasoning.

<!--
Agent note: each pull is a custom component in packages/alloy-pipelines/gateway-provider.yaml, and
charts/materialize-monitoring/templates/_alloy_provider_helpers.tpl renders only the instances. The metric
sets on this page are that file's; change them there, not here first. packages/queries/infra-cloud.yaml and
packages/dashboards/src/grafana/infra_cloud/ draw the database and bucket pulls, and
packages/queries/infra-autoscaling.yaml and infra_autoscaling/cloud.rs the EKS, Compute Engine and AKS
pulls, and packages/queries/infra-networking.yaml and infra_networking/cloud.rs the load balancer and NAT
pulls, so a metric added to a pull reaches no panel until those follow, and a metric removed leaves a panel
empty. The numbers on this page
(series counts, API calls, sample ages) were measured on 2026-09-26 against the wrapper-provisioned
GKE and EKS test installs, and on 2026-09-29 against the AKS one, with Alloy v1.20.0 locally and
v1.19.2 in-cluster, and are the ones to re-measure if the exporters are bumped. The Azure call counts
are read from the exporter's code (Alloy v1.19.2, azure-metrics-exporter 5092ac0), since it exports
no counter of its own. The EKS, Compute Engine quota and AKS pulls were measured on 2026-09-30 by
running the rendered components under a local Alloy v1.20.0 against the same three installs' accounts,
not in-cluster: the test installs' gateway identities hold none of the grants they need.
No quota refusal has been observed, so `exceeded` is documented from Cloud Monitoring's descriptors.
The load balancer and NAT gateway pulls were measured on 2026-10-16 the same way, under a local Alloy
v1.20.0 against the three installs' accounts. None of those installs has an internal GCP load balancer,
so the `l3/internal` half of that pull is documented from Cloud Monitoring's descriptors.
-->

## What is pulled

| Provider | Service | Default metrics | Resolution |
|---|---|---|---|
| `cloudwatch` | RDS instance | `CPUUtilization`, `CPUCreditBalance`, `FreeableMemory`, `FreeStorageSpace`, `DatabaseConnections`, `ReadLatency`, `WriteLatency`, `DiskQueueDepth`, `BurstBalance`, `EBSIOBalance%`, `EBSByteBalance%`, `MaximumUsedTransactionIDs` | One minute, summarised per five-minute period |
| `cloudwatch` | S3 bucket | `BucketSizeBytes` per storage class, `NumberOfObjects` | Daily |
| `gcp` | Cloud SQL instance | `cpu/utilization`, `memory/utilization`, `disk/utilization`, `postgresql/num_backends`, `postgresql/transaction_id_utilization`, `up` | One minute |
| `gcp` | GCS bucket | `storage/v2/total_bytes`, `storage/v2/total_count`, split into live, noncurrent and soft-deleted objects | Daily, repeated every five minutes |
| `azure` | PostgreSQL Flexible Server | `cpu_percent`, `cpu_credits_remaining`, `memory_percent`, `storage_percent`, `active_connections`, `connections_failed`, `disk_queue_depth`, `disk_iops_consumed_percentage`, `disk_bandwidth_consumed_percentage`, `maximum_used_transactionIDs`, `is_db_alive` | One minute, summarised per five-minute bucket |
| `azure` | Blob Storage account | `BlobCapacity`, `BlobCount` | Hourly, refreshed about once a day |
| `azure` | Blob Storage account | `Availability`, `SuccessServerLatency`, `SuccessE2ELatency` | One minute, summarised per five-minute bucket |
| `cloudwatch` | EKS cluster's nodes | `StatusCheckFailed_System`, `StatusCheckFailed_Instance`, `StatusCheckFailed_AttachedEBS` per node | One minute, summarised per five-minute period |
| `cloudwatch` | EKS cluster's managed node groups | `GroupDesiredCapacity`, `GroupInServiceInstances`, `GroupPendingInstances`, `GroupMaxSize` | One minute, summarised per five-minute period |
| `cloudwatch` | The region, when any EKS cluster is listed | On-Demand vCPUs in use in the standard families, account-wide (`AWS/Usage` `ResourceCount`) | One minute, summarised per five-minute period |
| `gcp` | Compute Engine region and its zones | `quota/cpus_per_vm_family` and `quota/local_ssd_total_storage_per_vm_family`: usage, limit and refusals | A few times a day; refusals as they happen |
| `azure` | AKS cluster | `cluster_autoscaler_unschedulable_pods_count`, `cluster_autoscaler_cluster_safe_to_autoscale`, `cluster_autoscaler_scale_down_in_cooldown`, `cluster_autoscaler_unneeded_nodes_count` | One minute, summarised per five-minute bucket |
| `azure` | AKS cluster's node scale sets | `VmAvailabilityMetric` per node VM | One minute, summarised per five-minute bucket |
| `cloudwatch` | Network Load Balancer | `ActiveFlowCount`, and `TCP_Client_Reset_Count`, `TCP_Target_Reset_Count` and `TCP_ELB_Reset_Count` | One minute, summarised per five-minute period |
| `cloudwatch` | Network Load Balancer | `NewFlowCount`, `ProcessedBytes` | The newest published minute |
| `cloudwatch` | Network Load Balancer target group | `HealthyHostCount`, `UnHealthyHostCount` | One minute, summarised per five-minute period |
| `cloudwatch` | NAT gateway | `ErrorPortAllocation`, `PacketsDropCount`, `IdleTimeoutCount`, `ConnectionAttemptCount`, `ConnectionEstablishedCount`, `ActiveConnectionCount` | One minute, summarised per five-minute period |
| `gcp` | Load balancer forwarding rule, external or internal | `l3/external` and `l3/internal` `ingress_bytes_count`, `ingress_packets_count`, `egress_bytes_count`, `egress_packets_count` | The newest published minute |
| `gcp` | Cloud NAT gateway | `nat_allocation_failed`, `port_usage`, `allocated_ports`, `open_connections`, `dropped_sent_packets_count` | One minute |
| `azure` | AKS cluster's load balancers | `VipAvailability` per frontend address, `DipAvailability` per frontend and backend address | One minute, summarised per five-minute bucket |
| `azure` | AKS cluster's load balancers | `ByteCount`, `SYNCount` per frontend address and direction | The newest minute |
| `azure` | NAT gateway | `DatapathAvailability`, `PacketDropCount`, and `SNATConnectionCount` by connection state | One minute, summarised per five-minute bucket |

The metric sets are fixed, and values only name the resources.
Each pull is a custom component in the gateway's `gateway-provider` pipeline, so changing what is pulled is a change to that pipeline.
CloudWatch takes one instance of it per listed resource; GCP and Azure take one per service, whose filter names every listed resource.
The Cloud SQL entries are metric-type prefixes, so `postgresql/num_backends` also pulls `num_backends_by_state` and `num_backends_by_application`, and `up` also pulls `uptime`.

Azure applies one aggregation list to every metric in a call.
The Flexible Server pull therefore asks for all four and keeps the one or two each metric needs.
Those are the statistics the RDS pull asks CloudWatch for: average and maximum CPU, minimum credits, maximum connections, and so on.
That is 11 series per server, and 12 on a Burstable tier, which also publishes `cpu_credits_remaining`.

### The nodes and the quota behind them

A node that cannot be had looks the same from inside the cluster whatever the reason: pods stay pending, and the autoscaler reports a failed launch.
These pulls say which reason it is, as far as the provider publishes one.

| Question | EKS | GKE | AKS |
|---|---|---|---|
| Is the host under a node failing | `StatusCheckFailed_System` | — | `VmAvailabilityMetric`, with `Context` saying whether the platform took it down |
| Is a node group short of its desired size | Desired against in-service, for managed node groups | — | Unschedulable pods while the autoscaler is allowed to scale |
| Is quota the limit | vCPUs in use, account-wide; the limit is not in CloudWatch | CPUs and local SSD per family, against the limit, and each refusal | — |

EKS nodes are the one resource found by tag rather than named, because Karpenter and the node groups replace them.
EKS and Karpenter tag every node they launch with `aws:eks:cluster-name`, and each managed node group's Auto Scaling group with `eks:cluster-name`,
and the pull matches those tags to the listed cluster names exactly.
Karpenter's node pools have no group of their own, so the group metrics cover only managed node groups; Karpenter's own metrics cover the rest.
The vCPU usage counts every On-Demand instance in the account and region, not only the cluster's, because that is what the quota counts.
Its limit is the Service Quotas console's `L-1216C47A`.

The Compute Engine quotas are per machine family: C4 and C4A each have their own, and the regional `CPUS` quota covers only older families.
A family appears only once something in the project uses it, so a project running only C4 and C4A has a handful of series per region.
Zonal limits read 2^63 − 1 where no zonal quota applies.
A region is matched with its zones and nothing else, so `europe-west1` does not also pull `europe-west10`.

AKS runs its cluster autoscaler inside the managed control plane.
Its metrics reach no scrape, and Azure Monitor publishes these four instead.
Each keeps the aggregation that shows the worst of its five minutes:
the most pods waiting and the most unneeded nodes, any cooldown, and any moment it was unsafe to scale.
The node pools are scale sets in the cluster's node resource group, which the pull finds through the cluster.

### Load balancers and NAT gateways

Kubernetes knows that a load balancer exists and the address it was given.
What the load balancer is doing, and what the NAT gateway every outbound connection crosses is doing, only the provider can say.

| Question | AWS | Google Cloud | Azure |
|---|---|---|---|
| Does the load balancer have a healthy backend | Healthy and unhealthy targets per target group | — | Each backend's health probe results, per frontend address |
| Is the load balancer itself carrying traffic | — | — | Azure's own probe of each frontend address |
| What does it carry | Open and new connections, bytes, and resets by sender | Packets and bytes in each direction | New connections, and bytes in each direction |
| Is the NAT gateway out of source ports | Port allocation errors and dropped packets | Allocation failures, per-VM port use, and drops by reason | Failed SNAT connections and dropped packets |
| What else does the NAT gateway say | Idle timeouts, open connections, and attempts against established | Open connections | Azure's own probe of its data path |

**Each provider's resource is named differently, following how its monitoring API keys it.**

- **AWS**: each Network Load Balancer is named by its ARN, with its target groups' ARNs beside it,
  because CloudWatch publishes health only per target group, keyed by both.
  Application Load Balancers are not pulled.
  NAT gateways are named by ID.
- **Google Cloud**: a LoadBalancer Service on GKE is a passthrough Network Load Balancer,
  named by its forwarding rule or found by the VPC network its backends are in.
  GKE names the rule only once it makes it: `a` followed by the Service's UID without dashes, cut to 32 characters, for an external rule,
  and `k8s2-` and a hash for an internal one under L4 ILB subsetting; the rule's description names the Service.
  A rule cannot be listed before its Service exists, so a Service the same release creates, such as Grafana's, can only be reached by its network.
  `networks` reaches every passthrough load balancer in the network, whoever made it,
  which on a network that holds only the cluster is the cluster's own.
  Cloud NAT gateways are named by the NAT's name, not its Cloud Router's.
- **Azure**: AKS puts every LoadBalancer Service behind one load balancer named `kubernetes`,
  and internal ones behind `kubernetes-internal`, in the cluster's node resource group.
  Every AKS cluster's are named the same, so the pull is named by the AKS cluster and finds them through it, as the AKS node pull does.
  The API server's own load balancer is left out.
  NAT gateways are listed by name.

Three things about these pulls read differently from the others:

- **Health is a count of backends, not a percentage.**
  The Terraform wrappers create the Materialize Services with `externalTrafficPolicy: Local`,
  so only nodes running a `balancerd` or console pod pass the health check.
  On a six-node AKS cluster a healthy Service reads 17% on Azure's own average, so the dashboard counts the backends at 100% instead.
- **Counts arrive in two shapes.**
  A five-minute total ends at the pull, so its newest minutes are not yet published and it reads short.
  The resets, NAT errors and SNAT counts keep that window, since a short burst is what they exist to catch,
  and are read as "any" or as a ratio of two totals from the same window.
  Bytes and new connections take the newest whole minute instead, which arrives complete, and read as a rate.
- **Google Cloud's load balancer and NAT drop counts are per-minute DELTAs.**
  The exporter adds the newest point of each pull to a counter, so `rate()` over it reads about a fifth of the truth at a five-minute interval.
  The counter's last increment is one whole minute, and so is its value when the series is new or has just reset, which is how the dashboard reads it.
  Cloud Monitoring writes nothing for a minute in which a client location sent no traffic, so these series come and go.
  Both pulls are stamped at the scrape, which keeps each counter continuous while the exporter holds it,
  so an idle minute reads as zero rather than as the last busy one.
  External rules publish each series under two resource types, the current one and the legacy `tcp_lb_rule`, and the pull keeps only the current one.

### Metric names

| Provider | Name shape | Example | Resource label |
|---|---|---|---|
| `cloudwatch` | `aws_<service>_<metric>_<statistic>` | `aws_rds_free_storage_space_minimum` | `dimension_DBInstanceIdentifier`, `dimension_BucketName` |
| `gcp` | `stackdriver_<resource type>_<metric type>` | `stackdriver_cloudsql_database_cloudsql_googleapis_com_database_cpu_utilization` | `database_id` (`project:instance`), `bucket_name` |
| `azure` | `azure_<resource type>_<metric>_<aggregation>_<unit>`, lowercased | `azure_microsoft_dbforpostgresql_flexibleservers_storage_percent_maximum_percent` | `resourceName`, and `resourceID` lowercased |

The node and quota pulls follow the same shapes.

| Provider | Families | Resource labels |
|---|---|---|
| `cloudwatch` | `aws_ec2_status_check_failed_{system,instance,attached_ebs}_maximum` | `dimension_InstanceId`, plus `tag_karpenter_sh_nodepool` or `tag_eks_nodegroup_name` |
| `cloudwatch` | `aws_autoscaling_group_{desired_capacity,in_service_instances,pending_instances,max_size}_average` | `dimension_AutoScalingGroupName`, `tag_eks_nodegroup_name` |
| `cloudwatch` | `aws_usage_resource_count_maximum` | `dimension_Class="Standard/OnDemand"`, `region` |
| `gcp` | `stackdriver_compute_googleapis_com_location_compute_googleapis_com_quota_{cpus,local_ssd_total_storage}_per_vm_family_{usage,limit,exceeded}` | `location` (region or zone), `vm_family`, `limit_name` |
| `azure` | `azure_microsoft_containerservice_managedclusters_cluster_autoscaler_*` | `resourceName` |
| `azure` | `azure_microsoft_compute_virtualmachinescalesets_vmavailabilitymetric_minimum_count` | `dimensionVmname` (`<scale set>_<index>`), `dimensionContext`, `resourceName` (the scale set) |

The load balancer and NAT gateway pulls do too.

| Provider | Families | Resource labels |
|---|---|---|
| `cloudwatch` | `aws_networkelb_*`, such as `aws_networkelb_healthy_host_count_minimum` | `dimension_LoadBalancer` (`net/<name>/<id>`), and `dimension_TargetGroup` (`targetgroup/<name>/<id>`) on the health families |
| `cloudwatch` | `aws_natgateway_*`, such as `aws_natgateway_error_port_allocation_sum` | `dimension_NatGatewayId` |
| `gcp` | `stackdriver_loadbalancing_googleapis_com_external_network_load_balancer_rule_loadbalancing_googleapis_com_l_3_external_*` and `stackdriver_internal_{tcp,udp}_lb_rule_loadbalancing_googleapis_com_l_3_internal_*` | `forwarding_rule_name`, and the client's country or network |
| `gcp` | `stackdriver_nat_gateway_router_googleapis_com_nat_*` | `gateway_name`, and `reason` on the drops |
| `azure` | `azure_microsoft_network_loadbalancers_*` | `resourceName` (`kubernetes` or `kubernetes-internal`), `resourceGroup` (the node resource group), and the frontend address |
| `azure` | `azure_microsoft_network_natgateways_*` | `resourceName` |

The Azure exporter names a split dimension `dimension<Name>` when a pull splits by two,
such as `dimensionFrontendIPAddress` beside `dimensionBackendIPAddress`, and plain `dimension` when it splits by one.
`VipAvailability` therefore carries the frontend address as `dimension`, and `SNATConnectionCount` its connection state.

Each of these pulls also stamps every series it writes, `up` included, with `provider_pull`: `cloudwatch_nlb`,
`cloudwatch_nat_gateway`, `gcp_load_balancing`, `gcp_cloud_nat`, `azure_load_balancer` or `azure_nat_gateway`.
`infra-net` reads it off `up` to render a pull's rows only where that pull runs.

A node joins to `kube_node_info` on its provider ID: the instance ID on EKS, and the scale set and index on AKS.
The discovery pull also writes an `aws_<service>_info` series carrying every tag on the resource, which the pull drops, since tags are free text;
the two tags it keeps are set by EKS and Karpenter.

CloudWatch series also carry `region`, `account_id` and `account_alias`, and `name`, which is `rds` or `s3` on every series.
Join on the `dimension_*` label.

Azure series also carry `resourceGroup`, `subscriptionID` and `subscriptionName`.
They also carry the `interval` and `timespan` each pull reads, as ISO 8601 durations.
The Blob Storage families are named for the blob service, as `azure_microsoft_storage_storageaccounts_blobservices_*`.
No resource tag is copied onto a series; the exporter copies `owner` by default, and the pull turns that off.

Every provider series carries `job="integrations/cloudwatch"`, `job="integrations/gcp"` or `job="integrations/azure"`.
`instance` is the resource on RDS and S3 series, since each resource has its own pull, and `eks` on the EKS pull, which covers every listed cluster.
On Cloud Monitoring series it is `cloudsql`, `gcs`, `compute_quota` or `compute_quota_exceeded`,
since one pull covers every listed resource of a service.
Azure series are the same, with `postgres`, `blob_capacity`, `blob_requests`, `aks_autoscaler` or `aks_nodes`.
On the load balancer and NAT gateway pulls, CloudWatch's `instance` is the dimension it reads, `net/<name>/<id>`,
`targetgroup/<name>/<id>` or the NAT gateway ID.
Cloud Monitoring's is `load_balancing` or `cloud_nat`, and Azure's is `lb_availability`, `lb_probes`, `lb_traffic`,
`nat_gateway` or `nat_connections`.

## Configuring it

Each provider lists the resources to watch.
Nothing else is pulled, and a provider that is enabled with no resources fails at render.

```yaml
pipeline:
  metrics:
    provider:
      cloudwatch:
        enabled: true
        region: us-east-1
        rds:
          instances: [mz-prod-db]
        s3:
          buckets: [mz-prod-storage-a1b2]
        eks:
          clusters: [mz-prod-eks]
        nlb:
          loadBalancers:
            - arn: arn:aws:elasticloadbalancing:us-east-1:111122223333:loadbalancer/net/mz-prod/0123456789abcdef
              targetGroups:
                - arn:aws:elasticloadbalancing:us-east-1:111122223333:targetgroup/mz-prod-pgwire/0123456789abcdef
                - arn:aws:elasticloadbalancing:us-east-1:111122223333:targetgroup/mz-prod-console/fedcba9876543210
        natGateway:
          ids: [nat-0123456789abcdef0]
      gcp:
        enabled: true
        projectId: my-project
        cloudSql:
          instances: [mz-prod-pg]
        gcs:
          buckets: [mz-prod-storage]
        compute:
          regions: [us-east1]
        loadBalancing:
          networks: [mz-prod-network]
        cloudNat:
          gateways: [mz-prod-nat]
      azure:
        enabled: true
        subscriptionId: 00000000-0000-0000-0000-000000000000
        postgres:
          servers: [mz-prod-pg]
        blob:
          storageAccounts: [mzprodstorage]
        aks:
          clusters: [mz-prod-aks]
        loadBalancer:
          clusters: [mz-prod-aks]
        natGateway:
          names: [mz-prod-nat-gateway]
```

A Cloud SQL entry is the instance name, not the `project:region:instance` connection name.
A Flexible Server entry is the server name, not its FQDN, and a storage account entry is the account name, not its endpoint.
Both kinds of Azure name are unique across Azure, so no resource group is needed.
An EKS or AKS entry is the cluster's name, not its ARN or resource ID.
AKS names are unique only within a resource group, so two clusters of one name in the subscription are both pulled, and told apart by `resourceGroup`.
A Compute Engine entry is a region, such as `us-east1`; its zones are included, and a zone is refused.
A Network Load Balancer or target group is its ARN, or the end of it that CloudWatch uses, such as `net/mz-prod/0123456789abcdef`.
An ARN in a region other than `region` is refused, since CloudWatch is regional and the pull would return nothing.
A forwarding rule, VPC network or Cloud NAT gateway is its name, not its address or URL, and a NAT gateway ID is `nat-` and hex, not its name.
Azure NAT gateway names are unique only within a resource group, so two gateways of one name in the subscription are both pulled.
The chart refuses anything else at render, since the names sit inside the Resource Graph query's filter.
`subscriptionId` is the subscription's GUID.
On Azure Government or Azure China, set `cloudEnvironment` to `azureusgovernmentcloud` or `azurechinacloud`.

The chart refuses, at render, the values that would otherwise stop the gateway starting: a scrape timeout longer than the interval, and two buckets whose names differ only in `.` and `-`, which would become the same component label.
Alloy accepts both in `alloy validate` and exits on them at load, which would stop logs and metrics along with the pull.

The full set of keys is in the [values reference](../../../reference/helm/materialize-monitoring-values/), under `pipeline.metrics.provider`.

## Identity and grants

Credentials never travel through values.
The pull runs as the gateway pod's own cloud identity, bound through `alloy-gateway.serviceAccount.annotations`.

| Provider | Identity | Grant |
|---|---|---|
| `cloudwatch` | IRSA (`eks.amazonaws.com/role-arn`), EKS Pod Identity, or static keys as `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` in the `mzmon-alloy-gateway-env` Secret | `cloudwatch:GetMetricStatistics`, which also covers the load balancer and NAT gateway pulls. `iam:ListAccountAliases` fills `account_alias`; without it every pull logs a warning. EKS clusters also need `cloudwatch:GetMetricData`, `cloudwatch:ListMetrics`, `tag:GetResources` and `autoscaling:DescribeAutoScalingGroups` |
| `gcp` | Workload Identity (`iam.gke.io/gcp-service-account`) | `roles/monitoring.viewer` on the project, which also reads its quota, load balancers and Cloud NAT |
| `azure` | Workload identity (`azure.workload.identity/client-id`, plus a pod label), or a service principal as `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `AZURE_CLIENT_SECRET` in the `mzmon-alloy-gateway-env` Secret | Monitoring Reader on each named server, storage account and NAT gateway, and on each named AKS cluster and its node resource group, which holds its load balancers |

Azure workload identity needs two things on the gateway, not one.
The annotation names the identity.
The Entra webhook injects it only into pods labelled `azure.workload.identity/use: "true"`, set through `alloy-gateway.controller.podLabels`.
The chart's Terraform module sets the label whenever the gateway's annotations include `azure.workload.identity/client-id`.
On Azure the grant can be scoped to each resource, unlike on the other two clouds.
Resource Graph returns only the resources the identity can read, so the grant is also what decides which named resources are pulled.

On GCP, the service account the Terraform module creates for the Google Cloud Metrics exporter holds `roles/monitoring.metricWriter`, which writes metrics and cannot read them.
Reading needs `roles/monitoring.viewer` added to the same account.

The chart warns at render when a provider is enabled and the gateway's service account carries no matching annotation.
On Azure it also warns when the annotation has no pod label beside it.
It cannot tell whether EKS Pod Identity, a static key, or a direct Workload Identity principal is in use instead, so the warning is advisory.

The providers fail differently without a credential.
CloudWatch resolves its credential on each pull, so a missing one produces empty pulls and the gateway keeps running.
The GCP exporter resolves its credential when it starts.
On GKE the metadata server always supplies one, so the result is again empty pulls.
Outside Google Cloud there is no metadata server, so `GOOGLE_APPLICATION_CREDENTIALS` has to point at a mounted key or a Workload Identity Federation configuration.
Without one the exporter cannot start, and the gateway fails to load along with every log and metric it carries.
Azure, like CloudWatch, resolves its credential on the first pull, so the gateway keeps running.
On AKS without the pod label, the pull falls through to the node's managed identity, which Resource Graph refuses with a 403.
`up` is 0, and the gateway logs `service discovery failed`.

## Lag

Provider data is minutes old when it arrives, and that decides how it can be used.

| Source | Age on arrival | Timestamp |
|---|---|---|
| CloudWatch RDS | A few minutes | The scrape's |
| CloudWatch S3 | Up to a day | The scrape's |
| Cloud SQL | About three minutes | Cloud Monitoring's |
| GCS | Over ten minutes | Cloud Monitoring's |
| Azure Flexible Server, Blob requests | About a minute | The scrape's |
| Azure Blob capacity | Up to a day | The scrape's |
| CloudWatch EC2, Auto Scaling, vCPU usage | A few minutes | The scrape's |
| Compute Engine quota usage and limit | Up to a day; measured at three points a day | The scrape's |
| Compute Engine quota refusals | A few minutes | Cloud Monitoring's |
| Azure AKS autoscaler and node VMs | About a minute | The scrape's |
| CloudWatch Network Load Balancer and NAT gateway | A few minutes | The scrape's |
| Cloud Monitoring load balancers | Five to seven minutes | The scrape's |
| Cloud NAT | About four minutes | The scrape's |
| Azure load balancers and NAT gateways | About a minute | The scrape's |

CloudWatch and Azure samples are stamped at scrape time.
Stamping a daily S3 datapoint with its own time would put it a day in the past, and the bundled Thanos Receive, which accepts no out-of-order samples, refuses samples that old.
Compute Engine quota usage and limits are stamped at scrape time for the same reason,
since each pull reads back a day to find a point that may be hours old.
A usage point therefore reads as current until the next one lands, and a change can take hours to show.

Cloud Monitoring samples keep their own timestamps.
An instant query at "now" looks back five minutes by default and finds no GCS sample at all.
Queries on provider families need `last_over_time(<series>[15m])` or a wider window.

**Every gateway restart leaves a gap as long as the lag.**
Remote write forwards only samples stamped after it started, so that a restart does not resend what was already written.
A Cloud Monitoring sample stamped before the restart is therefore never sent.
After each restart, Cloud SQL series resume a few minutes later and GCS series over ten minutes later.
CloudWatch and Azure samples carry the scrape's time, so they have no gap,
and neither do the Cloud Monitoring load balancer and Cloud NAT pulls,
which are stamped at the scrape for the reason [Load balancers and NAT gateways](#load-balancers-and-nat-gateways) gives.

No provider signal backs a fast page.
Provider alerts cover the slow-moving conditions — storage headroom, burst-credit exhaustion, connection ceilings — with `for:` windows well above the publication delay.

## Cost

The provider bills each pull, and the bill depends on configuration, not on how many dashboards are open.

| Provider | Calls per pull | Measured |
|---|---|---|
| `cloudwatch` | 12 `GetMetricStatistics` calls per RDS instance and 2 per bucket | 30 calls for two instances and three buckets, in-cluster |
| `gcp` | One descriptor listing per metric prefix, and one time-series listing per matching metric type | 19 calls for two instances and two buckets, counted by `stackdriver_monitoring_api_calls_total` |
| `azure` | One Resource Graph query per service, then one metrics call per resource, since each set is under Azure's twenty-per-call limit. Blob Storage is two pulls, capacity and requests | 9 calls for two servers and two accounts, counted from the exporter's code, since it exports no call counter |
| `cloudwatch` EKS | One `tag:GetResources` and one `DescribeAutoScalingGroups`, one `ListMetrics` per metric, `GetMetricData` billed per metric requested, and one `GetMetricStatistics` for vCPU usage | 26 metrics requested for eight nodes and one node group, counted by `yace_cloudwatch_getmetricdata_metrics_requested_total` |
| `gcp` quota | Four descriptor listings and four time-series listings for usage and limit, and two of each for refusals | 12 calls, counted by `stackdriver_monitoring_api_calls_total` |
| `azure` AKS | Two Resource Graph queries, then one metrics call per cluster and one per node scale set | 5 calls for one cluster with two node pools |
| `cloudwatch` load balancers and NAT | 6 `GetMetricStatistics` calls per Network Load Balancer, 2 per target group, and 6 per NAT gateway | Counted from the configuration, for the reason [Checking it works](#checking-it-works) gives |
| `gcp` load balancers | Four descriptor listings, and one time-series listing per metric type found, external and internal | 12 calls, counted by `stackdriver_monitoring_api_calls_total` |
| `gcp` Cloud NAT | Five descriptor listings and five time-series listings | 10 calls, counted by `stackdriver_monitoring_api_calls_total` |
| `azure` load balancers and NAT | Three Resource Graph queries for the load balancers and two for the NAT gateways, then one metrics call per resource in each | 10 calls for one cluster's load balancer and one NAT gateway, counted from the exporter's code |

`scrapeInterval` is the main lever, and defaults to five minutes.
Rates change, so current provider pricing is the reference for what a call costs.

The gateway runs several replicas, and every replica runs the exporter.
The scrape of it is clustered, so one replica owns it and each provider is called once per interval.

## Which destinations receive it

Each provider assigns its families a tier through `metricImportance`, which defaults to `extended`.
The families `infra-cloud` and `infra-autoscaling` draw are also named in the query registry, at `diagnostic`, the lowest tier.
A registry tier admits a metric at that tier and above, so `metricImportance` decides for every destination floor above `diagnostic`.

| Destination `minMetricImportance` | Receives provider families at the default |
|---|---|
| `all` (the bundled Thanos) | Yes |
| `diagnostic`, `extended` | Yes |
| `recommended`, `essential` | No |

A destination that bills per series, such as Datadog or a BYOC fan-out, therefore does not receive them unless its floor or the provider's tier is changed.

The Google Cloud destination deserves the same care on an install that also pulls from GCP.
Its default floor is `recommended`, which keeps the pulled families out.
Raising either would write each series back into Cloud Monitoring as a `prometheus.googleapis.com/` metric: a billed second copy of data Cloud Monitoring already holds.
The pull does not read those types back, so it does not loop.

## What is deliberately not pulled

| Signal | Why |
|---|---|
| GCS `api/request_count` | A per-minute DELTA, and the exporter adds only the newest point of each pull to its counter. At a five-minute interval it reports about a fifth of the real count. The clients report the same requests exactly |
| S3 request metrics | Opt-in per bucket on the AWS side, and billed as custom metrics |
| Azure Blob `Transactions` | The window each pull reads ends at the scrape, so its newest five-minute bucket is a minute or so short, and a count read from it under-counts. `Availability` already falls with throttling and server errors, and the clients report the same requests exactly |
| Azure resource tags | The exporter copies the `owner` tag onto every series by default. Tags are free text, and the pull copies none |
| Resources found by tag | Tag discovery pulls every matching resource in the account, and bills for each. EKS nodes are the exception, found by the cluster tag EKS and Karpenter set, because they are replaced too often to name. Google Cloud load balancers listed by network are another, because GKE names them only once it makes them |
| The EC2 vCPU quota's limit | CloudWatch publishes the usage but not the limit, which only CloudWatch metric math and the Service Quotas API return, and the exporter supports neither |
| Capacity refusals | No provider publishes a metric for a launch it could not fulfil, such as `InsufficientInstanceCapacity`, `ZONE_RESOURCE_POOL_EXHAUSTED` or `AllocationFailed`. Karpenter and the cluster autoscaler report them |
| Azure compute quota | Not an Azure Monitor metric; it is only in the Compute usage API |
| GKE node pool instance groups | `instance_group/size` has no target size beside it, so it says no more than the node count, and the group names are truncated beyond reliable matching |
| The GKE cluster autoscaler | GKE publishes no metrics for it, only its visibility log in Cloud Logging and Kubernetes events |
| AKS `kube_*` and `node_*` platform metrics | kube-state-metrics and node-exporter already report the same, at full resolution |
| Application Load Balancers | The Terraform wrappers put Materialize behind Network Load Balancers |
| Google Cloud load balancer round-trip latency | `rtt_latencies` is a distribution, which the exporter turns into a histogram per client location, and it covers only TCP |
| NAT gateway bytes | Open connections and failures say whether egress works, and cAdvisor already reports what each pod sends |
| Azure load balancer SNAT ports | The Terraform wrappers send outbound traffic through a NAT gateway, so the load balancer allocates none |
| VPC flow logs and cross-zone traffic | Flow logs are logs, not metrics, and cross-zone volume is a billing question no provider publishes per cluster |

## Checking it works

**On CloudWatch and GCP, `up` does not say whether a pull succeeded.**
An exporter whose provider call fails still answers its scrape, with no provider series in it.
Measured with credentials missing — for CloudWatch anywhere, and for GCP where a metadata server exists — both exporters return HTTP 200 and report healthy, so `up` stays 1.
`up` only says the exporter exists and is being scraped.

On Azure it says half of it.
A Resource Graph query that fails — no identity, or one with no access to the subscription — fails the scrape, and `up` is 0.
A metrics call that fails for one resource only drops that resource's series and logs a warning.
A resource the identity cannot read is never found at all.
`up` stays 1 in both cases.

| Question | Query |
|---|---|
| Is the exporter running and scraped | `up{job=~"integrations/(cloudwatch\|gcp\|azure)"}` |
| Did the last GCP pull fail | `stackdriver_monitoring_last_scrape_error == 1` |
| Is CloudWatch returning data, per instance | `count by (dimension_DBInstanceIdentifier) (aws_rds_cpuutilization_average)` |
| Is Cloud Monitoring returning data, per instance | `count by (database_id) (last_over_time(stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up[15m]))` |
| Is Azure returning data, per server | `count by (resourceName) (azure_microsoft_dbforpostgresql_flexibleservers_is_db_alive_minimum_count)` |
| Are EKS nodes being found | `count(aws_ec2_status_check_failed_system_maximum)`, against the cluster's node count |
| Is Compute Engine quota arriving | `count by (location) (stackdriver_compute_googleapis_com_location_compute_googleapis_com_quota_cpus_per_vm_family_usage)` |
| Are AKS node VMs being found | `count by (resourceName) (azure_microsoft_compute_virtualmachinescalesets_vmavailabilitymetric_minimum_count)` |
| Which load balancer and NAT pulls are running | `up{provider_pull=~".+"}` |
| Is CloudWatch returning target health, per target group | `count by (dimension_TargetGroup) (aws_networkelb_healthy_host_count_minimum)` |
| Is Azure finding the AKS load balancers | `count by (resourceName, resourceGroup) (azure_microsoft_network_loadbalancers_vipavailability_average_count)` |

CloudWatch publishes no equivalent of the GCP error series.
Its `yace_cloudwatch_getmetricstatistics_requests_total` counts billed calls, but it is one counter per gateway replica, and every CloudWatch target a replica owns reports it.
Measured with five targets across two replicas, the five series read 12, 14, 26, 28 and 2: running totals of 28 and 2 calls, not 82.
It cannot be summed across `instance`, so the cost is best read from the configuration: 12 calls per RDS instance and 2 per bucket, each interval.
A CloudWatch pull that fails for want of an identity or a grant is visible only as missing series and as errors in the gateway's logs, such as `Couldn't get account Id`.
An EKS pull missing `tag:GetResources` finds no nodes, and one missing `autoscaling:DescribeAutoScalingGroups` finds no node groups,
while the vCPU usage still arrives.

The Collection tab of the Infrastructure Cloud Provider dashboard (`infra-cloud`) draws these checks for every provider it finds.

