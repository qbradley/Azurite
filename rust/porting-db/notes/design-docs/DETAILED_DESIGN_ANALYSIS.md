# OrionDB Design Documents - Detailed Technical Analysis

## Executive Summary

The OrionDB/HorizonDB system is Azure's next-generation PostgreSQL-compatible database platform with a disaggregated architecture. Key design decisions focus on:

1. **Performance Optimization** through caching (LFC), columnar storage, and intelligent spillover
2. **Scalability** via distributed PageServer architecture and resource isolation
3. **Reliability** through WAL replication and comprehensive recovery mechanisms
4. **Cost Efficiency** through tiered storage (hot/cold) and granular billing

---

## I. DATABASE ENGINE ARCHITECTURE

### A. Storage Disaggregation Model

**Key Decision:** Separate compute from storage
- **Compute Nodes:** PostgreSQL instances with local SSD for cache/temp tables
- **PageServer:** Manages page storage and retrieval
- **WAL Server:** Centralized write-ahead log service

**Benefits:**
- Independent scaling of compute and storage
- Simplified failover without data movement
- Multi-tenant resource isolation

**Trade-offs:**
- Network round-trips for page fetches
- Cache coherency challenges
- Additional complexity in consistency guarantees

---

### B. Local File Cache (LFC) Strategy

**Three-Level Cache Hierarchy:**

```
1. SHARED BUFFER (Hot)
   ├─ Most frequently accessed pages
   ├─ Fast but limited size
   └─ Managed by PostgreSQL

2. LFC (Warm)
   ├─ Local SSD on compute node
   ├─ Larger capacity than shared buffer
   ├─ Improves hit rate for working set
   └─ Reduces PageServer round-trips

3. PageServer (Cold)
   ├─ Remote storage
   ├─ Consistent source of truth
   └─ Higher latency access
```

**LFC Improvement Iterations:**

- **V1:** Basic local cache with eviction policies
- **V2:** Advanced techniques including:
  - Predictive eviction
  - Adaptive cache sizing based on workload
  - Per-SKU optimization for different hardware tiers

**Sizing by SKU:** Different compute tiers have different LFC cache sizes and policies
- Small SKU: Conservative cache sizing, higher PageServer hit rate
- Large SKU: Larger cache, more aggressive prefetching

---

### C. Memory Pressure & Spillover

**Challenge:** Queries can exceed available memory

**Solution: Two-Tier Spillover**

```
Query Execution Memory
│
├─ In-Memory (SHARED_BUFFERS)
│  ├─ Hash joins
│  ├─ Sort operations
│  └─ Aggregates
│
├─ Local SSD Spillover
│  ├─ Temp tablespaces
│  ├─ Large intermediate results
│  └─ Unlogged tables
│
└─ PageServer (if necessary)
   └─ Last resort, very high latency
```

**OOM Risk Mitigation:**

1. **Resource Limits via Cgroups**
   - Per-node memory limits
   - Per-query memory limits (via work_mem)
   - Enforcement at execution time

2. **Early Spillover**
   - Detect memory pressure before OOM
   - Proactively spill to SSD
   - Reduce pressure on global memory

3. **Query Cancellation**
   - Fail fast if spillover would be excessive
   - Better than OOM crash

---

### D. Columnar Indexes (POC)

**Architecture Overview:**

```
  CREATE INDEX idx USING columnar (col1, col2, col3)
                    ↓
            Custom Access Method
                    ↓
        [Row Group 1] [Row Group 2] [Row Group 3]
           (65K rows)    (65K rows)    (65K rows)
                ↓              ↓              ↓
        Vortex-encoded  Vortex-encoded  Vortex-encoded
           + Stats        + Stats        + Stats
                ↓              ↓              ↓
        PostgreSQL Buffer Pages (8KB chunks)
```

**Key Design Decisions:**

1. **Row Group Organization**
   - Default: 65,536 rows per row group
   - Self-contained unit for compression
   - Independent column encoding

2. **Vortex Compression**
   - Modern format (not Parquet which is from 2013)
   - Supports SIMD operations on compressed data
   - Advanced encodings: FastLanes, ALP, BtrBlocks
   - Automatic encoding selection

