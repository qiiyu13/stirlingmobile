# ADR-003: qpdf Rewrite in Rust vs NDK Compile

**Status:** Superseded for v1 — see update below. Original decision retained for context.
**Date:** 2026-07-02
**Deciders:** Stirling Mobile team

**Update (solo/AI-orchestrated scope):** v1 ships **Path A directly** (compile `libqpdf` for NDK), decided upfront rather than attempting Path B first. Reasoning: this decision's original bet was that Path B's engineering time (6-8 weeks) would buy architectural purity, but under solo/AI-orchestrated staffing the real cost of Path B isn't the engineering — it's the human-bound work of building and validating a 1000+ PDF benchmark corpus against qpdf's decade of tuning, which doesn't compress with AI. Path A is immediate and known-correct; the Week-22 checkpoint below no longer applies. Path B (Rust-native rewrite) is a possible v1.1+ effort if the C++ dependency proves costly to maintain, but is not the default v1 direction anymore. See R3 in 14-risk-register.md.

## Context

Stirling server uses qpdf (a C++ CLI/library) for PDF optimization: linearization, object stream compression, unused object removal, and cross-reference table optimization. For mobile, we have two paths: compile qpdf for Android NDK, or rewrite its optimization logic in Rust with direct access to lopdf's PDF model.

## Decision

Write the **optimization logic in Rust** using lopdf's internal PDF representation (Path B). Keep compiling libqpdf for Android NDK as an **emergency fallback** (Path A).

## Options Considered

### Option A: Compile libqpdf for Android NDK
- **Pros**: 100% output parity with Stirling server. Proven optimization (10 years of tuning). Immediate — just compile and wrap.
- **Cons**: Adds C++ build dependency. JNI bridge to C++. Two PDF representations (lopdf for other tools, qpdf for optimization — context switching). C++ toolchain complexity for iOS later.

### Option B: Rewrite optimization in Rust (lopdf-native)
- **Pros**: Single PDF representation (lopdf). No C++ dependency. Better integration — optimizer can see lopdf's internal structure. iOS-compatible from day 1. Full control over optimization pipeline.
- **Cons**: Must re-implement 10 years of qpdf tuning. Risk of missing edge cases. Initial version may produce larger files than qpdf.

## Decision Rationale

Path B wins because:
1. Architectural purity — one PDF representation, one language
2. iOS path — qpdf C++ also compiles for iOS, but Rust path is cleaner
3. Integration — direct lopdf access means optimizer can make smarter decisions (e.g., deduplicate across objects using lopdf's internal reference tracking)
4. We can measure: `make benchmark` compares output size against qpdf. If our optimizer is within 5% of qpdf's output size, we're good. If not, Path A is ready.

## Consequences

### Positive
- Single-language stack (Rust) for all PDF operations
- Direct lopdf integration enables deeper optimizations
- No C++ build chain to maintain
- iOS-compatible without extra work

### Negative
- 6-8 weeks of engineering to match qpdf's optimization quality
- First release may produce slightly larger files than qpdf
- Must build corpus of test PDFs to validate optimization quality

### Success Criteria
- Output size within 5% of qpdf for 90% of test corpus
- Never produces invalid PDF (validation test)
- Optimization time within 2x of qpdf (acceptable given mobile CPU constraints)

### Fallback (original plan, no longer active for v1)
If Path B cannot meet success criteria by end of Phase 5 (Week 22), we compile libqpdf for Android NDK as a bridge (1-2 weeks to integrate) and continue improving the Rust optimizer for v1.1. **Superseded**: v1 now starts with Path A directly (see Update at top) — this checkpoint doesn't appear in 11-roadmap.md anymore.
