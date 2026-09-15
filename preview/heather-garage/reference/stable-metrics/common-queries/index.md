# Common Queries




# Common Queries

These queries are used in various dashboards and may be a good
starting point for a dashboard developer or for someone looking
into particulars of a dashboard.

> [!TIP]
> The underlined values in each query are placeholders, and they are editable.
> Click one, type your environment's value, and every other occurrence of that placeholder on the page follows — so copying a query gives you something you can paste straight into Prometheus.
> Press Enter to commit an edit or Escape to discard it.
>
> You can also set placeholders up front by naming them in this page's URL:
>
>     ?mzSqlPrefix=v2_mz_&mzNamespaceList=materialize-prod
>
> The two directions are the same mechanism: editing in place rewrites the URL to match, so once the page reads the way you want, the address bar holds a link you can bookmark or share.
> Only placeholders you have changed appear there, and a customized value is underlined with a solid rather than a dashed line.
> Reload without the query string to get the defaults back.



## infra-logs

<p>Logs from the platform a Materialize deployment runs on: the monitoring stack
itself, the Kubernetes system components, and the nodes underneath both.</p>
<p>Separate from <code>materialize-logs.yaml</code> rather than a widening of it, for two
reasons that are structural rather than stylistic.</p>
<p><strong>The selector set differs.</strong> These carry <code>component</code> and <code>container</code> filters.
<code>component</code> is what tells one Loki or Thanos process from another — <code>loki</code>
alone splits into <code>canary</code>, <code>querier</code>, <code>ingester</code>, <code>query-frontend</code>,
<code>index-gateway</code>, <code>compactor</code>, <code>distributor</code> and <code>ruler</code> — and <code>container</code> is
the only picker that reaches workloads with no <code>app</code> at all, which on a
representative install is the whole of <code>kube-system</code>. Neither dimension means
anything to a Materialize environment, and adding them to the shared queries
would oblige every dashboard using those to define pickers it has no use for.</p>
<p><strong>The node journal is not reachable from a namespace.</strong> Journal lines carry
<code>unit</code>, <code>component</code>, <code>job</code>, <code>level</code> and <code>service_name</code> and <strong>no <code>namespace</code>,
<code>app</code> or <code>container</code></strong>, because they come from the node rather than from a pod.
Any selector that requires a namespace excludes them by construction, which is
why they have their own queries here and their own tab on the dashboard.</p>
<p>The Kubernetes-event queries are <em>not</em> duplicated: <code>materialize.events.cluster.*</code>
in <code>materialize-events.yaml</code> is already scoped by the same namespace picker and
carries no Materialize-specific filter, so an infrastructure dashboard uses it
as it stands.</p>

<h4 id="infra.logs.stream">infra.logs.stream
  <a class="anchor" href="#infra.logs.stream">#</a>
</h4>
The log feed for the selected namespaces, apps, components, containers
and levels, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.stream-tabs" id="infra.logs.stream-tab-0" checked>
  <label for="infra.logs.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.warnings.stream">infra.logs.warnings.stream
  <a class="anchor" href="#infra.logs.warnings.stream">#</a>
</h4>
Warning-and-worse lines from the platform, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.warnings.stream-tabs" id="infra.logs.warnings.stream-tab-0" checked>
  <label for="infra.logs.warnings.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.rate.by_component">infra.logs.rate.by_component
  <a class="anchor" href="#infra.logs.rate.by_component">#</a>
</h4>
Log lines per second by application and sub-component — which process of
which workload is doing the talking.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.rate.by_component-tabs" id="infra.logs.rate.by_component-tab-0" checked>
  <label for="infra.logs.rate.by_component-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.rate.by_namespace">infra.logs.rate.by_namespace
  <a class="anchor" href="#infra.logs.rate.by_namespace">#</a>
</h4>
Log lines per second by namespace — where in the cluster the volume is.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.rate.by_namespace-tabs" id="infra.logs.rate.by_namespace-tab-0" checked>
  <label for="infra.logs.rate.by_namespace-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.warnings.rate">infra.logs.warnings.rate
  <a class="anchor" href="#infra.logs.warnings.rate">#</a>
</h4>
Warning-and-worse lines per minute across the platform, as one series.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.warnings.rate-tabs" id="infra.logs.warnings.rate-tab-0" checked>
  <label for="infra.logs.warnings.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.node.stream">infra.logs.node.stream
  <a class="anchor" href="#infra.logs.node.stream">#</a>
</h4>
The node journal, newest first — <code>kubelet</code>, <code>containerd</code>, the node
problem detector, and the rest of what systemd runs on each node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.node.stream-tabs" id="infra.logs.node.stream-tab-0" checked>
  <label for="infra.logs.node.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.node.warnings">infra.logs.node.warnings
  <a class="anchor" href="#infra.logs.node.warnings">#</a>
</h4>
Warning-and-worse lines from the node journal.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.node.warnings-tabs" id="infra.logs.node.warnings-tab-0" checked>
  <label for="infra.logs.node.warnings-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.logs.node.rate.by_unit">infra.logs.node.rate.by_unit
  <a class="anchor" href="#infra.logs.node.rate.by_unit">#</a>
</h4>
Node journal lines per second by systemd unit.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.logs.node.rate.by_unit-tabs" id="infra.logs.node.rate.by_unit-tab-0" checked>
  <label for="infra.logs.node.rate.by_unit-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>

## infra-nodes

<p>What <code>kubectl describe node</code> would tell you, for whoever cannot run it.</p>
<p>These answer the questions an operator asks <em>about one machine</em>: what is it,
how big is it, how much of it is already promised, is Kubernetes willing to
put work on it, and what has it been saying. The measurements of what the
machine is actually <em>doing</em> live in <code>node-health.yaml</code> and <code>node-debug.yaml</code>,
which read node-exporter; this file is the Kubernetes side, plus the node&rsquo;s
own journal and the events filed against it.</p>
<p>Two identifier conventions meet here, and the difference is the thing to know:</p>
<ul>
<li>
<p><strong>kube-state-metrics names a node <code>node=&quot;&lt;kubernetes name&gt;&quot;</code>.</strong> Every query
in this file scopes with <code>node=&quot;$node&quot;</code> written literally, the same way the
node-exporter families write <code>instance=~&quot;$nodeList&quot;</code> literally. A dashboard
using either must define the variable; no render parameter supplies it.</p>
</li>
<li>
<p><strong>node-exporter names the same machine <code>instance=&quot;&lt;ip&gt;:9100&quot;</code>.</strong> The join is
<code>node_uname_info</code>, whose <code>nodename</code> is the Kubernetes name — which is what
the <code>$nodeList</code> variable resolves through. Nothing in this file needs the
join, because nothing in this file reads node-exporter.</p>
</li>
</ul>
<p>Loki knows the node a third way: <code>node</code> is <strong>structured metadata</strong> on journal
lines, not a stream label, so it is filtered in the pipeline (<code>| node=...</code>)
rather than in the selector. Node events are Kubernetes events whose involved
object is the node itself, which is <code>kind=&quot;Node&quot;</code> with the node&rsquo;s name.</p>
<p>Two conventions apply to every query here that aggregates:</p>
<ul>
<li>
<p><strong>Deduplicate across kube-state-metrics replicas.</strong> <code>instance</code> is the scrape
target, so a bare <code>sum</code> or <code>count</code> over an HA deployment adds each object
once per replica. Every aggregation keeps <code>instance</code> in the inner step and
collapses it with an outer <code>max</code>, which is the shape
<code>materialize-kubernetes.yaml</code> established. Queries that only ever <code>max</code> are
already safe, since <code>max</code> across identical replicas is idempotent.</p>
</li>
<li>
<p><strong>Terminal pods do not count against the node.</strong> A <code>Succeeded</code> or <code>Failed</code>
pod has released its CPU and memory, and the scheduler no longer counts it
against the pod limit — kube-state-metrics agrees, and stops reporting
<code>kube_pod_container_resource_*</code> for it. But <code>kube_pod_info</code> keeps reporting
it until garbage collection, so anything counting <em>pods</em> rather than their
resources has to subtract them explicitly or it overstates how full the node
is. Completed Jobs are the common case.</p>
</li>
</ul>
<p><code>%%{interval}</code> is the rate window, including its brackets.</p>

<h4 id="infra.nodes.info.kubelet">infra.nodes.info.kubelet
  <a class="anchor" href="#infra.nodes.info.kubelet">#</a>
</h4>
The kubelet version this node runs, which is the version Kubernetes itself is on here.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.info.kubelet-tabs" id="infra.nodes.info.kubelet-tab-0" checked>
  <label for="infra.nodes.info.kubelet-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, kubelet_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.info.os">infra.nodes.info.os
  <a class="anchor" href="#infra.nodes.info.os">#</a>
</h4>
The node&rsquo;s operating system image.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.info.os-tabs" id="infra.nodes.info.os-tab-0" checked>
  <label for="infra.nodes.info.os-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, os_image<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.info.kernel">infra.nodes.info.kernel
  <a class="anchor" href="#infra.nodes.info.kernel">#</a>
</h4>
The node&rsquo;s kernel version.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.info.kernel-tabs" id="infra.nodes.info.kernel-tab-0" checked>
  <label for="infra.nodes.info.kernel-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, kernel_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.info.runtime">infra.nodes.info.runtime
  <a class="anchor" href="#infra.nodes.info.runtime">#</a>
</h4>
The container runtime that starts and stops containers on this node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.info.runtime-tabs" id="infra.nodes.info.runtime-tab-0" checked>
  <label for="infra.nodes.info.runtime-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, container_runtime_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.info.address">infra.nodes.info.address
  <a class="anchor" href="#infra.nodes.info.address">#</a>
</h4>
The address the cluster reaches this node on.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.info.address-tabs" id="infra.nodes.info.address-tab-0" checked>
  <label for="infra.nodes.info.address-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, internal_ip<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.created">infra.nodes.created
  <a class="anchor" href="#infra.nodes.created">#</a>
</h4>
Wall-clock time the node joined the cluster.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.created-tabs" id="infra.nodes.created-tab-0" checked>
  <label for="infra.nodes.created-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_created{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">1000</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.capacity.cpu">infra.nodes.capacity.cpu
  <a class="anchor" href="#infra.nodes.capacity.cpu">#</a>
</h4>
Cores the node reports to Kubernetes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.capacity.cpu-tabs" id="infra.nodes.capacity.cpu-tab-0" checked>
  <label for="infra.nodes.capacity.cpu-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_capacity{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.capacity.memory">infra.nodes.capacity.memory
  <a class="anchor" href="#infra.nodes.capacity.memory">#</a>
</h4>
Bytes of RAM the node reports to Kubernetes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.capacity.memory-tabs" id="infra.nodes.capacity.memory-tab-0" checked>
  <label for="infra.nodes.capacity.memory-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_capacity{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.capacity.pods">infra.nodes.capacity.pods
  <a class="anchor" href="#infra.nodes.capacity.pods">#</a>
</h4>
The most pods Kubernetes will place on this node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.capacity.pods-tabs" id="infra.nodes.capacity.pods-tab-0" checked>
  <label for="infra.nodes.capacity.pods-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_capacity{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">pods</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.capacity.ephemeral_storage">infra.nodes.capacity.ephemeral_storage
  <a class="anchor" href="#infra.nodes.capacity.ephemeral_storage">#</a>
</h4>
Bytes of node-local disk available to pods for scratch space.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.capacity.ephemeral_storage-tabs" id="infra.nodes.capacity.ephemeral_storage-tab-0" checked>
  <label for="infra.nodes.capacity.ephemeral_storage-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_capacity{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">ephemeral_storage</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.allocation.cpu">infra.nodes.allocation.cpu
  <a class="anchor" href="#infra.nodes.allocation.cpu">#</a>
</h4>
Fraction of the node&rsquo;s schedulable CPU already promised to pods through
their requests.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.allocation.cpu-tabs" id="infra.nodes.allocation.cpu-tab-0" checked>
  <label for="infra.nodes.allocation.cpu-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_allocatable{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.allocation.memory">infra.nodes.allocation.memory
  <a class="anchor" href="#infra.nodes.allocation.memory">#</a>
</h4>
Fraction of the node&rsquo;s schedulable memory already promised to pods
through their requests.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.allocation.memory-tabs" id="infra.nodes.allocation.memory-tab-0" checked>
  <label for="infra.nodes.allocation.memory-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_allocatable{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.allocation.pods">infra.nodes.allocation.pods
  <a class="anchor" href="#infra.nodes.allocation.pods">#</a>
</h4>
Fraction of the node&rsquo;s pod slots in use.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.allocation.pods-tabs" id="infra.nodes.allocation.pods-tab-0" checked>
  <label for="infra.nodes.allocation.pods-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">unless</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_status_phase{phase<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">Succeeded|Failed</span>&#34;} <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_allocatable{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">pods</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.pods.by_namespace">infra.nodes.pods.by_namespace
  <a class="anchor" href="#infra.nodes.pods.by_namespace">#</a>
</h4>
What is actually running on this node, grouped by namespace.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.pods.by_namespace-tabs" id="infra.nodes.pods.by_namespace-tab-0" checked>
  <label for="infra.nodes.pods.by_namespace-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">unless</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_status_phase{phase<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">Succeeded|Failed</span>&#34;} <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.condition.ready">infra.nodes.condition.ready
  <a class="anchor" href="#infra.nodes.condition.ready">#</a>
</h4>
Whether Kubernetes considers the node healthy enough to run work.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.condition.ready-tabs" id="infra.nodes.condition.ready-tab-0" checked>
  <label for="infra.nodes.condition.ready-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_status_condition{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, condition<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">Ready</span>&#34;, status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">true</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.conditions">infra.nodes.conditions
  <a class="anchor" href="#infra.nodes.conditions">#</a>
</h4>
The node&rsquo;s pressure and availability conditions — memory, disk, PIDs and
network — each 1 when the condition is active.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.conditions-tabs" id="infra.nodes.conditions-tab-0" checked>
  <label for="infra.nodes.conditions-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, condition<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_node_status_condition{
</span></span><span style="display:flex;"><span>    node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;,
</span></span><span style="display:flex;"><span>    condition<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">MemoryPressure|DiskPressure|PIDPressure|NetworkUnavailable</span>&#34;,
</span></span><span style="display:flex;"><span>    status<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">true</span>&#34;
</span></span><span style="display:flex;"><span>  }
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.unschedulable">infra.nodes.unschedulable
  <a class="anchor" href="#infra.nodes.unschedulable">#</a>
</h4>
Whether the node has been cordoned against new work.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.unschedulable-tabs" id="infra.nodes.unschedulable-tab-0" checked>
  <label for="infra.nodes.unschedulable-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_spec_unschedulable{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.taints">infra.nodes.taints
  <a class="anchor" href="#infra.nodes.taints">#</a>
</h4>
The taints on this node, which restrict what may be scheduled.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.taints-tabs" id="infra.nodes.taints-tab-0" checked>
  <label for="infra.nodes.taints-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>node, key, value, effect<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_node_spec_taint{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.pods.by_phase">infra.nodes.pods.by_phase
  <a class="anchor" href="#infra.nodes.pods.by_phase">#</a>
</h4>
Pods on this node, counted by lifecycle phase.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.pods.by_phase-tabs" id="infra.nodes.pods.by_phase-tab-0" checked>
  <label for="infra.nodes.pods.by_phase-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>phase<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>phase, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_status_phase <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> kube_pod_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.pods.not_ready">infra.nodes.pods.not_ready
  <a class="anchor" href="#infra.nodes.pods.not_ready">#</a>
</h4>
Pods on this node that are not reporting Ready.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.pods.not_ready-tabs" id="infra.nodes.pods.not_ready-tab-0" checked>
  <label for="infra.nodes.pods.not_ready-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_status_ready{condition<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">true</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> kube_pod_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">unless</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>kube_pod_status_phase{phase<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">Succeeded</span>&#34;} <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.pods.restarts">infra.nodes.pods.restarts
  <a class="anchor" href="#infra.nodes.pods.restarts">#</a>
</h4>
Container restarts for pods on this node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.pods.restarts-tabs" id="infra.nodes.pods.restarts-tab-0" checked>
  <label for="infra.nodes.pods.restarts-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_status_restarts_total
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">and</span> <span style="color:#66d9ef">on</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> kube_pod_info{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.pods.budgets">infra.nodes.pods.budgets
  <a class="anchor" href="#infra.nodes.pods.budgets">#</a>
</h4>
What each pod on this node reserved and what it is capped at — CPU and
memory, requests beside limits.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.pods.budgets-tabs" id="infra.nodes.pods.budgets-tab-0" checked>
  <label for="infra.nodes.pods.budgets-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_limits{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_requests{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_container_resource_limits{node<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">$node</span>&#34;, resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">memory</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.journal.rate.by_unit">infra.nodes.journal.rate.by_unit
  <a class="anchor" href="#infra.nodes.journal.rate.by_unit">#</a>
</h4>
Journal lines per second from this node, split by systemd unit.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.journal.rate.by_unit-tabs" id="infra.nodes.journal.rate.by_unit-tab-0" checked>
  <label for="infra.nodes.journal.rate.by_unit-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.journal.warnings">infra.nodes.journal.warnings
  <a class="anchor" href="#infra.nodes.journal.warnings">#</a>
</h4>
Warning-and-worse journal lines from this node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.journal.warnings-tabs" id="infra.nodes.journal.warnings-tab-0" checked>
  <label for="infra.nodes.journal.warnings-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.journal.stream">infra.nodes.journal.stream
  <a class="anchor" href="#infra.nodes.journal.stream">#</a>
</h4>
The systemd journal from this node, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.journal.stream-tabs" id="infra.nodes.journal.stream-tab-0" checked>
  <label for="infra.nodes.journal.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.events.rate.by_reason">infra.nodes.events.rate.by_reason
  <a class="anchor" href="#infra.nodes.events.rate.by_reason">#</a>
</h4>
Kubernetes events filed against this node, by reason.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.events.rate.by_reason-tabs" id="infra.nodes.events.rate.by_reason-tab-0" checked>
  <label for="infra.nodes.events.rate.by_reason-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="infra.nodes.events.stream">infra.nodes.events.stream
  <a class="anchor" href="#infra.nodes.events.stream">#</a>
</h4>
Kubernetes events filed against this node, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="infra.nodes.events.stream-tabs" id="infra.nodes.events.stream-tab-0" checked>
  <label for="infra.nodes.events.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>

## materialize-clusters

Inventory and sizing of a Materialize deployment&rsquo;s clusters and replicas.
Adapted from the Overview dashboard&rsquo;s &ldquo;Cluster Objects / Replicas&rdquo; tab.
<h4 id="materialize.clusters.count">materialize.clusters.count
  <a class="anchor" href="#materialize.clusters.count">#</a>
</h4>
How many clusters exist, split into the Materialize-managed system
clusters (mz_catalog_server, mz_system, mz_probe, …) that every
environment has and the user clusters you created. The gap between the
two is your own footprint.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.clusters.count-tabs" id="materialize.clusters.count-tab-0" checked>
  <label for="materialize.clusters.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>compute_cluster_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>compute_cluster_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">^s.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.clusters.count-tabs" id="materialize.clusters.count-tab-1">
  <label for="materialize.clusters.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>count_not_null<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">compute_cluster_id</span>}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>count_not_null<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:s</span><span style="color:#f92672">*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">compute_cluster_id</span>}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.clusters.replicas.count">materialize.clusters.replicas.count
  <a class="anchor" href="#materialize.clusters.replicas.count">#</a>
