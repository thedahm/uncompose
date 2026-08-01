// Sequential Reviewer — implement-then-review loop
//
// Each cycle: an implementer agent picks one `ready-for-agent` issue and
// commits a fix on a dedicated branch, then a reviewer agent refines the same
// branch inside the same sandbox. The branch is pushed for a human to merge
// (see docs/agents/afk-workflow.md); the loop never merges or closes issues.
//
// Usage: npm run sandcastle

import { execSync } from "node:child_process";
import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";

// Maximum number of implement→review cycles to run before stopping.
// Each cycle works on one issue. Raise this to process more issues per run.
const MAX_ITERATIONS = 10;

// Hooks run inside the sandbox before the agent starts each iteration.
// cargo fetch warms the crate cache so the agent's first build isn't
// blocked on downloads. The engine's uv sync is left to the agent — it only
// pays that cost when an issue actually touches engine/.
const hooks = {
  sandbox: { onSandboxReady: [{ command: "cargo fetch" }] },
};

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);

  const branch = `sandcastle/sequential-reviewer/${Date.now()}`;

  // One sandbox shared by both phases, so the implementer and reviewer work
  // on the same real, named branch.
  const sandbox = await sandcastle.createSandbox({
    branch,
    sandbox: docker(),
    hooks,
  });

  try {
    // Phase 1: Implement.
    // One inner iteration so each outer pass implements a single issue on its
    // own branch, then hands it to the reviewer. A higher value lets the agent
    // drain the whole backlog onto this one branch in a single pass, which
    // defeats the per-issue review.
    const implement = await sandbox.run({
      name: "implementer",
      maxIterations: 1,
      agent: sandcastle.claudeCode("claude-opus-4-8"),
      promptFile: "./.sandcastle/implement-prompt.md",
    });

    if (!implement.commits.length) {
      // The backlog is empty, or the head issue was blocked and dequeued
      // (the prompt has the agent swap its label to needs-info without
      // committing). Rerun to continue past a dequeued issue.
      console.log("Implementation agent made no commits. Stopping.");
      break;
    }

    console.log(`\nImplementation complete on branch: ${branch}`);
    console.log(`Commits: ${implement.commits.length}`);

    // Phase 2: Review the branch produced by Phase 1, refining or correcting
    // it directly on the branch.
    await sandbox.run({
      name: "reviewer",
      maxIterations: 1,
      agent: sandcastle.claudeCode("claude-opus-4-8"),
      promptFile: "./.sandcastle/review-prompt.md",
      promptArgs: {
        BRANCH: branch,
      },
    });

    console.log("\nReview complete.");

    // Push and open the PR from the host (the sandbox has no push
    // credentials). --fill-first titles the PR from the implementer's commit.
    // Merging stays human per #26 — the loop's job ends at an open PR.
    try {
      execSync(`git push -u origin ${branch}`, { stdio: "inherit" });
      execSync(`gh pr create --head ${branch} --base main --fill-first`, {
        stdio: "inherit",
      });
    } catch {
      console.warn(
        `Push or PR creation failed — branch ${branch} is still available locally.`,
      );
    }
  } finally {
    await sandbox.close();
  }
}

console.log("\nAll done.");
