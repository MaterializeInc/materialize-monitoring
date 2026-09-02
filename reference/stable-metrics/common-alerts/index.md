# Common Alerts




# Common Alerts

> [!WARNING]
> Many of these alerts are not suited for all deployments.
> Do not use the entire set as is!



## infra-alerts

<p>Alerting rules for the infrastructure Materialize depends on.</p>
<p>Each alert carries its firing expression as an inline query, is graded by a
<code>severity</code> label (critical &gt; warning &gt; notice), and is tagged with a <code>component</code>
label naming the subsystem it watches.</p>

<h4 id="crdb-disk-usage-critical">crdb-disk-usage-critical
  <a class="anchor" href="#crdb-disk-usage-critical">#</a>
</h4>
CockroachDB disk usage is above 90% and likely to impact Materialize.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  crdb_dedicated_capacity_used <span style="color:#f92672">/</span> crdb_dedicated_capacity
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">90</span>
</span></span></code></pre></div>
<h4 id="crdb-disk-usage-high">crdb-disk-usage-high
  <a class="anchor" href="#crdb-disk-usage-high">#</a>
</h4>
CockroachDB disk usage is above 70% and may need a capacity increase.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  crdb_dedicated_capacity_used <span style="color:#f92672">/</span> crdb_dedicated_capacity
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">70</span>
</span></span></code></pre></div>
<h4 id="crdb-disk-usage-elevated">crdb-disk-usage-elevated
  <a class="anchor" href="#crdb-disk-usage-elevated">#</a>
</h4>
CockroachDB disk usage is above 30% and worth keeping an eye on.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  crdb_dedicated_capacity_used <span style="color:#f92672">/</span> crdb_dedicated_capacity
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">30</span>
</span></span></code></pre></div>
<h4 id="crdb-cpu-usage-critical">crdb-cpu-usage-critical
  <a class="anchor" href="#crdb-cpu-usage-critical">#</a>
