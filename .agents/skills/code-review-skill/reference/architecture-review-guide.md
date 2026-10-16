# Architecture Review Guide

A guide to reviewing architecture design, helping assess whether a codebase's architecture is sound and its design appropriate.

## SOLID Principles Checklist

### S - Single Responsibility Principle (SRP)

**What to check:**
- Does this class/module have only one reason to change?
- Do all the methods in the class serve the same purpose?
- If you had to describe this class to a non-technical person, could you do it in one sentence?

**Signals to look for in review:**
```
⚠️ Class name contains generalized words like "And", "Manager", "Handler", "Processor"
⚠️ A class exceeds 200-300 lines of code
⚠️ A class has more than 5-7 public methods
⚠️ Different methods operate on completely different data
```

**Review questions:**
- "What responsibilities does this class have? Can it be split up?"
- "If requirement X changes, which methods need to change? What about requirement Y?"

### O - Open/Closed Principle (OCP)

**What to check:**
- When adding new functionality, do you need to modify existing code?
- Can new behavior be added through extension (inheritance, composition)?
- Are there large volumes of if/else or switch statements handling different types?

**Signals to look for in review:**
```
⚠️ switch/if-else chains handling different types
⚠️ Adding new functionality requires modifying a core class
⚠️ Type checks (instanceof, typeof) scattered throughout the code
```

**Review questions:**
- "If you were to add a new X type, which files would need to change?"
- "Will this switch statement grow as new types are added?"

### L - Liskov Substitution Principle (LSP)

**What to check:**
- Can a subclass be used entirely in place of its parent class?
- Does the subclass change the expected behavior of the parent's methods?
- Does a subclass throw exceptions that the parent does not declare?

**Signals to look for in review:**
```
⚠️ Explicit type casting
⚠️ A subclass method throws NotImplementedException
⚠️ A subclass method is an empty implementation or only returns
⚠️ Places that use the base class need to check the concrete type
```

**Review questions:**
- "If you replace the parent with the subclass, does the caller's code need to change?"
- "Does this method's behavior in the subclass conform to the parent's contract?"

### I - Interface Segregation Principle (ISP)

**What to check:**
- Is the interface small and focused enough?
- Are implementing classes forced to implement methods they don't need?
- Do clients depend on methods they don't use?

**Signals to look for in review:**
```
⚠️ Interface has more than 5-7 methods
⚠️ Implementing classes have empty methods or throw NotImplementedException
⚠️ Interface name is too broad (IManager, IService)
⚠️ Different clients only use a subset of the interface's methods
```

**Review questions:**
- "Are all methods of this interface used by every implementing class?"
- "Can this large interface be split into smaller, dedicated interfaces?"

### D - Dependency Inversion Principle (DIP)

**What to check:**
- Do high-level modules depend on abstractions rather than concrete implementations?
- Is dependency injection used instead of directly constructing objects?
- Are abstractions defined by high-level modules rather than low-level ones?

**Signals to look for in review:**
```
⚠️ High-level modules directly construct concrete classes from low-level modules
⚠️ Importing concrete implementation classes instead of interfaces/abstract classes
⚠️ Configuration and connection strings hardcoded in business logic
⚠️ Hard to write unit tests for a given class
```

**Review questions:**
- "Can this class's dependencies be swapped with mocks during testing?"
- "If you were to swap the database/API implementation, how many places would need to change?"

---

## Identifying Architecture Anti-Patterns

### Fatal Anti-Patterns

| Anti-pattern | Signals | Impact |
|--------|----------|------|
| **Big Ball of Mud** | No clear module boundaries; any code can call any other code | Hard to understand, modify, and test |
| **God Object** | A single class takes on too many responsibilities, knows too much, does too much | High coupling, hard to reuse and test |
| **Spaghetti Code** | Chaotic control flow, goto or deep nesting, hard to trace execution paths | Hard to understand and maintain |
| **Lava Flow** | Old code nobody dares to touch, lacking documentation and tests | Technical debt accumulates |

