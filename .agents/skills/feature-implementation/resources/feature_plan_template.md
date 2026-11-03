# Feature Plan: [Feature Title]

- **Feature ID / Slug**: `[feature_name_or_id]`
- **Date**: [YYYY-MM-DD]
- **Status**: Draft <!-- Draft | Approved | In Progress | Completed -->

---

## 1. Overview & Goals

Provide a clear description of the feature, business context, problem being solved, and success criteria.

### Goals

- Goal 1
- Goal 2

### Non-Goals

- What is intentionally out of scope.

---

## 2. Architecture & Technical Design

Explain the technical approach, architectural considerations, and integrations (e.g. Golem agent contracts, `#[agent_definition]` traits, Schema types, state persistence, HTTP/MCP endpoints, external HTTP or DB integration).

### Key Decisions

- Decision 1: Rationale
- Decision 2: Rationale

---

## 3. Proposed Changes & File Impact

List all files that will be created, modified, or removed. Group by component or module.

| Action     | File Path                                          | Description                 |
| :--------- | :------------------------------------------------- | :-------------------------- |
| `[NEW]`    | [`path/to/file.rs`](../../path/to/file.rs)         | Purpose of new file         |
| `[MODIFY]` | [`path/to/existing.rs`](../../path/to/existing.rs) | Specific changes to be made |
| `[DELETE]` | [`path/to/old.rs`](../../path/to/old.rs)           | Reason for removal          |

### Detailed Changes

#### [Component / Module Name]

- **`[path/to/file.rs]`**:
  - Detailed description of structs, traits, methods, or logic changes.

---

## 4. Verification Plan

### Automated Checks

- **Code Formatting**:
  ```bash
  cargo fmt --check
  ```
- **Compilation & Type Check**:
  ```bash
  cargo check --target wasm32-wasip2
  ```
- **Linter (Clippy)**:
  ```bash
  cargo clippy --target wasm32-wasip2
  ```
- **Golem Component Build**:
  ```bash
  golem build --yes
  ```
- **Automated Unit & Integration Tests**:
  ```bash
  cargo test
  ```

### Manual / Integration Verification

- Steps to deploy, invoke agents, or verify runtime behaviors:
  ```bash
  golem agent invoke '<agent_name>(...)' '<component>/<agent_name>.{<method>}' '<args>'
  ```
- HTTP API checks:
  ```bash
  curl -s http://localhost:9006/<endpoint>
  ```

---

## 5. Risks & Open Questions

- **Risk 1**: Mitigation strategy
- **Open Question 1**: Awaiting user input
