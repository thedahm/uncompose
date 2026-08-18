// sandcastle-kit e2b741b — synced copy, edit in sandcastle-kit
// Calibrated Planner with Review — spec-delivery orchestration loop, label-aware
//
// A second, separately-launchable loop entry point. Its flow is identical to
// the existing loop (main.mts) with two overlays:
//
//   1. Per-ticket adherence: the implementer's model and effort resolve per
//      ticket from the sub-issue's `model:<tier>` / `effort:<level>` GitHub
//      labels, via the pure resolution module and the config roster. An
//      unlabeled dimension falls back to the loop's implementer default for
//      that dimension. Planner, reviewer, merger, and final-review keep their
//      loop-configured models — adherence governs the implementer only.
//
//   2. Evals: a sub-issue carrying the `eval` label plus an `## Eval arms`
//      body section races each arm as a normal implementer in its own sandbox
//      on its own arm-suffixed branch, in parallel. Then one judge — fixed at
//      the roster's strongest tier at high effort — reads the ticket, each
//      arm's diff, and each arm's checks output, and posts a single verdict
//      comment (winner, per-arm assessment, calibration note) on the ticket.
//      The winning branch enters the completely normal reviewer → merge path
//      as if it were the only implementation; loser branches are deleted after
//      the verdict lands. The judge never merges and never edits calibration.
//      Non-eval tickets in the same run are unaffected.
//
// One run delivers one spec issue (label: "spec") by working its
// ready-for-agent sub-issues and, once they're all closed, opening a
// human-mergeable PR against main:
//   Phase 1 (Plan):             A fable agent analyzes the spec's open
//                               ready-for-agent sub-issues, builds a dependency
//                               graph, and outputs a <plan> JSON listing at most
//                               MAX_PARALLEL unblocked issues with branch names.
//   Phase 2 (Execute + Review): For each issue, a sandbox is created via
//                               createSandbox(). The implementer runs first
//                               (100 iterations) on the model/effort resolved
//                               from the ticket's labels. If it produces
//                               commits, a reviewer runs in the same sandbox on
//                               the same branch (1 iteration). All issue
//                               pipelines run concurrently via
//                               Promise.allSettled().
//   Phase 3 (Merge):            A single agent merges each completed branch
//                               into the spec branch via a PR (opened and
//                               merged by the agent, for the paper trail).
//   Phase 4 (Finalize):         Once every GitHub sub-issue of the spec is
//                               closed: open the spec PR against main
//                               (spec-pr), post a final review as a PR
//                               comment (final-reviewer), then address that
//                               review in one round (address-final-review).
//                               The spec PR is left open for a human to merge.
//
// The outer loop repeats up to MAX_ITERATIONS times so that newly unblocked
// issues are picked up after each round of merges.
//
// Usage:
//   npx tsx .sandcastle/main-calibrated.mts
// Or add to package.json:
//   "scripts": { "sandcastle:calibrated": "npx tsx .sandcastle/main-calibrated.mts" }

import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { z } from "zod";

import {
  EFFORTS,
  ResolutionError,
  type EvalArm,
  type Resolution,
  type Roster,
  armBranch,
  hasEvalLabel,
  matchWinnerBranch,
  parseEvalArms,
  resolveArm,
  resolveJudge,
  resolveTicket,
  TIERS,
} from "./resolution.mts";

// The planner emits its plan as JSON inside <plan> tags; Output.object extracts
// and validates it against this schema. We use Zod here, but any Standard
// Schema validator works just as well — Valibot, ArkType, etc. See
// https://standardschema.dev.
const planSchema = z.object({
  issues: z.array(
    z.object({ id: z.string(), title: z.string(), branch: z.string() }),
  ),
});

// The spec-pr agent emits the number of the PR it opened inside <pr> tags.
const prSchema = z.object({ number: z.number() });

// The judge emits the winning arm's branch name inside <verdict> tags; the loop
// routes that branch into the normal review/merge path and deletes the losers.
const verdictSchema = z.object({ winner: z.string() });

