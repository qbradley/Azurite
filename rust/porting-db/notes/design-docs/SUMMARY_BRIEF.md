# WALServer Design Documents - Quick Reference

## File Access Note
**Status**: The Word documents are encrypted with CDFV2 encryption and cannot be read directly with standard tools. This summary is based on:
- File names and timestamps
- Document sizes and relationships
- Implementation code analysis from the Rust codebase
- Available TODOs and metadata

---

## 23 Design Documents Inventory

### WALServer Quorum Protocols (11 docs)
Focus: Distributed consensus, fault tolerance, epoch-based versioning

**Core Components:**
1. **Correctness problems and solutions** - Reconciliation, failover, ring-fence protocols
2. **Replicaset Epoch Management** - Configuration versioning and tracking
3. **Detecting log divergence** - Consistency checking via epochs
4. **WalServer Triggered Reconciliation** - Server-initiated recovery
5. **Replica Seeding** - New replica bootstrapping
6. **Replica Build - Work Items** - Implementation roadmap
7. **Reconciliation + Replica Build** - Recovery coordination
8. **LatencyOptimizedQuorumAlgo** - Performance-tuned consensus (272KB)
9. **WalServer Read Cache** - Read optimization caching
10. **WalServer Full node VM** - Deployment architecture
11. **Basebackup issue with latest LSN** - Backup consistency

### WALServer Memtable Management (3 docs)
Focus: Memory limits, eviction, resource optimization

1. **Memory Management - Hard Limit Design** - Enforced memory thresholds
2. **WAL Server Memory Management** - Overall memory architecture
3. **Memory Architecture Proposal** - Redesign proposal (235KB)

### WAL File Materialization (9 docs)
Focus: Converting logical logs to physical durability

1. **WAL File Materialization** - Core strategy
2. **Replica Build Changes** - Integration with sync
3. **Materialization Rollout Planning** - Deployment phases
4. **Log Retention Sequence Number** - LSN-based retention
5. **Incremental Copy with Dynamic Catchup Target** - Adaptive sync (285KB)
6. **Ensuring Physical Atomicity for File Range Copy** - Atomic operations
7. **Dynamic Catchup Performance Evaluation - V1** - Benchmarks (974KB largest)
8. **Dynamic Catchup Performance Experiment - V2** - Updated results (450KB)
9. **Enforce Single Replica Build in WALServer** - Serialized builds

---

## Key Architecture Decisions

| Area | Design | Trade-off |
|------|--------|-----------|
| **Consensus** | Epoch-based (not Raft) | Simpler but custom protocol |
| **Memory** | Hard limits with backpressure | Predictable, may reject writes |
| **File Ops** | copy_file_range() atomicity | Efficient but Linux-only |
| **Replica Build** | Serialized (single at a time) | Simple but slower recovery |
| **Sync** | Dynamic catchup with moving targets | Adaptive but complex |

---

## Core Data Structures (from code)

```rust
WalServerEpoch         // Versioning for config changes
WalWriterEpochAndInstanceId  // Client identification
Reconcile {            // Consistency protocol
    wal_server_epoch: Option<WalServerEpoch>,
    last_reconciled_seq_no: Option<SequenceNumber>,
    force: Option<ForceReconciliationReason>,
}
MaterializedFileInfo {  // Physical file metadata
    file_info: FileInfo,
    file_header_lba: u64,
    range_id: u32,
    start_offset: u32,
    end_offset: u32,
}
```

---

## Write Path Overview

```
PostgreSQL → WalFS (cache) → WALServer Quorum
  ↓
Primary validates epoch → adds to memtable → sends to F+1 replicas
  ↓
Majority ACK → marked committed at LSN → ACK to WalFS
  ↓
Background: Convert WAL → physical files (copy_file_range) → Azure Blob Storage
  ↓
Async Replication: Replicas pull materialized files with dynamic catchup
```

---

## Consistency Guarantees

- **Write**: Durable when majority quorum acknowledges + epoch fencing prevents stale writes
- **Read**: "Read your own writes" via min_snapshot_seqno with optional wait/long-poll
- **Failover**: New primary has higher epoch, old primary's writes fenced via RingFence protocol

---

## Implementation Status (Aug 2025)

**Completed**: FR protocol, reconciliation PR merged

**In Progress**: 
- Epoch consistency during failover
- WalFS + quorum manager integration
- Service Fabric UpdateEpoch handling
- Async cancellation token support

**Needs Testing**: Epoch change on WalFS during failover, SF config validation

---

## Most Important Documents (Recommended Reading Order)

1. Replicaset Epoch Management → understand versioning
2. Correctness problems & solutions → understand fault tolerance
3. WalServer Triggered Reconciliation → understand recovery
4. Memory Management - Hard Limit Design → understand resource constraints
5. WAL File Materialization → understand storage pipeline
6. Incremental Copy with Dynamic Catchup → understand replication
7. Performance Evaluation V1 & V2 → understand system limits

---

## Critical Challenges

1. **Epoch consistency** - Ensuring all nodes learn new epoch during failover
2. **Memory pressure** - Backpressure vs. rejection trade-off
3. **Dynamic catchup** - "Moving target" correctness problem
4. **Service Fabric** - Coordination with SF epoch updates
5. **Feature rollout** - Backward compatibility with non-materialized format

---

*Summary generated: March 13, 2025*
*Full detailed summary available in: /home/azureuser/DESIGN_DOCUMENTS_SUMMARY.md*
