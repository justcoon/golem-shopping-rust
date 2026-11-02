# Feature Implementation Walkthrough: [Feature Title]

- **Feature ID / Slug**: `[feature_name_or_id]`
- **Date**: [YYYY-MM-DD]
- **Status**: Ready for Review <!-- Ready for Review | Approved -->

---

## 1. Executive Summary

Brief summary of what was implemented, how it satisfies the original feature plan, and overall status.

---

## 2. Changes Implemented

### File Modifications

| Action     | File Path                                          | Summary of Changes     |
| :--------- | :------------------------------------------------- | :--------------------- |
| `[NEW]`    | [`path/to/file.rs`](../../path/to/file.rs)         | Implementation summary |
| `[MODIFY]` | [`path/to/existing.rs`](../../path/to/existing.rs) | Implementation summary |
| `[DELETE]` | [`path/to/old.rs`](../../path/to/old.rs)           | Implementation summary |

### Key Logic & API Changes

- Detailed highlights of agents, `#[agent_definition]` traits, Schema types, methods, or workflows added or modified.

---

## 3. Verification & Validation Results

### 3.1 Code Formatting

Command executed:

```bash
cargo fmt --check
```

**Result**:

```text
[Output confirming all files match formatting standards]
```

### 3.2 Compilation & Type Checking

Command executed:

```bash
cargo check --target wasm32-wasip2
```

**Result**:

```text
[Output confirming zero compilation or type errors]
```

### 3.3 Linter Verification (Clippy)

Command executed:

```bash
cargo clippy --target wasm32-wasip2
```

**Result**:

```text
[Output confirming zero clippy warnings or errors]
```

### 3.4 Golem Build

Command executed:

```bash
golem build --yes
```

**Result**:

```text
[Output confirming successful build of WASM component and metadata]
```

### 3.5 Automated Unit & Integration Tests

Command executed:

```bash
cargo test
```

**Result**:

```text
[Test runner output showing passed unit & integration tests]
```

### 3.6 Runtime / CLI Smoke Testing

Command executed:

```bash
golem agent invoke '<agent_name>(...)' '<component>/<agent_name>.{<method>}' '<args>'
```

Or HTTP endpoint check:

```bash
curl -s http://localhost:9006/<endpoint>
```

**Result**:

```text
[Output or response verifying runtime behavior]
```

---

## 4. How to Run & Verify

Instructions for the user or reviewer to test the feature themselves:

1. Build or deploy the application:
   ```bash
   golem build --yes
   ./deploy.sh # or golem deploy --yes
   ```
2. Agent invocations or HTTP API requests:
   ```bash
   golem agent invoke ...
   curl http://localhost:9006/...
   ```

---

## 5. Sign-off / Next Steps

- Awaiting user approval.
- Follow-up recommendations or future enhancements.
