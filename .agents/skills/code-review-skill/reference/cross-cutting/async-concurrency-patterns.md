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
| **Goroutines + Channels** | Go | Lightweight coroutines + CSP communication | Minimal syntax, low overhead | Manual cancellation propagation |
| **async/await + Event Loop** | Python, TypeScript | Single-threaded cooperative multitasking | Lock-free, easy to reason about | Cannot block the event loop |
| **async/await + Tokio** | Rust | Futures + runtime scheduling | Zero-cost abstraction, compile-time safety | Steep learning curve |
| **Coroutines + Flow** | Kotlin | Suspending functions + structured concurrency | Automatic cancellation, lifecycle binding | Dispatchers selection is complex |
| **async/await + Actors** | Swift | Structured concurrency + Actor isolation | Compile-time data race checking | Swift 6 migration cost |
| **async/await + TPL** | C# | Task + thread pool | Mature ecosystem, ConfigureAwait | Implicit thread switching |
| **Threads + Mutexes** | C++, Java, All | OS threads + shared memory | True parallelism | Complex lock management, deadlock risk |

### When to choose what

```
I/O-bound (network, database, files):
  → async/await (Python, TS, Rust, Swift, C#)
  → goroutines (Go)
  → coroutines (Kotlin)

CPU-bound (computation, image processing):
  → thread pool (Java, C++, C#)
  → multiprocessing (Python)
  → spawn_blocking (Rust tokio)
  → Dispatchers.Default (Kotlin)

Mixed:
  → async + spawn_blocking (Rust)
  → async + run_in_executor (Python)
  → goroutines + sync.Mutex (Go)
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

### Pitfall 4: Goroutine / Task leak

Starting a concurrent task without ensuring it exits.

```go
// ❌ Go: goroutine leak
func process() {
    ch := make(chan int)
    go func() {
        result := <-ch  // if nobody sends, the goroutine blocks forever
    }()
    // the function returns, but the goroutine is still waiting
}
```

```python
# ❌ Python: Task leak
async def process():
    task = asyncio.create_task(long_running())
    # the function returns, but the task keeps running
```

**Solution**: use context/done channel (Go), TaskGroup (Python), structured concurrency (Kotlin/Swift).

### Pitfall 5: Blocking in an async context

```python
# ❌ Python: synchronous I/O in an async function blocks the event loop
async def handle():
    result = requests.get(url)  # blocking! the entire event loop stalls
    return result

# ✅ use async I/O or move the blocking operation to a thread pool
async def handle():
    result = await aiohttp.get(url)  # non-blocking
    return result

# or move the synchronous code to a thread pool
async def handle():
    result = await asyncio.to_thread(requests.get, url)
    return result
```

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

```kotlin
// ✅ Kotlin: coroutineScope ensures all child coroutines complete when the scope ends
suspend fun processItems(items: List<Item>) = coroutineScope {
    items.forEach { item ->
        launch { processItem(item) }  // child coroutine
    }
    // waits for all child coroutines to complete when the scope ends
}

// if processItems is cancelled, all child coroutines are cancelled automatically
```

```swift
// ✅ Swift: async let + TaskGroup
func processItems() async throws {
    async let resultA = fetchA()  // runs concurrently
    async let resultB = fetchB()
    let combined = try await (resultA, resultB)  // waits for both
}
```

```python
# ✅ Python 3.11+: TaskGroup
async def process_items():
    async with asyncio.TaskGroup() as tg:
        for item in items:
            tg.create_task(process_item(item))
    # waits for all tasks to complete when the TaskGroup exits
    # if one task fails, the remaining tasks are cancelled automatically
```

### 2. Cancellation propagation

Ensure the cancellation signal propagates correctly to all child tasks.

```go
// ✅ Go: context propagates cancellation
func processAll(ctx context.Context, items []Item) error {
    g, ctx := errgroup.WithContext(ctx)
    for _, item := range items {
        item := item
        g.Go(func() error {
            return processItem(ctx, item)
        })
    }
    return g.Wait()  // if any fails, the context is cancelled and the remaining tasks receive the signal
}
```

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

```go
// ✅ Go: a buffered channel acts as natural backpressure
func process(items <-chan Item) <-chan Result {
    results := make(chan Result, 10)  // buffers 10 results
    go func() {
        for item := range items {
            results <- processItem(item)  // blocks when the buffer is full
        }
        close(results)
    }()
    return results
}
```

```kotlin
// ✅ Kotlin: Flow has built-in backpressure
fun itemsFlow(): Flow<Item> = flow {
    for (item in fetchAll()) {
        emit(item)  // suspends while the collector is not ready
    }
}
// use buffer() to control the buffering strategy
itemsFlow()
    .buffer(capacity = 10, onBufferOverflow = BufferOverflow.SUSPEND)
    .collect { process(it) }
```

### 4. Limiting concurrency

Prevent launching too many tasks at once and exhausting resources.

```python
# ✅ Python: Semaphore limits concurrency
async def fetch_all(urls: list[str], max_concurrent: int = 10):
    semaphore = asyncio.Semaphore(max_concurrent)

    async def fetch_one(url: str):
        async with semaphore:
            return await aiohttp.get(url)

    return await asyncio.gather(*[fetch_one(url) for url in urls])
