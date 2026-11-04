# Spec Delta

## ADDED Requirements

### Requirement: A working directory lasts until restart

An agent's working directory SHALL be kept only while Knot runs and SHALL be
cleared whenever the agent restarts, with or without its conversation, since
it starts in its folder again.

#### Scenario: Restart clears it

- **WHEN** an agent with a working directory set is restarted
- **THEN** it has no working directory, and its git is read from its folder
