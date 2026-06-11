# Phase 12 Vision — Native Git and Software Delivery Workflows

Phase 12 is future planning only; Bahamut does **not** currently provide these capabilities. The goal is a clean-room, local-first Git and delivery workspace that helps users inspect, stage, review, test, package and release software without surrendering control to autonomous background agents.

## Scope

- Native Git graph, branch, stash, tag, worktree and submodule inspection.
- Commit composition with explicit staged hunks and metadata-only audit records.
- Pull request and release preparation surfaces.
- CI/CD status connectors and deployment checklists.
- Policy-aware safeguards for destructive Git operations.

## Shared platform foundation

Phase 12 depends on the shared future foundation: connector SDK, metadata model, graph abstraction, scheduler, event and telemetry bus, policy engine, secrets boundary, permission model, audit system, extension API, agent tool registry, deployment modes, retention controls and interoperability formats.

## Clean-room and licence principles

Bahamut may interoperate with Git implementations and hosting APIs, but Git workflow UX and code must be implemented clean-room. Any use of Git libraries, porcelain wrappers, hosting SDKs or delivery integrations requires licence review before adoption; GPL/AGPL/SSPL, source-available, telemetry-heavy or unclear terms must be flagged before implementation.