// ---------------------------------------------------------------------------
// Configuration
//
// Repo-specific knobs live in .sandcastle/sandcastle.config.json so this file
// stays byte-identical across repos (synced from sandcastle-kit):
//   repoChecks         — human-readable markdown describing the repo's check
//                        commands; injected into the implement/merge/
//                        address-final-review prompts as {{REPO_CHECKS}}
//   copyToWorktree     — paths copied from the host checkout into each worktree
//                        before its sandbox starts (dependency/build caches)
//   maxParallel        — optional, default 2
//   maxIterations      — optional, default 10
//   specLabel          — optional, default "spec"; the label that marks the one
//                        open spec issue this run delivers
//   roster             — tier -> { provider, model }; the single source of
//                        truth the calibrated loop resolves `model:` labels
//                        through. Shared with the estimator and judge.
//   implementerDefaults — { tier, effort } used for a dimension a ticket does
//                        not label. Default: opus / high.
//   orchestrator       — optional { tier?, effort? }; the roster tier that runs
//                        the fable seats (planner, reviewer, spec-pr,
//                        final-reviewer). Default: fable. Effort is only passed
//                        when explicitly set. SANDCASTLE_ORCHESTRATOR (a tier)
//                        and SANDCASTLE_ORCHESTRATOR_EFFORT win over the config
//                        for one-off overrides.
// ---------------------------------------------------------------------------

const rosterEntrySchema = z.object({ provider: z.string(), model: z.string() });

const configSchema = z.object({
  repoChecks: z.string(),
  copyToWorktree: z.array(z.string()),
  maxParallel: z.number().int().positive().default(2),
  maxIterations: z.number().int().positive().default(10),
  specLabel: z.string().default("spec"),
  // Keyed by arbitrary strings (not the tier enum) so a partial roster is
  // valid config: resolution refuses on a referenced tier that is absent,
  // with a clearer message than a schema error would give.
  roster: z.record(z.string(), rosterEntrySchema),
  implementerDefaults: z
    .object({ tier: z.enum(TIERS), effort: z.enum(EFFORTS) })
    .default({ tier: "opus", effort: "high" }),
  orchestrator: z
    .object({
      tier: z.enum(TIERS).default("fable"),
      effort: z.enum(EFFORTS).optional(),
    })
    .default({ tier: "fable" }),
});

const parsedConfig = configSchema.safeParse(
  JSON.parse(readFileSync("./.sandcastle/sandcastle.config.json", "utf8")),
);
if (!parsedConfig.success) {
  // The calibrated loop needs config the original loop never had (the roster
  // above all). A repo synced before this loop existed has a config without
  // it, so say what to add rather than surfacing a raw validator dump.
  const issues = parsedConfig.error.issues
    .map((issue) => `  - ${issue.path.join(".") || "(root)"}: ${issue.message}`)
    .join("\n");
  throw new Error(
    `.sandcastle/sandcastle.config.json is not valid for the calibrated loop:\n${issues}\n` +
      "Re-run bin/sync from a sandcastle-kit checkout — it adds any missing `roster` / " +
      "`implementerDefaults` keys to an existing config — or copy them from " +
      "loop/sandcastle.config.template.json by hand.",
  );
}
const config = parsedConfig.data;

const roster: Roster = config.roster;

// The orchestrator tier runs the fable seats — planner, reviewer, spec-pr,
// final-reviewer. Default fable; override per run with
// SANDCASTLE_ORCHESTRATOR=<tier> (e.g. opus, to conserve fable quota) or per
// repo via config.orchestrator. Resolves through the roster like everything
// else, so it names a tier, never a model id. Effort is only passed to the
// agent when explicitly set, keeping the unconfigured default at today's
// behavior. The merger, address, judge, and implementer seats are governed by
// their own knobs and are untouched by this one.
const orchestratorTier = (process.env.SANDCASTLE_ORCHESTRATOR ??
  config.orchestrator.tier) as (typeof TIERS)[number];
if (!TIERS.includes(orchestratorTier)) {
  throw new Error(
    `orchestrator tier "${orchestratorTier}" is not one of ${TIERS.join("|")} — fix SANDCASTLE_ORCHESTRATOR or config.orchestrator`,
  );
}
const orchestratorEntry = roster[orchestratorTier];
if (!orchestratorEntry) {
  throw new Error(
    `orchestrator tier "${orchestratorTier}" is absent from the roster in sandcastle.config.json`,
  );
}
if (orchestratorEntry.provider !== "claudeCode") {
  throw new Error(
    `orchestrator tier "${orchestratorTier}" resolves to provider "${orchestratorEntry.provider}", but the calibrated loop only drives claudeCode`,
  );
}
const orchestratorEffort = (process.env.SANDCASTLE_ORCHESTRATOR_EFFORT ??
  config.orchestrator.effort) as (typeof EFFORTS)[number] | undefined;
if (orchestratorEffort !== undefined && !EFFORTS.includes(orchestratorEffort)) {
  throw new Error(
    `orchestrator effort "${orchestratorEffort}" is not one of ${EFFORTS.join("|")}`,
  );
}
const orchestratorAgent = () =>
  sandcastle.claudeCode(
    orchestratorEntry.model,
    orchestratorEffort ? { effort: orchestratorEffort } : {},
  );
