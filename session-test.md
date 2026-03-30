# Session Workflow Test

This file was created on branch `test/session-workflow` to demonstrate the branch session workflow.

## What happened

1. Started on `dev` (baseline)
2. Called `branch_session_start` → created `test/session-workflow`
3. Workspace automatically switched to the feature branch
4. Created this file as a test change
5. Will checkpoint, then end the session

## Expected behavior

- This file only exists on the feature branch
- Ending the session with `merge: true` merges it into `dev`
- Ending without `merge` leaves `dev` clean — this file won't be there