</h4>
How many replicas back the selected clusters, and how many of those are
redundancy beyond the first. Every cluster needs one replica to run;
anything above that is capacity or availability headroom you&rsquo;ve opted
into, so a non-zero &ldquo;additional&rdquo; count is the quick check that HA is
actually configured where you expect it.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.clusters.replicas.count-tabs" id="materialize.clusters.replicas.count-tab-0" checked>
  <label for="materialize.clusters.replicas.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>compute_cluster_id, compute_replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, compute_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>compute_cluster_id, compute_replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, compute_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, compute_replica_name<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">r1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.clusters.replicas.count-tabs" id="materialize.clusters.replicas.count-tab-1">
  <label for="materialize.clusters.replicas.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>count_not_null<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, compute_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">compute_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">compute_replica_id</span>}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  count_not_null<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, compute_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>compute_replica_name<span style="color:#960050;background-color:#1e0010">:</span>r1<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">compute_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">compute_replica_id</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.clusters.replicas.sizes">materialize.clusters.replicas.sizes
  <a class="anchor" href="#materialize.clusters.replicas.sizes">#</a>
</h4>
The replica fleet grouped by configured size. Most deployments settle
on a handful of sizes; a long tail of one-off sizes usually means an
experiment or a half-finished migration. The total agrees with the
replica count.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.clusters.replicas.sizes-tabs" id="materialize.clusters.replicas.sizes-tab-0" checked>
  <label for="materialize.clusters.replicas.sizes-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>size<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, compute_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.clusters.replicas.sizes-tabs" id="materialize.clusters.replicas.sizes-tab-1">
  <label for="materialize.clusters.replicas.sizes-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, compute_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">size</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.clusters.info">materialize.clusters.info
  <a class="anchor" href="#materialize.clusters.info">#</a>
</h4>
A reference row per (cluster, replica): ids, names, size, version, and
scheduling metadata. The &ldquo;what does my fleet actually look like&rdquo; lookup
— most useful for grabbing a cluster or replica id to scope the rest of
a dashboard to.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.clusters.info-tabs" id="materialize.clusters.info-tab-0" checked>
  <label for="materialize.clusters.info-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, compute_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, compute_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.clusters.info-tabs" id="materialize.clusters.info-tab-1">
  <label for="materialize.clusters.info-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, compute_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, compute_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">compute_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">compute_cluster_name</span>,<span style="color:#960050;background-color:#1e0010">compute_replica_id</span>,<span style="color:#960050;background-color:#1e0010">compute_replica_name</span>,<span style="color:#960050;background-color:#1e0010">size</span>,<span style="color:#960050;background-color:#1e0010">mz_version</span>}
</span></span></code></pre></div>
  </div>
</div>

## materialize-compute

Queries for the compute side of a Materialize deployment — the indexes,
materialized views, and subscribes that run as dataflows on cluster replicas,
plus their freshness, hydration, and resource footprint.
<h4 id="materialize.compute.materialized_views.count">materialize.compute.materialized_views.count
  <a class="anchor" href="#materialize.compute.materialized_views.count">#</a>
</h4>
Materialized views Materialize is actively maintaining. Each one is a
query whose result is kept continuously up to date, so this tracks
roughly how much standing compute the environment carries.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.materialized_views.count-tabs" id="materialize.compute.materialized_views.count-tab-0" checked>
  <label for="materialize.compute.materialized_views.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>mzd_views_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.materialized_views.count-tabs" id="materialize.compute.materialized_views.count-tab-1">
  <label for="materialize.compute.materialized_views.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}mzd_views_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.indexes.count">materialize.compute.indexes.count
  <a class="anchor" href="#materialize.compute.indexes.count">#</a>
</h4>
Indexes in the catalog. An index is an in-memory arrangement that makes
reads against its relation effectively instant, in exchange for memory —
so growth here is a leading indicator of cluster memory growth.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.indexes.count-tabs" id="materialize.compute.indexes.count-tab-0" checked>
  <label for="materialize.compute.indexes.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>indexes_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.indexes.count-tabs" id="materialize.compute.indexes.count-tab-1">
  <label for="materialize.compute.indexes.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}indexes_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.views.count">materialize.compute.views.count
  <a class="anchor" href="#materialize.compute.views.count">#</a>
</h4>
Non-materialized views — query templates evaluated on demand. They cost
nothing until something reads them, so this is a catalog-shape signal
rather than a load one.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.views.count-tabs" id="materialize.compute.views.count-tab-0" checked>
  <label for="materialize.compute.views.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>views_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.views.count-tabs" id="materialize.compute.views.count-tab-1">
  <label for="materialize.compute.views.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}views_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.subscribes.active">materialize.compute.subscribes.active
  <a class="anchor" href="#materialize.compute.subscribes.active">#</a>
</h4>
Live SUBSCRIBE sessions — long-running queries that stream updates to a
client as data changes. A handful of <code>system</code> subscribes are
Materialize&rsquo;s own internal probes; a persistently climbing <code>user</code> count
is a classic leaked-connection signal.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.subscribes.active-tabs" id="materialize.compute.subscribes.active-tab-0" checked>
  <label for="materialize.compute.subscribes.active-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>session_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_active_subscribes{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.subscribes.active-tabs" id="materialize.compute.subscribes.active-tab-1">
  <label for="materialize.compute.subscribes.active-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_active_subscribes{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">session_type</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.indexes.by_type">materialize.compute.indexes.by_type
  <a class="anchor" href="#materialize.compute.indexes.by_type">#</a>
</h4>
Indexes split by the kind of relation they sit on. Workloads normally
lean heavily on indexes over views (the standard &ldquo;keep a query&rsquo;s result
hot&rdquo; pattern); a large share of indexes on base tables is unusual and
usually worth a second look.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.indexes.by_type-tabs" id="materialize.compute.indexes.by_type-tab-0" checked>
  <label for="materialize.compute.indexes.by_type-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>relation_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>indexes_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.indexes.by_type-tabs" id="materialize.compute.indexes.by_type-tab-1">
  <label for="materialize.compute.indexes.by_type-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}indexes_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">relation_type</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.hydration.currently_hydrating">materialize.compute.hydration.currently_hydrating
  <a class="anchor" href="#materialize.compute.hydration.currently_hydrating">#</a>
</h4>
Collections still rebuilding their in-memory state — a live
hydration-queue proxy. After a restart, replica creation, or some DDL, a
dataflow has to rebuild from persisted storage before it can serve, and
until it does it produces no results.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.hydration.currently_hydrating-tabs" id="materialize.compute.hydration.currently_hydrating-tab-0" checked>
  <label for="materialize.compute.hydration.currently_hydrating-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_dataflow_wallclock_lag_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, instance_id<span style="color:#f92672">!=</span>&#34;&#34;, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">1</span>e15
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.hydration.currently_hydrating-tabs" id="materialize.compute.hydration.currently_hydrating-tab-1">
  <label for="materialize.compute.hydration.currently_hydrating-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  count_not_null<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_dataflow_wallclock_lag_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>,<span style="color:#960050;background-color:#1e0010">collection_id</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.hydration.queue_size">materialize.compute.hydration.queue_size
  <a class="anchor" href="#materialize.compute.hydration.queue_size">#</a>
</h4>
Collections waiting in each replica&rsquo;s hydration queue. environmentd
schedules hydration in batches; a backlog means work is arriving faster
than the replica can rebuild it.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.hydration.queue_size-tabs" id="materialize.compute.hydration.queue_size-tab-0" checked>
  <label for="materialize.compute.hydration.queue_size-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id, replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_compute_controller_hydration_queue_size{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.hydration.queue_size-tabs" id="materialize.compute.hydration.queue_size-tab-1">
  <label for="materialize.compute.hydration.queue_size-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_compute_controller_hydration_queue_size{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>,<span style="color:#960050;background-color:#1e0010">replica_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.hydration.slowest_collections">materialize.compute.hydration.slowest_collections
  <a class="anchor" href="#materialize.compute.hydration.slowest_collections">#</a>
