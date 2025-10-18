<!--
SYNC IMPACT REPORT
Version: 0.0.0 → 1.0.0
Rationale: Initial constitution creation with core principles

Principles Defined:
- I. Modularity & Testability (Rust)
- II. Dependency Injection (Rust)
- III. Replicable Development Environment
- IV. Strict TypeScript & Linting (Frontend)
- V. Test-First Development

Sections Added:
- Core Principles (5 principles)
- Development Workflow
- Governance

Templates Status:
✅ plan-template.md - Constitution Check section present, compatible with new principles
✅ spec-template.md - User story and requirements format supports all principles
✅ tasks-template.md - Task structure supports test-first and modular implementation

Deferred Items:
- RATIFICATION_DATE: Set to today (2025-10-18) as initial adoption
-->

# Raft Constitution

## Core Principles

### I. Modularity & Testability

**Every Rust component MUST be modular and independently testable.**

- Break functionality into small, focused modules with clear boundaries
- Each module must have a single, well-defined responsibility
- Modules must be independently testable without requiring the full application context
- Avoid tight coupling between modules - prefer loose coupling through well-defined interfaces
- Public APIs must be minimal and carefully considered

**Rationale**: Modular code enables parallel development, easier debugging, faster test execution, and better code reuse. Testability ensures reliability and facilitates safe refactoring.

### II. Dependency Injection

**All Rust components MUST use dependency injection for external dependencies.**

- Services, repositories, and external resources must be injected, not instantiated directly
- Use trait objects or generic parameters for dependency abstraction
- Concrete implementations must be provided at composition root (main, integration tests)
- Avoid global state and static dependencies
- Constructor injection preferred; builder pattern acceptable for complex construction

**Rationale**: Dependency injection enables unit testing with mocks/stubs, promotes loose coupling, makes dependencies explicit, and facilitates different configurations (dev, test, production).

### III. Replicable Development Environment

**Development environments MUST be fully replicable using Docker and docker-compose.**

- All runtime dependencies (databases, caches, message queues) must be defined in docker-compose.yml
- Development setup should require minimal manual steps: `docker-compose up` should provide a working environment
- Environment configuration must use .env files with documented variables
- Version-pin all Docker images and dependencies
- Include seed data and migrations in containerized setup

**Rationale**: Replicable environments eliminate "works on my machine" issues, accelerate onboarding, ensure consistency across development and CI, and reduce configuration drift.

### IV. Strict TypeScript & Linting (Frontend)

**All frontend code MUST use TypeScript with strict mode and comprehensive linting.**

- TypeScript strict mode must be enabled (no implicit any, strict null checks, etc.)
- ESLint must be configured with strict rulesets (e.g., eslint:recommended, @typescript-eslint/recommended)
- Prettier must enforce consistent code formatting
- No ESLint or TypeScript errors allowed in CI/CD pipeline
- Type definitions must be comprehensive - avoid `any` except where absolutely necessary with explicit justification

**Rationale**: Strict typing catches bugs at compile time, improves IDE support and refactoring safety, serves as live documentation, and ensures code quality consistency across the team.

### V. Test-First Development (NON-NEGOTIABLE)

**Tests MUST be written before implementation (TDD).**

- For each new feature or bug fix: Write failing test → Verify it fails → Implement → Verify it passes
- Unit tests required for all business logic and pure functions
- Integration tests required for API endpoints, database interactions, and cross-module workflows
- Contract tests required for public APIs and module boundaries
- Tests must be independent, repeatable, and fast
- Red-Green-Refactor cycle strictly enforced

**Rationale**: TDD ensures testable design, prevents regression, provides living documentation, and gives confidence in refactoring. Writing tests first forces clarity about requirements and design.

## Development Workflow

### Code Quality Gates

All code changes MUST pass these gates before merge:

1. **Type Check**: TypeScript strict mode (frontend), Rust compiler warnings as errors (backend)
2. **Lint**: ESLint with no errors (frontend), Clippy with no warnings (backend)
3. **Tests**: All existing tests pass + new tests for changes
4. **Format**: Code formatted with Prettier (frontend), rustfmt (backend)
5. **Build**: Clean build with no errors or warnings

### Docker-First Development

- Developers should primarily work within Docker containers or against Docker-hosted services
- Local environment setup documentation must include Docker-based instructions
- CI/CD must use the same Docker images as development
- Any service added to the application must be added to docker-compose.yml

## Governance

### Constitution Authority

This constitution supersedes all other development practices and guidelines. In case of conflict, this document takes precedence.

### Amendment Process

Constitutional amendments require:

1. Documented proposal with rationale and impact analysis
2. Team review and consensus
3. Migration plan for existing code (if applicable)
4. Update to this document with version increment
5. Sync check across all template files (.specify/templates/)

### Versioning Policy

- **MAJOR**: Breaking principle changes (e.g., removing a core principle, fundamental philosophy change)
- **MINOR**: New principles added or substantial expansion of existing principles
- **PATCH**: Clarifications, wording improvements, non-semantic refinements

### Compliance Review

- All feature specifications must include constitution compliance check
- Implementation plans must document any principle violations and justifications
- Regular audits to ensure codebase alignment with constitutional principles

### Complexity Justification

Any code that violates these principles must be explicitly justified in the implementation plan with:
- Why the violation is necessary
- What simpler alternatives were considered
- Why those alternatives are insufficient

**Version**: 1.0.0 | **Ratified**: 2025-10-18 | **Last Amended**: 2025-10-18
