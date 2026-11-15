# Recorded Series




# Recorded Series

These are the series the chart's recording rules write, generated from the query registry.
A recorded series is a metric name this repository mints, so it reads the same whatever produced the measurement underneath it.
The Thanos ruler evaluates the rules and writes the results back through the alloy-gateway, so a recorded series is queryable wherever the gateway's other metrics are.

Every `ext:*` series is part of the normalized layer for an external dependency.
One name is recorded by each **adapter** that can measure it, and the `flavor` label names the adapter.

| Label | On | Holds |
|---|---|---|
| `flavor` | Every `ext:*` series | The adapter that recorded it: `persist` is Materialize's own measurement, `rds`, `cloudsql` and `azure-postgres` are a cloud provider's, and `postgres` is the postgres-exporter's |
| `namespace` | Series from `persist` | The Materialize environment's namespace |
| `resource` | Series from a cloud provider or the exporter | The database's name at the provider, or the exporter target's name, as declared in `externalDependencies.consensus` |
| `database` | `ext:consensus_database_bytes` | The name of one database on the instance |
| `version` | `ext:consensus_version_info` | The version the database server reports |

An adapter that cannot measure a series records nothing for it, rather than a zero.
The provider and exporter adapters record only the databases `externalDependencies.consensus` declares; see [Configuring Alerting](../../alerting/configuring/#recorded-series).

Recorded-series names are covered by the [stability policy](../stability/) from the release that first ships them, like alert names.


<h3 id="ext-consensus_up"><code>ext:consensus_up</code>
  <a class="anchor" href="#ext-consensus_up">#</a>
</h3>
<h4><code>flavor="persist"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_persist</code></small></h4>
1 while Materialize&rsquo;s calls to the metadata database succeed, per
environment namespace, and 0 when none has succeeded in five minutes.
An environment makes about 80 calls a second while idle, so five
minutes without a success is a database it cannot reach rather than one
it had no reason to call. A call that hangs counts as not succeeding.
Absent when the environment is not being scraped.
<p><strong>Recorded where:</strong> <code>materialize</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_persist_external_succeeded_count{op<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">consensus_.*</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4><code>flavor="cloudsql"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_cloudsql</code></small></h4>
1 while Cloud SQL reports each metadata database instance up, and 0
when it reports it down.
<p><strong>Recorded where:</strong> <code>cloud-monitoring</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>stackdriver_cloudsql_database_cloudsql_googleapis_com_database_up{database_id<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i).+:(${consensusCloudsqlResources})</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">database_id</span>&#34;, &#34;<span style="color:#e6db74">.+:(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>flavor="azure-postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_azure_postgres</code></small></h4>
1 while Azure reports each metadata database flexible server alive, and
0 when it reports it down.
<p><strong>Recorded where:</strong> <code>azure-monitor</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>azure_microsoft_dbforpostgresql_flexibleservers_is_db_alive_minimum_count{resourceName<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusAzurePostgresResources}</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">resourceName</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>flavor="postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_postgres</code></small></h4>
1 while the postgres-exporter can connect to each metadata database,
and 0 when it cannot.
From <code>pg_up</code>. Absent while the exporter itself cannot be scraped,
which is a fault in the exporter rather than in the database.
<p><strong>Recorded where:</strong> <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    pg_up{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;},
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">target</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_commit_latency_seconds-p99"><code>ext:consensus_commit_latency_seconds:p99</code>
  <a class="anchor" href="#ext-consensus_commit_latency_seconds-p99">#</a>
</h3>
<h4><code>flavor="persist"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_persist</code></small></h4>
The 99th-percentile time a commit to the metadata database takes, over
five minutes, per environment namespace.
A commit is persist&rsquo;s compare-and-set, <code>op=&quot;consensus_cas&quot;</code>, the only
consensus operation with a latency histogram. Measured by the client,
so it includes the network and any wait for a pooled connection.
Absent when there was no commit in the window.
<p><strong>Recorded where:</strong> <code>materialize</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_persist_external_op_latency_bucket{op<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">consensus_cas</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;=</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_xid_used_ratio"><code>ext:consensus_xid_used_ratio</code>
  <a class="anchor" href="#ext-consensus_xid_used_ratio">#</a>
</h3>
<h4><code>flavor="rds"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_rds</code></small></h4>
The fraction of PostgreSQL&rsquo;s transaction-ID space each RDS metadata
database has used, against the 2^31 at which it stops accepting writes.
From CloudWatch&rsquo;s <code>MaximumUsedTransactionIDs</code>.
<p><strong>Recorded where:</strong> <code>cloudwatch</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>aws_rds_maximum_used_transaction_ids_maximum{dimension_DBInstanceIdentifier<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusRdsResources}</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">dimension_DBInstanceIdentifier</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">2147483648</span>
</span></span></code></pre></div>
<h4><code>flavor="cloudsql"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_cloudsql</code></small></h4>
The fraction of PostgreSQL&rsquo;s transaction-ID space each Cloud SQL
metadata database has used.
Cloud SQL publishes this as a ratio of the same 2^31 limit the other
adapters divide by.
<p><strong>Recorded where:</strong> <code>cloud-monitoring</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>stackdriver_cloudsql_database_cloudsql_googleapis_com_database_postgresql_transaction_id_utilization{database_id<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i).+:(${consensusCloudsqlResources})</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">database_id</span>&#34;, &#34;<span style="color:#e6db74">.+:(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>flavor="azure-postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_azure_postgres</code></small></h4>
The fraction of PostgreSQL&rsquo;s transaction-ID space each metadata database
flexible server has used, against the 2^31 at which it stops accepting
writes.
<p><strong>Recorded where:</strong> <code>azure-monitor</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>azure_microsoft_dbforpostgresql_flexibleservers_maximum_used_transactionids_maximum_count{resourceName<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusAzurePostgresResources}</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">resourceName</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">2147483648</span>
</span></span></code></pre></div>
<h4><code>flavor="postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_postgres</code></small></h4>
The fraction of PostgreSQL&rsquo;s transaction-ID space each metadata
database&rsquo;s instance has used, against the 2^31 at which it stops
accepting writes.
The oldest <code>age(datfrozenxid)</code> of any database on the instance, since
wraparound stops the whole instance whichever database reaches it.
<p><strong>Recorded where:</strong> <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    pg_database_wraparound_age_datfrozenxid_seconds{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;},
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">target</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">2147483648</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_storage_used_ratio"><code>ext:consensus_storage_used_ratio</code>
  <a class="anchor" href="#ext-consensus_storage_used_ratio">#</a>
</h3>
<h4><code>flavor="cloudsql"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_cloudsql</code></small></h4>
The fraction of each Cloud SQL metadata database&rsquo;s provisioned disk in
use.
<p><strong>Recorded where:</strong> <code>cloud-monitoring</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>stackdriver_cloudsql_database_cloudsql_googleapis_com_database_disk_utilization{database_id<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i).+:(${consensusCloudsqlResources})</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">database_id</span>&#34;, &#34;<span style="color:#e6db74">.+:(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>flavor="azure-postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_azure_postgres</code></small></h4>
The fraction of each metadata database flexible server&rsquo;s provisioned
storage in use.
<p><strong>Recorded where:</strong> <code>azure-monitor</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    last_over_time<span style="color:#f92672">(</span>azure_microsoft_dbforpostgresql_flexibleservers_storage_percent_maximum_percent{resourceName<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusAzurePostgresResources}</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">resourceName</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">100</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_connections_used_ratio"><code>ext:consensus_connections_used_ratio</code>
  <a class="anchor" href="#ext-consensus_connections_used_ratio">#</a>
</h3>
<h4><code>flavor="postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_postgres</code></small></h4>
Client connections open on each metadata database&rsquo;s instance, as a
fraction of the connections an ordinary role may open.
The ceiling is <code>max_connections</code> less <code>superuser_reserved_connections</code>
and, from PostgreSQL 16, <code>reserved_connections</code>, which is where an
ordinary role is refused. Connections to every database on the
instance count, not only Materialize&rsquo;s, because they share the
ceiling.
While a rollout runs two exporter pods, each probes the database, and
their copies are counted once.
<p><strong>Recorded where:</strong> <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>pg_stat_activity_count{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;, backend_type<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">client backend</span>&#34;}<span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>        <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>pg_settings_max_connections{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">-</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>pg_settings_superuser_reserved_connections{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">-</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>          <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>pg_settings_reserved_connections{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>          <span style="color:#f92672">or</span> <span style="color:#ae81ff">0</span> <span style="color:#f92672">*</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>pg_settings_max_connections{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>        <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">target</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_database_bytes"><code>ext:consensus_database_bytes</code>
  <a class="anchor" href="#ext-consensus_database_bytes">#</a>
</h3>
<h4><code>flavor="postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_postgres</code></small></h4>
The size of each database on each metadata database&rsquo;s instance, with
the database&rsquo;s name as <code>database</code>.
Every database on the instance, not only Materialize&rsquo;s: a neighbour
filling the volume is what this series exists to attribute, and the
instance&rsquo;s total cannot.
<p><strong>Recorded where:</strong> <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource, database<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      pg_database_size_bytes{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;},
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">target</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">database</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">datname</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="ext-consensus_version_info"><code>ext:consensus_version_info</code>
  <a class="anchor" href="#ext-consensus_version_info">#</a>
</h3>
<h4><code>flavor="postgres"</code> <small>from <code>ext-consensus</code>, group <code>ext_consensus_postgres</code></small></h4>
1, with the PostgreSQL version each metadata database runs as
<code>version</code>.
The server&rsquo;s own version number, such as <code>16.4.0</code>. Join it onto
another <code>ext:consensus_*</code> series with <code>group_left (version)</code> on
<code>flavor</code> and <code>resource</code>.
<p><strong>Recorded where:</strong> <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>resource, version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      pg_static{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;},
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">resource</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">target</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">version</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">short_version</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="mzmon-heartbeat"><code>mzmon:heartbeat</code>
  <a class="anchor" href="#mzmon-heartbeat">#</a>
</h3>
<h4><code>chart_version="__mzmon_chart_version__"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat</code></small></h4>
1 at every evaluation, labelled with the version of the
materialize-monitoring chart that installed the rule.
Reads no metric, so it stops only when the ruler, its query path, or
its writes through the gateway stop. Each ruler replica writes its
own, under its own <code>ruler_replica</code>.
<p><strong>Recorded where:</strong> <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">vector</span><span style="color:#f92672">(</span><span style="color:#ae81ff">1</span><span style="color:#f92672">)</span></span></span></code></pre></div>
<h3 id="mzmon-cluster_created_timestamp_seconds"><code>mzmon:cluster_created_timestamp_seconds</code>
  <a class="anchor" href="#mzmon-cluster_created_timestamp_seconds">#</a>
</h3>
<h4><small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_kube_state_metrics</code></small></h4>
When the cluster&rsquo;s <code>kube-system</code> namespace was created, in seconds
since the epoch.
Stands in for when the cluster was created. A cluster rebuilt from the
same values reports a new value from then on, and a cloned one
reports its own beside the original&rsquo;s.
<p><strong>Recorded where:</strong> <code>kube-state-metrics</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#f92672">(</span>kube_namespace_created{namespace<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">kube-system</span>&#34;}<span style="color:#f92672">)</span></span></span></code></pre></div>
<h3 id="mzmon-environment_info"><code>mzmon:environment_info</code>
  <a class="anchor" href="#mzmon-environment_info">#</a>
</h3>
<h4><small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_kube_state_metrics</code></small></h4>
1 for each Materialize environment in the cluster, by
<code>environment_id</code>.
Read from the <code>Materialize</code> resources. Carries the environment&rsquo;s UUID
and not its name, which is the resource&rsquo;s name.
<p><strong>Recorded where:</strong> <code>kube-state-metrics</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>environment_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_mz_environment_info<span style="color:#f92672">)</span></span></span></code></pre></div>
<h3 id="mzmon-build_info-count"><code>mzmon:build_info:count</code>
  <a class="anchor" href="#mzmon-build_info-count">#</a>
</h3>
<h4><code>component="kubelet"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_kube_state_metrics</code></small></h4>
Nodes running each kubelet version.
<code>version</code> is the kubelet&rsquo;s, as the node reports it, which is the
cluster&rsquo;s Kubernetes version once an upgrade finishes. Each node
counts once however many kube-state-metrics replicas report it.
<p><strong>Recorded where:</strong> <code>kube-state-metrics</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, kubelet_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info<span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">version</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">kubelet_version</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>component="environmentd"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_materialize</code></small></h4>
Running environmentd processes of each Materialize version.
<p><strong>Recorded where:</strong> <code>materialize</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_start_time_environmentd<span style="color:#f92672">)</span></span></span></code></pre></div>
<h4><code>component="alloy"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_alloy</code></small></h4>
Running Alloy collectors, agents and gateways together, of each version.
<p><strong>Recorded where:</strong> <code>alloy</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>alloy_build_info<span style="color:#f92672">)</span></span></span></code></pre></div>
<h4><code>component="loki"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_loki</code></small></h4>
Running Loki processes of each version, every component together.
<p><strong>Recorded where:</strong> <code>loki</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>loki_build_info<span style="color:#f92672">)</span></span></span></code></pre></div>
<h4><code>component="node-exporter"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_node_exporter</code></small></h4>
Running node-exporter processes of each version.
<p><strong>Recorded where:</strong> <code>node-exporter</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_exporter_build_info<span style="color:#f92672">)</span></span></span></code></pre></div>
<h4><code>component="postgres"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_postgres</code></small></h4>
Metadata databases running each PostgreSQL version, read through the
postgres-exporter.
Covers each <code>externalDependencies.consensus</code> entry with
<code>flavor: postgres</code>, whichever service runs the database: Amazon RDS,
Cloud SQL, Azure Database for PostgreSQL, or a cluster of your own.
Absent where none is declared, or the exporter doesn&rsquo;t run.
<p><strong>Recorded where:</strong> <code>thanos</code>, <code>postgres-exporter</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>target, short_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      pg_static{job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">postgres-exporter</span>&#34;, target<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">(?i)${consensusPostgresResources}</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">version</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">short_version</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4><code>component="thanos"</code> <small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_thanos</code></small></h4>
Running Thanos processes of each version, every component together.
<p><strong>Recorded where:</strong> <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>thanos_build_info<span style="color:#f92672">)</span></span></span></code></pre></div>
<h3 id="mzmon-node_uname_info-count"><code>mzmon:node_uname_info:count</code>
  <a class="anchor" href="#mzmon-node_uname_info-count">#</a>
</h3>
<h4><small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_node_exporter</code></small></h4>
Nodes with each CPU architecture and kernel release.
<code>machine</code> and <code>release</code> are <code>uname -m</code> and <code>uname -r</code> on the node,
such as <code>aarch64</code> and <code>6.12.40-64.114.amzn2023.aarch64</code>. The node&rsquo;s
host name stays behind.
<p><strong>Recorded where:</strong> <code>node-exporter</code>, <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>machine, release<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>nodename, machine, release<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_uname_info<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="mzmon-rules-count"><code>mzmon:rules:count</code>
  <a class="anchor" href="#mzmon-rules-count">#</a>
</h3>
<h4><small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_thanos</code></small></h4>
The rules the Thanos ruler evaluates, alerting and recording, on the
replica that loaded the most.
Includes these heartbeat rules. A count well below the install&rsquo;s
usual one is a rule file the ruler refused to load.
<p><strong>Recorded where:</strong> <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>prometheus_rule_group_rules{app<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">thanos</span>&#34;, container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">ruler</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h3 id="mzmon-rule_evaluation_failures-rate5m"><code>mzmon:rule_evaluation_failures:rate5m</code>
  <a class="anchor" href="#mzmon-rule_evaluation_failures-rate5m">#</a>
</h3>
<h4><small>from <code>mzmon-heartbeat</code>, group <code>mzmon_heartbeat_thanos</code></small></h4>
Failed rule evaluations per second over five minutes, on the Thanos
ruler replica with the most.
0 while every rule evaluates. A failing rule records nothing and
never fires, so its alert goes missing from <code>ALERTS</code> without a sign
anywhere else.
<p><strong>Recorded where:</strong> <code>thanos</code>.</p>
        
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>prometheus_rule_evaluation_failures_total{app<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">thanos</span>&#34;, container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">ruler</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>