</h4>
The 15 collections that took longest to finish hydrating. Hydration time
scales with the size of the state being rebuilt, so large materialized
views and indexes naturally top the list.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.hydration.slowest_collections-tabs" id="materialize.compute.hydration.slowest_collections-tab-0" checked>
  <label for="materialize.compute.hydration.slowest_collections-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">topk</span><span style="color:#f92672">(</span><span style="color:#ae81ff">15</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_hydration_time_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, hydrated<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.hydration.slowest_collections-tabs" id="materialize.compute.hydration.slowest_collections-tab-1">
  <label for="materialize.compute.hydration.slowest_collections-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>top<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_hydration_time_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, hydrated<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>,<span style="color:#960050;background-color:#1e0010">collection_id</span>},
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">15</span>, &#39;<span style="color:#e6db74">max</span>&#39;, &#39;<span style="color:#e6db74">desc</span>&#39;
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.freshness.lag_by_cluster">materialize.compute.freshness.lag_by_cluster
  <a class="anchor" href="#materialize.compute.freshness.lag_by_cluster">#</a>
</h4>
How far behind real time each cluster&rsquo;s most-lagged collection is — the
worst-case freshness across every index, materialized view, and source
on the cluster.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.freshness.lag_by_cluster-tabs" id="materialize.compute.freshness.lag_by_cluster-tab-0" checked>
  <label for="materialize.compute.freshness.lag_by_cluster-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_dataflow_wallclock_lag_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, instance_id<span style="color:#f92672">!=</span>&#34;&#34;, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.freshness.lag_by_cluster-tabs" id="materialize.compute.freshness.lag_by_cluster-tab-1">
  <label for="materialize.compute.freshness.lag_by_cluster-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_dataflow_wallclock_lag_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.freshness.lag_total_by_cluster">materialize.compute.freshness.lag_total_by_cluster
  <a class="anchor" href="#materialize.compute.freshness.lag_total_by_cluster">#</a>
</h4>
The lag of every collection on each cluster, added together — one number
for how far behind the cluster is in total.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.freshness.lag_total_by_cluster-tabs" id="materialize.compute.freshness.lag_total_by_cluster-tab-0" checked>
  <label for="materialize.compute.freshness.lag_total_by_cluster-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_dataflow_wallclock_lag_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, instance_id<span style="color:#f92672">!=</span>&#34;&#34;, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.freshness.lag_total_by_cluster-tabs" id="materialize.compute.freshness.lag_total_by_cluster-tab-1">
  <label for="materialize.compute.freshness.lag_total_by_cluster-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_dataflow_wallclock_lag_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.freshness.top_collections">materialize.compute.freshness.top_collections
  <a class="anchor" href="#materialize.compute.freshness.top_collections">#</a>
</h4>
The 15 collections whose results are furthest behind real time —
the per-collection breakdown behind the per-cluster freshness lag,
labeled by object name.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.freshness.top_collections-tabs" id="materialize.compute.freshness.top_collections-tab-0" checked>
  <label for="materialize.compute.freshness.top_collections-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">topk</span><span style="color:#f92672">(</span><span style="color:#ae81ff">15</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_dataflow_wallclock_lag_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, instance_id<span style="color:#f92672">!=</span>&#34;&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.freshness.top_collections-tabs" id="materialize.compute.freshness.top_collections-tab-1">
  <label for="materialize.compute.freshness.top_collections-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>top<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_dataflow_wallclock_lag_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>,<span style="color:#960050;background-color:#1e0010">collection_id</span>},
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">15</span>, &#39;<span style="color:#e6db74">max</span>&#39;, &#39;<span style="color:#e6db74">desc</span>&#39;
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.dataflows.count">materialize.compute.dataflows.count
  <a class="anchor" href="#materialize.compute.dataflows.count">#</a>
</h4>
Active dataflows on each replica. Every index, materialized view, and
live SUBSCRIBE runs as one or more dataflows, so this count rises with
DDL and subscribe activity.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.dataflows.count-tabs" id="materialize.compute.dataflows.count-tab-0" checked>
  <label for="materialize.compute.dataflows.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_compute_replica_history_dataflow_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.dataflows.count-tabs" id="materialize.compute.dataflows.count-tab-1">
  <label for="materialize.compute.dataflows.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_compute_replica_history_dataflow_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_replica_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.dataflows.count_by_worker">materialize.compute.dataflows.count_by_worker
  <a class="anchor" href="#materialize.compute.dataflows.count_by_worker">#</a>
</h4>
The dataflow count broken out per worker. Workers in a replica run in
lockstep and should see exactly the same dataflows, so their series
should overlap perfectly.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.dataflows.count_by_worker-tabs" id="materialize.compute.dataflows.count_by_worker-tab-0" checked>
  <label for="materialize.compute.dataflows.count_by_worker-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, worker_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  mz_compute_replica_history_dataflow_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.dataflows.count_by_worker-tabs" id="materialize.compute.dataflows.count_by_worker-tab-1">
  <label for="materialize.compute.dataflows.count_by_worker-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_compute_replica_history_dataflow_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_replica_id</span>,<span style="color:#960050;background-color:#1e0010">worker_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.dataflows.elapsed_rate">materialize.compute.dataflows.elapsed_rate
  <a class="anchor" href="#materialize.compute.dataflows.elapsed_rate">#</a>
</h4>
CPU-cores busy inside dataflows, per cluster — the whole of dataflow
work: arrangement maintenance, query evaluation, and hydration. Capped
by cluster size (a 400cc cluster can&rsquo;t exceed 400 cores).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.dataflows.elapsed_rate-tabs" id="materialize.compute.dataflows.elapsed_rate-tab-0" checked>
  <label for="materialize.compute.dataflows.elapsed_rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>dataflow_elapsed_seconds_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.dataflows.elapsed_rate-tabs" id="materialize.compute.dataflows.elapsed_rate-tab-1">
  <label for="materialize.compute.dataflows.elapsed_rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}dataflow_elapsed_seconds_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.arrangements.maintenance_rate">materialize.compute.arrangements.maintenance_rate
  <a class="anchor" href="#materialize.compute.arrangements.maintenance_rate">#</a>
</h4>
CPU-cores spent maintaining arrangements — the in-memory indexed
snapshots behind every index and materialized view — summed across a
replica&rsquo;s workers, so an N-worker replica can reach N.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.arrangements.maintenance_rate-tabs" id="materialize.compute.arrangements.maintenance_rate-tab-0" checked>
  <label for="materialize.compute.arrangements.maintenance_rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_arrangement_maintenance_seconds_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.arrangements.maintenance_rate-tabs" id="materialize.compute.arrangements.maintenance_rate-tab-1">
  <label for="materialize.compute.arrangements.maintenance_rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_arrangement_maintenance_seconds_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_replica_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.arrangements.maintenance_rate_by_worker">materialize.compute.arrangements.maintenance_rate_by_worker
  <a class="anchor" href="#materialize.compute.arrangements.maintenance_rate_by_worker">#</a>
</h4>
The same maintenance CPU, split per worker — each worker tops out at 1.0.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.arrangements.maintenance_rate_by_worker-tabs" id="materialize.compute.arrangements.maintenance_rate_by_worker-tab-0" checked>
  <label for="materialize.compute.arrangements.maintenance_rate_by_worker-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>cluster_environmentd_materialize_cloud_cluster_id, cluster_environmentd_materialize_cloud_replica_id, worker_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_arrangement_maintenance_seconds_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.arrangements.maintenance_rate_by_worker-tabs" id="materialize.compute.arrangements.maintenance_rate_by_worker-tab-1">
  <label for="materialize.compute.arrangements.maintenance_rate_by_worker-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_arrangement_maintenance_seconds_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_cluster_id</span>,<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_replica_id</span>,<span style="color:#960050;background-color:#1e0010">worker_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.arrangements.records.system">materialize.compute.arrangements.records.system
  <a class="anchor" href="#materialize.compute.arrangements.records.system">#</a>
</h4>
Row counts of arrangements for Materialize&rsquo;s internal system collections
(collection id starts with <code>s</code>). These back the catalog and internal
probes, not user data, so they shouldn&rsquo;t grow with your workload —
unexpected growth here can point at a Materialize bug.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.system-tabs" id="materialize.compute.arrangements.records.system-tab-0" checked>
  <label for="materialize.compute.arrangements.records.system-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>arrangement_record_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, collection_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">s.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.system-tabs" id="materialize.compute.arrangements.records.system-tab-1">
  <label for="materialize.compute.arrangements.records.system-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}arrangement_record_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, collection_id<span style="color:#960050;background-color:#1e0010">:s</span><span style="color:#f92672">*</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">collection_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.arrangements.records.user">materialize.compute.arrangements.records.user
  <a class="anchor" href="#materialize.compute.arrangements.records.user">#</a>
</h4>
Row counts of arrangements for your compute objects (collection id starts
with <code>u</code>) — the row count of every user index and materialized view, and
the primary driver of cluster memory. Growth on a collection tracks the
size of its underlying data.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.user-tabs" id="materialize.compute.arrangements.records.user-tab-0" checked>
  <label for="materialize.compute.arrangements.records.user-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>arrangement_record_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, collection_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">u.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.user-tabs" id="materialize.compute.arrangements.records.user-tab-1">
  <label for="materialize.compute.arrangements.records.user-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}arrangement_record_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, collection_id<span style="color:#960050;background-color:#1e0010">:u</span><span style="color:#f92672">*</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">collection_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.compute.arrangements.records.transient">materialize.compute.arrangements.records.transient
  <a class="anchor" href="#materialize.compute.arrangements.records.transient">#</a>
</h4>
Row counts of transient (collection id <code>t*</code>) and uncategorized (<code>none</code>)
arrangements — short-lived intermediates from query optimization and
dataflow execution. Normally small and ephemeral.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.transient-tabs" id="materialize.compute.arrangements.records.transient-tab-0" checked>
  <label for="materialize.compute.arrangements.records.transient-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>arrangement_record_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;, collection_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">t.*|none</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.compute.arrangements.records.transient-tabs" id="materialize.compute.arrangements.records.transient-tab-1">
  <label for="materialize.compute.arrangements.records.transient-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}arrangement_record_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}, <span style="color:#f92672">(</span>collection_id<span style="color:#960050;background-color:#1e0010">:t</span><span style="color:#f92672">*</span> OR collection_id<span style="color:#960050;background-color:#1e0010">:</span>none<span style="color:#f92672">)</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">collection_id</span>}
</span></span></code></pre></div>
  </div>
</div>

## materialize-connections

Sessions, query activity, and SQL control-plane (adapter) traffic for a
Materialize deployment. Adapted from the Overview dashboard&rsquo;s
&ldquo;Connections / Activity&rdquo; tab.
<h4 id="materialize.connections.sessions.active">materialize.connections.sessions.active
  <a class="anchor" href="#materialize.connections.sessions.active">#</a>
</h4>
Open SQL sessions, split into <code>system</code> (Materialize&rsquo;s internal probing —
a few are always present) and <code>user</code> (client connections).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.sessions.active-tabs" id="materialize.connections.sessions.active-tab-0" checked>
  <label for="materialize.connections.sessions.active-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>session_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_active_sessions{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.sessions.active-tabs" id="materialize.connections.sessions.active-tab-1">
  <label for="materialize.connections.sessions.active-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_active_sessions{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">session_type</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.queries.rate">materialize.connections.queries.rate
  <a class="anchor" href="#materialize.connections.queries.rate">#</a>
</h4>
Queries per second by session type — <code>user</code> tracks your client traffic,
<code>system</code> is the steady single-digit baseline of internal health checks.
Bursty is normal.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.queries.rate-tabs" id="materialize.connections.queries.rate-tab-0" checked>
  <label for="materialize.connections.queries.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>session_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_query_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.queries.rate-tabs" id="materialize.connections.queries.rate-tab-1">
  <label for="materialize.connections.queries.rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_query_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">session_type</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.adapter.command_rate">materialize.connections.adapter.command_rate
  <a class="anchor" href="#materialize.connections.adapter.command_rate">#</a>
</h4>
Commands per second through the adapter — the SQL protocol layer
(parse, execute, prepare, fetch). Normally runs higher than the query
rate, since one query is several commands.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.adapter.command_rate-tabs" id="materialize.connections.adapter.command_rate-tab-0" checked>
  <label for="materialize.connections.adapter.command_rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_adapter_commands{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.adapter.command_rate-tabs" id="materialize.connections.adapter.command_rate-tab-1">
  <label for="materialize.connections.adapter.command_rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_adapter_commands{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.queries.distribution">materialize.connections.queries.distribution
  <a class="anchor" href="#materialize.connections.queries.distribution">#</a>
</h4>
The mix of query kinds over the selected window — a workload-shape
signal, not a rate. Heavy <code>set_variable</code>/<code>fetch</code> traffic is normal
(that&rsquo;s how Postgres clients manage session state); heavy
<code>insert</code>/<code>update</code>/<code>delete</code> on something you think of as read-mostly is
worth a look.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.queries.distribution-tabs" id="materialize.connections.queries.distribution-tab-0" checked>
  <label for="materialize.connections.queries.distribution-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>statement_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>mz_query_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='range' title='range'>[1h]</span><span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.queries.distribution-tabs" id="materialize.connections.queries.distribution-tab-1">
  <label for="materialize.connections.queries.distribution-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_query_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">statement_type</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_count<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.queries.rate_by_statement">materialize.connections.queries.rate_by_statement
  <a class="anchor" href="#materialize.connections.queries.rate_by_statement">#</a>
</h4>
Query rate broken down by statement type and session type, fully
time-resolved — the moving picture behind the distribution donut. A
spike in <code>select</code>/<code>user</code> is the thing to line up against peek latency
to confirm the system kept pace.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.queries.rate_by_statement-tabs" id="materialize.connections.queries.rate_by_statement-tab-0" checked>
  <label for="materialize.connections.queries.rate_by_statement-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>statement_type, session_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_query_total{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.queries.rate_by_statement-tabs" id="materialize.connections.queries.rate_by_statement-tab-1">
  <label for="materialize.connections.queries.rate_by_statement-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_query_total{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">statement_type</span>,<span style="color:#960050;background-color:#1e0010">session_type</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.peek_latency.p50">materialize.connections.peek_latency.p50
  <a class="anchor" href="#materialize.connections.peek_latency.p50">#</a>
</h4>
Median read-query latency — the typical time to look up the current
state of an arrangement, which is the operation behind every <code>SELECT</code>
against an index. Your &ldquo;what does a normal query feel like&rdquo; number.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p50-tabs" id="materialize.connections.peek_latency.p50-tab-0" checked>
  <label for="materialize.connections.peek_latency.p50-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.50</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_compute_peek_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p50-tabs" id="materialize.connections.peek_latency.p50-tab-1">
  <label for="materialize.connections.peek_latency.p50-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p50<span style="color:#960050;background-color:#1e0010">:</span>mz_compute_peek_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.peek_latency.p90">materialize.connections.peek_latency.p90
  <a class="anchor" href="#materialize.connections.peek_latency.p90">#</a>
</h4>
90th-percentile read-query latency — how slow the slowest 10% of
queries feel. Catches the contention bursts and cold paths that the
median hides.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p90-tabs" id="materialize.connections.peek_latency.p90-tab-0" checked>
  <label for="materialize.connections.peek_latency.p90-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.90</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_compute_peek_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p90-tabs" id="materialize.connections.peek_latency.p90-tab-1">
  <label for="materialize.connections.peek_latency.p90-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p90<span style="color:#960050;background-color:#1e0010">:</span>mz_compute_peek_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.peek_latency.p99">materialize.connections.peek_latency.p99
  <a class="anchor" href="#materialize.connections.peek_latency.p99">#</a>
</h4>
Tail read-query latency — the slowest 1% of queries, the ones users
complain about.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p99-tabs" id="materialize.connections.peek_latency.p99-tab-0" checked>
  <label for="materialize.connections.peek_latency.p99-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_compute_peek_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.peek_latency.p99-tabs" id="materialize.connections.peek_latency.p99-tab-1">
  <label for="materialize.connections.peek_latency.p99-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p99<span style="color:#960050;background-color:#1e0010">:</span>mz_compute_peek_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, instance_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.connections.adapter.commands_by_application">materialize.connections.adapter.commands_by_application
  <a class="anchor" href="#materialize.connections.adapter.commands_by_application">#</a>
</h4>
SQL control-plane command totals per client <code>application_name</code> over the
window, so you can see which clients drive the adapter and which are
failing. Most clients set <code>application_name</code> in their connection string;
those that don&rsquo;t bucket as <code>unrecognized</code>/<code>unspecified</code> (normal).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.connections.adapter.commands_by_application-tabs" id="materialize.connections.adapter.commands_by_application-tab-0" checked>
  <label for="materialize.connections.adapter.commands_by_application-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>application_name, status<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>mz_adapter_commands{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='range' title='range'>[1h]</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.connections.adapter.commands_by_application-tabs" id="materialize.connections.adapter.commands_by_application-tab-1">
  <label for="materialize.connections.adapter.commands_by_application-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_adapter_commands{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">application_name</span>,<span style="color:#960050;background-color:#1e0010">status</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_count<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>

## materialize-events

<p>Kubernetes events from the namespaces a Materialize deployment occupies: the
operator&rsquo;s own namespace and the environments&rsquo; namespace.</p>
<p>Events are logs, not metrics. They arrive through
<code>loki.source.kubernetes_events</code> in the monitoring gateway, which reads them
from the Kubernetes API and forwards them to Loki, so every query here is
LogQL against the logs datasource rather than PromQL. The gateway&rsquo;s processor
lifts <code>reason</code>, <code>name</code>, <code>kind</code>, <code>count</code>, <code>node</code> and <code>reportingcontroller</code> out
of each event into structured metadata, which is what these queries filter and
group on; the event <code>type</code> becomes the <code>level</code> stream label, <code>Normal</code> as
<code>INFO</code> and <code>Warning</code> as <code>WARN</code>.</p>
<p><strong>Two scopes live here.</strong> The <code>deployment</code> and <code>operator</code> queries below are
rollout-scoped: they answer &ldquo;is this upgrade going through&rdquo;, and the
<code>env-upgrade</code> dashboard is their consumer. The <code>cluster</code> queries at the end are
the general-purpose view, scoped by the same Loki-discovered namespace picker a
logs dashboard uses, and are deliberately separate definitions rather than the
same ones widened — the rollout queries carry filters (generation, reporting
controller) that a general event browser has no business inheriting.</p>
<p>Kubernetes keeps events for about an hour. Loki keeps them for as long as the
deployment&rsquo;s retention says, which is what makes a rollout that finished
yesterday still explainable.</p>
<p><strong>An event&rsquo;s namespace is the involved object&rsquo;s, not the reporter&rsquo;s.</strong> The
operator runs in its own namespace and reconciles resources in the
environments&rsquo; namespace, and every event it publishes is filed against the
resource — so the operator&rsquo;s own events are found in the <em>environment</em>
namespace, where nothing else about them suggests they would be. That is why
the queries below scope to both namespaces and pick the operator out by
<code>reportingcontroller</code>, which is the reporter&rsquo;s identity and the only field
that actually says an event came from orchestratord.</p>
<p><strong>Only the deployment-wide feeds filter by generation.</strong> The operator&rsquo;s own
events are filed against the <code>Materialize</code>, <code>Balancer</code> and <code>Console</code> resources,
which carry no generation at all, so <code>%%{mzGenerationEventFilter}</code> could only
ever be a no-op on them. The filter itself keeps generation-less objects on
purpose — on a representative deployment only 6 of 70 event names carry a
generation, and dropping the other 64 would take the whole rollout narrative
with them.</p>

<h4 id="materialize.events.deployment.stream">materialize.events.deployment.stream
  <a class="anchor" href="#materialize.events.deployment.stream">#</a>
</h4>
Every Kubernetes event from the operator and environment namespaces,
newest first — the unfiltered record of what the cluster did.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.deployment.stream-tabs" id="materialize.events.deployment.stream-tab-0" checked>
  <label for="materialize.events.deployment.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.deployment.warnings">materialize.events.deployment.warnings
  <a class="anchor" href="#materialize.events.deployment.warnings">#</a>
</h4>
Kubernetes events the reporting component flagged as warnings — a pod
that will not schedule, an image that will not pull, a container failing
its probes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.deployment.warnings-tabs" id="materialize.events.deployment.warnings-tab-0" checked>
  <label for="materialize.events.deployment.warnings-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.deployment.rate.by_reason">materialize.events.deployment.rate.by_reason
  <a class="anchor" href="#materialize.events.deployment.rate.by_reason">#</a>
</h4>
How often each kind of event is being reported, by reason. The shape of
a rollout: <code>Pulled</code>, <code>Created</code> and <code>Started</code> rise together as pods are
replaced, and fall back to nothing when it finishes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.deployment.rate.by_reason-tabs" id="materialize.events.deployment.rate.by_reason-tab-0" checked>
  <label for="materialize.events.deployment.rate.by_reason-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.deployment.warning.rate">materialize.events.deployment.warning.rate
  <a class="anchor" href="#materialize.events.deployment.warning.rate">#</a>
</h4>
Warning events per interval across both namespaces, as one series — the
at-a-glance answer to whether anything is complaining right now.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.deployment.warning.rate-tabs" id="materialize.events.deployment.warning.rate-tab-0" checked>
  <label for="materialize.events.deployment.warning.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.operator.lifecycle">materialize.events.operator.lifecycle
  <a class="anchor" href="#materialize.events.operator.lifecycle">#</a>
</h4>
Every phase the Materialize resource moved through, as the operator
reported it: <code>Applying</code>, <code>ReadyToPromote</code>, <code>WaitingForApproval</code>,
<code>Promoting</code>, <code>Applied</code>, and the two that end a rollout badly,
<code>RolloutTimeout</code> and <code>FailedDeploy</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.operator.lifecycle-tabs" id="materialize.events.operator.lifecycle-tab-0" checked>
  <label for="materialize.events.operator.lifecycle-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.operator.lifecycle.rate">materialize.events.operator.lifecycle.rate
  <a class="anchor" href="#materialize.events.operator.lifecycle.rate">#</a>
</h4>
Lifecycle transitions over time, by phase — where a rollout got to, and
when.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.operator.lifecycle.rate-tabs" id="materialize.events.operator.lifecycle.rate-tab-0" checked>
  <label for="materialize.events.operator.lifecycle.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.operator.reconciliation.failures">materialize.events.operator.reconciliation.failures
  <a class="anchor" href="#materialize.events.operator.reconciliation.failures">#</a>
</h4>
Why the operator could not reconcile a resource. The event carries the
error&rsquo;s whole cause chain, which is usually the actionable half — an
admission webhook that is down, a secret that does not exist yet, a
license key that will not parse.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.operator.reconciliation.failures-tabs" id="materialize.events.operator.reconciliation.failures-tab-0" checked>
  <label for="materialize.events.operator.reconciliation.failures-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.operator.reconciliation.failures.rate">materialize.events.operator.reconciliation.failures.rate
  <a class="anchor" href="#materialize.events.operator.reconciliation.failures.rate">#</a>
</h4>
Reconciliation failures over time, by the kind of resource that failed.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.operator.reconciliation.failures.rate-tabs" id="materialize.events.operator.reconciliation.failures.rate-tab-0" checked>
  <label for="materialize.events.operator.reconciliation.failures.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.cluster.stream">materialize.events.cluster.stream
  <a class="anchor" href="#materialize.events.cluster.stream">#</a>
</h4>
Every Kubernetes event in the selected namespaces, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.cluster.stream-tabs" id="materialize.events.cluster.stream-tab-0" checked>
  <label for="materialize.events.cluster.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.cluster.warnings">materialize.events.cluster.warnings
  <a class="anchor" href="#materialize.events.cluster.warnings">#</a>
</h4>
Kubernetes events the reporting component flagged as warnings, across the
selected namespaces.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.cluster.warnings-tabs" id="materialize.events.cluster.warnings-tab-0" checked>
  <label for="materialize.events.cluster.warnings-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.cluster.rate.by_reason">materialize.events.cluster.rate.by_reason
  <a class="anchor" href="#materialize.events.cluster.rate.by_reason">#</a>
</h4>
How often each kind of event is being reported, by reason — the shape of
what the cluster is doing.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.cluster.rate.by_reason-tabs" id="materialize.events.cluster.rate.by_reason-tab-0" checked>
  <label for="materialize.events.cluster.rate.by_reason-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.events.cluster.rate.by_namespace">materialize.events.cluster.rate.by_namespace
  <a class="anchor" href="#materialize.events.cluster.rate.by_namespace">#</a>
</h4>
Event rate by namespace — where in the cluster things are happening.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.events.cluster.rate.by_namespace-tabs" id="materialize.events.cluster.rate.by_namespace-tab-0" checked>
  <label for="materialize.events.cluster.rate.by_namespace-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>

## materialize-generations

<p>What each deployment generation of an environment is doing, during and after a
blue/green rollout.</p>
<p>A rollout stands a <em>new</em> generation of <code>environmentd</code> and its replicas up beside
the old one, lets it rehydrate from persisted storage, and promotes it only once
it has caught up. Both generations are live and scraped at the same time, so
every ordinary panel sums them together — which is exactly the wrong thing while
the question is whether one of them is ready yet.</p>
<p><strong>The generation is not a label.</strong> orchestratord records it as a Kubernetes
annotation, which neither kube-state-metrics nor cAdvisor surfaces. Where it
does reach a query is the object <em>name</em>, in two shapes:
<code>…-environmentd-&lt;generation&gt;-&lt;ordinal&gt;</code> and, for a replica,
<code>…-gen-&lt;generation&gt;-&lt;ordinal&gt;</code>. <code>%%{mzGenerationFilter}</code> selects on those, and
<code>%%{mzGenerationPattern}</code> is the same shape as a capture, for the
<code>label_replace</code> that lifts the number into a <code>generation</code> label panels can group
by. Both live in the render context, so they cannot drift apart.</p>

<h4 id="materialize.generations.active">materialize.generations.active
  <a class="anchor" href="#materialize.generations.active">#</a>
</h4>
How many deployment generations are currently running — one between
rollouts, two while one is in flight.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.active-tabs" id="materialize.generations.active-tab-0" checked>
  <label for="materialize.generations.active-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_compute_commands_total{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>}
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.version">materialize.generations.version
  <a class="anchor" href="#materialize.generations.version">#</a>
</h4>
The Materialize version each generation is running — what the rollout is
actually changing.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.version-tabs" id="materialize.generations.version-tab-0" checked>
  <label for="materialize.generations.version-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, mz_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max_over_time</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>}
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='range' title='range'>[1h]</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.pods">materialize.generations.pods
  <a class="anchor" href="#materialize.generations.pods">#</a>
</h4>
Pods belonging to each generation — its environmentd and the cluster
replicas standing behind it.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.pods-tabs" id="materialize.generations.pods-tab-0" checked>
  <label for="materialize.generations.pods-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>pod, generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      container_memory_working_set_bytes{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>}
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.hydrating">materialize.generations.hydrating
  <a class="anchor" href="#materialize.generations.hydrating">#</a>
</h4>
Collections still rebuilding their in-memory state, split by generation —
the panel that answers whether a new generation is ready to promote.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.hydrating-tabs" id="materialize.generations.hydrating-tab-0" checked>
  <label for="materialize.generations.hydrating-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_dataflow_wallclock_lag_seconds{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>,
</span></span><span style="display:flex;"><span>        <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>},
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;,
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#f92672">!=</span>&#34;&#34;,
</span></span><span style="display:flex;"><span>        <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">1</span>e15
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.collections">materialize.generations.collections
  <a class="anchor" href="#materialize.generations.collections">#</a>
</h4>
Collections each generation is tracking — the denominator for hydration,
and the shape of a new generation building out its dataflows.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.collections-tabs" id="materialize.generations.collections-tab-0" checked>
  <label for="materialize.generations.collections-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_dataflow_wallclock_lag_seconds{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>,
</span></span><span style="display:flex;"><span>        <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>},
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;,
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#f92672">!=</span>&#34;&#34;,
</span></span><span style="display:flex;"><span>        <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.lag.max">materialize.generations.lag.max
  <a class="anchor" href="#materialize.generations.lag.max">#</a>
