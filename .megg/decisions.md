---
created: 2025-12-15T09:58:30.000Z
updated: 2025-12-15T09:58:30.000Z
type: memory
---
# Architectural Decisions

## 2024-12-15: Frontend Asset Embedding

### Context
The project goal is a single deployable binary. Initial implementation used `ServeDir::new("frontend/dist")` which looks for files at runtime relative to the current working directory.

### Problem
When the binary is moved or run from a different location (e.g., `./target/release/steering-center`), it can't find `frontend/dist/` and serves a blank white page. The build script masked this by always running from project root.

### Decision
Use `rust-embed` to embed frontend assets into the binary at compile time.

### Consequences
- **Build order matters**: Frontend must be built BEFORE `cargo build` since assets are embedded at compile time
- **Binary size increases**: All frontend assets are bundled into the executable
- **True portability**: Binary can be copied anywhere and run without external dependencies (except SQLite db file)
- **Development workflow changes**: Need to rebuild Rust after frontend changes to see updates in release binary

### Alternatives Considered
1. **Require specific working directory** - Rejected: fragile, bad UX
2. **Config file for asset path** - Rejected: adds deployment complexity, defeats "single binary" goal
3. **Embed assets with `include_dir!`** - Viable but `rust-embed` has better ergonomics and mime type handling

### Status
Implementing `rust-embed` solution.