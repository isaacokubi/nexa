# Averqun Concurrency (Design)

Averqun's concurrency model is intended to make data races impossible in safe code while retaining native scheduling and low overhead. This document is aspirational; the bootstrap interpreter is single-threaded.

## Proposed primitives
- `spawn`: create a task with explicit ownership transfer.
- `await`: suspend until a task result is available.
- `Channel<T>`: typed message passing.
- `Mutex<T>` / `RwLock<T>`: guarded shared state.
- `Atomic<T>`: explicit atomic operations with documented memory ordering.
- Structured task scopes: child tasks cannot silently outlive their parent scope.

## Safety rules
Mutable state is either exclusively owned, protected by synchronization, or accessed atomically. Sending a value across task boundaries requires a Send-like capability. Shared references across threads require a Sync-like capability. Cancellation and failure propagation must be explicit.

## Runtime strategy
The standard library may offer a lightweight work-stealing executor and an async I/O reactor, but runtime choice must not be embedded in language syntax. Blocking operations should be distinguishable from cooperative tasks.

## Required validation
Race-focused tests, cancellation tests, scheduler stress tests, deadlock diagnostics, deterministic test scheduling, and benchmarks are release blockers.
