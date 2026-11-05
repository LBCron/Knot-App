# Design

**Candidate ADR.** This decides how Knot learns where an agent is working, and
the choice constrains later features that want the same answer.

## Options

- **Observe it.** Rejected: there is nothing reliable to observe. An ACP
  session's cwd is fixed at `session/new`. A `claude` process does not
  `chdir` when its shell tool runs `cd` (every agent process on a working
  machine still reports its starting directory). OSC 7 would need
  `gpui-terminal` to expose it, would cover terminal agents only, and is only
  emitted by shells configured to send it.
- **The user sets it.** Possible later, as a menu action, but the agent knows
  first and best.
- **The agent reports it** (chosen). Agents already coordinate through Knot's
  MCP server and are told to keep their status current. Reporting a working
  tree is the same kind of statement, made through the same channel.

## Decisions

- **Runtime only, with a fallback.** `working_dir` is not saved; whenever it is
  unset, Knot reads git from `Agent.folder` as before. A restart or a
  deactivation clears it, because the agent's next session starts in its
  folder. A stale value can therefore last at most until the agent restarts,
  is deactivated or reports again.
- **A working-tree root, checked.** The path must exist and be the root of a
  git working tree (`knot_git::is_working_tree`), so the panel always has a
  repository to read. A subfolder is refused with a message naming the
  requirement. `~/` is expanded, as `create-worktree` does.
- **Clearing.** An empty path, or a path equal to the agent's folder, clears
  the override, so returning home needs no special argument.
- **One reader-side accessor.** Every git reader goes through
  `Agent::effective_folder`, so no reader can keep using the starting folder.
  The caches follow a changed directory through `ReadFolders` (#371).
- **Launch is unchanged.** The working directory does not change where an
  agent's session or terminal starts. That stays `Agent.folder`.
