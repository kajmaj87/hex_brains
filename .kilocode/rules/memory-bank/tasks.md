## After completing any code changes
- Whenever you finish a task that involves code changes you **must** run `./verify.sh` before reporting to user. If the script fails you **must** fix the issues immediately and rerun `./verify.sh` until it passes completely.
- Optionally, you can run `./verify.sh` during the job to check progress and catch issues early.
- Do not run the gui to test unless you absolutely have to. In such case use timout N before the command to make sure it quits on its own so it does not stop your workflow.