const implementerDefaults = config.implementerDefaults;

// Maximum number of plan→execute→merge cycles before stopping.
// Raise this if your backlog is large; lower it for a quick smoke-test run.
const MAX_ITERATIONS = config.maxIterations;

// Maximum number of issues worked in parallel per cycle. The planner is asked
// to select at most this many; the slice in the loop enforces it regardless.
// The env var wins over the config file for one-off overrides.
const MAX_PARALLEL = Number(
  process.env.SANDCASTLE_MAX_PARALLEL ?? config.maxParallel,
);

// The arm count an eval is expected to stay within (2, or 3 with written
// justification). Enforcement is HITL — the estimator proposes and a human
// approves the arms section — so this only governs the warning the loop prints
// when an approved body races more than this.
const ARM_CAP = 3;

// Hooks run inside the sandbox before the agent starts each iteration.
// npm install ensures the sandbox always has fresh dependencies.
const hooks = {
  sandbox: { onSandboxReady: [{ command: "npm install" }] },
};

// Copy dependency and build caches (e.g. node_modules, target) from the host
// into the worktree before each sandbox starts. Avoids a full install and a
// cold build from scratch; the hook above handles platform-specific binaries
// and any packages added since the last copy.
const copyToWorktree = config.copyToWorktree;

// ---------------------------------------------------------------------------
// Spec resolution
//
// One run delivers exactly one open issue labeled "spec". Sub-issue branches
// merge into the spec branch — today, the branch checked out when the run
// starts. (Multi-spec support would resolve a worktree + branch per spec
// here; everything downstream already keys off specBranch.)
// ---------------------------------------------------------------------------

const sh = (command: string) => execSync(command, { encoding: "utf8" }).trim();

const specBranch = sh("git rev-parse --abbrev-ref HEAD");
const repo = sh("gh repo view --json nameWithOwner --jq .nameWithOwner");

const specCandidates: { number: number; title: string }[] = JSON.parse(
  sh(
    `gh issue list --state open --label "${config.specLabel}" --json number,title`,
  ),
);
if (specCandidates.length !== 1) {
  throw new Error(
    `Expected exactly one open issue labeled "${config.specLabel}", found ${specCandidates.length}. Multi-spec runs are not supported yet.`,
  );
}
const spec = specCandidates[0]!;
console.log(`Spec: #${spec.number} ${spec.title} → branch ${specBranch}`);

// The spec's open sub-issues, via GitHub's sub-issue relationship. This is
// the truth condition for finalization: the spec PR is cut only when this
// list is empty, so sub-issues added mid-run (or left open as
// ready-for-human) block the finalize phase.
type SubIssue = {
  number: number;
  title: string;
  body: string | null;
  labels: { name: string }[];
};

const openSubIssues = (): SubIssue[] =>
  JSON.parse(
    sh(
      `gh api "repos/${repo}/issues/${spec.number}/sub_issues?per_page=100"`,
    ),
  ).filter((issue: { state: string }) => issue.state === "open");

// A ticket's labels and body. The body carries the `## Eval arms` section for
// eval tickets; the labels carry model:/effort: adherence and the `eval`
// marker. Both already ride along in the sub_issues response, so the cycle's
// single fetch serves every planned ticket; the per-issue gh call is only a
// fallback for an issue the planner named that isn't an open sub-issue.
const issueMeta = (
  subIssues: SubIssue[],
  issueId: string,
): { labels: string[]; body: string } => {
  const known = subIssues.find(
    (issue) => String(issue.number) === String(issueId),
  );
  const raw =
    known ??
    (JSON.parse(sh(`gh issue view ${issueId} --json labels,body`)) as {
      labels: { name: string }[];
      body: string | null;
    });
  return { labels: raw.labels.map((l) => l.name), body: raw.body ?? "" };
};

// The calibrated loop only drives claudeCode; a roster entry pointing at any
// other provider is a refusal, not a silent substitution. Applied uniformly to
// the implementer, every eval arm, and the judge.
const requireClaudeCode = (resolution: Resolution): Resolution => {
  if (resolution.provider !== "claudeCode") {
    throw new ResolutionError(
      `tier "${resolution.tier}" resolves to provider "${resolution.provider}", but the calibrated loop only drives claudeCode. Fix the roster in sandcastle.config.json.`,
    );
  }
  return resolution;
};

