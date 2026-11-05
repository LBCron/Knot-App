---
id: ADR-0005
title: Agents report their working directory
status: accepted
date: 2026-11-05
tags: [agents, mcp, git]
---

# ADR-0005: Agents report their working directory

## Context

Issue #25: an agent that moves into a git worktree to do its work kept being
shown, in the git panel and the header's diff stat, as if it were still in the
folder it started in. Every git read used `Agent.folder`, the directory the
agent was created with.

Knot has nothing reliable to watch for the move:

- An ACP session's cwd is fixed at `session/new`. The protocol has no way to
  change it later.
- A `cd` run by an agent's shell tool does not change the agent process's own
  directory. On a busy machine, every `claude` process still reports the
  directory it started in.
- OSC 7 (the terminal's "current directory" escape) is not tracked.
  `gpui-terminal` would have to expose it, it would cover terminal agents
  only, and only shells configured to send it emit it. The Swift reference
  only logged it.

Agents already coordinate through Knot's MCP server, and the registration
prompt already asks them to keep their status current.

## Decision

An agent's working location is something the agent reports to Knot, not
something Knot derives. Agents report it with the `set-working-directory
{agentId, path}` MCP tool (#375).

- `path` must exist and be the root of a git working tree
  (`knot_git::is_working_tree`: the folder holds a `.git`). A leading `~/` is
  expanded the same way `create-worktree` does it. Relative paths, missing
  paths and subfolders are refused with a reason.
- The tool accepts **any** git working-tree root: a linked worktree, a plain
  clone, or another checkout of the same repository. It does not limit the
  path to worktrees this agent created with `create-worktree` or to
  repositories under the agent's folder. Knot keeps no record of who owns a
  worktree, and the agent is the one that knows where its work happens.
- An empty `path`, or the agent's own folder however it is spelled, clears the
  override.
- The tool acts only for the caller. It takes `agentId`, so the existing
  identity check covers it.
- The registration prompt tells agents to call the tool when they start, and
  when they stop, working in a different working tree.

## Options considered

- **Observe it** (process cwd, ACP session cwd, OSC 7). Rejected: none of
  these signals moves when the agent moves, as described in Context. OSC 7
  would also miss every ACP agent.
- **The user sets it** from a menu action. Not chosen now: the agent knows
  first and knows best, and a manual setting goes stale as soon as the agent
  moves again. It remains possible later as an addition, not a replacement.
- **Restrict the path to the agent's own worktrees** (those it created, or
  those under its folder). Rejected: Knot does not track worktree ownership.
  Agents also work in checkouts that Knot did not create, such as a worktree
  that Knot Master or the user set up for the task. The check that matters to
  the readers is "this is a repository I can read", and the working-tree-root
  check is exactly that.
- **The agent reports it through MCP** (chosen). It is the same kind of
  statement as `set-status`, made through the same channel.

## Consequences

- **Runtime only.** `Agent::working_dir` is never saved. Any restart clears
  it (in `recreate_terminal`), and so does deactivation, because the agent's
  next session starts in its folder again. Duplicates and forks start without
  one. A stale value therefore lasts at most until the agent restarts, is
  deactivated, or reports again. After a Knot restart, an agent that is still
  in a worktree shows its folder until it reports again.
- **Two kinds of reader.** `Agent::effective_folder()` returns `working_dir`
  when it is set and `folder` otherwise. Everything that describes where an
  agent *works* reads `effective_folder()`: the git panel, the header's diff
  stat and folder label, dashboard cards and Command Center cards.
  Everything that describes where an agent *lives* keeps reading `folder`: the
  sidebar row's folder name, the new-agent prefill, the workspace OpenSpec scan
  and the launch path. A new git reader must pick one deliberately, and
  `effective_folder()` is the default for anything that shows repository
  state.
- **The session does not move.** The working directory does not change where
  an agent's ACP session or terminal starts. That stays `Agent.folder`. Moving
  a live session would mean a new `session/new`, which loses context.
- **Repainting reuses existing paths.** `AgentStore::set_working_dir` revises
  the store only on a real change, so windows repaint through the same
  store-revision poll that `set-status` uses. #371's `ReadFolders` drops stale
  git reads and moves the panel's watch on the frame the directory changes.
- **Trust in the report.** Knot shows whatever working tree the agent names.
  An agent that never reports, or reports a tree it isn't working in, shows
  the wrong repository. That is the same trust Knot already places in
  `set-status`.
- **Reversal.** Deriving the location later (for example, if ACP adds a way
  to change the cwd) would replace the source of `working_dir`, not its
  readers. The `effective_folder()` split stays valid either way.