</h4>
The worst lag in each generation — how far behind real time its
most-lagged collection is.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.lag.max-tabs" id="materialize.generations.lag.max-tab-0" checked>
  <label for="materialize.generations.lag.max-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    mz_dataflow_wallclock_lag_seconds{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>,
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>},
</span></span><span style="display:flex;"><span>      instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;,
</span></span><span style="display:flex;"><span>      instance_id<span style="color:#f92672">!=</span>&#34;&#34;,
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.lag.total">materialize.generations.lag.total
  <a class="anchor" href="#materialize.generations.lag.total">#</a>
</h4>
Every hydrated collection&rsquo;s lag in each generation, added together — how
far behind the generation is in total.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.lag.total-tabs" id="materialize.generations.lag.total-tab-0" checked>
  <label for="materialize.generations.lag.total-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_dataflow_wallclock_lag_seconds{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>,
</span></span><span style="display:flex;"><span>        <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>},
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;,
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#f92672">!=</span>&#34;&#34;,
</span></span><span style="display:flex;"><span>        <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.lag.total_by_cluster">materialize.generations.lag.total_by_cluster
  <a class="anchor" href="#materialize.generations.lag.total_by_cluster">#</a>
</h4>
Total lag split by generation <em>and</em> cluster — which cluster in which
generation is carrying the lag.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.lag.total_by_cluster-tabs" id="materialize.generations.lag.total_by_cluster-tab-0" checked>
  <label for="materialize.generations.lag.total_by_cluster-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, instance_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation, instance_id, collection_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_dataflow_wallclock_lag_seconds{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>,
</span></span><span style="display:flex;"><span>        <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>},
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;,
</span></span><span style="display:flex;"><span>        instance_id<span style="color:#f92672">!=</span>&#34;&#34;,
</span></span><span style="display:flex;"><span>        <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>      &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.cpu">materialize.generations.cpu
  <a class="anchor" href="#materialize.generations.cpu">#</a>
</h4>
CPU used by each generation&rsquo;s pods — what a rollout costs while both
sides are up.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.cpu-tabs" id="materialize.generations.cpu-tab-0" checked>
  <label for="materialize.generations.cpu-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      container_cpu_usage_seconds_total{
</span></span><span style="display:flex;"><span>        <span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>}
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>      <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.generations.memory">materialize.generations.memory
  <a class="anchor" href="#materialize.generations.memory">#</a>
</h4>
Memory used by each generation&rsquo;s pods.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.generations.memory-tabs" id="materialize.generations.memory-tab-0" checked>
  <label for="materialize.generations.memory-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>generation<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">label_replace</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzGenerationFilter</span>}
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>,
</span></span><span style="display:flex;"><span>    &#34;<span style="color:#e6db74">generation</span>&#34;, &#34;<span style="color:#e6db74">$1</span>&#34;, &#34;<span style="color:#e6db74">pod</span>&#34;, &#34;<span style="color:#e6db74">${mzGenerationPattern}</span>&#34;
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>

## materialize-health

Common queries for checking the health of a Materialize deployment.
<h4 id="materialize.scraper.mzmon.environmentd">materialize.scraper.mzmon.environmentd
  <a class="anchor" href="#materialize.scraper.mzmon.environmentd">#</a>
</h4>
environmentd metrics are reaching the gateway. If this goes to 0 or
disappears, every environmentd-backed panel and alert for the
environment is blind — the environment may be perfectly healthy and you
simply can&rsquo;t see it, so rule this out first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.environmentd-tabs" id="materialize.scraper.mzmon.environmentd-tab-0" checked>
  <label for="materialize.scraper.mzmon.environmentd-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>up{
</span></span><span style="display:flex;"><span>  job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">monitoring/mzmon-materialize-environmentd</span>&#34;,
</span></span><span style="display:flex;"><span>  <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentNamespaceFilter' title='mzEnvironmentNamespaceFilter'>namespace=~"materialize-environment"</span>
</span></span><span style="display:flex;"><span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.environmentd-tabs" id="materialize.scraper.mzmon.environmentd-tab-1">
  <label for="materialize.scraper.mzmon.environmentd-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>up{<span style="color:#960050;background-color:#1e0010">job:monitoring/mzmon-materialize-environmentd</span>, <span style="color:#960050;background-color:#1e0010">${mzEnvironmentNamespaceFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.scraper.mzmon.clusterd">materialize.scraper.mzmon.clusterd
  <a class="anchor" href="#materialize.scraper.mzmon.clusterd">#</a>
</h4>
clusterd (compute replica) metrics are reaching the gateway. When this
drops, per-cluster compute signals — arrangements, peeks, dataflows —
go dark even though the replicas may still be serving.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.clusterd-tabs" id="materialize.scraper.mzmon.clusterd-tab-0" checked>
  <label for="materialize.scraper.mzmon.clusterd-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>up{
</span></span><span style="display:flex;"><span>  job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">monitoring/mzmon-materialize-clusterd</span>&#34;,
</span></span><span style="display:flex;"><span>  <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentNamespaceFilter' title='mzEnvironmentNamespaceFilter'>namespace=~"materialize-environment"</span>
</span></span><span style="display:flex;"><span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.clusterd-tabs" id="materialize.scraper.mzmon.clusterd-tab-1">
  <label for="materialize.scraper.mzmon.clusterd-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>up{<span style="color:#960050;background-color:#1e0010">job:monitoring/mzmon-materialize-clusterd</span>, <span style="color:#960050;background-color:#1e0010">${mzEnvironmentNamespaceFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.scraper.mzmon.orchestratord">materialize.scraper.mzmon.orchestratord
  <a class="anchor" href="#materialize.scraper.mzmon.orchestratord">#</a>
</h4>
The Materialize operator (orchestratord) is being scraped. Losing it
blinds you to cluster and replica lifecycle — creation, resize, and
rollout progress — not to the running workloads themselves.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.orchestratord-tabs" id="materialize.scraper.mzmon.orchestratord-tab-0" checked>
  <label for="materialize.scraper.mzmon.orchestratord-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>up{
</span></span><span style="display:flex;"><span>  job<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">materialize/mzmon-materialize-operator</span>&#34;,
</span></span><span style="display:flex;"><span>  <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>
</span></span><span style="display:flex;"><span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.scraper.mzmon.orchestratord-tabs" id="materialize.scraper.mzmon.orchestratord-tab-1">
  <label for="materialize.scraper.mzmon.orchestratord-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>up{<span style="color:#960050;background-color:#1e0010">job:materialize/mzmon-materialize-operator</span>, <span style="color:#960050;background-color:#1e0010">${mzOperatorNamespaceFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.health.clusters.status.percentage">materialize.health.clusters.status.percentage
  <a class="anchor" href="#materialize.health.clusters.status.percentage">#</a>
</h4>
The share of the environment&rsquo;s clusters currently reporting ready —
the at-a-glance health headline for the whole environment.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.health.clusters.status.percentage-tabs" id="materialize.health.clusters.status.percentage-tab-0" checked>
  <label for="materialize.health.clusters.status.percentage-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{
</span></span><span style="display:flex;"><span>    <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">==</span> <span style="color:#ae81ff">1</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">count</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{
</span></span><span style="display:flex;"><span>    <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.health.clusters.status.percentage-tabs" id="materialize.health.clusters.status.percentage-tab-1">
  <label for="materialize.health.clusters.status.percentage-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.health.environment.availability.percentage">materialize.health.environment.availability.percentage
  <a class="anchor" href="#materialize.health.environment.availability.percentage">#</a>
</h4>
An SLO-style snapshot: how much of the selected window the
environment&rsquo;s clusters were ready. Sustained dips are the signal that
something restarted or went down while you weren&rsquo;t watching.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.health.environment.availability.percentage-tabs" id="materialize.health.environment.availability.percentage-tab-0" checked>
  <label for="materialize.health.environment.availability.percentage-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>materialize_cloud_organization_namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg_over_time</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='range' title='range'>[1h]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.health.environment.availability.percentage-tabs" id="materialize.health.environment.availability.percentage-tab-1">
  <label for="materialize.health.environment.availability.percentage-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">materialize_cloud_organization_namespace</span>}<span style="color:#960050;background-color:#1e0010">.</span>rollup<span style="color:#f92672">(</span><span style="color:#66d9ef">avg</span>, <span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">range</span>}<span style="color:#f92672">)</span> <span style="color:#f92672">*</span> <span style="color:#ae81ff">100</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.info.version">materialize.info.version
  <a class="anchor" href="#materialize.info.version">#</a>
</h4>
The version of Materialize running in the environment. A single version
is the steady state; multiple values appear briefly during a rolling
upgrade.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.info.version-tabs" id="materialize.info.version-tab-0" checked>
  <label for="materialize.info.version-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>mz_version<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>compute_cluster_status{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.info.version-tabs" id="materialize.info.version-tab-1">
  <label for="materialize.info.version-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}compute_cluster_status{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">mz_version</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.info.max_lag">materialize.info.max_lag
  <a class="anchor" href="#materialize.info.max_lag">#</a>
</h4>
The worst lag seen anywhere in the environment over the
selected window — how far the most-behind collection&rsquo;s output trailed
real time. A top-level freshness pointer.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.info.max_lag-tabs" id="materialize.info.max_lag-tab-0" checked>
  <label for="materialize.info.max_lag-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max_over_time</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      mz_dataflow_wallclock_lag_seconds{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, instance_id<span style="color:#f92672">!=</span>&#34;&#34;, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">1</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#f92672">&lt;</span> <span style="color:#ae81ff">1</span>e9
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>[<span style="color:#960050;background-color:#1e0010">${rangeWindow}:</span><span style="color:#e6db74">1m</span>]
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.info.max_lag-tabs" id="materialize.info.max_lag-tab-1">
  <label for="materialize.info.max_lag-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_dataflow_wallclock_lag_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, <span style="color:#66d9ef">quantile</span><span style="color:#960050;background-color:#1e0010">:</span><span style="color:#ae81ff">1</span><span style="color:#960050;background-color:#1e0010">}.</span>rollup<span style="color:#f92672">(</span><span style="color:#66d9ef">max</span>, <span style="color:#ae81ff">3600</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>

## materialize-kubernetes

<p>Kubernetes-side view of a Materialize deployment: capacity, workload
readiness, per-pod resource usage, and networking. Adapted from the Overview
dashboard&rsquo;s &ldquo;Kubernetes Workloads&rdquo; tab and the k8s-sourced Summary panels.</p>
<p>These read kube-state-metrics and cAdvisor (via the kubelet), NOT Materialize
metrics — the same meta-monitoring surface other targets (e.g. Loki health)
will draw on. <code>%%{cAdvisorFilter}</code> is the container-scoping filter fragment
(namespace + drop empty/pause series); <code>%%{mzNamespaceList}</code> is the raw
namespace selector used by the kube_* and container_network_* metrics.</p>
<p>The percent-of-limit panels have an absolute-units sibling for deployments
whose metrics source (e.g. GKE&rsquo;s managed cAdvisor/KSM) doesn&rsquo;t expose resource
limits; the dashboard picks whichever fits the environment.</p>

<h4 id="materialize.kubernetes.cpu.capacity">materialize.kubernetes.cpu.capacity
  <a class="anchor" href="#materialize.kubernetes.cpu.capacity">#</a>
</h4>
Total CPU cores configured across the environment&rsquo;s containers (sum of
cAdvisor CPU limits), excluding the monitoring exporter — i.e. the CPU
available to the actual workload.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.capacity-tabs" id="materialize.kubernetes.cpu.capacity-tab-0" checked>
  <label for="materialize.kubernetes.cpu.capacity-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_cpu_quota{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> container_spec_cpu_period{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.capacity-tabs" id="materialize.kubernetes.cpu.capacity-tab-1">
  <label for="materialize.kubernetes.cpu.capacity-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_cpu_quota{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">container</span>} <span style="color:#f92672">/</span> <span style="color:#ae81ff">100000</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.memory.capacity">materialize.kubernetes.memory.capacity
  <a class="anchor" href="#materialize.kubernetes.memory.capacity">#</a>
</h4>
Total memory configured across the environment&rsquo;s containers (sum of
cAdvisor memory limits), excluding the monitoring exporter. Memory is
Materialize&rsquo;s dominant constraint — in-memory arrangements live in here.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.capacity-tabs" id="materialize.kubernetes.memory.capacity-tab-0" checked>
  <label for="materialize.kubernetes.memory.capacity-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_memory_limit_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.capacity-tabs" id="materialize.kubernetes.memory.capacity-tab-1">
  <label for="materialize.kubernetes.memory.capacity-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_memory_limit_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.cpu.capacity.all_containers">materialize.kubernetes.cpu.capacity.all_containers
  <a class="anchor" href="#materialize.kubernetes.cpu.capacity.all_containers">#</a>
</h4>
Total CPU cores configured across every container in the environment,
including the monitoring exporter — the Kubernetes view of what is
provisioned rather than what is available to the workload.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.capacity.all_containers-tabs" id="materialize.kubernetes.cpu.capacity.all_containers-tab-0" checked>
  <label for="materialize.kubernetes.cpu.capacity.all_containers-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_cpu_quota{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> container_spec_cpu_period{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.capacity.all_containers-tabs" id="materialize.kubernetes.cpu.capacity.all_containers-tab-1">
  <label for="materialize.kubernetes.cpu.capacity.all_containers-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_cpu_quota{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">container</span>} <span style="color:#f92672">/</span> <span style="color:#ae81ff">100000</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.memory.capacity.all_containers">materialize.kubernetes.memory.capacity.all_containers
  <a class="anchor" href="#materialize.kubernetes.memory.capacity.all_containers">#</a>
</h4>
Total memory configured across every container in the environment,
including the monitoring exporter — the Kubernetes view of what is
provisioned rather than what is available to the workload.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.capacity.all_containers-tabs" id="materialize.kubernetes.memory.capacity.all_containers-tab-0" checked>
  <label for="materialize.kubernetes.memory.capacity.all_containers-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_memory_limit_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.capacity.all_containers-tabs" id="materialize.kubernetes.memory.capacity.all_containers-tab-1">
  <label for="materialize.kubernetes.memory.capacity.all_containers-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_memory_limit_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.cpu.usage.percent">materialize.kubernetes.cpu.usage.percent
  <a class="anchor" href="#materialize.kubernetes.cpu.usage.percent">#</a>
</h4>
Current CPU usage per container type as a fraction of its limit,
averaged over the last 5 minutes — shows the worst-loaded container
types.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.usage.percent-tabs" id="materialize.kubernetes.cpu.usage.percent-tab-0" checked>
  <label for="materialize.kubernetes.cpu.usage.percent-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span><span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_resource_limits{resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;, namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.usage.percent-tabs" id="materialize.kubernetes.cpu.usage.percent-tab-1">
  <label for="materialize.kubernetes.cpu.usage.percent-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_cpu_usage_seconds_total{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_pod_container_resource_limits{<span style="color:#960050;background-color:#1e0010">resource:cpu</span>, <span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.cpu.usage.absolute">materialize.kubernetes.cpu.usage.absolute
  <a class="anchor" href="#materialize.kubernetes.cpu.usage.absolute">#</a>
</h4>
Current CPU usage per container type in cores (rate over 5 minutes), for
deployments whose metrics source doesn&rsquo;t expose CPU limits — read it
against the replica sizes you configured.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.usage.absolute-tabs" id="materialize.kubernetes.cpu.usage.absolute-tab-0" checked>
  <label for="materialize.kubernetes.cpu.usage.absolute-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span><span style="color:#960050;background-color:#1e0010">}</span>[<span style="color:#e6db74">5m</span>]<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.cpu.usage.absolute-tabs" id="materialize.kubernetes.cpu.usage.absolute-tab-1">
  <label for="materialize.kubernetes.cpu.usage.absolute-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_cpu_usage_seconds_total{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.memory.usage.percent">materialize.kubernetes.memory.usage.percent
  <a class="anchor" href="#materialize.kubernetes.memory.usage.percent">#</a>
