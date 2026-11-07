# Averqun Memory and Ownership Model (Design)

## Safety objective
Safe Averqun code must not permit use-after-free, double-free, dangling references, data races, or invalid pointer dereferences. This is a design specification, not an implemented borrow checker.

## Ownership
Every owned value has one responsible owner. Moving a non-Copy value transfers ownership and invalidates the source binding. Scope exit releases owned resources deterministically, subject to destructor rules.

## Borrowing
- Shared references (`&T`) permit read access and multiple simultaneous aliases.
- Mutable references (`&mut T`) permit exclusive access.
- Shared and mutable borrows cannot overlap incompatibly.
- References cannot outlive the value they refer to.
- Borrow checking is static; safe code does not need a tracing garbage collector.

## Copy and clone
Small explicitly Copy values may be duplicated implicitly. Heap-backed values are moved by default; duplication is explicit through Clone or a type-specific operation.

## Unsafe boundary
An `unsafe` block/module is a narrow escape hatch for FFI, raw pointers, allocators, and hardware operations. Unsafe code must document invariants. Unsafe does not disable type checking, and unsafe APIs must expose safe wrappers where feasible.

## Drop and resource management
Destruction order is deterministic and reverse declaration order for locals. Resource acquisition should be paired with ownership; exceptions are not the resource-management mechanism.

## Implementation gates
A borrow checker must have adversarial tests for aliasing, lifetime escape, move-after-use, partial moves, reborrowing, field projections, control-flow joins, and drop correctness before any memory-safety claim is made.
