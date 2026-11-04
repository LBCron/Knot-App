# Spec Delta

## ADDED Requirements

### Requirement: A working directory lasts until restart

An agent's working directory SHALL be kept only while Knot runs and SHALL be
cleared whenever the agent restarts, with or without its conversation, or is
deactivated, since its next session starts in its folder again.

#### Scenario: Restart clears it

- **WHEN** an agent with a working directory set is restarted
- **THEN** it has no working directory, and its git is read from its folder

#### Scenario: Deactivating clears it

- **WHEN** an agent with a working directory set is deactivated and later
  activated again
- **THEN** it has no working directory, and its git is read from its folder
