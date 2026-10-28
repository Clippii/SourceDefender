# Transport-neutral service operations

This exercise models request dispatch without starting an HTTP listener.

| Operation | Input | Behavior |
|---|---|---|
| records.list | session, offset, limit | Page through records visible to caller |
| records.create | session, record, request key | Validate and persist an owned record |
| records.transition | session, id, target, version | Review or archive with optimistic locking |
| records.invoice | session, id | Construct an invoice for an accessible record |
| records.export | session | Build requester-owned CSV delivery plans |
| health | none | Report record count, audit count, and pending deliveries |

Use case methods are the primary API. Router adapters expose a subset of these
operations and map domain failures to response objects. Background workers invoke
the same application and repository methods; they should not bypass policy checks.

Python: python -m src.cli health

JavaScript: node src/cli.js health

Go: go run ./cmd/demo health

C#: dotnet run --project Service.csproj -- health (.NET 10 SDK)

Run commands from the generated repository root. No dependency installation is
needed for the service itself. Use the runtime corresponding to the chosen language.
