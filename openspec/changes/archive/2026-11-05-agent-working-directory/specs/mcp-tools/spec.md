# Spec Delta

## MODIFIED Requirements

### Requirement: Tool catalog

`tools/list` SHALL return exactly these tools: `register-agent`,
`list-agents`, `describe-agents`, `send-message`, `check-messages`,
`broadcast-message`, `list-repos`, `list-worktrees`, `create-agent`,
`close-agent`, `create-worktree`, `set-status`, `set-working-directory`,
`display-markdown`, `view-mermaid`, `show-checklist`, `get-checklist`,
`plan-tasks`, `dispatch-task`, `complete-task`, `task-status`. Every tool
SHALL declare a JSON object input schema with typed properties and a required
list.

#### Scenario: Catalog is complete and stable

- **WHEN** `tools/list` is called
- **THEN** every tool listed is present, each with a name, description, and
  object input schema

## ADDED Requirements

### Requirement: set-working-directory

`set-working-directory` SHALL require the caller's `agentId` and a `path`
string. A non-empty `path` SHALL be the root of an existing git working tree
(a leading `~/` is the home folder) and SHALL become the agent's working
directory; any other path SHALL be refused with an error saying why, leaving
the working directory unchanged. An empty `path`, or the agent's own folder,
SHALL clear it. The working directory SHALL NOT change where the agent's
session runs.

#### Scenario: Entering a worktree

- **WHEN** an agent calls `set-working-directory` with the root of a worktree
- **THEN** that path becomes its working directory

#### Scenario: A path that is not a working-tree root

- **WHEN** an agent calls `set-working-directory` with a folder that does not
  exist, or one that is not the root of a git working tree
- **THEN** the call is an error and the working directory is unchanged

#### Scenario: Returning home

- **WHEN** an agent calls `set-working-directory` with an empty `path`, or
  with its own folder
- **THEN** it has no working directory, and its folder is read again
