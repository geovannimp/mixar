# SQL Injection Prevention Guide

Language-agnostic SQL injection prevention strategies with cross-language code examples.

> **Related**: [Security Review Guide](../security-review-guide.md) for comprehensive security checklist and decision framework.

## Attack Types

SQL injection (SQLi) is ranked #3 in the OWASP Top 10 (2021). Three common variants:

| Type | Description | Risk |
|------|-------------|------|
| **Classic (In-band)** | Attacker receives results directly in the HTTP response | Data exfiltration, authentication bypass |
| **Blind (Boolean/Time-based)** | Attacker infers data from response differences or timing | Slower but still viable for data extraction |
| **Out-of-band** | Attacker uses DNS/HTTP callbacks to exfiltrate data | Less common but harder to detect |

## Universal Prevention Strategy

1. **Parameterized queries** — always (the #1 defense)
2. **ORM safe usage** — understand what your ORM escapes
3. **Input validation** — whitelist over blacklist
4. **Least privilege** — database user with minimal permissions
5. **WAF** — web application firewall as defense-in-depth

---

## Cross-Language Examples

### TypeScript

```typescript
// ❌ Vulnerable: template literal
const query = `SELECT * FROM users WHERE id = ${userId}`;
const result = await client.query(query);

// ✅ pg parameterized ($1, $2, ...)
const result = await client.query(
    "SELECT * FROM users WHERE id = $1",
    [userId]
);

// ✅ Prisma ORM (parameterized by default)
const user = await prisma.user.findUnique({
    where: { id: userId },
});

// ❌ Prisma $queryRawUnsafe with string interpolation
await prisma.$queryRawUnsafe(
    `SELECT * FROM users WHERE id = ${userId}`
);

// ✅ Prisma $queryRaw with tagged template (safe)
await prisma.$queryRaw`
    SELECT * FROM users WHERE id = ${userId}
`;
```

---

## ORM Unsafe Usage Patterns

ORMs do NOT automatically prevent SQL injection in all cases:

**Rule**: Every ORM has a "raw SQL" escape hatch. String interpolation in that escape hatch = SQL injection. Always use the ORM's parameter binding mechanism.

---

## Dynamic Identifiers (Table/Column Names)

Placeholders can only bind **values**, not table names, column names, or SQL keywords. Dynamic identifiers must be validated against an allow-list, never interpolated.

---

## Review Checklist

- [ ] All SQL queries use parameterized queries (no string interpolation)
- [ ] ORM raw SQL methods use bound parameters, not string formatting
- [ ] Dynamic identifiers (table/column names) validated against whitelist
- [ ] Database user has least privilege (no DROP/ALTER for app user)
- [ ] No SQL queries constructed from user input without parameterization
- [ ] Static analysis tools run in CI