</h4>
Current memory usage per container type as a fraction of its limit —
shows the worst-loaded container types.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.usage.percent-tabs" id="materialize.kubernetes.memory.usage.percent-tab-0" checked>
  <label for="materialize.kubernetes.memory.usage.percent-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_memory_working_set_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    container_spec_memory_limit_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.usage.percent-tabs" id="materialize.kubernetes.memory.usage.percent-tab-1">
  <label for="materialize.kubernetes.memory.usage.percent-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_memory_working_set_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_memory_limit_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.memory.usage.absolute">materialize.kubernetes.memory.usage.absolute
  <a class="anchor" href="#materialize.kubernetes.memory.usage.absolute">#</a>
</h4>
Current memory (working set) per container type in bytes, for
deployments whose metrics source doesn&rsquo;t expose memory limits.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.usage.absolute-tabs" id="materialize.kubernetes.memory.usage.absolute-tab-0" checked>
  <label for="materialize.kubernetes.memory.usage.absolute-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_memory_working_set_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.memory.usage.absolute-tabs" id="materialize.kubernetes.memory.usage.absolute-tab-1">
  <label for="materialize.kubernetes.memory.usage.absolute-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_memory_working_set_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.last_restart">materialize.kubernetes.last_restart
  <a class="anchor" href="#materialize.kubernetes.last_restart">#</a>
</h4>
Seconds since the most recent container restart in the environment.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.last_restart-tabs" id="materialize.kubernetes.last_restart-tab-0" checked>
  <label for="materialize.kubernetes.last_restart-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">time</span><span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">-</span> <span style="color:#66d9ef">topk</span><span style="color:#f92672">(</span><span style="color:#ae81ff">1</span>,
