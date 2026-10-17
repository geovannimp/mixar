# Common Bugs Checklist

Quick-reference bug patterns organized by category. For detailed code examples, explanations, and comprehensive review checklists, see the dedicated language guides linked below.

## Universal Issues

### Logic Errors
- [ ] Off-by-one errors in loops and array access
- [ ] Incorrect boolean logic (De Morgan's law violations)
- [ ] Missing null/undefined checks
- [ ] Race conditions in concurrent code
- [ ] Incorrect comparison operators (`==` vs `===`, `=` vs `==`)
- [ ] Integer overflow/underflow
- [ ] Floating point comparison issues

### Resource Management
- [ ] Memory leaks (unclosed connections, listeners)
- [ ] File handles not closed
- [ ] Database connections not released
- [ ] Event listeners not removed
- [ ] Timers/intervals not cleared

### Error Handling
- [ ] Swallowed exceptions (empty catch blocks)
- [ ] Generic exception handling hiding specific errors
- [ ] Missing error propagation
- [ ] Incorrect error types thrown
- [ ] Missing finally/cleanup blocks

## TypeScript/JavaScript

- [ ] `==` instead of `===`
- [ ] Using `any` — prefer proper types or `unknown` with type guards
- [ ] Missing `await` on async calls
- [ ] Unhandled promise rejections (no try-catch around await)
- [ ] `this` context lost in callbacks
- [ ] Missing `key` prop in lists
- [ ] Closure capturing stale loop variable
- [ ] `parseInt` without radix parameter
- [ ] Modifying array/object during iteration

**Full guide:** [TypeScript Review Guide](typescript.md)

## Rust

**Ownership & Borrowing:**
- [ ] Unnecessary `clone()` to work around borrow checker
- [ ] `Arc<Mutex<T>>` when single-owner would suffice
- [ ] Storing borrows in structs when owned data is simpler
- [ ] Unnecessary `RefCell` (runtime checks vs compile-time)

**Unsafe Code:**
- [ ] `unsafe` block without `SAFETY:` comment explaining invariants
- [ ] `unsafe fn` without `# Safety` doc section
- [ ] Unsafe invariants split across modules

**Async & Concurrency:**
- [ ] Blocking in async context (`std::fs`, `std::thread::sleep`)
- [ ] Holding `std::sync::Mutex` across `.await`
- [ ] Spawned task missing `'static` lifetime bound
- [ ] Dropping a Future without awaiting (forgotten work)

**Error Handling:**
- [ ] `unwrap()`/`expect()` in production code
- [ ] Library using `anyhow` instead of `thiserror` (callers can't match)
- [ ] Swallowing error context (`map_err(|_| ...)`)
- [ ] Ignoring `must_use` return values

**Performance:**
- [ ] Unnecessary `.collect()` — prefer lazy iterators
- [ ] String concatenation in loops without `with_capacity`
- [ ] `Box<dyn Trait>` when `impl Trait` would work

**Full guide:** [Rust Review Guide](rust.md)

## Dart / Flutter

- [ ] Missing `const` on static widget subtrees, or `_buildFoo()` helpers instead of extracted widgets
- [ ] `!` / `as` / unconstrained `late` used to silence null safety
- [ ] Heavy `jsonDecode` / image / crypto work on the UI isolate
- [ ] `Future` or `Stream` created inside `build` (new instance every rebuild)
- [ ] `setState` / `BuildContext` used after `await` without `mounted` / `context.mounted`
- [ ] `ref.watch` / `context.watch` in a callback; `read` used in `build` (or the reverse)
- [ ] `BlocProvider.value` / `ChangeNotifierProvider.value` given a new instance constructed in `build` (use `create`; `value` does not dispose)
- [ ] `BlocProvider(create: ...)` captures a stale `id` — missing `ValueKey(id)` remount or `didUpdateWidget` reload (`UniqueKey()` remounts every rebuild)
- [ ] Platform channel `invokeMethod` without `PlatformException` handling
- [ ] List children holding `State` missing a stable `ValueKey` (or using `UniqueKey()` in `build`)
- [ ] `TextEditingController` / `AnimationController` / `StreamSubscription` not disposed

**Full guide:** [Dart / Flutter Review Guide](dart.md)

## SQL

- [ ] String concatenation for queries (SQL injection risk) — use parameterized queries
- [ ] Missing indexes on filtered/joined columns
- [ ] `SELECT *` instead of specific columns
- [ ] N+1 query patterns
- [ ] Missing `LIMIT` on large tables
- [ ] Not handling `NULL` comparisons correctly (`IS NULL` vs `= NULL`)
- [ ] Missing transactions for related operations
- [ ] Incorrect JOIN types
- [ ] Collation / case sensitivity surprises across databases (MySQL vs Postgres defaults)
- [ ] Date and timezone handling errors (naive timestamps, server-local `NOW()`, DST)

**See also:** [Security Review Guide](security-review-guide.md) for SQL injection prevention

## API Design

- [ ] Inconsistent resource naming
- [ ] Wrong HTTP methods (POST for idempotent operations)
- [ ] Missing pagination for list endpoints
- [ ] Incorrect status codes
- [ ] Missing rate limiting
- [ ] Missing input validation and sanitization
- [ ] Trusting client-side validation only

## Testing

- [ ] Testing implementation details instead of behavior
- [ ] Missing edge case tests
- [ ] Flaky tests (non-deterministic)
- [ ] Tests with external dependencies (no mocks)
- [ ] Missing negative tests (error cases)
- [ ] Overly complex test setup