3. **Z-Order Clustering**
   - Morton code interleaving of column bits
   - Enables multi-dimensional pruning
   - Significantly improves filter selectivity
   - Cost: Requires full rebuild when data changes

4. **Query Execution**
   - PostgreSQL planner hook intercepts plans
   - Converts compatible subtrees to DataFusion LogicalPlans
   - Custom scan executor feeds Vortex data to DataFusion
   - SIMD filters operate on compressed arrays

**Multi-Threaded Execution:**
- Main thread: Unsafe I/O from PG buffer pages
- Worker threads: Pure compute on Vortex arrays
- Channel-based backpressure using work_mem

---

## II. DATA PLANE & CONSISTENCY

### A. WAL (Write-Ahead Log) Design

**Centralized WAL Service Advantages:**
- Single source of truth for durability
- Simplified consistency model
- Enables efficient replication

**Durability Guarantees:**
- Synchronous write to multiple sites
- Quorum-based availability
- RPO = 0 for committed transactions

**Multi-Site Replication:**
- Primary WAL Server
- Secondary replicas (at least 2 for HA)
- Quorum protocol for consensus

---

### B. Data Consistency Mechanisms

**ACID Implementation:**

```
Atomicity:
  └─ WAL ensures all-or-nothing writes

Consistency:
  ├─ WAL provides crash consistency
  ├─ Quorum-based replication ensures durability
  └─ PageServer acts as consistent storage

Isolation:
  ├─ MVCC for snapshot isolation
  ├─ Row-level visibility tracking
  └─ Delete vectors for columnar data

Durability:
  ├─ WAL on stable storage
  ├─ Quorum replication
  └─ PageServer persistent storage
```

**Consistency Model:**
- Strong consistency for committed transactions
- Snapshot isolation for reads
- Linearizability for single-key operations

---

## III. PAGE SERVER ARCHITECTURE

### A. Design Components

```
Compute Node          Network          PageServer
  ┌──────────────────────────┬──────────────────┐
  │  PostgreSQL              │                  │
  │  ├─ Shared Buffer        │    GetPage       │
  │  ├─ LFC (local SSD)      │    RPC           │
  │  └─ Query Engine         │                  │
  └──────────────────────────┼──────────────────┘
                             │
                    PageServer Gateway
                    (Protocol conversion)
                             │
                             ▼
                        PageServer
                        ├─ Page cache
                        ├─ Local storage
                        └─ Remote storage (Fabric)
```

### B. Storage Tiers in PageServer

**Three-Tier Architecture:**

```
1. PageServer Memory Cache (Hot)
   ├─ Frequently accessed pages
   ├─ Ultra-fast access
   └─ Limited capacity (GB)

2. Local Storage (Warm)
   ├─ SSD/NVMe on PageServer node
   ├─ Large capacity (TB)
   └─ Single-node failure risk

3. Fabric/OneLake (Cold)
   ├─ Azure storage account
   ├─ Unlimited capacity
   ├─ Low cost
   └─ Higher latency (milliseconds)
```

**Cost Optimization:**
- Hot pages cached at PageServer
- Warm pages on local SSD
- Cold pages immediately tiered to Fabric
- Fabric becomes source of truth

### C. Storage Limiting & Database Size

**Enforcement Mechanisms:**

```
Database Size Quota
    │
    ├─ Hard limit: Prevent INSERT if quota exceeded
    │
    ├─ Soft limit: Warnings at 80%/90%
    │
    ├─ Monitoring: Track growth rate
    │
    └─ Alerts: Notify customer of approaching limit
```

**PageServer Responsibility:**
- Enforce database size quota
- Reject writes exceeding quota
- Efficient tracking without overhead

### D. API Contract for PageServer Gateway

**Key Endpoints:**

```
GetPage(key, lsn)
  ├─ Retrieves page at specific LSN
  ├─ Supports time-travel (flashback)
  └─ Returns page + metadata

GetPageAtLSN(rel_file_node, blk_num, lsn)
  ├─ Atomic read at specific point in time
  └─ Essential for consistency

PUT_PageVersions
  ├─ Write modified pages
  ├─ Includes LSN and WAL record
  └─ Enables crash recovery

WAL_Recovery(lsn)
  ├─ WAL replay to specific LSN
  └─ Returns final page state
```