### Design Anti-Patterns

| Anti-pattern | Signals | Recommendation |
|--------|----------|------|
| **Golden Hammer** | Using the same technology/pattern for every problem | Choose the solution that fits the problem |
| **Over-engineering (Gas Factory)** | Solving simple problems with complex solutions, abusing design patterns | YAGNI principle; keep it simple first, complex later |
| **Boat Anchor** | Unused code written for "may be needed in the future" | Delete unused code; write it when needed |
| **Copy-Paste Programming** | The same logic appears in multiple places | Extract a shared method or module |

### Review questions

```markdown
🔴 [blocking] "This class has 2000 lines of code; consider splitting it into multiple focused classes"
🟡 [important] "This logic is duplicated in 3 places; consider extracting it into a shared method?"
💡 [suggestion] "This switch statement could be replaced with the strategy pattern, which is easier to extend"
```

---

## Coupling and Cohesion Assessment

### Coupling Types (from best to worst)

| Type | Description | Example |
|------|------|------|
| **Message coupling** ✅ | Passing data through parameters | `calculate(price, quantity)` |
| **Data coupling** ✅ | Sharing a simple data structure | `processOrder(orderDTO)` |
| **Stamp coupling** ⚠️ | Sharing a complex data structure but using only part of it | Passing the whole User object but using only name |
| **Control coupling** ⚠️ | Passing a control flag that affects behavior | `process(data, isAdmin=true)` |
| **Common coupling** ❌ | Sharing global variables | Multiple modules read and write the same global state |
| **Content coupling** ❌ | Directly accessing another module's internals | Directly manipulating another class's private attributes |

### Cohesion Types (from best to worst)

| Type | Description | Quality |
|------|------|------|
| **Functional cohesion** | All elements accomplish a single task | ✅ Best |
| **Sequential cohesion** | Output serves as input for the next step | ✅ Good |
| **Communicational cohesion** | Operating on the same data | ⚠️ Acceptable |
| **Temporal cohesion** | Tasks executed at the same time | ⚠️ Poor |
| **Logical cohesion** | Logically related but functionally different | ❌ Bad |
| **Coincidental cohesion** | No obvious relationship | ❌ Worst |

### Metric Reference

```yaml
Coupling metrics:
  CBO (Coupling Between Objects):
    good: < 5
    warning: 5-10
    dangerous: > 10

  Ce (Efferent Coupling):
    description: How many external classes it depends on
    good: < 7

  Ca (Afferent Coupling):
    description: How many classes depend on it
    high value means: Changes have a large impact; needs to be stable

Cohesion metrics:
  LCOM4 (Lack of Cohesion of Methods):
    1: Single responsibility ✅
    2-3: May need to be split ⚠️
    >3: Should be split ❌
```

### Review questions

- "How many other modules does this module depend on? Can that be reduced?"
- "How many other places are affected by changing this class?"
- "Do all the methods of this class operate on the same data?"

---

## Layered Architecture Review

### Clean Architecture Layer Check

```
┌─────────────────────────────────────┐
│         Frameworks & Drivers        │ ← Outermost layer: Web, DB, UI
├─────────────────────────────────────┤
│         Interface Adapters          │ ← Controllers, Gateways, Presenters
├─────────────────────────────────────┤
│          Application Layer          │ ← Use Cases, Application Services
├─────────────────────────────────────┤
│            Domain Layer             │ ← Entities, Domain Services
└─────────────────────────────────────┘
          ↑ Dependency direction can only point inward ↑
```

### Dependency Rule Check

**Core rule: source code dependencies may only point inward**

```typescript
// ❌ Violates the dependency rule: the Domain layer depends on Infrastructure
// domain/User.ts
import { MySQLConnection } from '../infrastructure/database';

// ✅ Correct: the Domain layer defines the interface, Infrastructure implements it
// domain/UserRepository.ts (interface)
interface UserRepository {
  findById(id: string): Promise<User>;
}

// infrastructure/MySQLUserRepository.ts (implementation)
class MySQLUserRepository implements UserRepository {
  findById(id: string): Promise<User> { /* ... */ }
}
```

