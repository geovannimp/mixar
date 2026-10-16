# Async and Concurrency Patterns — Cross-Language Guide

> This document covers a comparison of concurrency models, common pitfalls, cross-language best practices, and structured concurrency patterns.

## Table of Contents

- [Concurrency model comparison](#concurrency-model-comparison)
- [Common pitfalls](#common-pitfalls)
- [Best practices](#best-practices)
- [Cross-language code examples](#cross-language-code-examples)
- [Review Checklist](#review-checklist)

---

## Concurrency model comparison

| Model | Language | Core concepts | Pros | Cons |
|------|------|----------|------|------|
| **async/await + Event Loop** | TypeScript | Single-threaded cooperative multitasking | Lock-free, easy to reason about | Cannot block the event loop |
| **async/await + Tokio** | Rust | Futures + runtime scheduling | Zero-cost abstraction, compile-time safety | Steep learning curve |
| **Threads + Mutexes** | All | OS threads + shared memory | True parallelism | Complex lock management, deadlock risk |

### When to choose what

```
I/O-bound (network, database, files):
  → async/await (TypeScript, Rust)

CPU-bound (computation, image processing):
  → thread pool
  → spawn_blocking (Rust tokio)

Mixed:
  → async + spawn_blocking (Rust)
```

---

## Common pitfalls

### Pitfall 1: Race Condition

Multiple concurrent tasks read and write shared state, and the result depends on execution order.

```
// generic pseudocode
counter = 0

task1: counter += 1   // read counter=0, write counter=1
task2: counter += 1   // read counter=0, write counter=1
// expected counter=2, actual counter=1
```

**Solution**: mutual exclusion locks, atomic operations, or encapsulating the shared state in an Actor.

### Pitfall 2: Deadlock

Two or more tasks each wait for a lock held by the other.

```
task1: lock(A); lock(B);  // holds A, waits for B
task2: lock(B); lock(A);  // holds B, waits for A
// both wait forever
```

**Solution**:
- Consistent lock acquisition order
- Timeout locks (tryLock with timeout)
- Avoid nested locks

### Pitfall 3: Starvation

Low-priority tasks never get a chance to execute.

```
// high-priority tasks keep arriving, low-priority tasks queue forever
```

**Solution**: fair locks, task priority queues, limiting the degree of concurrency.

### Pitfall 4: Task leak

Starting a concurrent task without ensuring it exits.

**Solution**: structured concurrency — tie the task lifetime to the scope that created it (see [Best practices](#best-practices)).

### Pitfall 5: Blocking in an async context

```rust
// ❌ Rust: blocking inside an async function
async fn handle() {
    let result = std::fs::read_to_string("large.txt");  // blocks the tokio runtime
}

// ✅ use spawn_blocking
async fn handle() {
    let result = tokio::task::spawn_blocking(|| {
        std::fs::read_to_string("large.txt")
    }).await?;
}
```

---

## Best practices

### 1. Structured concurrency

Ensure the lifetime of concurrent tasks is bound to the scope that created them. When the parent task is cancelled, child tasks are cancelled automatically.

### 2. Cancellation propagation

Ensure the cancellation signal propagates correctly to all child tasks.

```rust
// ✅ Rust: tokio::select! + JoinHandle
async fn process_with_timeout(item: Item) -> Result<Data> {
    tokio::select! {
        result = process(item) => result,
        _ = tokio::time::sleep(Duration::from_secs(30)) => {
            Err(anyhow!("processing timed out"))
        }
    }
}
```

### 3. Backpressure

When the producer is far faster than the consumer, the queue size must be bounded to prevent memory growth.

### 4. Limiting concurrency

Prevent launching too many tasks at once and exhausting resources.

---

## Cross-language code examples

### Rust: tokio + select + spawn_blocking

```rust
// ✅ bounded concurrency + timeout + blocking operation isolation
use tokio::sync::Semaphore;
use std::sync::Arc;

async fn process_batch(items: Vec<Item>, max_concurrent: usize) -> Result<Vec<Output>> {
    let sem = Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::new();

    for item in items {
        let permit = sem.clone().acquire_owned().await?;
        handles.push(tokio::spawn(async move {
            let _permit = permit;  // drop on completion
            tokio::select! {
                result = process(item) => result,
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    Err(anyhow!("timeout"))
                }
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await??);
    }
    Ok(results)
}
```

### TypeScript: Worker-pool concurrency limit

```typescript
// ✅ Worker-pool pattern: a fixed number of workers compete for the task queue
//    results are assigned by original index, keeping output order consistent with input.
async function processWithLimit<T, R>(
    items: T[],
    fn: (item: T) => Promise<R>,
    limit: number,
): Promise<R[]> {
    const results: R[] = [];
    let index = 0;

    const workers = Array.from({ length: limit }, async () => {
        while (index < items.length) {
            const i = index++;
            results[i] = await fn(items[i]);
        }
    });

    await Promise.all(workers);
    return results;
}
```

---

## Review Checklist

### Basic checks
- [ ] Concurrent tasks have an explicit exit mechanism (no leaks)
- [ ] Shared state has appropriate protection (mutex, actor, channel)
- [ ] No blocking operations are performed in an async context
- [ ] Cancellation signals propagate correctly to all child tasks

### Architecture checks
- [ ] Uses structured concurrency
- [ ] Concurrency is bounded (semaphore / bounded channel)
- [ ] Long-running tasks support timeouts
- [ ] Backpressure prevents memory growth

### Performance checks
- [ ] Concurrency granularity is reasonable (neither too fine nor too coarse)
- [ ] Use async for I/O-bound work and threads/processes for CPU-bound work
- [ ] Lock hold time is minimized
- [ ] No unnecessary awaits (operations that could be parallel run serially)

### Language-specific
- [ ] Rust: spawn_blocking isolates blocking operations, select! handles timeouts
- [ ] TypeScript: Promise.all + concurrency limiting, AbortController cancellation
