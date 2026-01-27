# Wiggle Puppy Rust Implementation

You are building a Rust implementation of the Wiggle Puppy autonomous AI agent loop.

## Instructions

Each session works on **ONE story only**.

### Step 1: Find the Next Story

1. Read `prd.json` to find the first story where:
   - `"passes": false`
   - All stories in `depends_on` have `"passes": true`
2. If **every story** in `prd.json` has `"passes": true`, output `<promise>COMPLETE</promise>` and exit

### Step 2: Break Down the Story into Tasks

1. Analyze the story's acceptance criteria
2. Use `TaskCreate` to create tasks for each piece of work:
   - Implementation tasks (can often run in parallel)
   - Verification task: `cargo check && cargo test && cargo clippy -- -D warnings`
3. Use `TaskUpdate` to set dependencies between tasks (e.g., verification depends on implementation)

### Step 3: Execute Tasks with Sub-Agents

1. Use `TaskList` to find available tasks (pending, not blocked)
2. **Launch sub-agents in parallel** for all available tasks using multiple `Task` tool calls:
   - **subagent_type**: `general-purpose`
   - **prompt**: Describe the specific task, reference PROMPT.md for architecture
   - Sub-agents implement and return results - they do NOT commit
3. As sub-agents complete, use `TaskUpdate` to mark tasks `completed`
4. Repeat until all tasks are done

### Step 4: Finalize

1. Update `prd.json` to set `"passes": true` for this story
2. Commit: `git add -A && git commit -m "feat(wiggle-puppy): <story title>"`
3. Append progress to `progress.txt`
4. Exit

### Step 5: Exit

When the story is complete exit the agent run.

## Key Files

- `prd.json` - Stories to implement
- `PROMPT.md` - Architecture and file structure
- `progress.txt` - Progress log

## Sub-Agent Guidelines

- Provide complete context and reference PROMPT.md
- Sub-agents implement code and run checks
- Parent agent handles commits and prd.json updates
- Launch multiple sub-agents in parallel when tasks are independent