---

## IV. RESOURCE ISOLATION & MULTI-TENANCY

### A. Cgroups-Based Resource Limiting

**Hierarchy:**

```
Cgroup Root
├─ Tenant A
│  ├─ CPU: 4 cores (50%)
│  ├─ Memory: 8 GB
│  ├─ I/O: 100 MB/s
│  └─ Processes
│      ├─ postgres (main)
│      ├─ pgpool (connection pooling)
│      └─ workers
│
├─ Tenant B
│  ├─ CPU: 2 cores (25%)
│  ├─ Memory: 4 GB
│  ├─ I/O: 50 MB/s
│  └─ ...
│
└─ System
   ├─ Monitoring agents
   └─ WAL client
```

**Resource Limits:**

| Resource | Limit Type | Enforcement |
|----------|-----------|-------------|
| CPU | Hard | Kernel scheduler |
| Memory | Hard | OOM killer if exceeded |
| I/O | Hard | Block I/O throttling |
| Network | Soft | Application-level |

**Monitoring:**
- Per-tenant resource usage tracking
- Alerting on limit violations
- Proactive scaling recommendations

---

## V. OPERATIONAL ARCHITECTURE

### A. Service Fabric Integration

**Deployment Model:**

```
Azure Cluster
  │
  ├─ Compute Service (Stateful)
  │  ├─ Partition 1: Tenant A
  │  ├─ Partition 2: Tenant B
  │  ├─ Replica 1 (Primary)
  │  └─ Replica 2 (Secondary)
  │
  ├─ PageServer Service (Stateful)
  │  ├─ Partition per tenant
  │  ├─ Primary → Secondary replication
  │  └─ Fabric-managed failover
  │
  ├─ WAL Server Service (Stateful)
  │  ├─ Quorum replicas
  │  └─ Consensus on write
  │
  └─ Backup Service (Stateless)
     ├─ Scale-out backup collection
     └─ Load-balanced across instances
```

**Service Fabric Benefits:**
- Automatic health monitoring
- Automatic failover
- Rolling updates without downtime
- Built-in load balancing

### B. gRPC/Tonic Service Mesh

**Communication Pattern:**

```
Compute → PageServer
  └─ gRPC over mTLS
     ├─ Efficient serialization
     ├─ HTTP/2 multiplexing
     ├─ Stream-based APIs
     └─ Built-in load balancing

Compute → WAL Server
  └─ gRPC
     ├─ Async append
     └─ High throughput

Compute → Backup Service
  └─ gRPC
     ├─ Streaming large files
     └─ Resumable uploads
```

**Proxyless Service Mesh:**
- No sidecar proxy overhead
- Tonic handles service discovery
- Direct point-to-point communication
- Lower latency than proxy model

### C. A/B Deployment Strategy

**Progressive Rollout:**

```
Stage 1: Canary (1% of tenants)
  ├─ Monitor error rate
  ├─ Check performance regression
  └─ Validate new features

Stage 2: Staged Rollout (10% → 50% → 100%)
  ├─ Daily/weekly increase
  ├─ Continuous monitoring
  └─ Easy rollback available

Stage 3: Full Deployment
  ├─ All tenants on new version
  └─ Version support window ends
```

**Validation Criteria:**
- Error rate increase < 0.1%
- Latency p99 increase < 5%
- No data consistency issues
- Feature flags for safe enable/disable

---

## VI. BILLING & METERING

### A. Billing Model

**Dimensions:**

```
Cost = Compute + Storage + Network + WAL + PageServer

Compute:
  └─ $/hour per vCore

Storage:
  ├─ Hot storage (compute local SSD): $$/GB/month
  ├─ Warm storage (PageServer SSD): $/GB/month
  └─ Cold storage (Fabric): $/GB/month

Network:
  ├─ Outbound data transfer: $/GB
  └─ Data exfiltration: $/GB (higher rate)

WAL:
  └─ GB of WAL written: $/GB

PageServer:
  ├─ Page operations: $/million
  └─ Storage: $/GB/month
```