// Resolve one ticket's implementer model + effort from its GitHub labels.
// A missing model:/effort: label means the loop's implementer default for that
// dimension. Refuses (ResolutionError) on conflicting/unknown labels, a tier
// missing from the roster, or a roster provider the loop cannot drive — the
// caller skips the ticket rather than run something the operator did not ask
// for.
const resolveImplementer = (labels: string[]): Resolution =>
  requireClaudeCode(
    resolveTicket({ labels, roster, defaults: implementerDefaults }),
  );

type PlannedIssue = { id: string; title: string; branch: string };

// One arm of an eval: the declared tier/effort combo (with the hypothesis the
// judge scores it against) and the branch it ran on.
type ArmRun = { arm: EvalArm; branch: string };

// One issue's outcome from the execute phase. A normal ticket contributes the
// single branch its implementer worked; an eval ticket contributes the arms it
// raced, which the judge phase narrows to one winner.
type Executed =
  | { kind: "normal"; issue: PlannedIssue; branch: string }
  | { kind: "eval"; issue: PlannedIssue; arms: ArmRun[] };

// A ticket the loop declined to run: the refusal has already been reported, so
// the allSettled reporter below stays quiet about it instead of logging the
// same skip a second time in raw-error form.
class SkippedTicket extends Error {
  constructor(issueId: string, reason: string) {
    super(`${issueId}: ${reason}`);
    this.name = "SkippedTicket";
  }
}

// Report a refusal once and stop the ticket. A ResolutionError is the module
// saying the operator's intent is ambiguous — that is a skip with a clear log
// line, never a guess. Anything else is a real failure and propagates as-is.
// (A function declaration, not the file's usual const arrow: TypeScript only
// narrows control flow on a `never`-returning call for a declared function.)
function refuse(issueId: string, what: string, err: unknown): never {
  if (err instanceof ResolutionError) {
    console.error(`  ✗ ${issueId}: ${what} — ${err.message} Skipping ticket.`);
    throw new SkippedTicket(issueId, err.message);
  }
  throw err;
}

// The reviewer run both paths share. A normal ticket is reviewed in its
// implementer's sandbox; an eval winner in a fresh sandbox on its branch —
// same agent, prompt, and iteration budget either way, so a judged winner is
// reviewed exactly as a normal implementation would be.
const reviewerRun = (branch: string) => ({
  name: "reviewer",
  maxIterations: 1,
  agent: orchestratorAgent(),
  promptFile: "./.sandcastle/review-prompt.md",
  promptArgs: { BRANCH: branch },
});

// Run an eval ticket's arms: parse the `## Eval arms` section, resolve every
// arm through the roster, then race each as a normal implementer in its own
// sandbox on its own arm-suffixed branch (forked from the spec branch), in
// parallel. A malformed/missing arms section or an unresolvable arm refuses the
// whole ticket (ResolutionError) before any sandbox is spent — the caller skips
// it. One arm's sandbox failure does not sink the others; the judge phase
// filters to arms that actually produced work.
const runEvalArms = async (
  issue: PlannedIssue,
  body: string,
): Promise<Executed> => {
  let resolvedArms: { arm: EvalArm; resolution: Resolution; branch: string }[];
  try {
    resolvedArms = parseEvalArms(body).map((arm) => ({
      arm,
      resolution: requireClaudeCode(resolveArm(arm, roster)),
      branch: armBranch(issue.branch, arm),
    }));
  } catch (err) {
    refuse(issue.id, "eval ticket refused", err);
  }

  // An eval is a comparison, not a tournament: the estimator caps proposals at
  // 2 arms (3 with written justification) and a human approves the body. The
  // loop still races what the approved body says — overriding a human decision
  // mid-run would be worse than the spend — but it says out loud that this one
  // is over the cap, so an accidental extra arm is visible in the narration.
  if (resolvedArms.length > ARM_CAP) {
    console.warn(
      `  ⚠ ${issue.id}: ${resolvedArms.length} eval arms declared, above the cap of ${ARM_CAP}. Racing all of them — trim the ticket's "## Eval arms" section if that was not intended.`,
    );
  }

  console.log(
    `  ⚑ ${issue.id}: eval — racing ${resolvedArms.length} arm(s) in parallel:`,
  );
  for (const { arm, branch } of resolvedArms) {
    const hyp = arm.hypothesis ? ` — ${arm.hypothesis}` : "";
    console.log(`      ${arm.tier}/${arm.effort} → ${branch}${hyp}`);
  }

  const armOutcomes = await Promise.allSettled(
    resolvedArms.map(async ({ arm, resolution, branch }) => {
      console.log(
        `  ▶ ${issue.id}: arm ${arm.tier}/${arm.effort} started (${branch})`,
      );
      const sandbox = await sandcastle.createSandbox({
        branch,
        baseBranch: specBranch,
        sandbox: docker(),
        hooks,
        copyToWorktree,
      });
      try {
        await sandbox.run({
          name: `implementer:${arm.tier}/${arm.effort}`,
          maxIterations: 100,
          agent: sandcastle.claudeCode(resolution.model, {
            effort: resolution.effort,
          }),
          promptFile: "./.sandcastle/implement-prompt.md",
          promptArgs: {
            TASK_ID: issue.id,
            ISSUE_TITLE: issue.title,
            BRANCH: branch,
            REPO_CHECKS: config.repoChecks,
          },
        });
      } finally {
        await sandbox.close();
      }
      console.log(
        `  ✔ ${issue.id}: arm ${arm.tier}/${arm.effort} complete (${branch})`,
      );
    }),
  );

  for (const [i, outcome] of armOutcomes.entries()) {
    if (outcome.status === "rejected") {
      const { arm, branch } = resolvedArms[i]!;
      console.error(
        `  ✗ ${issue.id}: arm ${arm.tier}/${arm.effort} (${branch}) failed: ${outcome.reason}`,
      );
    }
  }

  return {
    kind: "eval",
    issue,
    arms: resolvedArms.map(({ arm, branch }) => ({ arm, branch })),
  };
};

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

