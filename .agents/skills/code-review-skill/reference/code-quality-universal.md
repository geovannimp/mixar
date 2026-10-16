# Universal Code Quality Anti-Patterns

> A language-agnostic guide to code quality anti-patterns, covering core topics such as code reuse, leaky abstractions, parameter sprawl, nested conditionals, stringly-typed code, TOCTOU, and no-op updates. Applicable to PR reviews in every language.

## Table of Contents

- [Code Reuse Review](#code-reuse-review)
- [Parameter Sprawl](#parameter-sprawl)
- [Leaky Abstractions](#leaky-abstractions)
- [Stringly-Typed Code](#stringly-typed-code)
- [Nested Conditional Expressions](#nested-conditional-expressions)
- [Copy-Paste Variants](#copy-paste-variants)
- [No-Op Updates](#no-op-updates)
- [TOCTOU Race Conditions](#toctou-race-conditions)
- [Overly Broad Operations](#overly-broad-operations)
- [Redundant State](#redundant-state)
- [Universal Quality Review Checklist](#universal-quality-review-checklist)

---

## Code Reuse Review

Before accepting new code, search the existing codebase for reusable utilities.

### Search for existing utility functions

**What to check:**
- Does the new function duplicate the name or functionality of an existing utility?
- Can the inline logic be extracted into a call to an existing module?
- Check neighboring files and the shared/utils directory

---

## Parameter Sprawl

### Ever-growing function parameters

```typescript
// ❌ 6+ positional parameters
function renderWidget(
  title: string, width: number, height: number,
  theme: string, collapsible: boolean, icon: string
) { ... }

// ✅ Options object pattern
interface WidgetOptions {
  title: string;
  width?: number;
  height?: number;
  theme?: "light" | "dark";
  collapsible?: boolean;
  icon?: string;
}
function renderWidget(options: WidgetOptions) { ... }
```

**What to check:**
- Are there ≥ 4 function parameters? Consider an options object
- Is the new parameter just a boolean flag? Consider an enum or a strategy pattern
- Are there mutually exclusive parameters like `enable_x`, `disable_y`?

---

## Leaky Abstractions

### Exposing internal implementation details

```typescript
// ❌ The component receives the raw API response structure
<UserCard user={apiResponse.data.results[0]} />

// ✅ The component receives a domain type; the adapter handles the mapping
interface UserSummary {
  displayName: string;
  avatarUrl: string;
}
<UserCard user={adaptUser(apiResponse)} />
```

**What to check:**
- Does the function's return type leak the underlying implementation (ORM, HTTP client, file format)?
- Does the component/function depend on another system's data structures?
- Does it break an existing abstraction boundary?

---

## Stringly-Typed Code

### Using raw strings instead of constants/enums

```typescript
// ❌ Raw string event names — a typo won't be reported
emitter.emit("userCreated", data);
emitter.on("usercreated", handler); // bug: typo

// ✅ Constants or a branded type
const Events = {
  USER_CREATED: "userCreated",
  USER_SUSPENDED: "userSuspended",
} as const;
emitter.emit(Events.USER_CREATED, data);
```

**What to check:**
- Is a string used in place of an existing enum/union type?
- Are event names, action types, or status values scattered across multiple files?
- Are string comparisons case-sensitive but unvalidated?

---

## Nested Conditional Expressions

### Ternary chains and nested if/else

```typescript
// ❌ Nested ternary
const bg = isHovered
  ? isSelected ? "blue" : "gray"
  : isSelected ? "navy" : "white";

// ✅ Lookup table (lookup map)
const bgMap: Record<string, string> = {
  "true-true": "blue",
  "true-false": "gray",
  "false-true": "navy",
  "false-false": "white",
};
const bg = bgMap[`${isHovered}-${isSelected}`];
```

**What to check:**
- Are ternary expressions nested ≥ 2 levels deep?
- Is the if/else nesting ≥ 3 levels deep?
- Can it be replaced with a lookup table, early return, or match?

---

## Copy-Paste Variants

### Nearly duplicated code blocks

```typescript
// ❌ Copy-paste handler with only the URL changed
async function deletePost(id: string) {
  await fetch(`/api/posts/${id}`, { method: "DELETE" });
  router.push("/posts");
}
async function deleteComment(id: string) {
  await fetch(`/api/comments/${id}`, { method: "DELETE" });
  router.push("/comments");
}

// ✅ Parameterize it
async function deleteResource(resource: string, id: string) {
  await fetch(`/api/${resource}/${id}`, { method: "DELETE" });
  router.push(`/${resource}`);
}
```

**What to check:**
- Are there ≥ 2 code blocks that differ only in variable names/URLs/strings?
- Can a parameterized shared function be extracted?
- Can a template method or strategy eliminate the variant?

---

## No-Op Updates

### Unconditional state updates

```typescript
// ❌ Fires an update on every poll — even when the data hasn't changed
useEffect(() => {
  const interval = setInterval(() => {
    fetch("/api/status").then(r => r.json()).then(setStatus);
  }, 5000);
  return () => clearInterval(interval);
}, []);

// ✅ Update only when the value changes
useEffect(() => {
  const interval = setInterval(() => {
    fetch("/api/status")
      .then(r => r.json())
      .then(data => {
        setStatus(prev => isEqual(prev, data) ? prev : data);
      });
  }, 5000);
  return () => clearInterval(interval);
}, []);
```

**What to check:**
- Do polling / interval / event handlers update unconditionally?
- Does the wrapper function respect same-reference returns?
- Do DB writes check for actual changes?

---

## TOCTOU Race Conditions

### Time-of-Check-to-Time-of-Use

```typescript
// ❌ Check-then-act is unsafe in an async environment
if (!fileExists(path)) {
  await writeFile(path, content);
}

// ✅ Act directly + catch
try {
  await writeFile(path, content, { flag: "wx" });
} catch (e) {
  if (e.code === "EEXIST") { /* handle */ }
  else throw e;
}
```

**What to check:**
- Can the `if exists → operate` pattern be replaced with `try operate → catch`?
- Are multi-step state changes inside a transaction/lock?
- In an async operation, is there an await between the check and the act?

---

## Overly Broad Operations

### Reading too much data

```typescript
// ❌ Load all items, then filter
const allItems = await db.query("SELECT * FROM orders");
const pending = allItems.filter(o => o.status === "pending");

// ✅ Filter at the database layer
const pending = await db.query(
  "SELECT * FROM orders WHERE status = ?", ["pending"]
);
```

**What to check:**
- Is an entire collection/file read when only a small part is used?
- Can the filtering be pushed down to the database/storage layer?
- Does the API call support pagination/limit parameters?

---

## Redundant State

### State that could be derived

```typescript
// ❌ Stores both fullName and firstName + lastName
interface User {
  firstName: string;
  lastName: string;
  fullName: string;  // redundant
}

// ✅ fullName is a derived value
interface User {
  firstName: string;
  lastName: string;
}
const fullName = `${user.firstName} ${user.lastName}`;
```

**What to check:**
- Is there a field that could be derived from other fields?
- Does the cached value have an invalidation mechanism?
- Can the observer/effect be replaced with a direct call?

---

## Universal Quality Review Checklist

- [ ] **Reuse review**: Did you search existing utilities/helpers instead of reinventing the wheel?
- [ ] **Parameter count**: ≤ 3 function parameters? If more, use an options object?
- [ ] **Abstraction boundary**: Do return types avoid exposing internal implementation details (ORM, HTTP client, file format)?
- [ ] **Type safety**: Are magic strings avoided in place of existing enums/constants/union types?
- [ ] **Conditional depth**: Ternary nesting ≤ 1 level? if/else nesting ≤ 2 levels?
- [ ] **DRY**: No copy-paste-with-variation (≥ 2 nearly identical blocks)?
- [ ] **No-op protection**: Do polling / interval / event handlers have a change-detection guard?
- [ ] **TOCTOU**: `if exists → operate` replaced with `try operate → catch`?
- [ ] **Data precision**: Do you avoid reading an entire collection/file just to take a subset?
- [ ] **Redundant state**: No stored fields that could be derived from other fields?
