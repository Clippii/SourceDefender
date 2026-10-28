# Data dictionary

| Field | Meaning | Invariant |
|---|---|---|
| id | Stable record identity | Lowercase identifier, unique |
| kind | Scenario-specific record category | Listed in config/scenario.json |
| owner | Principal that owns the record | Required verified identity |
| title | Human-readable description | 3–160 characters, normalized whitespace |
| status | Lifecycle state | pending, active, archived, cancelled |
| version | Optimistic locking token | Starts at 1, increments once per transition |
| amount | Value in integer minor currency units | Nonnegative, bounded |
| tags | Search and routing labels | Unique, sorted, at most ten |

The scenario ships with 48 records and six synthetic users. No data describes
real people. A reviewer may list all records; an ordinary viewer sees their own.
An operator creates records for their own identity. A reviewer approves pending
records or archives active records.

Invoices retain a record reference, owner, subtotal, tax, total, recipient,
currency, and status. Totals use integer minor units. Export files use CSV with
formula-prefix escaping. Each export has a list of delivery destinations.

Audit events contain action, actor, record identity, and a UTC timestamp.
Queue entries contain a job identity, kind, payload, attempt count, next retry,
state, and the most recent failure reason.
