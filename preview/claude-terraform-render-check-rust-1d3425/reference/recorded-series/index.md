# Recorded Series




# Recorded Series

These are the series the chart's recording rules write, generated from the query registry.
A recorded series is a metric name this repository mints, so it reads the same whatever produced the measurement underneath it.
The Thanos ruler evaluates the rules and writes the results back through the alloy-gateway, so a recorded series is queryable wherever the gateway's other metrics are.

Every `ext:*` series is part of the normalized layer for an external dependency.
One name is recorded by each **adapter** that can measure it, and the `flavor` label names the adapter.

| Label | On | Holds |
|---|---|---|
| `flavor` | Every `ext:*` series | The adapter that recorded it: `persist` is Materialize's own measurement, and `rds`, `cloudsql` and `azure-postgres` are a cloud provider's |
| `namespace` | Series from `persist` | The Materialize environment's namespace |
| `resource` | Series from a cloud provider | The database's name at the provider, as declared in `externalDependencies.consensus` |

An adapter that cannot measure a series records nothing for it, rather than a zero.
The provider adapters record only the databases `externalDependencies.consensus` declares; see [Configuring Alerting](../../alerting/configuring/#recorded-series).

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