</h4>
CockroachDB CPU usage is critically high (&gt;89%) and likely to impact Materialize.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg_over_time</span><span style="color:#f92672">(</span>crdb_dedicated_sys_cpu_combined_percent_normalized[<span style="color:#e6db74">30m</span>]<span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">89</span>
</span></span></code></pre></div>
<h4 id="crdb-cpu-usage-high">crdb-cpu-usage-high
  <a class="anchor" href="#crdb-cpu-usage-high">#</a>
</h4>
CockroachDB CPU usage is above 85% over 2h and may impact Materialize.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg_over_time</span><span style="color:#f92672">(</span>crdb_dedicated_sys_cpu_combined_percent_normalized[<span style="color:#e6db74">2h</span>]<span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">85</span>
</span></span></code></pre></div>
<h4 id="crdb-query-latency">crdb-query-latency
  <a class="anchor" href="#crdb-query-latency">#</a>
</h4>
CockroachDB p95 SQL service latency has been above 250ms for an extended period.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.95</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>crdb_dedicated_sql_service_latency_bucket[<span style="color:#e6db74">20m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">250</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span>
</span></span></code></pre></div>
<h4 id="crdb-lsm-read-amplification-critical">crdb-lsm-read-amplification-critical
  <a class="anchor" href="#crdb-lsm-read-amplification-critical">#</a>
</h4>
CockroachDB LSM read amplification is critically high (&gt;150), indicating severe I/O overload.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_rocksdb_read_amplification<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">150</span>
</span></span></code></pre></div>
<h4 id="crdb-lsm-read-amplification-high">crdb-lsm-read-amplification-high
  <a class="anchor" href="#crdb-lsm-read-amplification-high">#</a>
</h4>
CockroachDB LSM read amplification is elevated (&gt;50), a sign writes may be outpacing compaction.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_rocksdb_read_amplification<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">50</span>
</span></span></code></pre></div>
<h4 id="crdb-ranges-unavailable">crdb-ranges-unavailable
  <a class="anchor" href="#crdb-ranges-unavailable">#</a>
</h4>
CockroachDB has unavailable ranges, which may indicate node failures or replication issues.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_ranges_unavailable<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="crdb-ranges-underreplicated">crdb-ranges-underreplicated
  <a class="anchor" href="#crdb-ranges-underreplicated">#</a>
</h4>
CockroachDB has under-replicated ranges, which may indicate node failures or replication issues.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_ranges_underreplicated<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="crdb-sql-memory-rapid-growth">crdb-sql-memory-rapid-growth
  <a class="anchor" href="#crdb-sql-memory-rapid-growth">#</a>
</h4>
CockroachDB SQL memory is growing faster than 8 MB/s, a sign of a runaway query heading toward OOM.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">deriv</span><span style="color:#f92672">(</span>crdb_dedicated_sql_mem_distsql_current[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">8</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span>
</span></span></code></pre></div>
<h4 id="crdb-sql-memory-pressure-high">crdb-sql-memory-pressure-high
  <a class="anchor" href="#crdb-sql-memory-pressure-high">#</a>
</h4>
CockroachDB distsql memory exceeds 18% of node RAM, indicating high user-query memory pressure.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  crdb_dedicated_sql_mem_distsql_current
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node_id<span style="color:#f92672">)</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  crdb_dedicated_sys_totalmem
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.18</span>
</span></span></code></pre></div>
<h4 id="crdb-write-intent-accumulation-critical">crdb-write-intent-accumulation-critical
  <a class="anchor" href="#crdb-write-intent-accumulation-critical">#</a>
</h4>
CockroachDB write-intent count has exceeded 10M for 10m, indicating large transactions holding locks.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_intentcount<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span>
</span></span></code></pre></div>
<h4 id="crdb-write-intent-accumulation-high">crdb-write-intent-accumulation-high
  <a class="anchor" href="#crdb-write-intent-accumulation-high">#</a>
</h4>
CockroachDB write-intent count has exceeded 10M, a sign of large transactions accumulating locks.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>crdb_dedicated_intentcount<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span>
</span></span></code></pre></div>
<h4 id="crdb-backup-missing">crdb-backup-missing
  <a class="anchor" href="#crdb-backup-missing">#</a>
</h4>
A CockroachDB backup may be missing — the last completed backup is more than 80 minutes old.
Labels:
<ul>
        <li><strong>component:</strong> cockroachdb</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#f92672">(</span><span style="color:#66d9ef">max_over_time</span><span style="color:#f92672">(</span>crdb_dedicated_schedules_backup_last_completed_time[<span style="color:#e6db74">60m</span>]<span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="cilium-bpf-map-pressure">cilium-bpf-map-pressure
  <a class="anchor" href="#cilium-bpf-map-pressure">#</a>
</h4>
A Cilium BPF map is filling up, which will cause networking issues if left unaddressed.
Labels:
<ul>
        <li><strong>component:</strong> cilium</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>map_name, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  cilium_bpf_map_pressure
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.7</span>
</span></span></code></pre></div>
<h4 id="cilium-drop-rate-elevated">cilium-drop-rate-elevated
  <a class="anchor" href="#cilium-drop-rate-elevated">#</a>
</h4>
Cilium is dropping more packets than expected, which can indicate networking issues.
Labels:
<ul>
        <li><strong>component:</strong> cilium</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>direction, reason, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>cilium_drop_count_total[<span style="color:#e6db74">5m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span>
</span></span></code></pre></div>
<h4 id="coredns-slow-queries">coredns-slow-queries
  <a class="anchor" href="#coredns-slow-queries">#</a>
</h4>
CoreDNS p99 request latency is above 500ms, which can cause or accompany broader outages.
Labels:
<ul>
        <li><strong>component:</strong> coredns</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, service<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>coredns_dns_request_duration_seconds_bucket{zone<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">.</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.5</span>
</span></span></code></pre></div>
<h4 id="node-unreachable">node-unreachable
  <a class="anchor" href="#node-unreachable">#</a>
</h4>
A Kubernetes node is marked unreachable, which may indicate a node or network problem.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>kube_node_spec_taint{key<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">node.kubernetes.io/unreachable</span>&#34;}
</span></span></code></pre></div>
<h4 id="k8s-horizontalpodautoscaler-replicas-critical">k8s-horizontalpodautoscaler-replicas-critical
  <a class="anchor" href="#k8s-horizontalpodautoscaler-replicas-critical">#</a>
</h4>
A HorizontalPodAutoscaler is above 90% of its maximum replicas — nearly out of headroom to scale.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_horizontalpodautoscaler_status_current_replicas{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">alloy|loki</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> kube_horizontalpodautoscaler_spec_max_replicas{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">alloy|loki</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.9</span>
</span></span></code></pre></div>
<h4 id="k8s-horizontalpodautoscaler-replicas-high">k8s-horizontalpodautoscaler-replicas-high
  <a class="anchor" href="#k8s-horizontalpodautoscaler-replicas-high">#</a>
</h4>
A HorizontalPodAutoscaler is above 70% of its maximum replicas.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_horizontalpodautoscaler_status_current_replicas
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> kube_horizontalpodautoscaler_spec_max_replicas
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.7</span>
</span></span></code></pre></div>
<h4 id="k8s-container-disk-usage">k8s-container-disk-usage
  <a class="anchor" href="#k8s-container-disk-usage">#</a>
</h4>
A container filesystem is above 70% usage and should be cleaned up or resized.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>kubernetes_io_hostname<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>container_fs_usage_bytes<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>kubernetes_io_hostname<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>container_fs_limit_bytes<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">70</span>
</span></span></code></pre></div>
<h4 id="k8s-node-disk-pressure">k8s-node-disk-pressure
  <a class="anchor" href="#k8s-node-disk-pressure">#</a>
</h4>
A Kubernetes node is under disk pressure, which can cause pods to fail scheduling or be evicted.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_node_status_condition{condition<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">DiskPressure</span>&#34;, status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">true</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="k8s-volume-usage">k8s-volume-usage
  <a class="anchor" href="#k8s-volume-usage">#</a>
</h4>
A Kubernetes persistent volume is above 70% usage and should be cleaned up or resized.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, persistentvolumeclaim<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kubelet_volume_stats_used_bytes{persistentvolumeclaim<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*cluster-s2-.*|.*cluster-u.*</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, persistentvolumeclaim<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kubelet_volume_stats_capacity_bytes{persistentvolumeclaim<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*cluster-s2-.*|.*cluster-u.*</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">70</span>
</span></span></code></pre></div>
<h4 id="alloy-log-drops">alloy-log-drops
  <a class="anchor" href="#alloy-log-drops">#</a>
</h4>
Alloy is dropping log entries — logs are being lost right now.
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, job, reason<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_write_dropped_entries_total[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="loki-push-err-high">loki-push-err-high
  <a class="anchor" href="#loki-push-err-high">#</a>
</h4>
Loki&rsquo;s push endpoint is returning 10%+ write errors, so logs are being rejected.
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count{status_code<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">4.*|5.*</span>&#34;, route<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*push.*</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count{route<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*push.*</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span>
</span></span></code></pre></div>
<h4 id="loki-panics">loki-panics
  <a class="anchor" href="#loki-panics">#</a>
</h4>
A Loki component has panicked.
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, job, app_kubernetes_io_component<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>loki_panic_total[<span style="color:#e6db74">10m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="loki-req-duration-high">loki-req-duration-high
  <a class="anchor" href="#loki-req-duration-high">#</a>
</h4>
Loki p95 request duration is above 1s (excluding tail/long-poll routes).
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.95</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, app_kubernetes_io_component<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_bucket{route<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">(?i).*tail.*|/schedulerpb.SchedulerForQuerier/QuerierLoop</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="loki-req-err-high">loki-req-err-high
  <a class="anchor" href="#loki-req-err-high">#</a>
</h4>
Loki read requests are returning 10%+ 5xx errors.
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component, route<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count{status_code<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">5.*</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component, route<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span>
</span></span></code></pre></div>
<h4 id="loki-writer-err-high">loki-writer-err-high
  <a class="anchor" href="#loki-writer-err-high">#</a>
</h4>
Loki write requests are erroring for 10%+ of requests.
Labels:
<ul>
        <li><strong>component:</strong> loki</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component, route<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count{status_code<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">error</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, app_kubernetes_io_component, route<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_request_duration_seconds_count[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span>
</span></span></code></pre></div>
<h4 id="clusterd-metrics-missing">clusterd-metrics-missing
  <a class="anchor" href="#clusterd-metrics-missing">#</a>
</h4>
All metrics for a Materialize pod have been missing for 60m — the scrape target is likely down.
Labels:
<ul>
        <li><strong>component:</strong> monitoring</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  up{job<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">materialize</span>&#34;, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">s5</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="critical-metrics-missing">critical-metrics-missing
  <a class="anchor" href="#critical-metrics-missing">#</a>
</h4>
More than 30% of a critical scrape job&rsquo;s targets have been down for 30m.
Labels:
<ul>
        <li><strong>component:</strong> monitoring</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>job, app<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    up{job<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cilium|crdb|environment_controller|kubernetes-nodes.*|kubernetes-pods|lvm-exporter|node-exporter|new-promsql-exporter|region_controller</span>&#34;} <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>job, app<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    up{job<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cilium|crdb|environment_controller|kubernetes-nodes.*|kubernetes-pods|lvm-exporter|node-exporter|new-promsql-exporter|region_controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">30</span>
</span></span></code></pre></div>
<h4 id="logging-collection-down">logging-collection-down
  <a class="anchor" href="#logging-collection-down">#</a>
</h4>
Loki has received no logs for 15m — log collection is down.
Labels:
<ul>
        <li><strong>component:</strong> monitoring</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>loki_distributor_bytes_received_total{namespace<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">loki</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="k8s-deployment-unavailable-critical">k8s-deployment-unavailable-critical
  <a class="anchor" href="#k8s-deployment-unavailable-critical">#</a>
</h4>
A core control-plane deployment is unavailable.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>deployment, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_deployment_status_condition{condition<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">Available</span>&#34;, status<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">true</span>&#34;, deployment<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="k8s-deployment-unavailable-warning">k8s-deployment-unavailable-warning
  <a class="anchor" href="#k8s-deployment-unavailable-warning">#</a>
</h4>
An important supporting deployment is unavailable.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>deployment, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_deployment_status_condition{condition<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">Available</span>&#34;, status<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">true</span>&#34;, deployment<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="k8s-deployment-unavailable-notice">k8s-deployment-unavailable-notice
  <a class="anchor" href="#k8s-deployment-unavailable-notice">#</a>
</h4>
A non-essential deployment is unavailable.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>deployment, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_deployment_status_condition{condition<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">Available</span>&#34;, status<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">true</span>&#34;, deployment<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">backend|egress-check.*|egress-noop.*|exporter|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="k8s-daemonset-saturating-cpu">k8s-daemonset-saturating-cpu
  <a class="anchor" href="#k8s-daemonset-saturating-cpu">#</a>
</h4>
Daemonsets are requesting nearly all the CPU reserved for them, squeezing clusterd headroom.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">1.77</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      kube_pod_container_resource_requests{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cilium-agent|csi-node-driver-registrar|ebs-plugin|eip-operator|lvm-exporter|node-driver-registrar|node-exporter|openebs-lvm-plugin|parca-agent|vector</span>&#34;, unit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">core</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">0.01</span>
</span></span></code></pre></div>
<h4 id="k8s-daemonset-saturating-mem">k8s-daemonset-saturating-mem
  <a class="anchor" href="#k8s-daemonset-saturating-mem">#</a>
</h4>
Daemonsets are requesting nearly all the memory reserved for them, squeezing clusterd headroom.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">10730942464</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      kube_pod_container_resource_requests{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cilium-agent|csi-node-driver-registrar|ebs-plugin|eip-operator|lvm-exporter|node-driver-registrar|node-exporter|openebs-lvm-plugin|parca-agent|vector</span>&#34;, unit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">byte</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">5243000</span>
</span></span></code></pre></div>
<h4 id="k8s-daemonset-high-cpu">k8s-daemonset-high-cpu
  <a class="anchor" href="#k8s-daemonset-high-cpu">#</a>
</h4>
Daemonsets are approaching the CPU budget reserved for them.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">1.77</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      kube_pod_container_resource_requests{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cilium-agent|csi-node-driver-registrar|ebs-plugin|eip-operator|lvm-exporter|node-driver-registrar|node-exporter|openebs-lvm-plugin|parca-agent|vector</span>&#34;, unit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">core</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">0.1</span>
</span></span></code></pre></div>
<h4 id="container-file-descriptors-critical">container-file-descriptors-critical
  <a class="anchor" href="#container-file-descriptors-critical">#</a>
</h4>
A core container&rsquo;s open file descriptors are above 70% of its limit and it may be killed.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_file_descriptors{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_ulimits_soft{ulimit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">max_open_files</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">70</span>
</span></span></code></pre></div>
<h4 id="container-file-descriptors-warning">container-file-descriptors-warning
  <a class="anchor" href="#container-file-descriptors-warning">#</a>
</h4>
A non-core container&rsquo;s open file descriptors are above 70% of its limit and it may be killed.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_file_descriptors{container<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_ulimits_soft{ulimit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">max_open_files</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">70</span>
</span></span></code></pre></div>
<h4 id="container-file-descriptors-elevated">container-file-descriptors-elevated
  <a class="anchor" href="#container-file-descriptors-elevated">#</a>
</h4>
A container&rsquo;s open file descriptors are above 20% of its limit.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>container_file_descriptors<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_ulimits_soft{ulimit<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">max_open_files</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">20</span>
</span></span></code></pre></div>
<h4 id="infra-pod-high-cpu-ratio">infra-pod-high-cpu-ratio
  <a class="anchor" href="#infra-pod-high-cpu-ratio">#</a>
</h4>
A vector pod has used more than its full CPU request for 6h and may be throttled.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{container<span style="color:#f92672">!=</span>&#34;&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">vector-.*</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">vector-.*</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="infra-pods-high-cpu-ratio">infra-pods-high-cpu-ratio
  <a class="anchor" href="#infra-pods-high-cpu-ratio">#</a>
</h4>
An infra pod is using more than its full CPU request and may be throttled.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">parca-agent-.*|vector-.*|cilium-egress-.*</span>&#34;}[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">parca-agent-.*|vector-.*|cilium-egress-.*</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="infra-memory-high">infra-memory-high
  <a class="anchor" href="#infra-memory-high">#</a>
</h4>
An important infra container is above 80% memory usage.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_spec_memory_limit_bytes{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="infra-memory-elevated">infra-memory-elevated
  <a class="anchor" href="#infra-memory-elevated">#</a>
</h4>
A supporting infra container is above 80% memory usage.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_spec_memory_limit_bytes{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="infra-oomkill-core-systems">infra-oomkill-core-systems
  <a class="anchor" href="#infra-oomkill-core-systems">#</a>
</h4>
A core infra container has been OOMKilled.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container, namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;} <span style="color:#66d9ef">offset</span> <span style="color:#e6db74">3h</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">ignoring</span><span style="color:#f92672">(</span>reason<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_last_terminated_reason{reason<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">OOMKilled</span>&#34;, container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">admission-webhook|aws-load-balancer-controller|balancer.*|cilium-agent|cilium-operator|controller|coredns|csi-attacher|csi-node-driver-registrar|csi-provisioner|csi-resizer|csi-snapshotter|daemonset-taint-remover|environment-controller|external-dns|.*internal-api.*|.*region-api.*|.*sync-server|karpenter|node-cache|node-driver-registrar|openebs-lvm.*|region-controller|.*scheduler|snapshot-controller</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="infra-oomkill-important-systems">infra-oomkill-important-systems
  <a class="anchor" href="#infra-oomkill-important-systems">#</a>
</h4>
An important infra container has been OOMKilled.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container, namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;} <span style="color:#66d9ef">offset</span> <span style="color:#e6db74">3h</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">ignoring</span><span style="color:#f92672">(</span>reason<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_last_terminated_reason{reason<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">OOMKilled</span>&#34;, container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">cert-manager.*|ebs-plugin|ebs-csi-controller|eip-operator|hubble-relay|kube-state-metrics|liveness-probe|metrics-server|new-promsql-exporter|node-driver-registrar|node-exporter|prometheus-adapter|sidecar-aws-sigv4-proxy</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="infra-oomkill-nonessential-systems">infra-oomkill-nonessential-systems
  <a class="anchor" href="#infra-oomkill-nonessential-systems">#</a>
</h4>
A non-essential infra container has been OOMKilled.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container, namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">awslimitchecker|backend|egress-check.*|egress-noop.*|exporter|external-uptime-checker|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">awslimitchecker|backend|egress-check.*|egress-noop.*|exporter|external-uptime-checker|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;} <span style="color:#66d9ef">offset</span> <span style="color:#e6db74">3h</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">ignoring</span><span style="color:#f92672">(</span>reason<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_last_terminated_reason{reason<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">OOMKilled</span>&#34;, container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">awslimitchecker|backend|egress-check.*|egress-noop.*|exporter|external-uptime-checker|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="k8s-infra-pod-pending-too-long">k8s-infra-pod-pending-too-long
  <a class="anchor" href="#k8s-infra-pod-pending-too-long">#</a>
</h4>
An infra pod has been Pending for 15m — the cluster may be unhealthy or out of capacity.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_status_phase{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, phase<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">Pending</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="pod-restart-rate-high">pod-restart-rate-high
  <a class="anchor" href="#pod-restart-rate-high">#</a>
</h4>
An important infra container is restarting frequently, which may indicate a crash loop.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>kube_pod_container_status_restarts_total{namespace<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, container<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">awslimitchecker|backend|egress-check.*|egress-noop.*|exporter|external-uptime-checker|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;}[<span style="color:#e6db74">10m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="pod-restart-rate-high-nonessential">pod-restart-rate-high-nonessential
  <a class="anchor" href="#pod-restart-rate-high-nonessential">#</a>
</h4>
A non-essential infra container is restarting frequently, which may indicate a crash loop.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>kube_pod_container_status_restarts_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">awslimitchecker|backend|egress-check.*|egress-noop.*|exporter|external-uptime-checker|frontend|hubble-ui|loki.*|memcached|metrics-proxy|nginx|overprovisioning-placeholder|pause|polarsignals-scraper|tbot|teleport|cert-manager-csi-driver|lvm-exporter|parca-agent|vector</span>&#34;}[<span style="color:#e6db74">10m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="pods-stuck-in-waiting">pods-stuck-in-waiting
  <a class="anchor" href="#pods-stuck-in-waiting">#</a>
</h4>
A pod has been stuck in Waiting for over 10m, which can indicate a scheduling or resource problem.
Labels:
<ul>
        <li><strong>component:</strong> kubernetes</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_status_waiting <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_start_time <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="egress-traffic-missing-metrics">egress-traffic-missing-metrics
  <a class="anchor" href="#egress-traffic-missing-metrics">#</a>
</h4>
Egress-gateway node throughput metrics are missing, which may indicate an egress-gateway problem.
Labels:
<ul>
        <li><strong>component:</strong> egress-gateway</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">absent</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">*</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#66d9ef">group_left</span> <span style="color:#f92672">(</span>workload<span style="color:#f92672">)</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, workload<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{workload<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize-egress</span>&#34;}, &#34;<span style="color:#e6db74">node</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">instance</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">^</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="excessive-egress-traffic">excessive-egress-traffic
  <a class="anchor" href="#excessive-egress-traffic">#</a>
</h4>
An egress-gateway node has very high traffic, which may indicate traffic is not routing through the internet gateway.
Labels:
<ul>
        <li><strong>component:</strong> egress-gateway</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">*</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#66d9ef">group_left</span> <span style="color:#f92672">(</span>workload<span style="color:#f92672">)</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, workload<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{workload<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize-egress</span>&#34;}, &#34;<span style="color:#e6db74">node</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">instance</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">^</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">5.5</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">8</span>
</span></span></code></pre></div>
<h4 id="high-egress-traffic">high-egress-traffic
  <a class="anchor" href="#high-egress-traffic">#</a>
</h4>
An egress-gateway node is above 90% of its allowed traffic and may soon exceed capacity.
Labels:
<ul>
        <li><strong>component:</strong> egress-gateway</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">*</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#66d9ef">group_left</span> <span style="color:#f92672">(</span>workload<span style="color:#f92672">)</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, workload<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{workload<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize-egress</span>&#34;}, &#34;<span style="color:#e6db74">node</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">instance</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">^</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">4.5</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span> <span style="color:#f92672">/</span> <span style="color:#ae81ff">8</span>
</span></span></code></pre></div>
<h4 id="low-egress-traffic">low-egress-traffic
  <a class="anchor" href="#low-egress-traffic">#</a>
</h4>
An egress-gateway node has unusually low traffic, which may indicate the gateway is unhealthy.
Labels:
<ul>
        <li><strong>component:</strong> egress-gateway</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">*</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#66d9ef">group_left</span> <span style="color:#f92672">(</span>workload<span style="color:#f92672">)</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, workload<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{workload<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize-egress</span>&#34;}, &#34;<span style="color:#e6db74">node</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">instance</span>&#34;, &#34;<span style="color:#e6db74">(.+)</span>&#34;<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">^</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">100</span>
</span></span></code></pre></div>
<h4 id="no-egress-traffic">no-egress-traffic
  <a class="anchor" href="#no-egress-traffic">#</a>
</h4>
An egress-gateway node has had no traffic for 5m, which may indicate external traffic is blocked.
Labels:
<ul>
        <li><strong>component:</strong> egress-gateway</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">5m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span> <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_bytes_total{device<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">eth0</span>&#34;}[<span style="color:#e6db74">5m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> kubelet_node_name{workload<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize-egress</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>

## materialize-alerts

<p>Alerting rules for Materialize.</p>
<p>Each alert carries its firing expression as an inline query, is graded by a
<code>severity</code> label (critical &gt; warning &gt; notice), and is tagged with a
<code>component</code> label. Per-environment alerts group abstractly and attach the
environment name via the <code>mzEnvironmentName</code> template function; the optional
<code>%%{excludeEnvironmentFilter}</code> fragment lets a deployment exclude environments
(e.g. internal test environments). Alerts that only apply to Materialize Cloud
carry <code>deploymentMode: cloud-only</code>.</p>

<h4 id="env-uptime-sla">env-uptime-sla
  <a class="anchor" href="#env-uptime-sla">#</a>
</h4>
environmentd is not accepting basic connections and may be unreachable.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  v2_mz_can_connect{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;=</span> <span style="color:#ae81ff">0.1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_start_time <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">5</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="env-uptime-slo">env-uptime-slo
  <a class="anchor" href="#env-uptime-slo">#</a>
</h4>
environmentd is not accepting basic connections and may be unreachable.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  v2_mz_can_connect{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;=</span> <span style="color:#ae81ff">0.1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_start_time <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">5</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="envd-simplest-query">envd-simplest-query
  <a class="anchor" href="#envd-simplest-query">#</a>
</h4>
environmentd is not responding to a SELECT 1 and may be unhealthy.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  v2_mz_envd_up{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;=</span> <span style="color:#ae81ff">0.1</span>
</span></span></code></pre></div>
<h4 id="env-query-views-critical">env-query-views-critical
  <a class="anchor" href="#env-query-views-critical">#</a>
</h4>
environmentd is not answering a simple query that reads from object storage.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  v2_mz_views_query_successful{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;=</span> <span style="color:#ae81ff">0.1</span>
</span></span></code></pre></div>
<h4 id="env-query-views-warning">env-query-views-warning
  <a class="anchor" href="#env-query-views-warning">#</a>
</h4>
environmentd is not answering a simple query that reads from object storage.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  v2_mz_views_query_successful{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;=</span> <span style="color:#ae81ff">0.1</span>
</span></span></code></pre></div>
<h4 id="clusterd-not-receiving-commands">clusterd-not-receiving-commands
  <a class="anchor" href="#clusterd-not-receiving-commands">#</a>
</h4>
A clusterd has not received commands from environmentd for 5m — the replica may be stalled or disconnected.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_cluster_server_last_command_received{server_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">compute</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*0</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_start_time <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">5</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="storage-collection-finalization-stuck">storage-collection-finalization-stuck
  <a class="anchor" href="#storage-collection-finalization-stuck">#</a>
</h4>
Storage shards have been stuck finalizing for 60m.
Labels:
<ul>
        <li><strong>component:</strong> storage</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_shard_finalization_outstanding
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="auth-errors">auth-errors
  <a class="anchor" href="#auth-errors">#</a>
</h4>
An elevated rate of unexpected authentication errors.
Labels:
<ul>
        <li><strong>component:</strong> auth</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, status<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_auth_request_count{status<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">(2|401).*</span>&#34;}[<span style="color:#e6db74">30m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.01</span>
</span></span></code></pre></div>
<h4 id="auth-refresh-failures">auth-refresh-failures
  <a class="anchor" href="#auth-refresh-failures">#</a>
</h4>
An elevated rate of failed auth-token refreshes.
Labels:
<ul>
        <li><strong>component:</strong> auth</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_auth_request_count{status<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">401.*</span>&#34;, path<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">refresh_token</span>&#34;}[<span style="color:#e6db74">15m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0.002</span>
</span></span></code></pre></div>
<h4 id="console-errors">console-errors
  <a class="anchor" href="#console-errors">#</a>
</h4>
The web console has returned an error for 2% or more of commands for 30m, across more than one environment.
Labels:
<ul>
        <li><strong>component:</strong> console</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_adapter_commands{application_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">web_console</span>&#34;, status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">error</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_adapter_commands{application_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">web_console</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">2</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_adapter_commands{application_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">web_console</span>&#34;, status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">error</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_adapter_commands{application_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">web_console</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">2</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="persist-failures">persist-failures
  <a class="anchor" href="#persist-failures">#</a>
</h4>
Failures in Persist that should be rare are happening frequently.
Labels:
<ul>
        <li><strong>component:</strong> persist</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_blob_failures, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_consensus_failures, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_state_update_state_slow_path, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_lease_timeout_read, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">3</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_compaction_noop, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">3</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_compaction_failed, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_external_blob_delete_noop_count, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_external_failed_count, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_cmd_failed_count, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_compaction_dropped, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_pushdown_parts_mismatched_stats_count, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_columnar_validation_count{result<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">invalid</span>&#34;}, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_txn_placeholder_schema_apply, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_columnar_op_count{op<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">validation</span>&#34;, result<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">invalid</span>&#34;}, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_schema_cache_fetch_state_count, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">or</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>metric<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>mz_persist_shard_unconsolidated_snapshot, &#34;<span style="color:#e6db74">metric</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">__name__</span>&#34;, &#34;<span style="color:#e6db74">(.*)</span>&#34;<span style="color:#f92672">)</span>[<span style="color:#e6db74">1m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">15s</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="envd-terminated">envd-terminated
  <a class="anchor" href="#envd-terminated">#</a>
</h4>
An environmentd was unexpectedly terminated — this should not happen.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>kube_pod_container_status_last_terminated_exitcode{container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">environmentd</span>&#34;} <span style="color:#f92672">!=</span> <span style="color:#ae81ff">166</span>
</span></span></code></pre></div>
<h4 id="environmentd-high-cpu">environmentd-high-cpu
  <a class="anchor" href="#environmentd-high-cpu">#</a>
</h4>
An environmentd has been above 80% CPU usage for 90m.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*environmentd.+</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;}[<span style="color:#e6db74">30m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_cpu_quota{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*environmentd.+</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> container_spec_cpu_period{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*environmentd.+</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="environmentd-high-memory">environmentd-high-memory
  <a class="anchor" href="#environmentd-high-memory">#</a>
</h4>
An environmentd has been above 80% memory usage for 30m.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*environmentd.+</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_spec_memory_limit_bytes{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*environmentd.+</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;} <span style="color:#f92672">!=</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="environmentd-cpu-throttled">environmentd-cpu-throttled
  <a class="anchor" href="#environmentd-cpu-throttled">#</a>
</h4>
An environmentd container is being CPU throttled more than 50% of the time.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, container, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_cfs_throttled_periods_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environmentd</span>&#34;}[<span style="color:#e6db74">30m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_cfs_periods_total{container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environmentd</span>&#34;}[<span style="color:#e6db74">30m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">50</span>
</span></span></code></pre></div>
<h4 id="environmentd-memory-elevated">environmentd-memory-elevated
  <a class="anchor" href="#environmentd-memory-elevated">#</a>
</h4>
An environmentd is above 80% memory usage (lower-severity companion to environmentd-high-memory).
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">POD</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;, container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environmentd</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_spec_memory_limit_bytes{container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">POD</span>&#34;, container<span style="color:#f92672">!=</span>&#34;&#34;, container<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environmentd</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="clusterd-error-kill">clusterd-error-kill
  <a class="anchor" href="#clusterd-error-kill">#</a>
</h4>
A clusterd was terminated with an unexpected exit code — this should not happen.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>kube_pod_container_status_restarts_total{namespace<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environment.+</span>&#34;, container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">clusterd</span>&#34;}[<span style="color:#e6db74">1h</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_last_terminated_exitcode<span style="color:#f92672">)</span> <span style="color:#f92672">!=</span> <span style="color:#ae81ff">137</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_last_terminated_exitcode<span style="color:#f92672">)</span> <span style="color:#f92672">!=</span> <span style="color:#ae81ff">135</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_last_terminated_exitcode<span style="color:#f92672">)</span> <span style="color:#f92672">!=</span> <span style="color:#ae81ff">166</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_last_terminated_exitcode<span style="color:#f92672">)</span> <span style="color:#f92672">!=</span> <span style="color:#ae81ff">167</span>
</span></span></code></pre></div>
<h4 id="system-cluster-terminated">system-cluster-terminated
  <a class="anchor" href="#system-cluster-terminated">#</a>
</h4>
A system cluster was unexpectedly terminated — this should not happen.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>kube_pod_container_status_last_terminated_exitcode{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*cluster-s.*</span>&#34;} <span style="color:#f92672">!=</span> <span style="color:#ae81ff">166</span>
</span></span></code></pre></div>
<h4 id="system-cluster-high-memory">system-cluster-high-memory
  <a class="anchor" href="#system-cluster-high-memory">#</a>
</h4>
A system cluster is above 80% memory usage — this should not happen for system clusters.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">100</span> <span style="color:#f92672">*</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_memory_limiter_memory_usage_bytes{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*cluster-s.+</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_memory_limiter_memory_limit_bytes{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*cluster-s.+</span>&#34;} <span style="color:#f92672">!=</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">80</span>
</span></span></code></pre></div>
<h4 id="clusterd-expiration-7d">clusterd-expiration-7d
  <a class="anchor" href="#clusterd-expiration-7d">#</a>
</h4>
A cluster replica will expire in less than a week.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>mz_dataflow_replica_expiration_remaining_seconds{pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*cluster.+</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">24</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">7</span>
</span></span></code></pre></div>
<h4 id="swap-cluster-oom">swap-cluster-oom
  <a class="anchor" href="#swap-cluster-oom">#</a>
</h4>
A swap-enabled cluster was OOMKilled while below 80% swap usage.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      kube_pod_container_status_last_terminated_reason{reason<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">OOMKilled</span>&#34;, namespace<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      container_last_seen{materialize_cloud_swap<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">true</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>[<span style="color:#e6db74">10m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max_over_time</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>container_memory_swap<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>      <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>container_spec_memory_swap_limit_bytes<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>[<span style="color:#e6db74">10m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">0.8</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_restarts_total<span style="color:#f92672">)</span>[<span style="color:#e6db74">10m</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="environment-pod-pending-critical">environment-pod-pending-critical
  <a class="anchor" href="#environment-pod-pending-critical">#</a>
</h4>
An environment pod has been Pending for 15m — the cluster may be unhealthy or out of capacity.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod_base<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_status_phase{namespace<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, phase<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">Pending</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">pod_base</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">(.*?)(?:-gen-[0-9]+-[0-9]+|([0-9]+)-[0-9]+)?</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="environment-pod-pending">environment-pod-pending
  <a class="anchor" href="#environment-pod-pending">#</a>
</h4>
An environment pod has been Pending for 15m — the cluster may be unhealthy or out of capacity.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_status_phase{namespace<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">environment.*</span>&#34;, phase<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">Pending</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="certificate-not-ready">certificate-not-ready
  <a class="anchor" href="#certificate-not-ready">#</a>
</h4>
A certificate has not become ready in 20m, which can prevent an environment from coming up.
Labels:
<ul>
        <li><strong>component:</strong> environmentd</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>name, namespace, condition<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  certmanager_certificate_ready_status{condition<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">True</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="console-query-latency">console-query-latency
  <a class="anchor" href="#console-query-latency">#</a>
</h4>
Web-console query p95 latency has exceeded 10s for 15m.
Labels:
<ul>
        <li><strong>component:</strong> console</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.95</span>,
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_time_to_first_row_seconds_bucket{instance_id<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">s2</span>&#34;, application_name<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">web_console</span>&#34;, <span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">10</span>
</span></span></code></pre></div>
<h4 id="new-clusterd-restarts">new-clusterd-restarts
  <a class="anchor" href="#new-clusterd-restarts">#</a>
</h4>
A previously healthy clusterd is restarting during a release.
Labels:
<ul>
        <li><strong>component:</strong> clusterd</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_container_status_restarts_total{container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">clusterd</span>&#34;}<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_created<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">24</span>,
</span></span><span style="display:flex;"><span>  &#34;<span style="color:#e6db74">pod_base</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">.*-(cluster-.*)-gen-([0-9]+)-[0-9]+$</span>&#34;
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod_base<span style="color:#f92672">)</span> <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>kube_pod_container_status_restarts_total{container<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">clusterd</span>&#34;}[<span style="color:#e6db74">12h</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#e6db74">1m</span>]<span style="color:#f92672">))</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">time</span><span style="color:#f92672">()</span> <span style="color:#f92672">-</span> kube_pod_created<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">24</span>,
</span></span><span style="display:flex;"><span>  &#34;<span style="color:#e6db74">pod_base</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">.*-(cluster-.*)-gen-([0-9]+)-[0-9]+$</span>&#34;
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
<h4 id="external-env-uptime">external-env-uptime
  <a class="anchor" href="#external-env-uptime">#</a>
</h4>
environmentd is unreachable from outside the network by the external uptime checker.
Labels:
<ul>
        <li><strong>component:</strong> external-uptime</li>
        <li><strong>deploymentMode:</strong> cloud-only</li>
        <li><strong>severity:</strong> critical</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_external_envd_up
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
<h4 id="external-env-uptime-failed">external-env-uptime-failed
  <a class="anchor" href="#external-env-uptime-failed">#</a>
</h4>
New external connections to environmentd are failing, per the external uptime checker.
Labels:
<ul>
        <li><strong>component:</strong> external-uptime</li>
        <li><strong>deploymentMode:</strong> cloud-only</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>connection_type, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_external_calls_count{status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">failed</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="external-uptime-checker-not-calling">external-uptime-checker-not-calling
  <a class="anchor" href="#external-uptime-checker-not-calling">#</a>
</h4>
The external uptime checker has stopped making calls — the checker itself may be down.
Labels:
<ul>
        <li><strong>component:</strong> external-uptime</li>
        <li><strong>deploymentMode:</strong> cloud-only</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>connection_type, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>mz_external_calls_count{status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">attempted</span>&#34;}[<span style="color:#e6db74">2m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
<h4 id="launchdarkly-stale-sse">launchdarkly-stale-sse
  <a class="anchor" href="#launchdarkly-stale-sse">#</a>
</h4>
The last LaunchDarkly server-side event is more than 40 minutes old — flag updates may not be reaching environmentd.
Labels:
<ul>
        <li><strong>component:</strong> launchdarkly</li>
        <li><strong>deploymentMode:</strong> cloud-only</li>
        <li><strong>severity:</strong> warning</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">timestamp</span><span style="color:#f92672">(</span>mz_parameter_frontend_last_sse_time_seconds{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> mz_parameter_frontend_last_sse_time_seconds{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;=</span> <span style="color:#ae81ff">40</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span></code></pre></div>
<h4 id="launchdarkly-stale-cse">launchdarkly-stale-cse
  <a class="anchor" href="#launchdarkly-stale-cse">#</a>
</h4>
The last LaunchDarkly client-side event is more than 40 minutes old — client analytics may be stale.
Labels:
<ul>
        <li><strong>component:</strong> launchdarkly</li>
        <li><strong>deploymentMode:</strong> cloud-only</li>
        <li><strong>severity:</strong> notice</li>
</ul>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">timestamp</span><span style="color:#f92672">(</span>mz_parameter_frontend_last_cse_time_seconds{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> mz_parameter_frontend_last_cse_time_seconds{<span style="color:#960050;background-color:#1e0010">${excludeEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;=</span> <span style="color:#ae81ff">40</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">60</span>
</span></span></code></pre></div>