### Review Checklist

**Layer boundary checks:**
- [ ] Does the Domain layer have external dependencies (database, HTTP, file system)?
- [ ] Does the Application layer directly operate on the database or call external APIs?
- [ ] Does the Controller contain business logic?
- [ ] Are there cross-layer calls (UI directly calling a Repository)?

**Separation of concerns checks:**
- [ ] Is business logic separated from presentation logic?
- [ ] Is data access encapsulated in a dedicated layer?
- [ ] Are configuration and environment-related code managed centrally?

### Review questions

```markdown
🔴 [blocking] "The Domain entity directly imports a database connection, violating the dependency rule"
🟡 [important] "The Controller contains business calculation logic; consider moving it to the Service layer"
💡 [suggestion] "Consider using dependency injection to decouple these components"
```

---

## Design Pattern Usage Assessment

### When to Use Design Patterns

| Pattern | Applicable scenario | Inapplicable scenario |
|------|----------|------------|
| **Factory** | Need to create different types of objects, with the type determined at runtime | Only one type, or the type is fixed |
| **Strategy** | The algorithm needs to switch at runtime, with multiple interchangeable behaviors | Only one algorithm, or the algorithm never changes |
| **Observer** | One-to-many dependency, where a state change needs to notify multiple objects | A simple direct call already satisfies the need |
| **Singleton** | There is a genuine need for a globally unique instance, such as configuration management | Objects that can be passed via dependency injection |
| **Decorator** | Need to add responsibilities dynamically, avoiding an inheritance explosion | Responsibilities are fixed and don't need dynamic composition |

### Over-Design Warning Signals

```
⚠️ Patternitis (pattern fever) signals:

1. A simple if/else is replaced with strategy pattern + factory + registry
2. An interface with only one implementation
3. An abstraction layer added for "may be needed in the future"
4. Line count increases substantially because of pattern application
5. Newcomers take a long time to understand the code structure
```

### Review Principles

```markdown
✅ Correct pattern usage:
- Solves a real extensibility problem
- Code is easier to understand and test
- Adding new functionality becomes simpler

❌ Overuse of patterns:
- Using a pattern for the sake of using it
- Adds unnecessary complexity
- Violates the YAGNI principle
```

### Review questions

- "What specific problem does using this pattern solve?"
- "What problems would the code have without this pattern?"
- "Does the value this abstraction layer brings outweigh its complexity?"

---

## Scalability Assessment

### Scalability Checklist

**Functional scalability:**
- [ ] Does adding new functionality require modifying core code?
- [ ] Are extension points provided (hooks, plugins, events)?
- [ ] Is configuration externalized (config files, environment variables)?

**Data scalability:**
- [ ] Does the data model support adding new fields?
- [ ] Is the scenario of data volume growth taken into account?
- [ ] Do queries have appropriate indexes?

**Load scalability:**
- [ ] Can it scale horizontally (adding more instances)?
- [ ] Are there stateful dependencies (session, local cache)?
- [ ] Do database connections use a connection pool?

### Extension Point Design Check

```typescript
// ✅ Good extension design: use events/hooks
class OrderService {
  private hooks: OrderHooks;

  async createOrder(order: Order) {
    await this.hooks.beforeCreate?.(order);
    const result = await this.save(order);
    await this.hooks.afterCreate?.(result);
    return result;
  }
}

// ❌ Poor extension design: hardcode all behavior
class OrderService {
  async createOrder(order: Order) {
    await this.sendEmail(order);        // hardcoded
    await this.updateInventory(order);  // hardcoded
    await this.notifyWarehouse(order);  // hardcoded
    return await this.save(order);
  }
}
```

### Review questions

