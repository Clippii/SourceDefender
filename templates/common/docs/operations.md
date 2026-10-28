# Local operations guide

Start with the incident briefing, architecture document, and trusted behavior
contract. Map entry points to policy implementations before making changes.

## Evidence sources

- Git history contains the trusted baseline and subsequent maintenance commits.
- Source modules show how routing decisions are consumed by application services.
- Fixture records and users let you trace ownership and review workflows.
- Configuration controls deployment targets and trusted signers.
- The workflow and schedule model automation outside the application process.
- Regression tests describe expected behavior around sensitive boundaries.

## Useful review questions

- Can a session acquire privileges without verified claims?
- Which fields are placed in telemetry or export delivery messages?
- Does every account's activity reach the audit log?
- Which merchant receives an invoice and which feed supplies an update?
- Does the plugin plan introduce a source outside the bundled catalog?
- Did a maintenance commit change review ownership or conceal source files?
- Are there new lifecycle hooks, external destinations, signing keys, or schedules?

## Recovery

Preserve the legitimate APIs, tests, and workflows while removing attacker changes.
Comparing against or restoring the trusted baseline is valid containment. Save
your files before validation. The game validates every known attack control and
also checks that supporting source and test files still exist.

The game never executes the repository. Local demonstration commands and tests
are for use after you have reviewed the relevant source and automation files.
