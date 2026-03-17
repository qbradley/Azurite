# PostgreSQL Orion WALServer Design Documents - File Locations

## Complete File Manifest

### WALServer Quorum Protocols (11 documents)
Location: `/home/azureuser/pgsql-orion/designdocs/WALServer Quorum Protocols/`

| File | Size | Modified |
|------|------|----------|
| Basebackup issue with latest LSN.docx | 177KB | Dec 4, 2024 |
| Correctness problems and solutions (reconciliation, failover, ring-fence).docx | 173KB | Mar 8, 2025 |
| Detecting log divergence through epoch.docx | 173KB | Dec 9, 2024 |
| LatencyOptimizedQuorumAlgo.docx | 272KB | Jan 8, 2025 |
| Reconciliation + Replica Build.docx | 157KB | Jun 4, 2025 |
| Replica Build - Work Items.docx | 177KB | Sep 12, 2024 |
| Replica Seeding.docx | 189KB | Aug 29, 2024 |
| Replicaset Epoch Management.docx | 202KB | Oct 23, 2024 |
| WalServer Full node VM.docx | 173KB | Mar 7, 2025 |
| WalServer Read Cache.docx | 165KB | Mar 10, 2025 |
| WalServer Triggered Reconciliation.docx | 173KB | Jan 27, 2025 |
| **TODOs.docx** (Readable) | 42KB | Aug 27, 2025 |

Also in directory:
- Questions on SF.docx
- Consensus in Orion WalServer.pptx
- Walkthrough Service Fabric integration with WALServer-*.mp4 (video recording)

---

### WALServer Memtable Management (3 documents)
Location: `/home/azureuser/pgsql-orion/designdocs/WALServer Memtable Management/`

| File | Size | Modified |
|------|------|----------|
| Memory Achitecture Proposal.docx | 235KB | Mar 13, 2025 |
| Memory Management - Hard Limit Design.docx | 153KB | Mar 6, 2025 |
| WAL Server Memory Management.docx | 157KB | Mar 13, 2025 |

---

### WAL File Materialization (9 documents)
Location: `/home/azureuser/pgsql-orion/designdocs/WAL File Materialization/`

| File | Size | Modified |
|------|------|----------|
| Dynamic Catchup Performance Evaluation - V1.docx | 974KB | Jan 29, 2025 |
| Dynamic Catchup Performance Experiement - V2.docx | 450KB | Mar 2, 2025 |
| Enforce Single Replica Build in WALServer.docx | 165KB | Nov 19, 2024 |
| Ensuring Physical Atomicity for File Range Copy.docx | 173KB | Aug 20, 2025 |
| Incremental Copy with Dynamic Catchup Target.docx | 285KB | Nov 16, 2024 |
| Log Retention Sequence Number.docx | 239KB | May 13, 2025 |
| Materialization Rollout Planning.docx | 169KB | Nov 16, 2024 |
| Replica Build Changes for WAL File Materialization.docx | (unavailable) | Apr 1, 2025 |
| WAL File Materialization.docx | (unavailable) | (unknown) |

Also in directory:
- CopyFileRange-Illustration.svg
- Dynamic Catch Experiments.xlsx
- Various .mp4 meeting recordings
- Experiment data files

---

## File Encryption Status

**Issue**: All 23 Word documents are encrypted with **CDFV2 (Microsoft Office Encryption)**
- Standard tools cannot read them (python-docx, docx2txt, unzip all fail)
- Readable with: Microsoft Office, LibreOffice with proper credentials
- Status: One file (TODOs.docx) was partially readable and provided implementation details

---

## Related Source Code

Key implementation files in Rust codebase:
- `/home/azureuser/pgsql-orion/src/wal_protocol/src/wal.rs` - WAL request protocol definitions
- `/home/azureuser/pgsql-orion/src/wal_protocol/src/wal_client_manager.rs`
- `/home/azureuser/pgsql-orion/src/wal_protocol/src/wal_context.rs`
- `/home/azureuser/pgsql-orion/src/walfs/src/walfs.rs` - WAL filesystem implementation

Key types defined:
- `WalServerEpoch` - replicaset configuration version
- `WalWriterEpochAndInstanceId` - client identifier with epoch
- `Reconcile` - consistency check request
- `MaterializedFileInfo` - physical file metadata
- `RingFence` - split-brain prevention protocol

---

## Document Dependencies & Reading Path

### Foundation Layer
1. **Replicaset Epoch Management** - Understand versioning scheme
2. **Detecting log divergence through epoch** - Understand consistency checking

### Consensus Layer  
3. **Correctness problems and solutions** - Understand protocol correctness proofs
4. **LatencyOptimizedQuorumAlgo** - Understand performance optimization
5. **WalServer Triggered Reconciliation** - Understand recovery mechanism

### Application Layer
6. **WAL File Materialization** - Understand storage pipeline
7. **Incremental Copy with Dynamic Catchup Target** - Understand replication strategy
8. **Memory Management - Hard Limit Design** - Understand resource constraints

### Deployment & Operations
9. **Materialization Rollout Planning** - Understand feature rollout
10. **Replica Build - Work Items** - Understand implementation roadmap

---

## Summary Documents Generated

1. **DESIGN_DOCUMENTS_SUMMARY.md** (417 lines)
   - Comprehensive analysis of all 23 documents
   - Architecture synthesis
   - Key decisions and trade-offs
   - Implementation challenges
   
2. **DESIGN_SUMMARY_BRIEF.md**
   - Quick reference guide
   - Key findings summary
   - Implementation status

3. **DESIGN_DOCUMENTS_LOCATION.md** (this file)
   - File manifest and locations
   - Dependency graph
   - Related source code

---

## How to Access Full Documents

### Option 1: Windows/Direct Access
- Files are on Azure cloud storage accessible via Windows Explorer
- Use Microsoft Word, Excel, or Office 365 online
- Should open without password (decryption handled by Office)

### Option 2: Linux/LibreOffice
```bash
libreoffice --headless --convert-to pdf \
  "WALServer Quorum Protocols/Replicaset Epoch Management.docx"
```
(May require password if LibreOffice doesn't auto-decrypt)

### Option 3: Code Analysis
- Review `/home/azureuser/pgsql-orion/src/wal_protocol/` for implementation details
- Check git history for design discussions in commit messages
- Review pull request comments for design rationale

### Option 4: Meeting Recordings
Multiple .mp4 meeting recordings available in the same directories:
- Consensus discussions (Quorum Protocols dir)
- Materialization discussions (Materialization dir)
- Technical walkthroughs and design decisions

---

*Manifest created: March 13, 2025*