### B. Usage Metering

**Collection Points:**

```
Compute Node
  ├─ Query execution metrics
  ├─ Buffer cache hits
  ├─ LFC hits
  ├─ Memory usage
  ├─ CPU time
  └─ I/O operations

PageServer
  ├─ Page requests
  ├─ Cache hit rate
  ├─ Tiering operations
  ├─ Storage usage
  └─ Concurrent connections

WAL Server
  ├─ Bytes written
  ├─ Replication operations
  └─ Checkpoint time
```

**Aggregation & Attribution:**
- Per-tenant metrics collected
- Hourly aggregation
- Monthly billing cycle
- Cost attribution via resource tagging

---

## VII. RELEASE MANAGEMENT

### A. Phase 1 Release (January 2025)

**Feature Set:**
- Core PostgreSQL compatibility (v14-v16)
- Basic compute scaling (vertical)
- LFC caching
- WAL replication (2+ replicas)
- PageServer storage
- Single-region deployment
- Azure Portal integration

**Known Limitations:**
- No logical replication failover
- Limited extension support
- Single-region only (multi-region in Phase 2)
- No point-in-time recovery

### B. Configuration Management

**Flow:**

```
Customer sets config in Azure Portal
           ↓
    OCS (Orion Control Service)
           ↓
    Validates and stores
           ↓
    Distributes to compute/storage nodes
           ↓
    PostgreSQL applies configuration
           ↓
    Monitoring validates changes
```

**Ownership Model:**
- Some parameters: Customer-owned (can change freely)
- Some parameters: Service-owned (only OCS can change)
- Some parameters: Read-only (query only)

**Example Divisions:**
- Customer-owned: max_connections, log_level, search_path
- Service-owned: shared_buffers, effective_cache_size
- Read-only: version, server_version_num

---

## VIII. KEY TECHNICAL DECISIONS & TRADE-OFFS

### Decision 1: Disaggregated Architecture

**Choice:** Separate compute from storage

**Rationale:**
- Scale independently
- Simplified operations
- Lower data movement on failover

**Trade-offs:**
```
PROS:
  ✓ Horizontal scalability
  ✓ Multi-tenant isolation
  ✓ Simplified recovery
  ✓ Cost efficiency

CONS:
  ✗ Network round-trips
  ✗ Cache coherency complexity
  ✗ Network bandwidth cost
  ✗ Higher latency than local storage
```

### Decision 2: Columnar Storage with Z-Order Clustering

**Choice:** Vortex compression + Z-order clustering for analytical workloads

**Rationale:**
- SIMD acceleration for analytical queries
- Modern compression (not 2013-era Parquet)
- Multi-column pruning

**Trade-offs:**
```
PROS:
  ✓ 10-100x better compression
  ✓ SIMD-accelerated filtering
  ✓ Better cache utilization
  ✓ Excellent for OLAP

CONS:
  ✗ Expensive to maintain (Z-order rebuild)
  ✗ Not suitable for OLTP
  ✗ Requires full buffer in memory
  ✗ Incremental inserts break ordering
```

### Decision 3: Local SSD for Spillover

**Choice:** Use local SSD as overflow for temp tables and query spillover

**Rationale:**
- Prevents OOM crashes
- Faster than network spillover
- Fits intermediate result sizes

**Trade-offs:**
```
PROS:
  ✓ Prevents OOM
  ✓ Fast spillover
  ✓ Cheap storage
  ✓ Reduces pressure on global memory

CONS:
  ✗ Loss on node failure (acceptable for temp data)
  ✗ Limited capacity
  ✗ SSD wear and endurance
  ✗ Competes with LFC for I/O
```

### Decision 4: Centralized WAL Server

**Choice:** Single WAL service for all databases

**Rationale:**
- Simplified consistency model
- Efficient replication
- Single point of durability

