# Ceremonies — Azurite TS→Rust Port

## Porting Review

| Field | Value |
|-------|-------|
| **Trigger** | auto |
| **When** | after |
| **Condition** | Rust file committed for a TS source file |
| **Facilitator** | Gandalf |
| **Participants** | Faramir, Aragorn, Samwise (if API-facing), Boromir |
| **Time budget** | focused |
| **Enabled** | ✅ yes |

**Agenda:**
1. Verify TS fidelity — does the Rust closely mirror the TS?
2. Check porting-db entry is complete and accurate
3. Identify any deviations and document rationale
4. Approve or reject

---

## Architecture Decision

| Field | Value |
|-------|-------|
| **Trigger** | auto |
| **When** | before |
| **Condition** | New module or subsystem being ported |
| **Facilitator** | Gandalf |
| **Participants** | Faramir, Aragorn, Samwise |
| **Time budget** | focused |
| **Enabled** | ✅ yes |

**Agenda:**
1. Review the TS module structure and dependencies
2. Decide translation strategy (type mappings, async patterns, error handling)
3. Record decisions in porting-db
4. Assign implementation work

---

## Retrospective

| Field | Value |
|-------|-------|
| **Trigger** | auto |
| **When** | after |
| **Condition** | build failure, test failure, or reviewer rejection |
| **Facilitator** | Gandalf |
| **Participants** | all-involved |
| **Time budget** | focused |
| **Enabled** | ✅ yes |

**Agenda:**
1. What happened? (facts only)
2. Root cause analysis
3. What should change in the porting process?
4. Action items for next iteration