// The last post-merge view of the spec's open sub-issues, or null when the
// cycle ended without one (nothing planned, nothing merged, a merger blocker).
// The finalize check below reuses it when it is fresh and fetches otherwise.
let stillOpen: SubIssue[] | null = null;

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);
  stillOpen = null;

  // -------------------------------------------------------------------------
  // Phase 1: Plan
  //
  // The planning agent reads the spec's open ready-for-agent sub-issues,
  // builds a dependency graph, and selects the issues that can be worked in
  // parallel right now (i.e., no blocking dependencies on other open
  // sub-issues).
  //
  // It outputs a <plan> JSON block — Output.object parses and validates it.
  // -------------------------------------------------------------------------
  const plan = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "planner",
    // One iteration is enough: the planner just needs to read and reason,
    // not write code. (Structured output requires maxIterations: 1.)
    maxIterations: 1,
    // Fable for planning: dependency analysis over a read-only pass is exactly
    // the shape it is fast and cheap at. The label roster governs the
    // implementer only — the planner keeps its loop-configured model.
    agent: orchestratorAgent(),
    promptFile: "./.sandcastle/plan-prompt.md",
    // Scope candidates to this spec's ready-for-agent sub-issues: the prompt's
    // embedded query fetches `repos/{repo}/issues/{spec}/sub_issues`, so the
    // spec issue itself (which also carries ready-for-agent), other specs'
    // tickets, and stray labeled backlog issues can never be planned.
    promptArgs: {
      MAX_PARALLEL: String(MAX_PARALLEL),
      REPO: repo,
      SPEC_ISSUE: String(spec.number),
    },
    // Extract and validate the <plan> JSON into a typed object. Throws
    // StructuredOutputError if the tag is missing, the JSON is malformed, or
    // validation fails — which aborts the loop.
    output: sandcastle.Output.object({ tag: "plan", schema: planSchema }),
  });

  // The prompt asks for at most MAX_PARALLEL issues; enforce it here too.
  const issues = plan.output.issues.slice(0, MAX_PARALLEL);

  if (issues.length === 0) {
    // No agent-workable issues — either the spec is done (the finalize check
    // below the loop decides) or what remains needs a human first.
    console.log("No unblocked issues to work on.");
    break;
  }

  console.log(
    `Planning complete. ${issues.length} issue(s) to work in parallel:`,
  );
  for (const issue of issues) {
    console.log(`  ${issue.id}: ${issue.title} → ${issue.branch}`);
    // Claim the issue (same convention wayfinder uses): assign it to the
    // authenticated account so in-flight work is visible on the board.
    sh(`gh issue edit ${issue.id} --add-assignee "@me"`);
  }

  // -------------------------------------------------------------------------
  // Phase 2: Execute + Review
  //
  // For each issue, create a sandbox via createSandbox() so the implementer
  // and reviewer share the same sandbox instance per branch. The implementer
  // runs first on the model/effort resolved from the ticket's labels; if it
  // produces commits, the reviewer runs in the same sandbox.
  //
  // Promise.allSettled means one failing pipeline doesn't cancel the others.
  // A ticket whose labels won't resolve is skipped here (rejected), never
  // silently run on a guessed model.
  // -------------------------------------------------------------------------

  // One fetch of the spec's open sub-issues serves every planned ticket's
  // labels and body this cycle (the sub_issues response carries both).
  const subIssues = openSubIssues();

  const settled = await Promise.allSettled(
    issues.map(async (issue: PlannedIssue): Promise<Executed> => {
      const { labels, body } = issueMeta(subIssues, issue.id);

      // Eval tickets take the arm-racing overlay; every other ticket is the
      // normal label-adhering implement → review path, unchanged.
      if (hasEvalLabel(labels)) {
        return runEvalArms(issue, body);
      }

      // Resolve model/effort from the ticket's labels before spending a
      // sandbox on it. A ResolutionError here skips the ticket.
      let resolution: Resolution;
      try {
        resolution = resolveImplementer(labels);
      } catch (err) {
        refuse(issue.id, "cannot resolve implementer model/effort", err);
      }

      console.log(
        `  ▶ ${issue.id}: implementer = ${resolution.tier}/${resolution.effort} (${resolution.model})`,
      );

      const sandbox = await sandcastle.createSandbox({
        branch: issue.branch,
        sandbox: docker(),
        hooks,
        copyToWorktree,
      });

      try {
        // Run the implementer on the resolved model + effort. This is the one
        // behavioral difference from the existing loop.
        const implement = await sandbox.run({
          name: "implementer",
          maxIterations: 100,
          agent: sandcastle.claudeCode(resolution.model, {
            effort: resolution.effort,
          }),
          promptFile: "./.sandcastle/implement-prompt.md",
          promptArgs: {
            TASK_ID: issue.id,
            ISSUE_TITLE: issue.title,
            BRANCH: issue.branch,
            REPO_CHECKS: config.repoChecks,
          },
        });

        // Only review if the implementer produced commits
        if (implement.commits.length > 0) {
          await sandbox.run(reviewerRun(issue.branch));
        }
      } finally {
        await sandbox.close();
      }

      return { kind: "normal", issue, branch: issue.branch };
    }),
  );

  // Log any agents that threw (network error, sandbox crash, etc.). A
  // SkippedTicket is a refusal that already printed its own line above.
  for (const [i, outcome] of settled.entries()) {
    if (
      outcome.status === "rejected" &&
      !(outcome.reason instanceof SkippedTicket)
    ) {
      console.error(
        `  ✗ ${issues[i]!.id} (${issues[i]!.branch}) failed: ${outcome.reason}`,
      );
    }
  }

  // Only pass branches with unmerged work to the merge phase. Gate on the
  // branch being ahead of the spec branch, not on commits made during this
  // run: a reused branch can carry commits from an earlier run (e.g. a run
  // whose merge phase failed), and those still need merging even when the
  // implementer verified the work and finished without committing anything
  // new.
  const branchAhead = (branch: string): number => {
    try {
      return Number(sh(`git rev-list --count "${specBranch}".."${branch}"`));
    } catch {
      return 0; // branch doesn't exist on the host — nothing to merge
    }
  };

  const executed: Executed[] = settled.flatMap(
    (outcome: PromiseSettledResult<Executed>) =>
      outcome.status === "fulfilled" ? [outcome.value] : [],
  );

  // -------------------------------------------------------------------------
  // Phase 2.5: Judge evals
  //
  // Runs after execute and before merge, when nothing else touches git. For
  // each eval ticket, one judge — the strongest roster tier at high effort —
  // reads the ticket, each arm's diff, and each arm's checks output, and posts
  // a single verdict comment (winner, per-arm assessment, calibration note) on
  // the ticket. The winner then enters the completely normal reviewer path;
  // loser branches are deleted only after the verdict comment has recorded
  // their outcome. Downstream phases are eval-unaware: a judged winner reaches
  // merge as an ordinary completed branch, indistinguishable from a normal one.
  //
  // `completed` is the merge worklist: one entry per issue, each carrying the
  // single branch to land (a normal branch, or an eval's winning arm).
  // -------------------------------------------------------------------------
  const completed: { issue: PlannedIssue; branch: string }[] = [];

  for (const result of executed) {
    if (result.kind === "normal") {
      completed.push({ issue: result.issue, branch: result.branch });
      continue;
    }

    // Only arms that actually produced work can be judged or merged.
    const viable = result.arms.filter(({ branch }) => branchAhead(branch) > 0);
    if (viable.length === 0) {
      console.error(
        `  ✗ ${result.issue.id}: no eval arm produced commits — nothing to judge. Skipping ticket.`,
      );
      continue;
    }
    const viableBranches = viable.map(({ branch }) => branch);

    const judge = requireClaudeCode(resolveJudge(roster));
    console.log(
      `  ⚖ ${result.issue.id}: judging ${viable.length} arm(s) on ${judge.tier}/${judge.effort} (${judge.model})`,
    );

    // A judge failure (a malformed <verdict>, a sandbox that died) costs this
    // one eval ticket, not the run: normal tickets already implemented this
    // cycle still reach the merge phase. The spec-branch restore happens either
    // way — the judge checks arm branches out in the host working tree to run
    // their checks, and leaving the host on an arm branch would misdirect the
    // next cycle's worktrees and block the losing branches from being deleted.
    let verdictWinner: string;
    try {
      const verdict = await sandcastle.run({
        hooks,
        sandbox: docker(),
        name: "judge",
        maxIterations: 1,
        agent: sandcastle.claudeCode(judge.model, { effort: judge.effort }),
        promptFile: "./.sandcastle/judge-prompt.md",
        promptArgs: {
          ISSUE_ID: result.issue.id,
          SPEC_BRANCH: specBranch,
          // A markdown list of the viable arms, one `branch — hypothesis` per
          // line: the hypothesis is what the judge scores the arm against and
          // what the calibration note is written from.
          ARMS: viable
            .map(
              ({ arm, branch }) =>
                `- ${branch} — ${arm.hypothesis ?? `${arm.tier}/${arm.effort} (no hypothesis given)`}`,
            )
            .join("\n"),
          REPO_CHECKS: config.repoChecks,
        },
        output: sandcastle.Output.object({
          tag: "verdict",
          schema: verdictSchema,
        }),
      });
      verdictWinner = verdict.output.winner;
    } catch (err) {
      console.error(
        `  ✗ ${result.issue.id}: judge failed (${err}). Skipping merge for this ticket; arm branches left for a human.`,
      );
      continue;
    } finally {
      sh(`git checkout "${specBranch}"`);
    }

    // The judge is asked for the branch verbatim; tolerate the decoration prose
    // attracts (backticks, quotes, an `origin/` prefix) rather than discard a
    // fully-judged eval over a formatting slip. Anything else is still a
    // refusal — guessing which arm was meant would corrupt the calibration
    // signal the eval exists to produce.
    const winner = matchWinnerBranch(verdictWinner, viableBranches);
    if (!winner) {
      console.error(
        `  ✗ ${result.issue.id}: judge returned winner "${verdictWinner}", not one of the viable arm branches (${viableBranches.join(", ")}). Skipping merge for this ticket; branches left for a human.`,
      );
      continue;
    }
    console.log(`  ⚖ ${result.issue.id}: winner = ${winner}`);

    // The winner enters the completely normal reviewer path, as if it were the
    // only implementation for the ticket.
    const reviewSandbox = await sandcastle.createSandbox({
      branch: winner,
      sandbox: docker(),
      hooks,
      copyToWorktree,
    });
    try {
      await reviewSandbox.run(reviewerRun(winner));
    } finally {
      await reviewSandbox.close();
    }

    // Delete the losing branches — only now, after the verdict comment has
    // recorded their outcome. Arm branches are local-only (only the merger
    // pushes), so the local delete is the point. The remote delete is a guard
    // for a branch a prior run happened to push, and runs only when this
    // checkout has a remote-tracking ref for it — a push from here creates one,
    // so the guard costs no round trip in the overwhelmingly common case where
    // the arm never left the host.
    const losers = result.arms
      .map(({ branch }) => branch)
      .filter((branch) => branch !== winner);
    for (const loser of losers) {
      try {
        sh(`git branch -D "${loser}"`);
      } catch {
        // never created locally (the arm failed before committing) — nothing
      }
      try {
        sh(`git rev-parse --verify --quiet "refs/remotes/origin/${loser}"`);
        sh(`git push origin --delete "${loser}"`);
      } catch {
        // no remote-tracking ref (the expected case), or the remote branch was
        // already gone — either way there is nothing left to delete
      }
      console.log(`  🗑 ${result.issue.id}: deleted losing branch ${loser}`);
    }

    completed.push({ issue: result.issue, branch: winner });
  }

  const mergeable = completed.filter((c) => branchAhead(c.branch) > 0);
  const completedIssues = mergeable.map((c) => c.issue);
  const completedBranches = mergeable.map((c) => c.branch);

  console.log(
    `\nExecution complete. ${completedBranches.length} branch(es) with unmerged commits:`,
  );
  for (const branch of completedBranches) {
    console.log(`  ${branch}`);
  }

  if (completedBranches.length === 0) {
    // No branch is ahead of the spec branch — nothing to merge this cycle.
    console.log("No unmerged commits on any branch. Nothing to merge.");
    continue;
  }

  // -------------------------------------------------------------------------
  // Phase 3: Merge
  //
  // One agent merges each completed branch into the spec branch via a PR it
  // opens and immediately merges (paper trail on GitHub). Conflicts are
  // resolved locally on the sub-issue branch, pushed, then the PR is merged.
  //
  // The {{BRANCHES}} and {{ISSUES}} prompt arguments are lists that the agent
  // uses to know which branches to merge and which issues to close.
  // -------------------------------------------------------------------------
  const merge = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "merger",
    maxIterations: 1,
    agent: sandcastle.claudeCode("claude-opus-4-8"),
    promptFile: "./.sandcastle/merge-prompt.md",
    promptArgs: {
      SPEC_BRANCH: specBranch,
      // A markdown list of branch names, one per line.
      BRANCHES: completedBranches.map((b) => `- ${b}`).join("\n"),
      // A markdown list of issue IDs and titles, one per line.
      ISSUES: completedIssues.map((i) => `- ${i.id}: ${i.title}`).join("\n"),
      REPO_CHECKS: config.repoChecks,
    },
  });

  if (!merge.completionSignal) {
    // The merger hit a blocker (auth, unresolvable conflict, …) and did not
    // finish. Replanning would just re-run the same doomed merge, so stop
    // and let a human look at the merger log.
    console.error(
      "\nMerger did not signal completion — see .sandcastle/logs/ for the blocker. Stopping.",
    );
    break;
  }

  console.log("\nBranches merged.");

  // This post-merge fetch is also the finalize check's input when the loop
  // breaks here, so the two never ask GitHub the same question twice in a row.
  stillOpen = openSubIssues();
  if (stillOpen.length === 0) {
    console.log("All spec sub-issues closed. Moving to finalize.");
    break;
  }
}

