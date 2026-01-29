---
description: Design for future automated AI implementation workflow
---

# Future Workflow: Automated Issue Implementation

> **Status**: Conceptual / Roadmap
> **Goal**: Reduce maintainer latency by automatically drafting implementation PRs for well-defined Domain Expert issues.

## Concept

We aim to deploy a GitHub Action `on: issue_comment` that triggers an AI agent to attempt a solution for the issue.

### 1. Trigger & Approval (The "Blessing")
**Recommendation**: The approval should happen on the **Issue**, before any code is written. This ensures the "Context Packet" is complete and worthy of implementation.

**Mechanism**:
1. **Triage**: A maintainer reviews the issue.
2. **Blessing**: If valid, they apply a label (e.g., `status: ready-for-implementation`).
3. **Execution**: A slash command (`/implement`) is used to kick off the agent.
   * *Constraint*: The workflow requires BOTH the label and the command (or the command checks for maintainer permissions) to proceed. This prevents unauthorized usage.

### 2. Context Gathering
The workflow script (`scripts/agent/gather_context.ts`) will:
- Scrape the Issue Body (parsing the "Domain Context", "Examples", and "Reference Materials").
- Fetch linked valid URLs (converting HTML to Markdown).
- Read the current `csln_core` schema and `README.md`.

### 3. Agent Execution
The system invokes the AI Model (e.g. Gemini Pro / Claude 3.5 Sonnet) with the **Systems Architect** and **Domain Expert** personas.

**System Prompt Strategy**:
> "You are an expert systems architect for CSLN. You have received a request from a Domain Expert.
> 1. Analyze the 'Real-World Examples' and 'Reference Materials'.
> 2. Determine if this requires a Schema change (modifying `csln_core/src/options.rs`) or just a Logic change (`csln_processor`).
> 3. Implement the minimal Rust code and a Regression Test in `tests/`.
> 4. Output a git patch."

### 4. Validation & PR
- The workflow applies the patch.
- Runs `cargo test`.
- Runs `node scripts/oracle.js` (if applicable).
- If successful, opens a **Draft PR** linked to the issue:
  > **Title**: `feat(ai): [Issue Title]`
  > **Body**: "Automatically generated implementation based on issue #123. \n\n**Human Review Required**: Verify edge cases in `tests/new_test.rs`."

## Roadmap Steps
1. [ ] Create `scripts/agent/` directory for context gathering scripts.
2. [ ] Define the "Meta-Prompt" that combines the implementation task with project rules.
3. [ ] Set up the GitHub Action YAML with access to an LLM API key.
