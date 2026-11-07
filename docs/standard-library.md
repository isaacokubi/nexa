# NEXA Standard Library Plan

The standard library is split into a small language core and platform/library modules. Modules listed here are planned unless marked bootstrap.

## Core
- Primitive types, Option/Result, traits, formatting, panic/abort primitives.
- Bootstrap: Number/String/Bool/Nil values and built-in print.

## Collections
- Array, Slice, Vec, Deque, List, Map, Set, Heap, BitSet.
- Iterators, ranges, sorting, searching, and stable hashing.

## Text
- UTF-8 String, StringBuilder, grapheme-aware helpers, parsing and formatting.

## I/O
- File, buffered reader/writer, stdin/stdout/stderr, paths, environment variables.

## Networking
- TCP/UDP, DNS, HTTP client/server building blocks, TLS integration.

## Concurrency
- Task executor, channels, synchronization, atomics, async I/O.

## Platform
- Process management, signals, clocks, filesystem, OS abstractions and FFI.

## Policy
Platform-specific behavior must be documented. APIs use Result for recoverable failures, avoid implicit global state, and preserve cross-platform semantics where practical.
