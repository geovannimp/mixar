# Error Handling Principles — Cross-Language Guide

> This document covers the core principles of error handling, common anti-patterns, error hierarchy design, and logging best practices. Each principle comes with cross-language code examples.

## Table of Contents

- [Core Principles](#core-principles)
- [Anti-patterns](#anti-patterns)
- [Error Hierarchy Design](#error-hierarchy-design)
- [Logging Best Practices](#logging-best-practices)
- [Cross-Language Code Examples](#cross-language-code-examples)
- [Review Checklist](#review-checklist)

---

## Core Principles

### Principle 1: Do Not Swallow Errors

Every error must be handled: propagated upward, logged, or converted into a more meaningful error. **Never** silently ignore it.

```
// Pseudocode
result = risky_operation()
if error:
    // You must do one of the following:
    //   1. return error to caller (propagate)
    //   2. log + return fallback (degrade)
    //   3. panic/crash (when unrecoverable)
```

### Principle 2: Add Context

Error messages should include a **description of the operation** and the **key parameters**, so a debugger can locate the problem without reading the call chain.

```
// ❌ No context
"failed"

// ✅ With context
"failed to process order #12345: payment gateway timeout after 30s"
```

### Principle 3: Use Specific Types

Use error types to distinguish failure causes, so callers can precisely handle different failure scenarios.

```
// ❌ Generic error
throw new Error("something went wrong")

// ✅ Specific types
throw new OrderNotFoundError(orderId)
throw new PaymentTimeoutException(gatewayName, timeoutMs)
```

### Principle 4: Fail Fast

Validate preconditions before starting an operation and fail as early as possible. This avoids inconsistent state caused by discovering an error only after partial execution.

### Principle 5: Handle Each Error Only Once

Do not handle the same error at every layer (logging AND returning AND wrapping). Choose one approach and let the caller decide how to handle it.

```
// ❌ Both log and return (duplicate handling)
if err:
    log.error("failed: %s", err)
    return err

// ✅ Wrap and return only, letting the top layer handle it uniformly
if err:
    return wrap_error("operation failed", err)
```

---

## Anti-patterns

### Anti-pattern 1: Swallowing an error

The same mistake shows up as an empty `catch`/`except` block, or as an error that is
captured and then never used: the failure disappears and the caller cannot react.

```rust
// ❌ Rust: unwrap() in production code
let result = risky().unwrap();  // panic on error
```

### Anti-pattern 2: Overly broad catch

The handler catches every failure type, so the kinds can no longer be distinguished or
handled differently. Catch only what you can actually handle.

### Anti-pattern 3: Losing the original exception

A new error is raised without chaining the cause, discarding the original stack trace and
its information. Attach the original error as the cause.

### Anti-pattern 4: Using exceptions for flow control

Ordinary control flow expressed as a raised error, or a panic/abort on input the caller
could have handled. It is slow, unclear, and hard to reason about.

### Anti-pattern 5: Ignoring return values

An error or status value the API returns is discarded and execution continues as if it
had succeeded.

---

## Error Hierarchy Design

### Three-Layer Error Architecture

```
┌────────────────────────────────────────────────────────────────────────────┐
│ Application Errors (application-level)                                     │
│   - AppError / ServiceError                                                │
│   - Caught by the global exception handler, returns a user-friendly response│
├────────────────────────────────────────────────────────────────────────────┤
│ Module Errors (module-level)                                               │
│   - PaymentError, AuthError, ValidationError                               │
│   - Each business module defines its own error types                       │
├────────────────────────────────────────────────────────────────────────────┤
│ Infrastructure Errors (infrastructure-level)                               │
│   - IOError, NetworkError, DatabaseError                                   │
│   - Low-level errors from the OS, network, and database                    │
└────────────────────────────────────────────────────────────────────────────┘
```

### Design Rules

1. **Module-level errors inherit from an application-level base class**, which makes global catching easier
2. **Infrastructure errors are converted to module-level errors at module boundaries**, and are not exposed to upper layers
3. **Each error type contains enough context** for debugging (ID, timestamp, operation name)

---

## Logging Best Practices

### Choosing a Log Level

| Level | When to use | Example |
|------|---------|------|
| **ERROR** | Failures requiring human intervention | Payment failure, data inconsistency |
| **WARN** | Anomalies that can recover automatically | Successful retry, degraded handling |
| **INFO** | Normal business events | Order created, user logged in |
| **DEBUG** | Debugging information | Function arguments, intermediate state |

### Log Format

```
// ❌ No structured information
log.error("failed to process")

// ✅ Structured information + context
log.error("payment_failed", {
    "order_id": "12345",
    "gateway": "stripe",
    "error_code": "card_declined",
    "amount": 99.99,
    "duration_ms": 2340
})
```

### Log Security

- **Do not log sensitive information**: passwords, tokens, PII, full credit card numbers
- **Redact sensitive data**: `email: a***@example.com`
- **Log injection protection**: escape user input to prevent forged log lines

---

## Cross-Language Code Examples

### Rust

```rust
// ✅ thiserror defines the error type + context
#[derive(Debug, thiserror::Error)]
enum PaymentError {
    #[error("gateway {gateway} unreachable")]
    GatewayUnreachable {
        gateway: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("order {order_id} not found")]
    OrderNotFound { order_id: u64 },
}

async fn process_payment(gateway: &str, order_id: u64) -> Result<(), PaymentError> {
    let response = client.post(url)
        .send()
        .await
        .map_err(|e| PaymentError::GatewayUnreachable {
            gateway: gateway.into(),
            source: e,
        })?;
    Ok(())
}
```

### TypeScript

```typescript
// ✅ Custom error class + context
class PaymentError extends Error {
    constructor(
        message: string,
        public readonly orderId: string,
        public readonly gateway: string,
        public readonly cause?: Error,
    ) {
        super(message);
        this.name = 'PaymentError';
    }
}

async function processPayment(orderId: string): Promise<Receipt> {
    try {
        const response = await fetch(url, { method: 'POST', body: payload });
        if (!response.ok) {
            throw new PaymentError(
                `gateway returned ${response.status}`,
                orderId,
                gatewayName,
            );
        }
        return await response.json();
    } catch (err) {
        if (err instanceof TypeError) {
            throw new PaymentError('gateway unreachable', orderId, gatewayName, err);
        }
        throw err;
    }
}
```

---

## Review Checklist

### Core Checks
- [ ] No empty catch blocks or silently ignored errors
- [ ] Error messages include an operation description and key parameters
- [ ] Specific error types are used (not generic Error/Exception)
- [ ] Exception chain preserved (cause chain)
- [ ] Preconditions are validated before the operation starts (fail fast)

### Architecture Checks
- [ ] A clear error hierarchy is defined (application/module/infrastructure)
- [ ] A global exception handler catches unhandled errors
- [ ] API boundaries convert internal errors into appropriate HTTP status codes

### Logging Checks
- [ ] Error logs include structured context
- [ ] No sensitive information is logged (passwords, tokens, PII)
- [ ] Log levels are used correctly (ERROR vs WARN vs INFO)

### Language-Specific
- [ ] Rust: propagate with `?`, custom Error types
