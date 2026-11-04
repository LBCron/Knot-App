# Proposal

## Why

An agent that moves into a git worktree to do its work keeps being shown, in
the git panel and the header's diff stat, as if it were still in the folder it
started in (#25). Every git read uses `Agent.folder`, and Knot has no way to
see the move: an ACP session's cwd is fixed when it opens, a `cd` inside an
agent's tools does not change the agent process's own directory, and the
terminal's OSC 7 is not tracked (the Swift reference only logged it). The
agent has to say where it is working.

## What Changes

- A new MCP tool, `set-working-directory {agentId, path}`, sets the calling
  agent's working directory to `path`, which must be the root of a git
  working tree. An empty `path`, or the agent's own folder, clears it.
- The working directory is runtime only: never saved, and cleared whenever the
  agent restarts, since a restarted agent starts in its folder again.
- The git panel, the header's diff stat and the dashboard and Command Center
  cards read git from the working directory when one is set, and from the
  agent's folder otherwise. #371 already makes those reads drop what they
  read before on the frame the directory changes.
- The registration prompt tells agents to call the tool when they start and
  stop working in a different working tree.

## Impact

- `knot-agents`: `Agent::working_dir`, `Agent::effective_folder`,
  `AgentStore::set_working_dir`; restarts clear it.
- `knot-mcp-tools`: `set-working-directory`.
- `knot-agent-launch`: one sentence in the registration prompt.
- `knot`: git readers and the header use `effective_folder`.

Candidate ADR: an agent's working location becomes something the agent
reports, not something Knot derives. See `design.md`.