// ---------------------------------------------------------------------------
// Phase 4: Finalize
//
// Only when every GitHub sub-issue of the spec is closed: open the spec PR
// against main, review it, address the review. Otherwise report what's still
// open so a human can triage (label ready-for-agent, or close) and rerun.
// ---------------------------------------------------------------------------

const remaining = stillOpen ?? openSubIssues();

if (remaining.length > 0) {
  console.log(`\nSpec #${spec.number} not finalized — open sub-issues remain:`);
  for (const issue of remaining) {
    const labels = issue.labels.map((l) => l.name).join(", ") || "no labels";
    console.log(`  #${issue.number} ${issue.title} [${labels}]`);
  }
  console.log(
    "Label them ready-for-agent (or close them) and rerun to finalize.",
  );
} else {
  console.log(`\nAll sub-issues of spec #${spec.number} closed. Finalizing.`);

  // Open the spec PR against main. fable-5: writing a human-mergeable PR
  // title/description well is a comprehension-and-judgment task.
  const specPr = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "spec-pr",
    maxIterations: 1,
    agent: orchestratorAgent(),
    promptFile: "./.sandcastle/spec-pr-prompt.md",
    promptArgs: {
      SPEC_ISSUE: String(spec.number),
      SPEC_TITLE: spec.title,
      SPEC_BRANCH: specBranch,
    },
    output: sandcastle.Output.object({ tag: "pr", schema: prSchema }),
  });
  const prNumber = String(specPr.output.number);
  console.log(`Spec PR #${prNumber} opened.`);

  // Final review: skill-first (/code-review), posted as a PR comment.
  const review = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "final-reviewer",
    maxIterations: 1,
    agent: orchestratorAgent(),
    promptFile: "./.sandcastle/final-review-prompt.md",
    promptArgs: {
      PR_NUMBER: prNumber,
      SPEC_ISSUE: String(spec.number),
      SPEC_BRANCH: specBranch,
    },
  });
  if (!review.completionSignal) {
    throw new Error(
      "Final reviewer did not signal completion — the review comment may be missing. Not running the address step; check .sandcastle/logs/.",
    );
  }
  console.log("Final review posted.");

  // Address the review in a single round; the PR then waits for a human.
  const addressed = await sandcastle.run({
    hooks,
    sandbox: docker(),
    name: "address-final-review",
    maxIterations: 30,
    agent: sandcastle.claudeCode("claude-opus-5"),
    promptFile: "./.sandcastle/address-final-review-prompt.md",
    promptArgs: {
      PR_NUMBER: prNumber,
      SPEC_BRANCH: specBranch,
      REPO_CHECKS: config.repoChecks,
    },
  });
  if (!addressed.completionSignal) {
    console.error(
      `address-final-review did not signal completion — PR #${prNumber} may have unaddressed feedback. Check .sandcastle/logs/.`,
    );
  } else {
    console.log(
      `Review addressed. PR #${prNumber} is ready for human review and merge.`,
    );
  }
}

console.log("\nAll done.");
