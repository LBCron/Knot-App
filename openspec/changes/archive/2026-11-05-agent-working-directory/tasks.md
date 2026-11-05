# Tasks

## 1. Agents report their working directory (#25)

- [x] 1.1 `Agent::working_dir` (runtime only), `effective_folder`, and
      `AgentStore::set_working_dir`; any restart clears it
- [x] 1.2 `set-working-directory` MCP tool: validated working-tree root, empty
      path or own folder clears, acts only as the caller
- [x] 1.3 Git panel, header, dashboard and Command Center read git from
      `effective_folder`
- [x] 1.4 Registration prompt names the tool
- [x] 1.5 Tests for the store, the tool and the window following the change