**Trade-offs:**
```
PROS:
  ✓ Easy to understand
  ✓ Efficient for write-heavy workloads
  ✓ Single quorum to manage
  ✓ Simplified recovery

CONS:
  ✗ Potential bottleneck
  ✗ Single point of failure (mitigated by quorum)
  ✗ Network overhead
  ✗ Dependency on WAL service availability
```

### Decision 5: Cgroups-Based Resource Limiting

**Choice:** Linux cgroups for strict resource isolation

**Rationale:**
- Kernel-level enforcement
- Multi-tenant isolation
- Prevents resource starvation

**Trade-offs:**
```
PROS:
  ✓ Strong isolation
  ✓ Fair resource sharing
  ✓ Prevents noisy neighbors
  ✓ Kernel enforcement

CONS:
  ✗ Complex tuning
  ✗ Difficult to adjust dynamically
  ✗ Memory overhead for tracking
  ✗ Can cause query failures under load
```

---

## IX. ARCHITECTURE DIAGRAMS

### Full System Architecture

```
                    ┌─────────────────────────────────┐
                    │     Azure Control Plane (OCS)    │
                    │  - Config management             │
                    │  - Monitoring & alerting         │
                    │  - Billing & metering            │
                    └─────────────────────────────────┘
                                    │
            ┌───────────────────────┼───────────────────────┐
            │                       │                       │
            ▼                       ▼                       ▼
    ┌───────────────┐      ┌──────────────┐      ┌──────────────┐
    │  Compute Node │      │  Page Server │      │  WAL Server  │
    │               │      │              │      │              │
    │ ┌───────────┐ │      │ ┌──────────┐ │      │ ┌──────────┐ │
    │ │PostgreSQL │ │      │ │Page Cache│ │      │ │Quorum    │ │
    │ │           │ │      │ │(memory) │ │      │ │Replicas  │ │
    │ ├───────────┤ │      │ ├──────────┤ │      │ │(3x)      │ │
    │ │SB: 4GB    │ │      │ │Local SSD │ │      │ │          │ │
    │ │LFC: 32GB  │ │      │ │(warm)    │ │      │ └──────────┘ │
    │ │Temp: 64GB │ │      │ ├──────────┤ │      │              │
    │ └───────────┘ │      │ │Fabric    │ │      │ Write-Ahead  │
    │               │      │ │(cold)    │ │      │ Log Service  │
    │ Cgroups:      │      │ │OneLake   │ │      │              │
    │ - CPU: 8c     │      │ └──────────┘ │      │ Durability:  │
    │ - Mem: 64GB   │      │              │      │ - Sync write │
    │ - I/O: limit  │      │ 3-tier       │      │ - Quorum     │
    │               │      │ storage      │      │ - RPO: 0     │
    └───────────────┘      └──────────────┘      └──────────────┘
        │       │              │       │              │      │
        │ gRPC  │              │ gRPC  │              │      │
        │(GetPage)             │(Append)              │      │
        └───────┼──────────────┼───────┬──────────────┘      │
                │              │       │                      │
                └──────────────┼───────┴──────────────────────┘
                               │
                    ┌──────────▼───────────┐
                    │  Service Fabric      │
                    │  - Orchestration     │
                    │  - Health monitoring │
                    │  - Failover mgmt     │
                    └──────────────────────┘
```

### Data Flow for Query Execution

