# Spec Delta

## ADDED Requirements

### Requirement: A terminal agent's usage-limit notice is a session limit

When a Terminal-mode Claude agent's output has a line opening with a usage
limit notice, the system SHALL show the agent's Session limit status, with the
reset time the notice gives, and SHALL keep it through further output and hook
reports until the user submits a prompt.

#### Scenario: The wrap-up allowance

- **WHEN** a Terminal-mode Claude agent prints "Usage limit reached · wrapping
  up" and goes on working, then stops
- **THEN** its dot turns red with "Session limit" when the notice appears, and
  stays so while it works and after it stops

#### Scenario: The next prompt lifts it

- **WHEN** the user submits a prompt to that agent
- **THEN** the agent is Working, and no longer shows "Session limit"

#### Scenario: Prose about limits

- **WHEN** an agent's output mentions "usage limit reached" mid-line
- **THEN** its status is unaffected

### Requirement: Autopilot does not answer a limited agent

The system SHALL NOT classify the closing turn of an agent showing a session
limit.

#### Scenario: The wrap-up's Stop

- **WHEN** a limited agent's wrap-up ends in a Stop with Autopilot enabled
- **THEN** no classification starts
