# PostgreSQL exporter




# PostgreSQL exporter

The chart can run [`postgres_exporter`](https://github.com/prometheus-community/postgres_exporter) against the PostgreSQL databases a deployment depends on:
Materialize's metadata database and the database behind the bundled Grafana.
This page is for the operator who turns it on.

The exporter connects to each database as an ordinary SQL client and reports what the database says about itself.
That covers connections against the ceiling, transactions, locks, the size of every database on the instance, transaction-ID age, and vacuum on the tables of the database it connects to.
It works the same on RDS, Cloud SQL, Azure Database for PostgreSQL, and a self-managed server.

It complements the two views the stack has without it.
Materialize measures every call it makes to the metadata database, which says whether the database is slow, and needs nothing configured.
A [cloud provider pull](../cloud-provider-metrics/) sees the machine the database runs on, such as CPU, memory, storage headroom, and burst credits, and nothing inside it.
The exporter sees inside the database and not the machine, so it answers why: a stalled autovacuum on persist's consensus table, a session holding a transaction open, or a neighboring database filling the volume.
For the reasoning, see the [external dependency design](../../../reference/development/design-docs/20260920-external-dependency-monitoring/#three-vantage-points-none-of-them-a-substitute).

<!--
Agent note: the subchart is prometheus-community/prometheus-postgres-exporter, aliased postgres-exporter.
The chart renders its own ServiceMonitor (templates/scrapers/monitor-postgres-exporter.yaml); the queries are
packages/queries/infra-postgres.yaml and the ext:consensus_* adapter is the ext_consensus_postgres group in
packages/queries/ext-consensus.yaml. Metric names and labels on this page were read from postgres_exporter v0.20.1.
-->

## How it runs

One Deployment serves every database.
The Alloy gateway scrapes it once per target on `/probe`, and the exporter opens a connection to that database for the length of the scrape.
Each scrape's series carry the target's name as `target` and `job="postgres-exporter"`, including `up`.

The exporter reads each target's credentials from a file, `postgres_exporter.yml`, and from nowhere else.
The chart mounts that file from a Secret you create, so no credential passes through Helm values.

## Turn it on

To watch the metadata database and Grafana's database, follow these steps:

1. Create a role for the exporter on each PostgreSQL instance.
   Run the following as a role that can grant `pg_monitor`, such as the administrator role your database service gives you:

   ```sql
   CREATE ROLE mzmon_exporter WITH LOGIN PASSWORD 'PASSWORD' CONNECTION LIMIT 3;
   GRANT pg_monitor TO mzmon_exporter;
   GRANT CONNECT ON DATABASE DATABASE_NAME TO mzmon_exporter;
   ```

   Replace the following:

   - `PASSWORD`: a password for the role.
   - `DATABASE_NAME`: the database the exporter connects to, such as `materialize` for the metadata database, or `postgres` for Grafana's.
     For more information, see [Choose the database to connect to](#choose-the-database-to-connect-to).

   `pg_monitor` is a predefined role that reads every statistics view and setting, and it grants no access to table data.
   A scrape holds one connection, so the connection limit leaves room for an overlapping scrape and nothing more.

2. Create the Secret in the release namespace.
   It holds one key, `postgres_exporter.yml`, with one `auth_modules` entry per target, named after the target:

   ```yaml
   auth_modules:
     consensus:
       type: userpass
       userpass:
         username: mzmon_exporter
         password: PASSWORD
       options:
         sslmode: require
     grafana:
       type: userpass
       userpass:
         username: mzmon_exporter
         password: PASSWORD
       options:
         sslmode: require
   ```

   ```bash
   kubectl create secret generic postgres-exporter -n monitoring --from-file=postgres_exporter.yml
   ```

   Provision it with External Secrets Operator, Vault Agent, SOPS, or your cloud's CSI driver, so that the password never lands in Git.
   Each entry under `options` becomes a connection parameter.
   For `sslmode: verify-full`, mount the server's CA certificate with `postgres-exporter.extraVolumes` and `postgres-exporter.extraVolumeMounts`, and point `sslrootcert` at it.

3. Turn on the exporter, list the targets, and say which one is the metadata database:

   ```yaml
   tags:
     postgres-exporter: true

   postgres-exporter:
     targets:
       - name: consensus
         host: METADATA_DB_HOST
         database: materialize
       - name: grafana
         host: GRAFANA_DB_HOST

   externalDependencies:
     consensus:
       - flavor: postgres
         resourceId: consensus
   ```

   Replace `METADATA_DB_HOST` and `GRAFANA_DB_HOST` with each database's host, as a pod in the cluster resolves it.
   Each target also takes `database`, which defaults to `postgres`, `port`, which defaults to `5432`, and `authModule`, which defaults to the target's name.

The exporter reads the Secret only when it starts.
After you change the Secret, restart the exporter:

```bash
kubectl rollout restart deployment/postgres-exporter -n monitoring
```

The exporter needs network access to each database.
It runs in the cluster that already reaches the metadata database, so the path Materialize uses is the one it needs.
A database behind a private endpoint, such as a Cloud SQL private IP, is reachable only from networks the endpoint allows.

### Turn it on with Terraform

The `materialize-monitoring` Terraform module creates the Secret for you.
Set `enable_postgres_exporter`, and the exporter watches Grafana's database whenever the module configures one, as a target named `grafana`.
List any other database in `postgres_exporter_targets`, with its password in `postgres_exporter_passwords`:

```hcl
module "monitoring" {
  # ...
  enable_postgres_exporter = true

  postgres_exporter_targets = [{
    name              = "consensus"
    host              = METADATA_DB_HOST
    database          = "materialize"
    user              = "mzmon_exporter"
    metadata_database = true
  }]
  postgres_exporter_passwords = {
    consensus = var.metadata_exporter_password
  }
}
```

Replace `METADATA_DB_HOST` with the metadata database's host.
`metadata_database = true` adds the target to `externalDependencies.consensus`.
A database a cloud provider pull watches is declared in `metadata_databases` instead, by its flavor and the provider's name for it.
The module rolls the exporter when a password changes.

The per-cloud wrappers in `materialize-terraform-self-managed` connect as the users they already hold: the metadata database's administrator and Grafana's database owner.
On RDS, Cloud SQL, and Azure Database for PostgreSQL, both hold `pg_monitor` through the service's administrator role (`rds_superuser`, `cloudsqlsuperuser`, or `azure_pg_admin`), so the exporter sees every session.
Both can also write.
For a role that can only read statistics, create `mzmon_exporter` as described earlier, and pass it to the wrapper instead.

### Choose the database to connect to

Point each target's `database` at a database whose tables you want statistics for, and leave it at `postgres` otherwise.
Per-table statistics, the `pg_stat_user_tables_*` and `pg_statio_user_tables_*` families, come from that database alone, at about 29 series per table.
For the metadata database, that's where persist's `consensus` and `timestamp_oracle` tables live, so connect to `materialize`.
Grafana's database holds about 90 tables, so connecting to it costs about 2,600 series that no panel reads.
Everything instance-wide, including `pg_database_size_bytes`, covers every database on the instance whichever one the connection opens.
The chart keeps the size collector unscoped, because a neighboring database filling the volume is what per-database sizes exist to attribute.

### Grafana's database

The bundled Grafana keeps users, preferences, API keys, and annotations in its database when you use the [`grafana-postgres` profile](https://github.com/MaterializeInc/materialize-monitoring/blob/main/charts/materialize-monitoring/profiles/grafana-postgres.values.yaml).
Its target is scraped like any other, and you can pick it on the dashboard.
Leave it out of `externalDependencies.consensus`, so that it's never recorded as a metadata database.

## What it collects

The exporter's default collectors run, plus the following, which the chart turns on:

- **`--collector.database_wraparound`:** `pg_database_wraparound_age_datfrozenxid_seconds` per database, the transaction-ID age that wraparound is measured against.
  The name ends in `_seconds`, but it counts transactions.
- **`--collector.stat_activity_autovacuum`:** `pg_stat_activity_autovacuum_timestamp_seconds`, the start time of each autovacuum worker running now, by table.

The families the dashboard reads are in the `extended` [metric importance](../../../reference/list-metrics/) tier.
A destination that keeps only `recommended` metrics and above drops them.
The exporter publishes more families than the dashboard reads, and a destination keeps those only at `all`.

## Where it shows up

The **Database Internals** tab of the **Materialize Consensus (Metadata)** dashboard has a **PostgreSQL** section.
It renders when the exporter is scraping, and its **Database** picker chooses the target.
For more information, see [Available Dashboards](/materialize-monitoring/preview/renovate-loki-20-x/dashboards/all/).

The target you name in `externalDependencies.consensus` is also recorded into the normalized metadata-database series, with `flavor="postgres"` and the target's name as `resource`.
The following table lists the series the exporter adds:

| Recorded series | From |
|---|---|
| `ext:consensus_up` | `pg_up` |
| `ext:consensus_connections_used_ratio` | Client connections against `max_connections`, less the reserved slots |
| `ext:consensus_xid_used_ratio` | The oldest database's transaction-ID age, against 2^31 |
| `ext:consensus_database_bytes` | `pg_database_size_bytes`, with the database's name as `database` |
| `ext:consensus_version_info` | `pg_static`, with the server version as `version` |

For the rest of the normalized series, see [Recorded Series](/materialize-monitoring/preview/renovate-loki-20-x/reference/recorded-series/).

## Troubleshoot it

If the PostgreSQL section doesn't render, check `up{job="postgres-exporter"}`.
No series means nothing scrapes the exporter: check that `postgres-exporter.targets` isn't empty and that the Alloy gateway is running.

If a target's `up` is 0, the exporter refused the probe.
The usual cause is a Secret with no `auth_modules` entry named after the target or its `authModule`, which the exporter answers with HTTP status 400.

If `up` is 1 and `pg_up` is 0, the exporter can't connect to the database.
The exporter's log names the target and the error, which is usually one of the following:

- **`password authentication failed`:** the entry's credentials are wrong, or the Secret changed and the exporter wasn't restarted.
- **A timeout:** the pod has no network path to the database.

If connection counts look low, the role is likely missing `pg_monitor`.
Without it, PostgreSQL hides the state of other roles' sessions, and the exporter can't count them.

