# N+1 Query Problem — Cross-Language General Guide

> N+1 queries are the most common performance anti-pattern in ORMs and database access layers. This document covers the problem definition, detection methods, general solutions, and cross-language code examples.

## Table of Contents

- [Problem Definition](#problem-definition)
- [Performance Impact](#performance-impact)
- [Detection Methods](#detection-methods)
- [General Solutions](#general-solutions)
- [Language-Specific Implementations](#language-specific-implementations)
- [Review Checklist](#review-checklist)

---

## Problem Definition

An N+1 query is: **1 query fetches N records, followed by N additional queries triggered inside a loop** to fetch the associated data.

```
Request flow:
  1 query   → fetch N primary records
  N queries → one query per primary record for its associated data
  ─────────
  Total: 1 + N queries
```

### Consequences

| Problem | Impact |
|------|------|
| **Query count grows linearly** | 100 records = 101 SQL statements, 1000 records = 1001 |
| **Network latency accumulates** | Every query has a round-trip latency (RTT); N round trips >> 1 batch query |
| **Connection pool exhaustion** | A large number of queries occupies all database connections, slowing down the entire application |
| **Hard to spot during development** | Development environments have little data, so N+1 is not obvious; performance collapses when production data volumes are large |

---

## Performance Impact

### Scenario Comparison: Fetching 100 Users and Their Orders

| Approach | SQL count | Latency (assuming RTT=1ms) | Applicable scenario |
|------|----------|---------------------|---------|
| N+1 lazy loading | 101 | ~101ms | Very small data volumes |
| Eager loading (JOIN) | 1 | ~1ms | One-to-many, moderate data volumes |
| Eager loading (IN) | 2 | ~2ms | Many-to-many, large data sets |
| DataLoader / batch | 2 | ~2ms | GraphQL / complex graph queries |

### SQL Count Comparison

```sql
-- ❌ N+1: 1 + 100 = 101 queries
SELECT * FROM users;                          -- 1 query
SELECT * FROM orders WHERE user_id = 1;       -- query 2
SELECT * FROM orders WHERE user_id = 2;       -- query 3
...
SELECT * FROM orders WHERE user_id = 100;     -- query 101

-- ✅ Batch: 2 queries
SELECT * FROM users;
SELECT * FROM orders WHERE user_id IN (1,2,...,100);
```

---

## Detection Methods

### 1. ORM SQL Logging

Enable SQL logging and observe the query count in a test or development environment:

```python
# Django
import logging
logging.getLogger('django.db.backends').setLevel(logging.DEBUG)

# SQLAlchemy
import logging
logging.getLogger('sqlalchemy.engine').setLevel(logging.INFO)
```

```java
// Spring Boot application.yml
spring:
  jpa:
    show-sql: true
    properties:
      hibernate.format_sql: true
```

```csharp
// EF Core
optionsBuilder.LogTo(Console.WriteLine, LogLevel.Information);
```

### 2. Query Count Assertions

Assert the SQL query count in tests:

```python
# Django: django-assert-num-queries
from django.test.utils import CaptureQueriesContext
from django.db import connection

with CaptureQueriesContext(connection) as ctx:
    list(User.objects.select_related("profile").all())
assert len(ctx) <= 2  # expect at most 2 queries
```

```java
// Hibernate: p6spy or datasource-proxy
// count SQL executions in tests
assertThat(sqlCount).isLessThanOrEqualTo(2);
```

### 3. APM / Database Monitoring Tools

- **Django Debug Toolbar** — displays query count and time in real time
- **p6spy** (Java) — intercepts at the JDBC layer and logs every SQL statement
- **MiniProfiler** (.NET) — inline SQL statistics in the page
- **DataDog / New Relic** — slow query alerting in production

---

## General Solutions

### Solution 1: Eager Loading (JOIN preloading)

Fetch primary records and associated records in a single JOIN query. Suitable for one-to-one and one-to-many.

### Solution 2: Batch Fetching (IN clause batch query)

Two queries: primary records + a `WHERE id IN (...)` batch fetch of associated records. Suitable for many-to-many and large data sets.

### Solution 3: DataLoader Pattern

In GraphQL or complex graph query scenarios, collect all required IDs and merge them into a single batch query.

```
// DataLoader pseudocode
class DataLoader<K, V> {
    load(K key) → V         // register the request, do not query immediately
    loadAll([K]) → [V]      // merge into a single batch query
}
```

### Solution 4: Projection

Query only the fields you need to reduce the amount of transferred data:

```sql
-- ❌ fetch all columns
SELECT * FROM users JOIN profiles ON ...

-- ✅ project only the needed fields
SELECT u.name, p.avatar_url FROM users u JOIN profiles p ON ...
```

---

## Language-Specific Implementations

### Python / Django

> See the Django Guide for details

```python
# ForeignKey / OneToOne → select_related (SQL JOIN)
books = Book.objects.select_related("publisher")

# M2M / reverse FK → prefetch_related (2 queries + Python merge)
authors = Author.objects.prefetch_related("books")

# nested preloading
authors = Author.objects.prefetch_related("books__publisher")

# fine-grained control with Prefetch objects
from django.db.models import Prefetch
authors = Author.objects.prefetch_related(
    Prefetch("books", queryset=Book.objects.filter(published=True), to_attr="published_books")
)
```

### Python / SQLAlchemy (FastAPI)

> See the FastAPI Guide for details

```python
from sqlalchemy.orm import selectinload

# selectinload: IN clause batch loading (recommended for async scenarios)
stmt = select(Order).options(selectinload(Order.customer))

# joinedload: JOIN loading
stmt = select(Order).options(joinedload(Order.customer))
```

### Java / JPA (Spring Boot)

> See the Java Guide for details

```java
// ❌ FetchType.EAGER or triggering lazy loading inside a loop
@OneToMany(fetch = FetchType.EAGER)  // dangerous!

// ✅ JOIN FETCH
@Query("SELECT u FROM User u JOIN FETCH u.orders")
List<User> findAllWithOrders();

// ✅ @EntityGraph (declarative)
@EntityGraph(attributePaths = {"orders", "profile"})
List<User> findAll();

// ✅ @BatchSize (reduces N+1 to N/batchSize + 1)
@OneToMany
@BatchSize(size = 50)
private List<Order> orders;
```

### C# / EF Core

> See the C# Guide for details

```csharp
// ❌ N+1: foreach triggers lazy loading
foreach (var blog in await context.Blogs.ToListAsync())
    foreach (var post in blog.Posts)  // queries on every iteration!

// ✅ Include + ThenInclude
var blogs = await context.Blogs
    .Include(b => b.Posts)
    .ToListAsync();

// ✅ Projection (safest, avoids over-fetching)
var data = await context.Blogs
    .Select(b => new { b.Url, PostTitles = b.Posts.Select(p => p.Title) })
    .ToListAsync();
```

### PHP / Laravel / Doctrine

> See the PHP Guide for details

```php
// ❌ query inside the loop
foreach ($orders as $order) {
    $customer = $customerRepo->find($order->customerId);
    render($order, $customer);
}

// ✅ batch preloading
$customerIds = array_unique(array_map(fn($o) => $o->customerId, $orders));
$customers = $customerRepo->findByIds($customerIds);

foreach ($orders as $order) {
    render($order, $customers[$order->customerId] ?? null);
}

// Laravel Eloquent: with()
$orders = Order::with('customer')->get();

// Doctrine: JOIN FETCH
$dql = 'SELECT o, c FROM Order o JOIN o.customer c';
```

### TypeScript / Prisma

```typescript
// ❌ N+1
const users = await prisma.user.findMany();
for (const user of users) {
    user.posts = await prisma.post.findMany({ where: { userId: user.id } });
}

// ✅ include (Prisma automatically generates a JOIN or batch query)
const users = await prisma.user.findMany({
    include: { posts: true },
});

// ✅ nested include
const users = await prisma.user.findMany({
    include: {
        posts: {
            include: { comments: true },
        },
    },
});
```

---

## Review Checklist

### Detection
- [ ] SQL logging or query count monitoring is enabled
- [ ] Tests contain query count assertions
- [ ] APM tools are configured with N+1 alerts

### Fixing
- [ ] ForeignKey / OneToOne relationships use JOIN eager loading
- [ ] M2M / reverse relationships use IN batch preloading
- [ ] Avoid triggering database queries inside loops
- [ ] Use projections to fetch only the needed fields

### Architecture
- [ ] List APIs are paginated to avoid loading too many records at once
- [ ] GraphQL scenarios use DataLoader
- [ ] Caching strategy (Redis) handles frequently read associated data
