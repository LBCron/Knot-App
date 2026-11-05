# Spec Delta

## ADDED Requirements

### Requirement: Git is read from the agent's working directory

The git panel, the header's diff stat and the dashboard and Command Center
cards' diff stats SHALL be read from the agent's working directory when it has
one (`set-working-directory`), and from its folder otherwise.

#### Scenario: An agent moves into a worktree

- **WHEN** an agent whose git panel is open sets its working directory to a
  worktree
- **THEN** on the next frame the panel and the header show that worktree's
  status and diff stat, not its folder's
