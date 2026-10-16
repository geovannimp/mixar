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

Enable SQL logging and observe the query count in a test or development environment.

### 2. Query Count Assertions

Assert the SQL query count in tests.

### 3. APM / Database Monitoring Tools

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