</span></span><span style="display:flex;"><span>    container_start_time_seconds{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.last_restart-tabs" id="materialize.kubernetes.last_restart-tab-1">
  <label for="materialize.kubernetes.last_restart-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>top<span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>container_start_time_seconds{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>},
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">1</span>, &#39;<span style="color:#e6db74">max</span>&#39;, &#39;<span style="color:#e6db74">desc</span>&#39;
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.readiness">materialize.kubernetes.pods.readiness
  <a class="anchor" href="#materialize.kubernetes.pods.readiness">#</a>
</h4>
Pods in the Materialize namespace grouped by phase (Running, Pending,
Failed, …).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.readiness-tabs" id="materialize.kubernetes.pods.readiness-tab-0" checked>
  <label for="materialize.kubernetes.pods.readiness-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>phase, namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>phase, namespace, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_pod_status_phase{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.readiness-tabs" id="materialize.kubernetes.pods.readiness-tab-1">
  <label for="materialize.kubernetes.pods.readiness-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_pod_status_phase{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">phase</span>,<span style="color:#960050;background-color:#1e0010">namespace</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.statefulsets.ready">materialize.kubernetes.statefulsets.ready
  <a class="anchor" href="#materialize.kubernetes.statefulsets.ready">#</a>
</h4>
StatefulSet replicas reporting Ready. environmentd and the cluster pods
are StatefulSets.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.statefulsets.ready-tabs" id="materialize.kubernetes.statefulsets.ready-tab-0" checked>
  <label for="materialize.kubernetes.statefulsets.ready-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_statefulset_status_replicas_ready{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.statefulsets.ready-tabs" id="materialize.kubernetes.statefulsets.ready-tab-1">
  <label for="materialize.kubernetes.statefulsets.ready-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_statefulset_status_replicas_ready{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.deployments.readiness">materialize.kubernetes.deployments.readiness
  <a class="anchor" href="#materialize.kubernetes.deployments.readiness">#</a>
</h4>
Deployment replica health — Ready vs Unavailable. Deployments back
stateless services (e.g. the promsql exporter).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.deployments.readiness-tabs" id="materialize.kubernetes.deployments.readiness-tab-0" checked>
  <label for="materialize.kubernetes.deployments.readiness-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_deployment_status_replicas_ready{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    kube_deployment_status_replicas_unavailable{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.deployments.readiness-tabs" id="materialize.kubernetes.deployments.readiness-tab-1">
  <label for="materialize.kubernetes.deployments.readiness-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_deployment_status_replicas_ready{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_deployment_status_replicas_unavailable{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.cpu_usage">materialize.kubernetes.pods.cpu_usage
  <a class="anchor" href="#materialize.kubernetes.pods.cpu_usage">#</a>
</h4>
CPU utilization per pod as a fraction of the pod&rsquo;s limit. Split so the
cluster/replica selectors filter the cluster pods while envd/balancer/
exporter stay visible.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.cpu_usage-tabs" id="materialize.kubernetes.pods.cpu_usage-tab-0" checked>
  <label for="materialize.kubernetes.pods.cpu_usage-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, pod<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_resource_limits{resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;, namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_cpu_usage_seconds_total{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, pod<span style="color:#960050;background-color:#1e0010">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  kube_pod_container_resource_limits{resource<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">cpu</span>&#34;, namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.cpu_usage-tabs" id="materialize.kubernetes.pods.cpu_usage-tab-1">
  <label for="materialize.kubernetes.pods.cpu_usage-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_cpu_usage_seconds_total{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_pod_container_resource_limits{<span style="color:#960050;background-color:#1e0010">resource:cpu</span>, <span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_cpu_usage_seconds_total{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>kube_pod_container_resource_limits{<span style="color:#960050;background-color:#1e0010">resource:cpu</span>, <span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.memory_usage">materialize.kubernetes.pods.memory_usage
  <a class="anchor" href="#materialize.kubernetes.pods.memory_usage">#</a>
</h4>
Memory usage per pod as a fraction of the pod&rsquo;s limit (working-set
basis), same cluster/non-cluster split as pod CPU.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.memory_usage-tabs" id="materialize.kubernetes.pods.memory_usage-tab-0" checked>
  <label for="materialize.kubernetes.pods.memory_usage-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_memory_working_set_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;, pod<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_memory_limit_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;, pod<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_memory_working_set_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;, pod<span style="color:#960050;background-color:#1e0010">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">/</span> <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod, container<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  container_spec_memory_limit_bytes{<span contenteditable='true' class='replaceable' data-replace='cAdvisorFilter' title='cAdvisorFilter'>container!="POD", container!=""</span>, container<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">new-promsql-exporter</span>&#34;, pod<span style="color:#960050;background-color:#1e0010">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.memory_usage-tabs" id="materialize.kubernetes.pods.memory_usage-tab-1">
  <label for="materialize.kubernetes.pods.memory_usage-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>container_memory_working_set_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_memory_limit_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>container_memory_working_set_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>container_spec_memory_limit_bytes{<span style="color:#960050;background-color:#1e0010">${cAdvisorFilter</span>}, <span style="color:#960050;background-color:#1e0010">!</span>container<span style="color:#960050;background-color:#1e0010">:</span>new<span style="color:#f92672">-</span>promsql<span style="color:#f92672">-</span>exporter, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>,<span style="color:#960050;background-color:#1e0010">container</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.network_rx">materialize.kubernetes.pods.network_rx
  <a class="anchor" href="#materialize.kubernetes.pods.network_rx">#</a>
</h4>
Network bytes/sec received per pod. For cluster pods, Rx tracks ingest
from upstream and inter-pod replication; for envd/balancer it&rsquo;s client
SQL traffic. Surges alongside hydration are normal catchup.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_rx-tabs" id="materialize.kubernetes.pods.network_rx-tab-0" checked>
  <label for="materialize.kubernetes.pods.network_rx-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_bytes_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_rx-tabs" id="materialize.kubernetes.pods.network_rx-tab-1">
  <label for="materialize.kubernetes.pods.network_rx-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_bytes_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_bytes_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.network_tx">materialize.kubernetes.pods.network_tx
  <a class="anchor" href="#materialize.kubernetes.pods.network_tx">#</a>
</h4>
Network bytes/sec transmitted per pod. For cluster pods Tx covers sink
output, inter-pod replication, and query results returning to envd; for
envd it&rsquo;s client query responses.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_tx-tabs" id="materialize.kubernetes.pods.network_tx-tab-0" checked>
  <label for="materialize.kubernetes.pods.network_tx-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_bytes_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_bytes_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_tx-tabs" id="materialize.kubernetes.pods.network_tx-tab-1">
  <label for="materialize.kubernetes.pods.network_tx-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_bytes_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_bytes_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.network_errors">materialize.kubernetes.pods.network_errors
  <a class="anchor" href="#materialize.kubernetes.pods.network_errors">#</a>
</h4>
Network rx + tx errors per pod per second (counted at the NIC/kernel
level).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_errors-tabs" id="materialize.kubernetes.pods.network_errors-tab-0" checked>
  <label for="materialize.kubernetes.pods.network_errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_errors_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_errors_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_errors_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_errors_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_errors-tabs" id="materialize.kubernetes.pods.network_errors-tab-1">
  <label for="materialize.kubernetes.pods.network_errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_errors_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_errors_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_errors_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_errors_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.kubernetes.pods.network_drops">materialize.kubernetes.pods.network_drops
  <a class="anchor" href="#materialize.kubernetes.pods.network_drops">#</a>
</h4>
Network packets dropped (rx + tx) per pod per second — when kernel
buffers fill faster than the app reads (rx) or egress rate-limiting
kicks in (tx).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_drops-tabs" id="materialize.kubernetes.pods.network_drops-tab-0" checked>
  <label for="materialize.kubernetes.pods.network_drops-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_packets_dropped_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_receive_packets_dropped_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_packets_dropped_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*-cluster-${mzClusterListRegex}-replica-${mzReplicaListRegex}-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>namespace, pod<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>container_network_transmit_packets_dropped_total{namespace<span style="color:#f92672">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzNamespaceList' title='mzNamespaceList'>materialize-environment</span>&#34;, pod<span style="color:#f92672">!~</span>&#34;<span style="color:#e6db74">.*-cluster-.*-replica-.*</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.kubernetes.pods.network_drops-tabs" id="materialize.kubernetes.pods.network_drops-tab-1">
  <label for="materialize.kubernetes.pods.network_drops-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_packets_dropped_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_receive_packets_dropped_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_packets_dropped_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}<span style="color:#f92672">-</span>replica<span style="color:#f92672">-</span><span style="color:#960050;background-color:#1e0010">$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>container_network_transmit_packets_dropped_total{<span style="color:#960050;background-color:#1e0010">namespace:${mzNamespaceList</span>}, <span style="color:#960050;background-color:#1e0010">!</span>pod<span style="color:#960050;background-color:#1e0010">:</span><span style="color:#f92672">*-</span>cluster<span style="color:#f92672">-*-</span>replica<span style="color:#f92672">-*</span><span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">namespace</span>,<span style="color:#960050;background-color:#1e0010">pod</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>

## materialize-logs

<p>Logs, as collected by the monitoring stack and stored in Loki.</p>
<p>Every query here is LogQL, and the scope is <strong>Loki-discovered end to end</strong>:
namespace, app and level come from Loki&rsquo;s own label values rather than from the
metrics pipeline. That is deliberate. Reading logs is frequently how an operator
works out why the <em>metrics</em> pipeline is broken, and a logs dashboard that
derived its scope from Prometheus would go blind at exactly the moment it is
most needed.</p>
<p>The label contract the agent and gateway produce is documented under
<a href="../../../logs-and-events/querying/">Logs and Events</a>. What matters here:
<code>namespace</code>, <code>app</code>, <code>level</code> and <code>container</code> are stream labels and belong in the
selector; <code>pod</code>, <code>node</code>, <code>organization_name</code> and the rest are structured
metadata and are filtered after a <code>|</code>. Narrowing the selector before the line
filters is the single biggest speedup.</p>
<p>Every selector here carries <code>%%{mzLogJobFilter}</code>, and it is not only a filter.
LogQL rejects a stream selector whose every matcher can match the empty string —
<em>&ldquo;queries require at least one regexp or equality matcher that does not have an
empty-compatible value&rdquo;</em> — and a dashboard built from <code>=~</code> pickers is exactly
that shape. The job picker&rsquo;s &ldquo;All&rdquo; is <code>.+</code> rather than the discovered values, so
it always contributes a non-empty matcher and the selector parses whatever the
other pickers are set to. Without it, &ldquo;All&rdquo; everywhere is a query error rather
than a wide result.</p>
<p>The event queries in <code>materialize-events.yaml</code> need no such anchor: they pin
<code>job=&quot;loki.source.kubernetes_events&quot;</code>, which is already a non-empty equality
matcher, and a second <code>job</code> matcher would AND with it and zero the panel.</p>
<p><code>level</code> is normalized by the pipeline where it can be and falls back to
<code>UNKNOWN</code>, so the levels present are a property of the workloads running rather
than a fixed vocabulary — which is why the dashboard discovers them instead of
hard-coding a list.</p>

<h4 id="materialize.logs.stream">materialize.logs.stream
  <a class="anchor" href="#materialize.logs.stream">#</a>
</h4>
The log feed for the selected namespaces, apps and levels, newest first.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.stream-tabs" id="materialize.logs.stream-tab-0" checked>
  <label for="materialize.logs.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.logs.rate.by_app">materialize.logs.rate.by_app
  <a class="anchor" href="#materialize.logs.rate.by_app">#</a>
</h4>
Log lines per second by application — which component is doing the
talking.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.rate.by_app-tabs" id="materialize.logs.rate.by_app-tab-0" checked>
  <label for="materialize.logs.rate.by_app-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.logs.rate.by_level">materialize.logs.rate.by_level
  <a class="anchor" href="#materialize.logs.rate.by_level">#</a>
</h4>
Log lines per second by severity — the shape of how much of the volume is
something going wrong.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.rate.by_level-tabs" id="materialize.logs.rate.by_level-tab-0" checked>
  <label for="materialize.logs.rate.by_level-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.logs.rate.total">materialize.logs.rate.total
  <a class="anchor" href="#materialize.logs.rate.total">#</a>
</h4>
Total log lines per second reaching Loki for the current selection,
averaged over each interval.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.rate.total-tabs" id="materialize.logs.rate.total-tab-0" checked>
  <label for="materialize.logs.rate.total-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.logs.warnings.rate">materialize.logs.warnings.rate
  <a class="anchor" href="#materialize.logs.warnings.rate">#</a>
</h4>
Warning-and-worse log lines per minute, as one series — the at-a-glance
answer to whether anything is complaining.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.warnings.rate-tabs" id="materialize.logs.warnings.rate-tab-0" checked>
  <label for="materialize.logs.warnings.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>
<h4 id="materialize.logs.warnings.stream">materialize.logs.warnings.stream
  <a class="anchor" href="#materialize.logs.warnings.stream">#</a>
</h4>
The warning-and-worse feed, newest first — what the components are
actually complaining about.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.logs.warnings.stream-tabs" id="materialize.logs.warnings.stream-tab-0" checked>
  <label for="materialize.logs.warnings.stream-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"></code></pre></div>
  </div>
</div>

## materialize-operator

<p>How the Materialize operator&rsquo;s reconciliation loop is behaving.</p>
<p>orchestratord watches the <code>Materialize</code>, <code>Balancer</code> and <code>Console</code> resources
and drives each toward the state its spec asks for. One trip through that
work is a <em>pass</em>, and a pass moves through named <em>steps</em>. Both are counted by
outcome, and both are timed, which is what lets a stuck rollout say not just
that it is stuck but which phase it is stuck in.</p>
<p>These are the operator&rsquo;s own metrics, scraped from its pods in the operator
namespace, so they are scoped by <code>%%{mzOperatorNamespaceFilter}</code> and by
nothing else. <strong>They carry no organization label</strong>, so the environment picker
does not narrow them: one operator reconciles every environment in the
cluster, and its loop is a single shared thing rather than a per-environment
one.</p>
<p>Only the replica holding the leadership lease reconciles. The others export
the same metric families sitting at zero, which is why every query here sums
across replicas rather than picking one out.</p>

<h4 id="materialize.operator.reconciling.replicas">materialize.operator.reconciling.replicas
  <a class="anchor" href="#materialize.operator.reconciling.replicas">#</a>
</h4>
How many operator replicas hold the leadership lease and are therefore
reconciling. This should be exactly one.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciling.replicas-tabs" id="materialize.operator.reconciling.replicas-tab-0" checked>
  <label for="materialize.operator.reconciling.replicas-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">or</span>chestratord_is_leader{<span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.environments.needing_update">materialize.operator.environments.needing_update
  <a class="anchor" href="#materialize.operator.environments.needing_update">#</a>
</h4>
How many environments in this cluster are still running an outdated pod
template — the count an upgrade is working to bring to zero.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.environments.needing_update-tabs" id="materialize.operator.environments.needing_update-tab-0" checked>
  <label for="materialize.operator.environments.needing_update-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  environmentd_needs_update{<span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.rate">materialize.operator.reconciliation.rate
  <a class="anchor" href="#materialize.operator.reconciliation.rate">#</a>
</h4>
Reconciliation passes per second across every controller — whether the
loop is turning at all.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.rate-tabs" id="materialize.operator.reconciliation.rate-tab-0" checked>
  <label for="materialize.operator.reconciliation.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliations_total{<span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.failures.total">materialize.operator.reconciliation.failures.total
  <a class="anchor" href="#materialize.operator.reconciliation.failures.total">#</a>
</h4>
Reconciliation passes that returned an error over the selected time
range.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.failures.total-tabs" id="materialize.operator.reconciliation.failures.total-tab-0" checked>
  <label for="materialize.operator.reconciliation.failures.total-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">increase</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliations_total{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>, outcome<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">failed</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='range' title='range'>[1h]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.outcomes">materialize.operator.reconciliation.outcomes
  <a class="anchor" href="#materialize.operator.reconciliation.outcomes">#</a>
</h4>
What reconciliation passes concluded, by outcome. The shape of a
rollout: <code>waiting</code> climbs while the new generation&rsquo;s pods come up, then
gives way to <code>applied</code> when they are ready.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.outcomes-tabs" id="materialize.operator.reconciliation.outcomes-tab-0" checked>
  <label for="materialize.operator.reconciliation.outcomes-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>outcome<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliations_total{<span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.failures.by_controller">materialize.operator.reconciliation.failures.by_controller
  <a class="anchor" href="#materialize.operator.reconciliation.failures.by_controller">#</a>
</h4>
Failing passes broken out by which controller failed and which of its
entry points was running — the first question after &ldquo;something is
failing&rdquo;.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.failures.by_controller-tabs" id="materialize.operator.reconciliation.failures.by_controller-tab-0" checked>
  <label for="materialize.operator.reconciliation.failures.by_controller-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>controller, event_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliations_total{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>, outcome<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">failed</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.duration">materialize.operator.reconciliation.duration
  <a class="anchor" href="#materialize.operator.reconciliation.duration">#</a>
</h4>
How long one reconciliation pass takes, at the 50th, 90th and 99th
percentiles.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.duration-tabs" id="materialize.operator.reconciliation.duration-tab-0" checked>
  <label for="materialize.operator.reconciliation.duration-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.5</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_duration_seconds_bucket{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.9</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_duration_seconds_bucket{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_duration_seconds_bucket{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.step.duration.p99">materialize.operator.reconciliation.step.duration.p99
  <a class="anchor" href="#materialize.operator.reconciliation.step.duration.p99">#</a>
</h4>
The slowest phase of a reconciliation pass, at the 99th percentile per
step — where the time in a pass actually goes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.step.duration.p99-tabs" id="materialize.operator.reconciliation.step.duration.p99-tab-0" checked>
  <label for="materialize.operator.reconciliation.step.duration.p99-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le, step<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_step_duration_seconds_bucket{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.steps.rate">materialize.operator.reconciliation.steps.rate
  <a class="anchor" href="#materialize.operator.reconciliation.steps.rate">#</a>
</h4>
Which phases of reconciliation are running, and how often. A rollout
moves through these in order, so the set that is active says where the
operator has got to.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.steps.rate-tabs" id="materialize.operator.reconciliation.steps.rate-tab-0" checked>
  <label for="materialize.operator.reconciliation.steps.rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>step<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_steps_total{<span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span><span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.operator.reconciliation.steps.incomplete">materialize.operator.reconciliation.steps.incomplete
  <a class="anchor" href="#materialize.operator.reconciliation.steps.incomplete">#</a>
</h4>
Steps that did not complete, by step and by how they ended — the panel
that turns &ldquo;reconciliation is failing&rdquo; into &ldquo;reconciliation is failing
<em>here</em>&rdquo;.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.operator.reconciliation.steps.incomplete-tabs" id="materialize.operator.reconciliation.steps.incomplete-tab-0" checked>
  <label for="materialize.operator.reconciliation.steps.incomplete-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>step, outcome<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">or</span>chestratord_reconciliation_steps_total{
</span></span><span style="display:flex;"><span>      <span contenteditable='true' class='replaceable' data-replace='mzOperatorNamespaceFilter' title='mzOperatorNamespaceFilter'>namespace=~"materialize"</span>, outcome<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span style="color:#e6db74">failed|abandoned</span>&#34;
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>

## materialize-storage

<p>Sources and sinks for a Materialize deployment — catalog shape, throughput,
lag, and upstream/downstream health. Adapted from the Overview dashboard&rsquo;s
&ldquo;Sources and Sinks&rdquo; tab.</p>
<p>The clusterd-side throughput/lag/error metrics (mz_source_* / mz_sink_<em>) carry
the long-form cluster_environmentd_materialize_cloud_</em> id labels. These queries
assume one Prometheus job per clusterd endpoint; if the same endpoint is
scraped by several jobs, a plain sum-rate reads N× — dedupe the job at the
deployment (fix the scrape config, or wrap the inner rate in <code>max without (job)</code>) rather than baking it into the canonical query.</p>

<h4 id="materialize.storage.sources.count">materialize.storage.sources.count
  <a class="anchor" href="#materialize.storage.sources.count">#</a>
</h4>
Active sources in the catalog — each is a continuous ingestion
connection from an external system (Kafka, Postgres, MySQL, S3, …), so
this is roughly how many upstream feeds the environment maintains.
Counts distinct source objects (the hidden per-source <code>_progress</code>
subsources are excluded), matching <code>mz_sources</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.count-tabs" id="materialize.storage.sources.count-tab-0" checked>
  <label for="materialize.storage.sources.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>storage_objects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, type<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">source</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.count-tabs" id="materialize.storage.sources.count-tab-1">
  <label for="materialize.storage.sources.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span>count_not_null<span style="color:#f92672">(</span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}storage_objects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, type<span style="color:#960050;background-color:#1e0010">:</span>source<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">id</span>}<span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.count">materialize.storage.sinks.count
  <a class="anchor" href="#materialize.storage.sinks.count">#</a>
</h4>
Active sinks in the catalog — each emits the results of a materialized
view or query to an external system (Kafka, Iceberg, …). Counts distinct
sink objects (excluding <code>_progress</code> subsources), matching <code>mz_sinks</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.count-tabs" id="materialize.storage.sinks.count-tab-0" checked>
  <label for="materialize.storage.sinks.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span><span style="color:#f92672">(</span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>storage_objects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, type<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">sink</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.count-tabs" id="materialize.storage.sinks.count-tab-1">
  <label for="materialize.storage.sinks.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span>count_not_null<span style="color:#f92672">(</span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}storage_objects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, type<span style="color:#960050;background-color:#1e0010">:</span>sink<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">id</span>}<span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.tables.count">materialize.storage.tables.count
  <a class="anchor" href="#materialize.storage.tables.count">#</a>
</h4>
User-created tables in the catalog. Tables are write-once-read-many;
<code>INSERT</code>s feed dataflows downstream. Mostly a catalog-shape signal — for
actual ingest activity look at source throughput.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.tables.count-tabs" id="materialize.storage.tables.count-tab-0" checked>
  <label for="materialize.storage.tables.count-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#f92672">(</span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>tables_count{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span><span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.tables.count-tabs" id="materialize.storage.tables.count-tab-1">
  <label for="materialize.storage.tables.count-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>default_zero<span style="color:#f92672">(</span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}tables_count{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sources.by_type">materialize.storage.sources.by_type
  <a class="anchor" href="#materialize.storage.sources.by_type">#</a>
</h4>
Sources by connector type (kafka / postgres / mysql / …) — what flavors
of upstream feed make up the ingest workload. Most environments
concentrate on one or two.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.by_type-tabs" id="materialize.storage.sources.by_type-tab-0" checked>
  <label for="materialize.storage.sources.by_type-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>object_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>id, object_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>storage_objects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, type<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">source</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.by_type-tabs" id="materialize.storage.sources.by_type-tab-1">
  <label for="materialize.storage.sources.by_type-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}storage_objects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, type<span style="color:#960050;background-color:#1e0010">:</span>source<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">object_type</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sources.catalog">materialize.storage.sources.catalog
  <a class="anchor" href="#materialize.storage.sources.catalog">#</a>
</h4>
A catalog of sources — one row per source (by name) with its connector
type, envelope, and the cluster it ingests on. The metric-side &ldquo;what
sources do I have&rdquo; reference.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.catalog-tabs" id="materialize.storage.sources.catalog-tab-0" checked>
  <label for="materialize.storage.sources.catalog-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>id, object_type, connection_type, envelope_type, cluster_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>storage_objects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, type<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">source</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.catalog-tabs" id="materialize.storage.sources.catalog-tab-1">
  <label for="materialize.storage.sources.catalog-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}storage_objects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, type<span style="color:#960050;background-color:#1e0010">:</span>source<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">id</span>,<span style="color:#960050;background-color:#1e0010">object_type</span>,<span style="color:#960050;background-color:#1e0010">connection_type</span>,<span style="color:#960050;background-color:#1e0010">envelope_type</span>,<span style="color:#960050;background-color:#1e0010">cluster_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sources.bytes_received">materialize.storage.sources.bytes_received
  <a class="anchor" href="#materialize.storage.sources.bytes_received">#</a>
</h4>
Inbound throughput per primary source — bytes/second pulled from
upstream. Subsources (e.g. per-table Postgres replication) roll up to
their primary, so each line is one logical source.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.bytes_received-tabs" id="materialize.storage.sources.bytes_received-tab-0" checked>
  <label for="materialize.storage.sources.bytes_received-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>parent_source_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_source_bytes_received{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.bytes_received-tabs" id="materialize.storage.sources.bytes_received-tab-1">
  <label for="materialize.storage.sources.bytes_received-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_source_bytes_received{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">parent_source_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sources.ingestion_by_replica">materialize.storage.sources.ingestion_by_replica
  <a class="anchor" href="#materialize.storage.sources.ingestion_by_replica">#</a>
</h4>
Messages ingested per second, split per source AND replica. Replicas
read their upstream independently and should track together.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.ingestion_by_replica-tabs" id="materialize.storage.sources.ingestion_by_replica-tab-0" checked>
  <label for="materialize.storage.sources.ingestion_by_replica-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>parent_source_id, cluster_environmentd_materialize_cloud_replica_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_source_messages_received{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.ingestion_by_replica-tabs" id="materialize.storage.sources.ingestion_by_replica-tab-1">
  <label for="materialize.storage.sources.ingestion_by_replica-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_source_messages_received{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">parent_source_id</span>,<span style="color:#960050;background-color:#1e0010">cluster_environmentd_materialize_cloud_replica_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sources.upstream_errors">materialize.storage.sources.upstream_errors
  <a class="anchor" href="#materialize.storage.sources.upstream_errors">#</a>
</h4>
Per-source upstream health, with two complementary signals — both
nominal at 0, so an empty panel is healthy and any series means a source
needs attention.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sources.upstream_errors-tabs" id="materialize.storage.sources.upstream_errors-tab-0" checked>
  <label for="materialize.storage.sources.upstream_errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>source_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_source_offset_commit_failures{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>source_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_source_offset_committed{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>source_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_source_offset_known{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sources.upstream_errors-tabs" id="materialize.storage.sources.upstream_errors-tab-1">
  <label for="materialize.storage.sources.upstream_errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_source_offset_commit_failures{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">source_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_source_offset_committed{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">source_id</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>mz_source_offset_known{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">source_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.by_type">materialize.storage.sinks.by_type
  <a class="anchor" href="#materialize.storage.sinks.by_type">#</a>
</h4>
Sinks by (type, envelope) — e.g. <code>kafka / upsert</code>, <code>kafka / debezium</code>,
<code>iceberg / upsert</code>. The envelope is how Materialize encodes changes:
<code>upsert</code> writes the latest value per key, <code>debezium</code> writes change
events with old+new values.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.by_type-tabs" id="materialize.storage.sinks.by_type-tab-0" checked>
  <label for="materialize.storage.sinks.by_type-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>object_type, envelope_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">group</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>id, object_type, envelope_type<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='mzSqlPrefix' title='mzSqlPrefix'>mz_</span>storage_objects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, type<span style="color:#960050;background-color:#1e0010">=</span>&#34;<span style="color:#e6db74">sink</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.by_type-tabs" id="materialize.storage.sinks.by_type-tab-1">
  <label for="materialize.storage.sinks.by_type-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzSqlPrefix</span>}storage_objects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, type<span style="color:#960050;background-color:#1e0010">:</span>sink<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">object_type</span>,<span style="color:#960050;background-color:#1e0010">envelope_type</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.throughput">materialize.storage.sinks.throughput
  <a class="anchor" href="#materialize.storage.sinks.throughput">#</a>
</h4>
Outbound throughput per sink — bytes/second successfully committed to
the downstream system (Kafka broker, Iceberg catalog, …).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.throughput-tabs" id="materialize.storage.sinks.throughput-tab-0" checked>
  <label for="materialize.storage.sinks.throughput-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_bytes_committed{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.throughput-tabs" id="materialize.storage.sinks.throughput-tab-1">
  <label for="materialize.storage.sinks.throughput-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_bytes_committed{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.lag">materialize.storage.sinks.lag
  <a class="anchor" href="#materialize.storage.sinks.lag">#</a>
</h4>
Bytes staged for a sink but not yet committed downstream — an in-flight
queue depth in bytes. Oscillates around a small value in normal
operation as commits happen periodically.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.lag-tabs" id="materialize.storage.sinks.lag-tab-0" checked>
  <label for="materialize.storage.sinks.lag-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">clamp_min</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_sink_bytes_staged{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_sink_bytes_committed{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>,
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.lag-tabs" id="materialize.storage.sinks.lag-tab-1">
  <label for="materialize.storage.sinks.lag-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">clamp_min</span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_bytes_staged{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">-</span> <span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_bytes_committed{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>},
</span></span><span style="display:flex;"><span>  <span style="color:#ae81ff">0</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.iceberg.commit_latency">materialize.storage.sinks.iceberg.commit_latency
  <a class="anchor" href="#materialize.storage.sinks.iceberg.commit_latency">#</a>
</h4>
Iceberg commit-duration percentiles (p50/p90/p99) — how long each
<code>COMMIT</code> against the Iceberg catalog takes (write a snapshot manifest,
ask the catalog to atomically swap it in).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.commit_latency-tabs" id="materialize.storage.sinks.iceberg.commit_latency-tab-0" checked>
  <label for="materialize.storage.sinks.iceberg.commit_latency-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.50</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_commit_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.90</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_commit_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">histogram_quantile</span><span style="color:#f92672">(</span><span style="color:#ae81ff">0.99</span>, <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>le<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_commit_duration_seconds_bucket{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">))))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.commit_latency-tabs" id="materialize.storage.sinks.iceberg.commit_latency-tab-1">
  <label for="materialize.storage.sinks.iceberg.commit_latency-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p50<span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_commit_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p90<span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_commit_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span>p99<span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_commit_duration_seconds{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.iceberg.commit_failures">materialize.storage.sinks.iceberg.commit_failures
  <a class="anchor" href="#materialize.storage.sinks.iceberg.commit_failures">#</a>
</h4>
Per-sink rate of failed and conflicting Iceberg commits. Conflicts
(concurrent-writer races on the snapshot pointer) are recoverable —
Materialize retries — but a high rate means something else is writing
the same Iceberg table; failures are commit-side errors (network, auth,
schema).
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.commit_failures-tabs" id="materialize.storage.sinks.iceberg.commit_failures-tab-0" checked>
  <label for="materialize.storage.sinks.iceberg.commit_failures-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_commit_failures{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_commit_conflicts{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.commit_failures-tabs" id="materialize.storage.sinks.iceberg.commit_failures-tab-1">
  <label for="materialize.storage.sinks.iceberg.commit_failures-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_commit_failures{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_commit_conflicts{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.iceberg.file_rate">materialize.storage.sinks.iceberg.file_rate
  <a class="anchor" href="#materialize.storage.sinks.iceberg.file_rate">#</a>
</h4>
Per-sink rate of files and snapshots written to Iceberg. Each commit
produces one snapshot with data files (new rows) and delete files
(tombstones for upserts). The data:delete ratio reflects your workload —
pure-insert sinks produce ~0 deletes; upsert-heavy ones roughly 1:1.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.file_rate-tabs" id="materialize.storage.sinks.iceberg.file_rate-tab-0" checked>
  <label for="materialize.storage.sinks.iceberg.file_rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_data_files_written{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_delete_files_written{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_iceberg_snapshots_committed{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.iceberg.file_rate-tabs" id="materialize.storage.sinks.iceberg.file_rate-tab-1">
  <label for="materialize.storage.sinks.iceberg.file_rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_data_files_written{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_delete_files_written{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_iceberg_snapshots_committed{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.kafka.tx_errors">materialize.storage.sinks.kafka.tx_errors
  <a class="anchor" href="#materialize.storage.sinks.kafka.tx_errors">#</a>
</h4>
Per-sink rate of TX errors from the librdkafka client — each is one
failed produce-request against the broker.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.tx_errors-tabs" id="materialize.storage.sinks.kafka.tx_errors-tab-0" checked>
  <label for="materialize.storage.sinks.kafka.tx_errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_rdkafka_txerrs{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.tx_errors-tabs" id="materialize.storage.sinks.kafka.tx_errors-tab-1">
  <label for="materialize.storage.sinks.kafka.tx_errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_rdkafka_txerrs{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.kafka.output_buffer">materialize.storage.sinks.kafka.output_buffer
  <a class="anchor" href="#materialize.storage.sinks.kafka.output_buffer">#</a>
</h4>
Messages sitting in the librdkafka output buffer, waiting to be sent to
the broker. Normal buffer fluctuates briefly as messages flow through.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.output_buffer-tabs" id="materialize.storage.sinks.kafka.output_buffer-tab-0" checked>
  <label for="materialize.storage.sinks.kafka.output_buffer-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>mz_sink_rdkafka_outbuf_msg_cnt{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span style="color:#f92672">))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.output_buffer-tabs" id="materialize.storage.sinks.kafka.output_buffer-tab-1">
  <label for="materialize.storage.sinks.kafka.output_buffer-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_rdkafka_outbuf_msg_cnt{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="materialize.storage.sinks.kafka.connect_rate">materialize.storage.sinks.kafka.connect_rate
  <a class="anchor" href="#materialize.storage.sinks.kafka.connect_rate">#</a>
</h4>
Connect and disconnect events per sink against the Kafka broker. Healthy
connections are persistent — a couple of connects at startup and zero
disconnects afterward.
<div class="book-tabs">
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.connect_rate-tabs" id="materialize.storage.sinks.kafka.connect_rate-tab-0" checked>
  <label for="materialize.storage.sinks.kafka.connect_rate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_rdkafka_connects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>sink_id<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>mz_sink_rdkafka_disconnects{<span contenteditable='true' class='replaceable' data-replace='mzEnvironmentFilter' title='mzEnvironmentFilter'>materialize_cloud_organization_name=~".*"</span>, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzClusterList' title='mzClusterList'>.*</span>&#34;, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">=~</span>&#34;<span contenteditable='true' class='replaceable' data-replace='mzReplicaList' title='mzReplicaList'>.*</span>&#34;<span style="color:#960050;background-color:#1e0010">}</span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)))</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="materialize.storage.sinks.kafka.connect_rate-tabs" id="materialize.storage.sinks.kafka.connect_rate-tab-1">
  <label for="materialize.storage.sinks.kafka.connect_rate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_rdkafka_connects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span><span style="color:#960050;background-color:#1e0010">:</span>mz_sink_rdkafka_disconnects{<span style="color:#960050;background-color:#1e0010">${mzEnvironmentFilter</span>}, cluster_environmentd_materialize_cloud_cluster_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzClusterList</span>}, cluster_environmentd_materialize_cloud_replica_id<span style="color:#960050;background-color:#1e0010">:$</span>{<span style="color:#960050;background-color:#1e0010">mzReplicaList</span>}<span style="color:#960050;background-color:#1e0010">}</span> <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">sink_id</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>

## node-debug

<p>The breakdowns you reach for once <code>node-health.yaml</code> has told you a node is in
trouble: which mode the CPU is in, where the memory went, which device is
slow, and where the packets are being lost. Adapted from the Node Exporter
Full dashboard (<a href="https://grafana.com/grafana/dashboards/1860" rel="external" class="external-link">https://grafana.com/grafana/dashboards/1860</a>, revision 45).</p>
<p>Split from <code>node-health.yaml</code> on tier rather than on subject. These sit at
<code>recommended</code>, so a deployment collecting only the essential tier still gets
the health surface and pays nothing for the detail. Nothing here should back
an alert — if something here is worth paging on, it belongs in
<code>node-health.yaml</code> instead.</p>
<p>The conventions are the same as <code>node-health.yaml</code>: <code>instance=~&quot;$nodeList&quot;</code> rather
than <code>=</code>, every query wrapped in <code>max by (instance, ...)</code> (or <code>min</code> where low
is the bad direction) so a second scrape job cannot double-count, inner
aggregations carrying <code>by (instance, job)</code> so the outer wrapper is what
collapses <code>job</code>, and <code>%%{interval}</code> as the rate window.</p>
<p>Only collectors this chart&rsquo;s allowlist enables are referenced. Notable
omissions, because the dashboard has panels for them and they will render
empty: <code>node_processes_*</code> (processes collector), <code>node_interrupts_total</code>
(interrupts), <code>node_tcp_connection_states</code> (tcpstat), <code>node_systemd_*</code>
(systemd), and <code>node_textfile_scrape_error</code> (textfile).</p>

<h4 id="node.debug.cpu.by_mode">node.debug.cpu.by_mode
  <a class="anchor" href="#node.debug.cpu.by_mode">#</a>
</h4>
CPU time by mode — system, user, iowait, and the interrupt modes —
averaged across cores.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.cpu.by_mode-tabs" id="node.debug.cpu.by_mode-tab-0" checked>
  <label for="node.debug.cpu.by_mode-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">system</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">user</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">iowait</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">without</span> <span style="color:#f92672">(</span>mode<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>      <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">.*irq</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">steal</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.cpu.by_mode-tabs" id="node.debug.cpu.by_mode-tab-1">
  <label for="node.debug.cpu.by_mode-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:system</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:user</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:iowait</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:*irq</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:steal</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.cpu.per_core">node.debug.cpu.per_core
  <a class="anchor" href="#node.debug.cpu.per_core">#</a>
</h4>
Non-idle CPU time per core, so a single saturated core is visible.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.cpu.per_core-tabs" id="node.debug.cpu.per_core-tab-0" checked>
  <label for="node.debug.cpu.per_core-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">1</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">idle</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.cpu.per_core-tabs" id="node.debug.cpu.per_core-tab-1">
  <label for="node.debug.cpu.per_core-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">1</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">mode:idle</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.schedstat.waiting">node.debug.schedstat.waiting
  <a class="anchor" href="#node.debug.schedstat.waiting">#</a>
</h4>
Time tasks spent runnable but not running, per core, from
<code>/proc/schedstat</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.schedstat.waiting-tabs" id="node.debug.schedstat.waiting-tab-0" checked>
  <label for="node.debug.schedstat.waiting-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_schedstat_waiting_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.schedstat.waiting-tabs" id="node.debug.schedstat.waiting-tab-1">
  <label for="node.debug.schedstat.waiting-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_schedstat_waiting_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.context_switches">node.debug.context_switches
  <a class="anchor" href="#node.debug.context_switches">#</a>
</h4>
Context switches and hardware interrupts per second.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.context_switches-tabs" id="node.debug.context_switches-tab-0" checked>
  <label for="node.debug.context_switches-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_context_switches_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_intr_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.context_switches-tabs" id="node.debug.context_switches-tab-1">
  <label for="node.debug.context_switches-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_context_switches_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_intr_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.memory.breakdown">node.debug.memory.breakdown
  <a class="anchor" href="#node.debug.memory.breakdown">#</a>
</h4>
Where RAM went: total, used by processes, reclaimable cache, and free.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.memory.breakdown-tabs" id="node.debug.memory.breakdown-tab-0" checked>
  <label for="node.debug.memory.breakdown-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_MemTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_memory_MemTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> node_memory_MemFree_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> node_memory_Cached_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> node_memory_Buffers_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> node_memory_SReclaimable_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_memory_Cached_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">+</span> node_memory_Buffers_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">+</span> node_memory_SReclaimable_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_MemFree_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.memory.breakdown-tabs" id="node.debug.memory.breakdown-tab-1">
  <label for="node.debug.memory.breakdown-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemTotal_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemTotal_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemFree_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Cached_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Buffers_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SReclaimable_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Cached_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">+</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Buffers_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">+</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SReclaimable_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemFree_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.memory.kernel">node.debug.memory.kernel
  <a class="anchor" href="#node.debug.memory.kernel">#</a>
</h4>
Kernel-side memory: slab total, reclaimable and unreclaimable slab, and
committed address space.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.memory.kernel-tabs" id="node.debug.memory.kernel-tab-0" checked>
  <label for="node.debug.memory.kernel-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_Slab_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SReclaimable_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SUnreclaim_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_Committed_AS_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.memory.kernel-tabs" id="node.debug.memory.kernel-tab-1">
  <label for="node.debug.memory.kernel-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Slab_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SReclaimable_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SUnreclaim_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_Committed_AS_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.memory.page_faults">node.debug.memory.page_faults
  <a class="anchor" href="#node.debug.memory.page_faults">#</a>
</h4>
Total and major page faults per second.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.memory.page_faults-tabs" id="node.debug.memory.page_faults-tab-0" checked>
  <label for="node.debug.memory.page_faults-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgfault{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgmajfault{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.memory.page_faults-tabs" id="node.debug.memory.page_faults-tab-1">
  <label for="node.debug.memory.page_faults-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgfault{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgmajfault{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.memory.reclaim">node.debug.memory.reclaim
  <a class="anchor" href="#node.debug.memory.reclaim">#</a>
</h4>
Pages scanned and reclaimed per second, split by who did the reclaiming:
<code>kswapd</code> (background) or <code>direct</code> (an allocating thread).
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.memory.reclaim-tabs" id="node.debug.memory.reclaim-tab-0" checked>
  <label for="node.debug.memory.reclaim-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgscan_kswapd{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgscan_direct{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgsteal_kswapd{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pgsteal_direct{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.memory.reclaim-tabs" id="node.debug.memory.reclaim-tab-1">
  <label for="node.debug.memory.reclaim-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgscan_kswapd{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgscan_direct{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgsteal_kswapd{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pgsteal_direct{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.disk.iops">node.debug.disk.iops
  <a class="anchor" href="#node.debug.disk.iops">#</a>
</h4>
Completed reads and writes per second, per device.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.disk.iops-tabs" id="node.debug.disk.iops-tab-0" checked>
  <label for="node.debug.disk.iops-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_reads_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_writes_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.disk.iops-tabs" id="node.debug.disk.iops-tab-1">
  <label for="node.debug.disk.iops-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_reads_completed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_writes_completed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.disk.throughput">node.debug.disk.throughput
  <a class="anchor" href="#node.debug.disk.throughput">#</a>
</h4>
Bytes read and written per second, per device.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.disk.throughput-tabs" id="node.debug.disk.throughput-tab-0" checked>
  <label for="node.debug.disk.throughput-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_read_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_written_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.disk.throughput-tabs" id="node.debug.disk.throughput-tab-1">
  <label for="node.debug.disk.throughput-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_read_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_written_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.disk.latency">node.debug.disk.latency
  <a class="anchor" href="#node.debug.disk.latency">#</a>
</h4>
Average time per completed read and per completed write, per device.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.disk.latency-tabs" id="node.debug.disk.latency-tab-0" checked>
  <label for="node.debug.disk.latency-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_reads_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">0</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">*</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_read_time_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_reads_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span><span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_writes_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">0</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">*</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_write_time_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_writes_completed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.disk.latency-tabs" id="node.debug.disk.latency-tab-1">
  <label for="node.debug.disk.latency-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_read_time_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_reads_completed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_write_time_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_writes_completed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.disk.queue_depth">node.debug.disk.queue_depth
  <a class="anchor" href="#node.debug.disk.queue_depth">#</a>
</h4>
Average I/O queue depth per device.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.disk.queue_depth-tabs" id="node.debug.disk.queue_depth-tab-0" checked>
  <label for="node.debug.disk.queue_depth-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_io_time_weighted_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.disk.queue_depth-tabs" id="node.debug.disk.queue_depth-tab-1">
  <label for="node.debug.disk.queue_depth-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_io_time_weighted_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.filesystem.inodes.available.ratio">node.debug.filesystem.inodes.available.ratio
  <a class="anchor" href="#node.debug.filesystem.inodes.available.ratio">#</a>
</h4>
Fraction of inodes still free, per mountpoint.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.filesystem.inodes.available.ratio-tabs" id="node.debug.filesystem.inodes.available.ratio-tab-0" checked>
  <label for="node.debug.filesystem.inodes.available.ratio-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, mountpoint<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_filesystem_files_free{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;, fstype<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">rootfs</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, mountpoint<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_filesystem_files{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;, fstype<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">rootfs</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.filesystem.inodes.available.ratio-tabs" id="node.debug.filesystem.inodes.available.ratio-tab-1">
  <label for="node.debug.filesystem.inodes.available.ratio-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_filesystem_files_free{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">!fstype:rootfs</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">mountpoint</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_filesystem_files{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">!fstype:rootfs</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">mountpoint</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.network.throughput">node.debug.network.throughput
  <a class="anchor" href="#node.debug.network.throughput">#</a>
</h4>
Bytes received and transmitted per second, per interface.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.network.throughput-tabs" id="node.debug.network.throughput-tab-0" checked>
  <label for="node.debug.network.throughput-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.network.throughput-tabs" id="node.debug.network.throughput-tab-1">
  <label for="node.debug.network.throughput-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_receive_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_transmit_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.network.saturation">node.debug.network.saturation
  <a class="anchor" href="#node.debug.network.saturation">#</a>
</h4>
Receive and transmit throughput as a fraction of the interface&rsquo;s
reported link speed.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.network.saturation-tabs" id="node.debug.network.saturation-tab-0" checked>
  <label for="node.debug.network.saturation-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>node_network_speed_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;} <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">0</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">*</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> node_network_speed_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>node_network_speed_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;} <span style="color:#f92672">&gt;</span> <span style="color:#66d9ef">bool</span> <span style="color:#ae81ff">0</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">*</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">/</span> node_network_speed_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.network.saturation-tabs" id="node.debug.network.saturation-tab-1">
  <label for="node.debug.network.saturation-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_receive_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_speed_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_transmit_bytes_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_speed_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.network.operstate">node.debug.network.operstate
  <a class="anchor" href="#node.debug.network.operstate">#</a>
</h4>
Whether each interface is operationally up, and whether it has carrier.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.network.operstate-tabs" id="node.debug.network.operstate-tab-0" checked>
  <label for="node.debug.network.operstate-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_network_up{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_network_carrier{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.network.operstate-tabs" id="node.debug.network.operstate-tab-1">
  <label for="node.debug.network.operstate-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_up{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_carrier{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.softnet.processed">node.debug.softnet.processed
  <a class="anchor" href="#node.debug.softnet.processed">#</a>
</h4>
Packets processed by the network softirq path, per CPU.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.softnet.processed-tabs" id="node.debug.softnet.processed-tab-0" checked>
  <label for="node.debug.softnet.processed-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_softnet_processed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.softnet.processed-tabs" id="node.debug.softnet.processed-tab-1">
  <label for="node.debug.softnet.processed-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_softnet_processed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.softnet.dropped">node.debug.softnet.dropped
  <a class="anchor" href="#node.debug.softnet.dropped">#</a>
</h4>
Packets dropped in the network softirq path because the backlog queue
was full, per CPU.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.softnet.dropped-tabs" id="node.debug.softnet.dropped-tab-0" checked>
  <label for="node.debug.softnet.dropped-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_softnet_dropped_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.softnet.dropped-tabs" id="node.debug.softnet.dropped-tab-1">
  <label for="node.debug.softnet.dropped-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_softnet_dropped_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.softnet.squeezed">node.debug.softnet.squeezed
  <a class="anchor" href="#node.debug.softnet.squeezed">#</a>
</h4>
Times the softirq handler exhausted its budget with work still queued,
per CPU.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.softnet.squeezed-tabs" id="node.debug.softnet.squeezed-tab-0" checked>
  <label for="node.debug.softnet.squeezed-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_softnet_times_squeezed_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.softnet.squeezed-tabs" id="node.debug.softnet.squeezed-tab-1">
  <label for="node.debug.softnet.squeezed-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_softnet_times_squeezed_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.sockets.tcp">node.debug.sockets.tcp
  <a class="anchor" href="#node.debug.sockets.tcp">#</a>
</h4>
TCP sockets by state: in use, allocated, orphaned, and TIME_WAIT.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.sockets.tcp-tabs" id="node.debug.sockets.tcp-tab-0" checked>
  <label for="node.debug.sockets.tcp-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_TCP_inuse{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_TCP_alloc{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_TCP_orphan{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_TCP_tw{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.sockets.tcp-tabs" id="node.debug.sockets.tcp-tab-1">
  <label for="node.debug.sockets.tcp-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_TCP_inuse{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_TCP_alloc{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_TCP_orphan{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_TCP_tw{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.sockets.memory">node.debug.sockets.memory
  <a class="anchor" href="#node.debug.sockets.memory">#</a>
</h4>
Kernel socket buffer memory held by TCP and UDP.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.sockets.memory-tabs" id="node.debug.sockets.memory-tab-0" checked>
  <label for="node.debug.sockets.memory-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_TCP_mem_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_sockstat_UDP_mem_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.sockets.memory-tabs" id="node.debug.sockets.memory-tab-1">
  <label for="node.debug.sockets.memory-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_TCP_mem_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_sockstat_UDP_mem_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.tcp.retransmits">node.debug.tcp.retransmits
  <a class="anchor" href="#node.debug.tcp.retransmits">#</a>
</h4>
TCP segment retransmits and SYN retransmits per second, against total
segments out.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.tcp.retransmits-tabs" id="node.debug.tcp.retransmits-tab-0" checked>
  <label for="node.debug.tcp.retransmits-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_Tcp_RetransSegs{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_TcpExt_TCPSynRetrans{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_Tcp_OutSegs{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.tcp.retransmits-tabs" id="node.debug.tcp.retransmits-tab-1">
  <label for="node.debug.tcp.retransmits-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_Tcp_RetransSegs{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_TcpExt_TCPSynRetrans{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_Tcp_OutSegs{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.tcp.errors">node.debug.tcp.errors
  <a class="anchor" href="#node.debug.tcp.errors">#</a>
</h4>
TCP listen-queue overflows, listen drops, receive-queue drops, and
timeouts per second.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.tcp.errors-tabs" id="node.debug.tcp.errors-tab-0" checked>
  <label for="node.debug.tcp.errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_TcpExt_ListenOverflows{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_TcpExt_ListenDrops{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_TcpExt_TCPRcvQDrop{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_TcpExt_TCPTimeouts{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.tcp.errors-tabs" id="node.debug.tcp.errors-tab-1">
  <label for="node.debug.tcp.errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_TcpExt_ListenOverflows{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_TcpExt_ListenDrops{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_TcpExt_TCPRcvQDrop{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_TcpExt_TCPTimeouts{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.udp.errors">node.debug.udp.errors
  <a class="anchor" href="#node.debug.udp.errors">#</a>
</h4>
UDP receive errors, receive-buffer errors, and packets to no listening
port, per second.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.udp.errors-tabs" id="node.debug.udp.errors-tab-0" checked>
  <label for="node.debug.udp.errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_Udp_InErrors{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_Udp_RcvbufErrors{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_netstat_Udp_NoPorts{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.udp.errors-tabs" id="node.debug.udp.errors-tab-1">
  <label for="node.debug.udp.errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_Udp_InErrors{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_Udp_RcvbufErrors{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_netstat_Udp_NoPorts{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.udp.queues">node.debug.udp.queues
  <a class="anchor" href="#node.debug.udp.queues">#</a>
</h4>
Bytes queued in UDP receive and transmit buffers.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.udp.queues-tabs" id="node.debug.udp.queues-tab-0" checked>
  <label for="node.debug.udp.queues-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_udp_queues{ip<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">v4</span>&#34;, queue<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">rx</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_udp_queues{ip<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">v4</span>&#34;, queue<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">tx</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.udp.queues-tabs" id="node.debug.udp.queues-tab-1">
  <label for="node.debug.udp.queues-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_udp_queues{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">ip:v4</span>, <span style="color:#960050;background-color:#1e0010">queue:rx</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_udp_queues{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">ip:v4</span>, <span style="color:#960050;background-color:#1e0010">queue:tx</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.arp.entries">node.debug.arp.entries
  <a class="anchor" href="#node.debug.arp.entries">#</a>
</h4>
ARP table entries per interface.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.arp.entries-tabs" id="node.debug.arp.entries-tab-0" checked>
  <label for="node.debug.arp.entries-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_arp_entries{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.arp.entries-tabs" id="node.debug.arp.entries-tab-1">
  <label for="node.debug.arp.entries-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_arp_entries{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.time.sync_status">node.debug.time.sync_status
  <a class="anchor" href="#node.debug.time.sync_status">#</a>
</h4>
Whether the kernel clock is synchronized (1) or NTP has given up (0).
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.time.sync_status-tabs" id="node.debug.time.sync_status-tab-0" checked>
  <label for="node.debug.time.sync_status-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_timex_sync_status{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.time.sync_status-tabs" id="node.debug.time.sync_status-tab-1">
  <label for="node.debug.time.sync_status-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_timex_sync_status{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.time.drift">node.debug.time.drift
  <a class="anchor" href="#node.debug.time.drift">#</a>
</h4>
Estimated clock offset, maximum error, and estimated error, in seconds.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.time.drift-tabs" id="node.debug.time.drift-tab-0" checked>
  <label for="node.debug.time.drift-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_timex_offset_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_timex_maxerror_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_timex_estimated_error_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.time.drift-tabs" id="node.debug.time.drift-tab-1">
  <label for="node.debug.time.drift-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_timex_offset_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_timex_maxerror_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_timex_estimated_error_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.entropy.available">node.debug.entropy.available
  <a class="anchor" href="#node.debug.entropy.available">#</a>
</h4>
Available entropy, against the pool size.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.entropy.available-tabs" id="node.debug.entropy.available-tab-0" checked>
  <label for="node.debug.entropy.available-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_entropy_available_bits{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_entropy_pool_size_bits{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.entropy.available-tabs" id="node.debug.entropy.available-tab-1">
  <label for="node.debug.entropy.available-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_entropy_available_bits{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_entropy_pool_size_bits{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.debug.exporter.scrape_duration">node.debug.exporter.scrape_duration
  <a class="anchor" href="#node.debug.exporter.scrape_duration">#</a>
</h4>
How long each node-exporter collector took on the last scrape.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.debug.exporter.scrape_duration-tabs" id="node.debug.exporter.scrape_duration-tab-0" checked>
  <label for="node.debug.exporter.scrape_duration-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, collector<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_scrape_collector_duration_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.debug.exporter.scrape_duration-tabs" id="node.debug.exporter.scrape_duration-tab-1">
  <label for="node.debug.exporter.scrape_duration-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_scrape_collector_duration_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">collector</span>}
</span></span></code></pre></div>
  </div>
</div>

## node-health

<p>Node-level health: the at-a-glance answers to &ldquo;is this machine in trouble&rdquo;,
adapted from the Node Exporter Full dashboard
(<a href="https://grafana.com/grafana/dashboards/1860" rel="external" class="external-link">https://grafana.com/grafana/dashboards/1860</a>, revision 45) and narrowed to
what actually alerts.</p>
<p>These read node-exporter, NOT Materialize metrics. Only collectors this
chart&rsquo;s allowlist enables are referenced — see the Node Exporter section of
the chart&rsquo;s values reference for the list and the reasoning. The deeper
breakdowns live in <code>node-debug.yaml</code> at the <code>recommended</code> tier.</p>
<p>Three conventions apply to every query here:</p>
<ul>
<li>
<p><strong><code>instance=~&quot;$nodeList&quot;</code>, never <code>instance=&quot;$nodeList&quot;</code>.</strong> A regex match makes the
selector work unchanged whether the dashboard variable resolves to one node
or many, so the same query backs a single-node view and a fleet view.</p>
</li>
<li>
<p><strong>Every query is wrapped in <code>max by (instance, ...)</code>, which drops <code>job</code>.</strong>
A node should only ever be scraped by one job. If a second one appears —
a pre-existing node-exporter alongside ours, or a migration with both
running — the same series arrives twice under different <code>job</code> labels, and
every <code>sum()</code> silently doubles while every binary operation between two
metrics loses its match. Aggregating <code>job</code> away makes both failure modes
impossible rather than merely unlikely. <code>max</code> is the default; <strong><code>min</code> where
low is the bad direction</strong> (available memory, free space, collector
success), so the aggregate always reports the worst case rather than hiding
it behind a healthy duplicate.</p>
</li>
<li>
<p>Where an inner aggregation is needed (averaging across CPUs, summing across
devices), it carries <code>by (instance, job)</code> and the outer <code>max</code>/<code>min</code> collapses
<code>job</code> afterwards. Aggregating both in one step would blend two jobs'
readings into one number instead of picking one.</p>
</li>
</ul>
<p><code>%%{interval}</code> is the rate window, including its brackets.</p>

<h4 id="node.cpu.utilization">node.cpu.utilization
  <a class="anchor" href="#node.cpu.utilization">#</a>
</h4>
Fraction of CPU time the node spent doing anything other than idling,
averaged across its cores.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.cpu.utilization-tabs" id="node.cpu.utilization-tab-0" checked>
  <label for="node.cpu.utilization-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">1</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">avg</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_cpu_seconds_total{mode<span style="color:#f92672">=</span>&#34;<span style="color:#e6db74">idle</span>&#34;, instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.cpu.utilization-tabs" id="node.cpu.utilization-tab-1">
  <label for="node.cpu.utilization-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#ae81ff">1</span> <span style="color:#f92672">-</span> <span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">mode:idle</span>, <span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.load.normalized">node.load.normalized
  <a class="anchor" href="#node.load.normalized">#</a>
</h4>
One-minute load average divided by the node&rsquo;s core count, so it is
comparable across instance sizes.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.load.normalized-tabs" id="node.load.normalized-tab-0" checked>
  <label for="node.load.normalized-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_load1{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">count</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, job, cpu<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_cpu_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.load.normalized-tabs" id="node.load.normalized-tab-1">
  <label for="node.load.normalized-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_load1{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> count_not_null<span style="color:#f92672">(</span><span style="color:#66d9ef">avg</span><span style="color:#960050;background-color:#1e0010">:</span>node_cpu_seconds_total{<span style="color:#960050;background-color:#1e0010">mode:idle</span>, <span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">cpu</span>}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.cpu.pressure">node.cpu.pressure
  <a class="anchor" href="#node.cpu.pressure">#</a>
</h4>
PSI: the fraction of wall time at least one task was stalled waiting for
CPU.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.cpu.pressure-tabs" id="node.cpu.pressure-tab-0" checked>
  <label for="node.cpu.pressure-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_pressure_cpu_waiting_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.cpu.pressure-tabs" id="node.cpu.pressure-tab-1">
  <label for="node.cpu.pressure-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_pressure_cpu_waiting_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.memory.available.ratio">node.memory.available.ratio
  <a class="anchor" href="#node.memory.available.ratio">#</a>
</h4>
Fraction of RAM the kernel estimates is available for new allocations
without swapping, from <code>MemAvailable</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.memory.available.ratio-tabs" id="node.memory.available.ratio-tab-0" checked>
  <label for="node.memory.available.ratio-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_memory_MemAvailable_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_memory_MemTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.memory.available.ratio-tabs" id="node.memory.available.ratio-tab-1">
  <label for="node.memory.available.ratio-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemAvailable_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_MemTotal_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.memory.pressure">node.memory.pressure
  <a class="anchor" href="#node.memory.pressure">#</a>
</h4>
PSI: the fraction of wall time at least one task was stalled on memory
(<code>waiting</code>), and the fraction where <em>every</em> task was (<code>stalled</code>).
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.memory.pressure-tabs" id="node.memory.pressure-tab-0" checked>
  <label for="node.memory.pressure-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_pressure_memory_waiting_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_pressure_memory_stalled_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.memory.pressure-tabs" id="node.memory.pressure-tab-1">
  <label for="node.memory.pressure-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_pressure_memory_waiting_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_pressure_memory_stalled_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.swap.used.ratio">node.swap.used.ratio
  <a class="anchor" href="#node.swap.used.ratio">#</a>
</h4>
Fraction of configured swap in use. Zero when the node has no swap
configured, rather than returning no data.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.swap.used.ratio-tabs" id="node.swap.used.ratio-tab-0" checked>
  <label for="node.swap.used.ratio-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SwapTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">-</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SwapFree_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SwapTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">and</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">(</span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_memory_SwapTotal_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span> <span style="color:#f92672">&gt;</span> <span style="color:#ae81ff">0</span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.swap.used.ratio-tabs" id="node.swap.used.ratio-tab-1">
  <label for="node.swap.used.ratio-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SwapTotal_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>    <span style="color:#f92672">-</span> <span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SwapFree_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_memory_SwapTotal_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.swap.activity">node.swap.activity
  <a class="anchor" href="#node.swap.activity">#</a>
</h4>
Pages swapped in and out per second, from <code>vmstat</code>.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.swap.activity-tabs" id="node.swap.activity-tab-0" checked>
  <label for="node.swap.activity-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pswpin{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_pswpout{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.swap.activity-tabs" id="node.swap.activity-tab-1">
  <label for="node.swap.activity-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pswpin{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_pswpout{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.memory.oom_kills">node.memory.oom_kills
  <a class="anchor" href="#node.memory.oom_kills">#</a>
</h4>
Rate of OOM-killer invocations on the node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.memory.oom_kills-tabs" id="node.memory.oom_kills-tab-0" checked>
  <label for="node.memory.oom_kills-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_vmstat_oom_kill{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.memory.oom_kills-tabs" id="node.memory.oom_kills-tab-1">
  <label for="node.memory.oom_kills-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_vmstat_oom_kill{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.filesystem.available.ratio">node.filesystem.available.ratio
  <a class="anchor" href="#node.filesystem.available.ratio">#</a>
</h4>
Fraction of each mounted filesystem still available, per mountpoint.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.filesystem.available.ratio-tabs" id="node.filesystem.available.ratio-tab-0" checked>
  <label for="node.filesystem.available.ratio-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, mountpoint<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_filesystem_avail_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;, fstype<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">rootfs</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, mountpoint<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_filesystem_size_bytes{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;, fstype<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">rootfs</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.filesystem.available.ratio-tabs" id="node.filesystem.available.ratio-tab-1">
  <label for="node.filesystem.available.ratio-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_filesystem_avail_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">!fstype:rootfs</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">mountpoint</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_filesystem_size_bytes{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">!fstype:rootfs</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">mountpoint</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.filesystem.readonly">node.filesystem.readonly
  <a class="anchor" href="#node.filesystem.readonly">#</a>
</h4>
Whether a filesystem has been remounted read-only, per mountpoint.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.filesystem.readonly-tabs" id="node.filesystem.readonly-tab-0" checked>
  <label for="node.filesystem.readonly-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, mountpoint<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_filesystem_readonly{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;, fstype<span style="color:#f92672">!=</span>&#34;<span style="color:#e6db74">rootfs</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.filesystem.readonly-tabs" id="node.filesystem.readonly-tab-1">
  <label for="node.filesystem.readonly-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_filesystem_readonly{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>, <span style="color:#960050;background-color:#1e0010">!fstype:rootfs</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">mountpoint</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.disk.io_utilization">node.disk.io_utilization
  <a class="anchor" href="#node.disk.io_utilization">#</a>
</h4>
Fraction of wall time each block device had at least one I/O in flight.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.disk.io_utilization-tabs" id="node.disk.io_utilization-tab-0" checked>
  <label for="node.disk.io_utilization-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_disk_io_time_seconds_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.disk.io_utilization-tabs" id="node.disk.io_utilization-tab-1">
  <label for="node.disk.io_utilization-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_disk_io_time_seconds_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.filefd.utilization">node.filefd.utilization
  <a class="anchor" href="#node.filefd.utilization">#</a>
</h4>
Allocated file descriptors as a fraction of the system-wide maximum.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.filefd.utilization-tabs" id="node.filefd.utilization-tab-0" checked>
  <label for="node.filefd.utilization-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_filefd_allocated{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_filefd_maximum{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.filefd.utilization-tabs" id="node.filefd.utilization-tab-1">
  <label for="node.filefd.utilization-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_filefd_allocated{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_filefd_maximum{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.network.errors">node.network.errors
  <a class="anchor" href="#node.network.errors">#</a>
</h4>
Receive and transmit error rates per interface.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.network.errors-tabs" id="node.network.errors-tab-0" checked>
  <label for="node.network.errors-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_errs_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_errs_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.network.errors-tabs" id="node.network.errors-tab-1">
  <label for="node.network.errors-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_receive_errs_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_transmit_errs_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.network.drops">node.network.drops
  <a class="anchor" href="#node.network.drops">#</a>
</h4>
Receive and transmit packet drop rates per interface.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.network.drops-tabs" id="node.network.drops-tab-0" checked>
  <label for="node.network.drops-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_drop_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_drop_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.network.drops-tabs" id="node.network.drops-tab-1">
  <label for="node.network.drops-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_receive_drop_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_network_transmit_drop_total{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">device</span>}<span style="color:#960050;background-color:#1e0010">.</span>as_rate<span style="color:#f92672">()</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.network.rx.total">node.network.rx.total
  <a class="anchor" href="#node.network.rx.total">#</a>
</h4>
Bytes per second received across every interface on the node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.network.rx.total-tabs" id="node.network.rx.total-tab-0" checked>
  <label for="node.network.rx.total-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_receive_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.network.tx.total">node.network.tx.total
  <a class="anchor" href="#node.network.tx.total">#</a>
</h4>
Bytes per second transmitted across every interface on the node.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.network.tx.total-tabs" id="node.network.tx.total-tab-0" checked>
  <label for="node.network.tx.total-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">sum</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  <span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, device<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>    <span style="color:#66d9ef">rate</span><span style="color:#f92672">(</span>node_network_transmit_bytes_total{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#960050;background-color:#1e0010"></span><span contenteditable='true' class='replaceable' data-replace='interval' title='interval'>[5m]</span><span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.conntrack.utilization">node.conntrack.utilization
  <a class="anchor" href="#node.conntrack.utilization">#</a>
</h4>
Netfilter connection-tracking table occupancy as a fraction of its
limit.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.conntrack.utilization-tabs" id="node.conntrack.utilization-tab-0" checked>
  <label for="node.conntrack.utilization-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_nf_conntrack_entries{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span><span style="display:flex;"><span><span style="color:#f92672">/</span>
</span></span><span style="display:flex;"><span><span style="color:#66d9ef">max</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>node_nf_conntrack_entries_limit{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}<span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.conntrack.utilization-tabs" id="node.conntrack.utilization-tab-1">
  <label for="node.conntrack.utilization-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_nf_conntrack_entries{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">/</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_nf_conntrack_entries_limit{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.uptime">node.uptime
  <a class="anchor" href="#node.uptime">#</a>
</h4>
Seconds since the node booted.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.uptime-tabs" id="node.uptime-tab-0" checked>
  <label for="node.uptime-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_time_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;} <span style="color:#f92672">-</span> node_boot_time_seconds{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.uptime-tabs" id="node.uptime-tab-1">
  <label for="node.uptime-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_time_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span><span style="display:flex;"><span>  <span style="color:#f92672">-</span> <span style="color:#66d9ef">max</span><span style="color:#960050;background-color:#1e0010">:</span>node_boot_time_seconds{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>}
</span></span></code></pre></div>
  </div>
</div>
<h4 id="node.collector.success">node.collector.success
  <a class="anchor" href="#node.collector.success">#</a>
</h4>
Whether each enabled node-exporter collector returned data on the last
scrape, per collector.
<div class="book-tabs">
  <input type="radio" class="toggle" name="node.collector.success-tabs" id="node.collector.success-tab-0" checked>
  <label for="node.collector.success-tab-0">PromQL</label>
  <div class="book-tabs-content markdown-inner">
          
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span> <span style="color:#66d9ef">by</span> <span style="color:#f92672">(</span>instance, collector<span style="color:#f92672">)</span> <span style="color:#f92672">(</span>
</span></span><span style="display:flex;"><span>  node_scrape_collector_success{instance<span style="color:#f92672">=~</span>&#34;<span style="color:#e6db74">$nodeList</span>&#34;}
</span></span><span style="display:flex;"><span><span style="color:#f92672">)</span>
</span></span></code></pre></div>
  </div>
  <input type="radio" class="toggle" name="node.collector.success-tabs" id="node.collector.success-tab-1">
  <label for="node.collector.success-tab-1">Datadog</label>
  <div class="book-tabs-content markdown-inner">
            
<div class="highlight"><pre tabindex="0" style="color:#f8f8f2;background-color:#272822;-moz-tab-size:4;-o-tab-size:4;tab-size:4;-webkit-text-size-adjust:none;"><code class="language-promql" data-lang="promql"><span style="display:flex;"><span><span style="color:#66d9ef">min</span><span style="color:#960050;background-color:#1e0010">:</span>node_scrape_collector_success{<span style="color:#960050;background-color:#1e0010">instance:$nodeList</span>} <span style="color:#66d9ef">by</span> {<span style="color:#960050;background-color:#1e0010">instance</span>,<span style="color:#960050;background-color:#1e0010">collector</span>}
</span></span></code></pre></div>
  </div>
</div>