```
┌─────────────────────────────────────────────────────────────┐
│                                                               │
│  Query: SELECT * FROM t WHERE col1 > 100 AND col2 < 50     │
│                                                               │
└─────────────────┬───────────────────────────────────────────┘
                  │
                  ▼
        ┌────────────────────┐
        │ PostgreSQL Parser  │
        │ + Planner          │
        └────────────────────┘
                  │
                  ▼
        ┌────────────────────┐
        │ Columnar Planner   │
        │ Hook               │
        │                    │
        │ Is columnar index? │─ NO → Use standard execution
        └────────────────────┘
                  │ YES
                  ▼
        ┌────────────────────────────────────────┐
        │ Convert to DataFusion LogicalPlan      │
        │ - IndexOnlyScan (columnar index)       │
        │ - Filter(col1 > 100 AND col2 < 50)    │
        │ - Project(all columns)                 │
        └────────────────────────────────────────┘
                  │
                  ▼
        ┌────────────────────────────────────────┐
        │ CustomScan Executor                    │
        │                                         │
        │ For each row group:                    │
        │  1. Check min/max stats → prune?       │
        │  2. Fetch Vortex columns from pages    │
        │  3. Stitch into contiguous arrays      │
        │  4. Hand off to worker threads         │
        └────────────────────────────────────────┘
                  │
      ┌───────────┴───────────┐
      │                       │
      ▼                       ▼
   Worker1              Worker2
   ┌────────────────┐   ┌────────────────┐
   │ SIMD Filter    │   │ SIMD Filter    │
   │ col1 > 100     │   │ col1 > 100     │
   │ AND            │   │ AND            │
   │ col2 < 50      │   │ col2 < 50      │
   │                │   │                │
   │ Boolean mask   │   │ Boolean mask   │
   └────────────────┘   └────────────────┘
      │                       │
      └───────────┬───────────┘
                  ▼
        ┌────────────────────┐
        │ Merge result sets  │
        │ from all workers   │
        └────────────────────┘
                  │
                  ▼
        ┌────────────────────┐
        │ Convert to Arrow   │
        │ RecordBatch        │
        └────────────────────┘
                  │
                  ▼
        ┌────────────────────┐
        │ PostgreSQL Executor│
        │ (process results)  │
        └────────────────────┘
                  │
                  ▼
            ┌──────────┐
            │ Results  │
            └──────────┘
```

---

## X. PERFORMANCE CHARACTERISTICS

### Typical Latencies

| Operation | Latency | Notes |
|-----------|---------|-------|
| Shared Buffer hit | <100µs | In-memory access |
| LFC hit | 1-5ms | SSD read |
| PageServer hit | 5-20ms | Network + processing |
| Fabric/Cold hit | 50-200ms | Network + storage |
| OOM Spillover | 10-50ms | Temporary SSD write |

### Throughput

| Operation | Throughput |
|-----------|-----------|
| Columnar filter (SIMD) | 100M+ rows/sec |
| PageServer GetPage | 10K-100K ops/sec |
| WAL server append | 10K-100K ops/sec |
| LFC reads | 1GB/sec (SSD) |

### Scalability

| Dimension | Limit | Mechanism |
|-----------|-------|-----------|
| Database size | 100TB+ | Fabric storage |
| Concurrent queries | Configurable | Cgroups + OCS |
| Replication lag | <100ms | WAL quorum |
| Failover time | <1s | Service Fabric |

---

## XI. RECOMMENDATIONS & FUTURE WORK

### Immediate Priorities (Phase 1 → 2)

1. **Multi-region Support**
   - Replicate WAL across regions
   - Fabric/OneLake cross-region redundancy
   - Geo-disaster recovery

2. **Columnar Index Enhancements**
   - Incremental clustering (Databricks liquid clustering approach)
   - Multi-index support
   - OLTP + OLAP mixed workload optimization

3. **Performance Optimization**
   - Predictive prefetching for LFC
   - Machine learning for cache eviction
   - Query plan optimization for spillover

### Medium-Term (Phase 2 → 3)

1. **Advanced Features**
   - Logical replication support with failover
   - Point-in-time recovery
   - Advanced backup strategies

2. **Operational Excellence**
   - Autonomous tuning via ML
   - Proactive resource scaling
   - Predictive alerting

3. **Cost Optimization**
   - Dynamic tier management
   - Compression-aware billing
   - Reserved capacity pricing

---

## XII. CONCLUSION

OrionDB represents a modern, cloud-native approach to PostgreSQL. Key innovations:

- **Disaggregated architecture** for independent scaling
- **Columnar indexes** with SIMD for analytical workloads
- **Intelligent caching** with multi-tier storage
- **Strict resource isolation** via cgroups for multi-tenancy
- **High availability** through quorum-based WAL replication
- **Cost efficiency** through tiered storage and granular billing

The design makes deliberate trade-offs favoring cloud-scale operations, multi-tenancy, and cost efficiency over traditional single-node database assumptions.

