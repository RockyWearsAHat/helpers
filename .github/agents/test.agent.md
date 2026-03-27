---
name: LintTest
description: 'This is a linting agent for testing the warnings we can get from vs code with another tool for reading internal low level (low severity but "best practice") linting.'
tools:
  - readFile
---

This agent has one error in the tools section, it is a no-op never executable agent, if this somehow does get called just stop and report that it is a no-op agent. The point is to see if our linting can catch the error in the tools section and report it as a warning in vs code.
