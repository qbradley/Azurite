# FASTER Rust Crate: Complete Analysis Documentation

This directory contains comprehensive documentation of the **pending I/O**, **hash index**, and **session state** subsystems in the FASTER Rust crate.

## 📋 Documentation Files

### 1. **FASTER_subsystems_analysis.md** (27 KB, 779 lines)
**👉 START HERE for comprehensive understanding**

Complete technical analysis covering:
- **Section 1:** Status/OperationStatus subsystem (why there's no "status array")
- **Section 2:** Pending I/O Manager — full workflow, data structures, Arc patterns
- **Section 3:** Hash Index subsystem — bucket layout, entry packing, overflow chains
- **Section 4:** Garbage collection & maintenance — invalidation, cleanup operations
- **Section 5:** Lock & atomic patterns — ordering rationales
- **Section 6:** Store & session modules — architecture overview
- **Section 7:** Scaling characteristics table
- **Section 8:** Atomic ordering reference
- **Section 9:** File reference guide
- **Section 10:** Key takeaways

**Best for:** Deep technical understanding of subsystems and their interactions

---

### 2. **FASTER_STRUCTURES_DETAILED.md** (19 KB, 611 lines)
**👉 USE THIS for code structure reference**

Line-by-line code documentation with exact file:line references:
- Quick file index table
- Complete struct definitions with field explanations
- Method signatures and implementations
- Size calculations
- Bit layout diagrams

**Structures covered:**
- OperationStatus enum
- PendingOperation<F>
- PendingIoContext<F>
- PendingIoManager
- FasterSession<F>
- HashBucketEntry (64-bit packed)
- AtomicHashBucketEntry
- HashBucket (cache-line layout)
- HashTable
- HashIndex

**Best for:** Understanding exact data structure layouts and code locations

---

### 3. **FASTER_quick_reference.txt** (14 KB, 330 lines)
**👉 USE THIS as a cheat sheet**

Quick-access reference with:
- File locations and sizes
- Section summaries (one per subsystem)
- Data structure sizes
- Key code locations (file:line)
- Scaling table
- Critical code locations
- Key takeaways

**Best for:** Quick lookup during development or code review

---

### 4. **Legacy Analysis Files** (existing documentation)
- `FASTER_CODEBASE_ANALYSIS.md` (22 KB) — Earlier codebase overview
- `FASTER_QUICK_REFERENCE.md` (9.8 KB) — Earlier quick reference
- `FASTER_TEST_EXAMPLES.md` (14 KB) — Test code examples

---

## 🎯 Quick Navigation by Topic

### Status/Operations
- **What is a "status array"?** → See Section 1 of `FASTER_subsystems_analysis.md`
- **OperationStatus struct** → See `FASTER_STRUCTURES_DETAILED.md` Section 1

### Pending I/O
- **How pending I/O works** → See Section 2 of `FASTER_subsystems_analysis.md`
- **PendingOperation struct** → See `FASTER_STRUCTURES_DETAILED.md` Section 3
- **PendingIoContext struct** → See `FASTER_STRUCTURES_DETAILED.md` Section 4
- **PendingIoManager** → See `FASTER_STRUCTURES_DETAILED.md` Section 5
- **Session pending queues** → See `FASTER_STRUCTURES_DETAILED.md` Section 6
- **O(n) operations** → See `FASTER_subsystems_analysis.md` Section 2 or Table in Section 7

### Hash Index
- **Hash bucket design** → See Section 3 of `FASTER_subsystems_analysis.md`
- **64-bit entry packing** → See `FASTER_STRUCTURES_DETAILED.md` Section 7
- **Bucket cache-line layout** → See `FASTER_STRUCTURES_DETAILED.md` Section 8
- **Two-phase insert protocol** → See `FASTER_subsystems_analysis.md` Section 3.1
- **Overflow chains** → See `FASTER_subsystems_analysis.md` Section 3.3

### GC & Maintenance
- **invalidate_entries_in_range()** → See Section 4.1 of `FASTER_subsystems_analysis.md`
- **cleanup_tentative_entries()** → See Section 4.2 of `FASTER_subsystems_analysis.md` (⚠️ UNSAFE to call concurrently)

### Atomics & Synchronization
- **Atomic ordering patterns** → See Section 5 & 8 of `FASTER_subsystems_analysis.md`
- **Lock patterns** → See `FASTER_quick_reference.txt` Section 5

### Scaling & Performance
- **O(n) operations** → See Section 7 of `FASTER_subsystems_analysis.md` (Scaling Table)
- **Data structure sizes** → See `FASTER_STRUCTURES_DETAILED.md` Summary table

---

## 📍 Key File Locations

All paths relative to: `/home/azureuser/FASTER/rust/crates/faster-core/src/`

| Component | File | Key Lines |
|-----------|------|-----------|
| **Status** | `status.rs` | 64-140 |
| **Pending Ops** | `store/session.rs` | 135-450 |
| **I/O Manager** | `store/pending_io.rs` | 1-1061 |
| **Hash Buckets** | `hash/bucket.rs` | 100-550 |
| **Hash Table** | `hash/table.rs` | 103-150 |
| **Hash Index** | `hash/index.rs` | 103-603 |
| **Session** | `store/session.rs` | 190-450 |
| **Store** | `store/kv.rs` | 1-100 |

---

## 🔍 Finding Specific Information

### "Where is the status array?"
→ **There is no status array.** See: `FASTER_subsystems_analysis.md` Section 1

### "How do pending I/O requests get tracked?"
→ See: `FASTER_subsystems_analysis.md` Section 2 (Data Structures 2.1-2.4)

### "What's the exact bit layout of a hash bucket entry?"
→ See: `FASTER_STRUCTURES_DETAILED.md` Section 6 or `FASTER_quick_reference.txt` Section 3

### "Which operations scale as O(n)?"
→ See: `FASTER_subsystems_analysis.md` Section 7 (Scaling Table)

### "How do I poll for completed I/O?"
→ See: `FASTER_STRUCTURES_DETAILED.md` Section 6 (take_completed_io method)

### "What atomic ordering is used?"
→ See: `FASTER_subsystems_analysis.md` Section 5 & 8

### "Is cleanup_tentative safe to call anytime?"
→ **NO.** See: `FASTER_subsystems_analysis.md` Section 4.2 (⚠️ CRITICAL WARNING)

---

## 📊 High-Level Summary

### Three Main Subsystems:

#### 1. **Status/Operations** (`status.rs`)
- `OperationStatus` enum returned directly
- No array storage
- Pending ops tracked in session `Vec<PendingOperation<F>>`

#### 2. **Pending I/O** (`store/pending_io.rs`)
- Two-level queue: `pending_ops` → `io_contexts`
- Arc-shared completion flags between device callback and poller
- O(n) polling on `take_completed_io()`
- Sector-aligned buffers from pool

#### 3. **Hash Index** (`hash/bucket.rs`, `hash/table.rs`, `hash/index.rs`)
- Latch-free with atomic CAS on 64-bit entries
- 7 entries + overflow pointer per 64-byte bucket
- Two-phase insert with tentative bit
- O(n) full-table scans for GC (invalidate, cleanup)

---

## 🔑 Key Takeaways

1. **No "status_array"** — Status returned directly, pending ops in session `Vec`s
2. **Two-level pending queue** — `pending_ops` (un-dispatched) + `io_contexts` (in-flight)
3. **Arc-shared completion state** — `AtomicBool` + `Mutex<IoStatus>` + `AtomicU32`
4. **Completely lock-free hash** — CAS-based inserts with tentative/commit phases
5. **Full-table GC scans** — O(n buckets × overflow chains) for invalidation/cleanup
6. **AcqRel ordering on CAS** — Acquire prior state, Release new entry

---

## 📚 Document Relationships

```
Quick Overview
    ↓
FASTER_subsystems_analysis.md (comprehensive 10-section analysis)
    ├─ Understanding subsystem architecture
    ├─ Learning how pending I/O works
    ├─ Understanding hash index design
    └─ Learning atomic patterns
    ↓
FASTER_STRUCTURES_DETAILED.md (code-level reference)
    ├─ Exact struct definitions
    ├─ Field-by-field explanations
    ├─ Code locations (file:line)
    └─ Size calculations
    ↓
FASTER_quick_reference.txt (cheat sheet)
    ├─ Quick lookups
    ├─ File locations
    └─ Scaling characteristics
```

---

## ✅ Verification Checklist

Use this checklist to verify you understand the subsystems:

- [ ] **Status:** Understand why there's no status array (it's returned directly)
- [ ] **Pending Ops:** Know the flow from `Pending` status → pending_ops → I/O → io_contexts → CompletedIo
- [ ] **Queues:** Understand `pending_ops` is un-dispatched, `io_contexts` is in-flight
- [ ] **Arc Sharing:** Know how completion flag is shared between callback and poller
- [ ] **Hash Bucket:** Understand 64-byte layout with 7 entries + overflow
- [ ] **Bit Packing:** Know address (48 bits) + tag (14 bits) + tentative (1 bit)
- [ ] **Two-Phase Insert:** Understand tentative bit workflow
- [ ] **GC:** Know full-table scans are O(n) (invalidate, cleanup)
- [ ] **Cleanup Warning:** Understand cleanup_tentative is recovery-only
- [ ] **Atomics:** Know AcqRel on CAS, Acquire on loads, Release on callbacks

---

## 📞 Cross-References

**For details on:**
- Epoch-based reclamation → See `hash/index.rs` and epoch module docs
- Record layout & serialization → See `record.rs` module
- Hybrid log management → See `hybrid_log/` module docs
- Buffer pooling → See `buffer_pool.rs` module
- Device I/O interface → See `device.rs` module

---

**Last Updated:** 2024-03-16  
**Crate Version:** Analyzed from master branch  
**Base Path:** `/home/azureuser/FASTER/rust/crates/faster-core/src/`