```

```go
// ✅ Go: errgroup + semaphore
func fetchAll(ctx context.Context, urls []string, maxConcurrent int) error {
    g, ctx := errgroup.WithContext(ctx)
    sem := make(chan struct{}, maxConcurrent)

    for _, url := range urls {
        url := url
        g.Go(func() error {
            sem <- struct{}{}        // acquire the semaphore
            defer func() { <-sem }() // release the semaphore
            return fetch(ctx, url)
        })
    }
    return g.Wait()
}
```

---

## Cross-language code examples

### Go: Goroutines + Channels + Context

```go
// ✅ Complete pattern: context cancellation + errgroup + bounded concurrency
func processBatch(ctx context.Context, items []Item) ([]Result, error) {
    g, ctx := errgroup.WithContext(ctx)
    results := make([]Result, len(items))
    sem := make(chan struct{}, 10)  // at most 10 concurrent

    for i, item := range items {
        i, item := i, item
        g.Go(func() error {
            select {
            case sem <- struct{}{}:
            case <-ctx.Done():
                return ctx.Err()
            }
            defer func() { <-sem }()

            result, err := process(ctx, item)
            if err != nil {
                return fmt.Errorf("item %d: %w", i, err)
            }
            results[i] = result
            return nil
        })
    }

    if err := g.Wait(); err != nil {
        return nil, err
    }
    return results, nil
}
```

### Python: asyncio + TaskGroup

```python
# ✅ Python 3.11+: structured concurrency + bounded concurrency + timeout
import asyncio

async def process_batch(items: list[Item], max_concurrent: int = 10) -> list[Result]:
    semaphore = asyncio.Semaphore(max_concurrent)

    async def process_one(item: Item) -> Result:
        async with semaphore:
            return await process(item)

    async with asyncio.TaskGroup() as tg:
        tasks = [tg.create_task(process_one(item)) for item in items]

    return [task.result() for task in tasks]
```

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

### Kotlin: Coroutines + Flow + Dispatchers

```kotlin
// ✅ structured concurrency + bounded concurrency + cancellation safety
suspend fun processBatch(items: List<Item>, maxConcurrent: Int = 10): List<Result> {
    val semaphore = Semaphore(maxConcurrent)

    return coroutineScope {
        items.map { item ->
            async(Dispatchers.IO) {
                semaphore.withPermit {
                    process(item)
                }
            }
        }.awaitAll()
    }
}

// ✅ Flow: streaming + backpressure
fun itemStream(): Flow<Result> = flow {
    for (item in fetchAllItems()) {
        emit(process(item))
    }
}
    .flowOn(Dispatchers.IO)
    .buffer(capacity = 10)
    .catch { e -> logger.error("stream failed", e) }
```

### Swift: async/await + TaskGroup + Actors

```swift
// ✅ structured concurrency + actor isolation
actor ResultCollector {
    private var results: [Result] = []
    func add(_ result: Result) { results.append(result) }
    func all() -> [Result] { results }
}

func processBatch(items: [Item], maxConcurrent: Int = 10) async throws -> [Result] {
    let collector = ResultCollector()

    try await withThrowingTaskGroup(of: Void.self) { group in
        var active = 0
        for item in items {
            if active >= maxConcurrent {
                try await group.next()
                active -= 1
            }
            group.addTask {
                let result = try await process(item)
                await collector.add(result)
            }
            active += 1
        }
    }

    return await collector.all()
}
```

### C#: async/await + SemaphoreSlim + CancellationToken

```csharp
// ✅ bounded concurrency + cancellation + exception handling
async Task<List<Result>> ProcessBatchAsync(
    List<Item> items,
    int maxConcurrent = 10,
    CancellationToken ct = default)
{
    using var semaphore = new SemaphoreSlim(maxConcurrent);
    var tasks = items.Select(async item =>
    {
        await semaphore.WaitAsync(ct);
        try
        {
            return await ProcessAsync(item, ct);
        }
        finally
        {
            semaphore.Release();
        }
    });

    var results = await Task.WhenAll(tasks);
    return results.ToList();
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
- [ ] Uses structured concurrency (TaskGroup / coroutineScope / errgroup)
- [ ] Concurrency is bounded (semaphore / bounded channel)
- [ ] Long-running tasks support timeouts
- [ ] Backpressure prevents memory growth

### Performance checks
- [ ] Concurrency granularity is reasonable (neither too fine nor too coarse)
- [ ] Use async for I/O-bound work and threads/processes for CPU-bound work
- [ ] Lock hold time is minimized
- [ ] No unnecessary awaits (operations that could be parallel run serially)

### Language-specific
- [ ] Go: context propagation, errgroup usage, reasonable channel buffering
- [ ] Python: event loop is not blocked, TaskGroup manages lifecycles
- [ ] Rust: spawn_blocking isolates blocking operations, select! handles timeouts
- [ ] Kotlin: coroutineScope structured concurrency, correct Dispatchers selection
- [ ] Swift: @MainActor guards UI, actor isolates mutable state
- [ ] C#: CancellationToken propagation, ConfigureAwait(false) in library code
- [ ] TypeScript: Promise.all + concurrency limiting, AbortController cancellation
