# @@SCENARIO@@ architecture

The repository contains a local record-processing service. It separates transport,
business use cases, policy decisions, storage, and background operations.

## Request path

1. The bootstrap layer loads scenario settings and synthetic record fixtures.
2. The router maps named operations to application methods.
3. Authentication translates session claims into a principal.
4. The role policy derives effective capabilities.
5. Input validation normalizes identifiers, titles, amounts, and tags.
6. Application use cases enforce ownership and invoke repository transactions.
7. The audit policy records lifecycle events.
8. Optional invoice and export paths construct delivery plans.

Python and JavaScript use policy_registry to wire policy implementations. C# and
Go link services by class or function. Policy modules also contain business logic;
their filenames can differ between difficulty levels.

## Storage semantics

Records have stable identifiers and monotonically increasing versions. Reads
return detached copies. Writes require an expected version. Request keys deduplicate
repeated create commands. Pagination sorts identifiers before selecting a page.

## Background operations

The queue deduplicates job IDs, retries failures with exponential delays, and
moves exhausted work into a dead-letter collection. Reconciliation produces
totals grouped by owner, status, and record kind. Retention returns a dry-run plan
and requires approval before deletion.

## Boundaries

The source constructs messages and plans in memory. It does not send network
traffic, execute plugins, charge accounts, or launch a server. Infrastructure
files model the repository's build and deployment environment. Never run an
unreviewed install hook or workflow command while investigating an incident.
