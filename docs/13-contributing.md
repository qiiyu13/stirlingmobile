# Stirling Mobile — Contributing

> **Status:** Draft v1.0

---

## 1. Code of Conduct

Be professional. Be direct. No drama.

---

## 2. Branch Strategy

```
main          ← Production (Play Store releases)
  ├─ develop  ← Integration branch
  │   ├─ feat/merge-tool
  │   ├─ feat/rust-security
  │   ├─ fix/oom-large-pdf
  │   └─ chore/update-tesseract
  └─ release/v1.0
```

- `main` is protected — only merged from `release/*` branches
- `develop` is the active development branch
- Feature branches branch from `develop`, merge back via PR
- Release branches branch from `develop`, merge to `main` after testing

---

## 3. Commit Convention

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

| Type | Use for |
|---|---|
| `feat` | New feature (tool, screen, capability) |
| `fix` | Bug fix |
| `refactor` | Code restructuring, no behavior change |
| `perf` | Performance improvement |
| `test` | Adding or fixing tests |
| `docs` | Documentation changes |
| `chore` | Build, CI, dependency updates |
| `style` | Formatting, linting, no logic change |

**Scopes:** `rust`, `kotlin`, `ui`, `jni`, `docs`, `ci`, `ocr`, `office`

Examples:
```
feat(rust): implement pages_merge with parallelism
fix(kotlin): prevent OOM on PDF >500MB by using file path transfer
perf(rust): 40% faster jpeg recompression in optimize_compress
test(rust): add property-based tests for split/merge roundtrip
docs: add architecture decision record for Collabora SDK
```

---

## 4. Pull Request Process

### Before opening a PR:

- [ ] All tests pass: `make check`
- [ ] No lint warnings: `make lint-engine && make lint-kotlin`
- [ ] New code has tests (unit for Rust, ViewModel tests for Kotlin)
- [ ] No commented-out code, no TODO without tracking issue
- [ ] Commit messages follow convention above
- [ ] Branch is rebased on `develop` (no merge commits)

### PR template:

```markdown
## What
Brief description of the change.

## Why
Why this change is needed. Reference issue # if applicable.

## Testing
- [ ] `make test-engine` passes
- [ ] `make test-kotlin` passes
- [ ] `make test-android` passes (or manual test steps if emulator unavailable)
- [ ] Tested on device: [model]

## Screenshots
If UI change, attach before/after screenshots.

## Checklist
- [ ] No new dependencies without ADR and license check
- [ ] Performance impact assessed (benchmark if >10% change)
- [ ] APK size impact checked (`make build-release && ls -lh apk`)
```

### Review requirements:

Solo/AI-orchestrated project (11-roadmap.md) — there's no second developer to approve. Self-review replaces peer review, so it has to be deliberate rather than a rubber stamp:

- Human review of every PR before merge, even AI-authored ones — especially crypto/signing code (09-security.md) and anything touching `unsafe`
- All CI checks green
- No unresolved review threads (self-authored TODOs count)
- Reviewer checks: correctness, performance, security, test coverage — same bar as a second developer would apply, just applied by the same human who's orchestrating

---

## 5. Code Style

### Rust

- Follow `rustfmt` defaults (enforced by CI)
- `cargo clippy -- -D warnings` must pass (enforced by CI)
- `#![forbid(unsafe_code)]` in all modules except `jni_bridge.rs`
- JNI functions: `#[no_mangle] pub extern "system" fn Java_com_stirlingmobile_engine_PdfEngine_*`
- Error handling: never `unwrap()` in production code, use `?` and custom error types
- Naming: `snake_case` for functions, `CamelCase` for types, descriptive not clever
- Comments: only when code is non-obvious, never for "what", sometimes for "why"

### Kotlin

- Follow `ktlint` defaults (enforced by CI)
- Compose: `@Composable` functions use `PascalCase`, emit `Unit`
- ViewModels: one per screen, expose `StateFlow<UiState>`, never expose `MutableStateFlow`
- Naming: descriptive, not abbreviated (`fileRepository` not `fileRepo`)
- Comments: same rule as Rust

---

## 6. Adding a New Tool

1. **Rust side** (`engine/src/ops/`):
   - Add function to appropriate module
   - Write unit tests (minimum: happy path + 2 error cases)
   - Add to `jni_bridge.rs` with proper error mapping

2. **Kotlin side** (`app/src/main/java/com/stirlingmobile/`):
   - Add JNI wrapper function to `PdfEngine.kt`
   - Create `ui/screens/tools/{ToolName}Screen.kt`
   - Create `viewmodel/{ToolName}ViewModel.kt`
   - Add ViewModel tests
   - Register in navigation and tool picker

3. **Documentation**:
   - Update `04-feature-catalog.md` status
   - Add tool description string to `values/strings.xml`

4. **Testing**:
   - Verify output matches Stirling server for same input
   - Test with 1-page, 100-page, corrupted, and password-protected PDFs

---

## 7. Dependency Policy

- **No new runtime dependencies without ADR** — write an Architecture Decision Record justifying the choice
- **License check** — must be MIT, Apache 2.0, BSD, MPL 2.0, ISC, or similar permissive. No GPL/LGPL/AGPL.
- **Size check** — estimate APK size impact. If > 2MB, needs explicit approval.
- **Security audit** — `cargo audit` / `gradle dependencyCheck` must pass

---

## 8. Release Process

1. Create release branch: `git checkout -b release/v1.x.0 develop`
2. Bump version in `app/build.gradle.kts` and `engine/Cargo.toml`
3. Update `docs/04-feature-catalog.md` with final status
4. Run full test suite: `make check`
5. Build release APK: `make build-release`
6. Manual QA on 3 devices (see `08-testing-strategy.md`)
7. Merge to `main`: `git checkout main && git merge release/v1.x.0`
8. Tag release: `git tag v1.x.0`
9. Push: `git push origin main --tags`
10. Upload to Play Store internal testing track