```markdown
💡 [suggestion] "If we need to support a new payment method in the future, is this design easy to extend?"
🟡 [important] "The logic here is hardcoded; consider using configuration or the strategy pattern?"
📚 [learning] "An event-driven architecture would make this feature easier to extend"
```

---

## Code Structure Best Practices

### Directory Organization

**Organized by feature/domain (recommended):**
```
src/
├── user/
│   ├── User.ts           (entity)
│   ├── UserService.ts    (service)
│   ├── UserRepository.ts (data access)
│   └── UserController.ts (API)
├── order/
│   ├── Order.ts
│   ├── OrderService.ts
│   └── ...
└── shared/
    ├── utils/
    └── types/
```

**Organized by technical layer (not recommended):**
```
src/
├── controllers/     ← Different domains mixed together
│   ├── UserController.ts
│   └── OrderController.ts
├── services/
├── repositories/
└── models/
```

### Naming Convention Checks

| Kind | Convention | Example |
|------|------|------|
| Class name | PascalCase, noun | `UserService`, `OrderRepository` |
| Method name | camelCase, verb | `createUser`, `findOrderById` |
| Interface name | I prefix or no prefix | `IUserService` or `UserService` |
| Constants | UPPER_SNAKE_CASE | `MAX_RETRY_COUNT` |
| Private attributes | underscore prefix or none | `_cache` or `#cache` |

### File Size Guidelines

```yaml
Recommended limits:
  Single file: < 300 lines
  Single function: < 50 lines
  Single class: < 200 lines
  Function parameters: < 4
  Nesting depth: < 4 levels

When exceeding the limits:
  - Consider splitting into smaller units
  - Use composition instead of inheritance
  - Extract helper functions or classes
```

### Review questions

```markdown
🟢 [nit] "This 500-line file could be split by responsibility"
🟡 [important] "Recommend organizing the directory structure by feature domain rather than technical layer"
💡 [suggestion] "The function name `process` isn't clear enough; consider renaming it to `calculateOrderTotal`?"
```

---

## Quick Reference Checklist

### 5-Minute Architecture Review Cheat Sheet

```markdown
□ Is the dependency direction correct? (outer layers depend on inner layers)
□ Are there circular dependencies?
□ Is the core business logic decoupled from the framework/UI/database?
□ Does it follow the SOLID principles?
□ Are there obvious anti-patterns?
```

### Red Flag Signals (must be addressed)

```markdown
🔴 God Object - a single class over 1000 lines
🔴 Circular dependency - A → B → C → A
🔴 Domain layer contains framework dependencies
🔴 Hardcoded configuration and secrets
🔴 External service calls with no interface
```

### Yellow Flag Signals (recommended to address)

```markdown
🟡 Coupling between objects (CBO) > 10
🟡 Method parameters over 5
🟡 Nesting depth over 4 levels
🟡 Duplicate code blocks > 10 lines
🟡 An interface with only one implementation
```

---

## Tool Recommendations

| Tool | Purpose | Language support |
|------|------|----------|
| **SonarQube** | Code quality, coupling analysis | Multi-language |
| **NDepend** | Dependency analysis, architecture rules | .NET |
| **JDepend** | Package dependency analysis | Java |
| **Madge** | Module dependency graph | JavaScript/TypeScript |
| **ESLint** | Code standards, complexity checks | JavaScript/TypeScript |
| **CodeScene** | Technical debt, hotspot analysis | Multi-language |

---

## Reference Resources

- [Clean Architecture - Uncle Bob](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [SOLID Principles in Code Review - JetBrains](https://blog.jetbrains.com/upsource/2015/08/31/what-to-look-for-in-a-code-review-solid-principles-2/)
- [Software Architecture Anti-Patterns](https://medium.com/@christophnissle/anti-patterns-in-software-architecture-3c8970c9c4f5)
- [Coupling and Cohesion in System Design](https://www.geeksforgeeks.org/system-design/coupling-and-cohesion-in-system-design/)
- [Design Patterns - Refactoring Guru](https://refactoring.guru/design-patterns)
