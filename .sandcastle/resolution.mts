// sandcastle-kit e2b741b — synced copy, edit in sandcastle-kit
// Pure resolution module — the seam the calibrated loop stands on.
//
// Two responsibilities, both pure (no gh, Docker, filesystem, or network):
//   1. resolveTicket() turns a ticket's GitHub labels plus the tier roster
//      into a concrete provider / model / effort. A missing label means the
//      caller's default for that dimension.
//   2. parseEvalArms() reads an `## Eval arms` issue-body section into a list
//      of tier/effort arms with optional one-line hypotheses.
//
// Ambiguity is never guessed away: conflicting labels, unknown tiers/efforts,
// a tier absent from the roster, and malformed or missing arm sections are all
// explicit refusals (ResolutionError). Callers skip the ticket with a clear
// log line rather than run something the operator did not ask for.

// The label vocabulary, ordered weakest → strongest. Labels and calibration
// entries speak only in tiers; the roster maps a tier to a provider + concrete
// model id, so a model-version bump is a one-line roster edit and no label ever
// goes stale.
//
// This one list is also the capability ranking: the eval judge runs at the
// strongest tier the roster provides, so arm comparisons stay credible and
// consistent across evals. fable ranks strongest — it is the tier this kit
// already runs its strongest fixed roles (planner, reviewer, final review) on,
// so a judge picked from it is never weaker than the reviewer it feeds.
// Keeping one ordered list means a new tier cannot be visible to labels but
// invisible to the judge.
export const TIERS = ["haiku", "sonnet", "opus", "fable"] as const;
export type Tier = (typeof TIERS)[number];

export const EFFORTS = ["low", "medium", "high", "max"] as const;
export type Effort = (typeof EFFORTS)[number];

// The label that marks a sub-issue as an eval: its arms race in parallel and a
// judge picks the winner. Visible and filterable in the GitHub UI.
export const EVAL_LABEL = "eval";

// tier -> provider + concrete model id. The provider field future-proofs for
// non-claudeCode harnesses; nothing but claudeCode ships today. A roster may
// be partial (a tier omitted); referencing an omitted tier is a refusal.
export type RosterEntry = { provider: string; model: string };
export type Roster = Partial<Record<Tier, RosterEntry>>;

export type Resolution = {
  tier: Tier;
  effort: Effort;
  provider: string;
  model: string;
};

export type ResolveInput = {
  // The ticket's GitHub label names (e.g. ["model:sonnet", "effort:high"]).
  labels: string[];
  // The single source of truth mapping tier -> provider + model.
  roster: Roster;
  // The caller's per-dimension defaults, used when a label is absent.
  defaults: { tier: Tier; effort: Effort };
};

export type EvalArm = { tier: Tier; effort: Effort; hypothesis?: string };

// A refusal: the inputs are ambiguous or invalid and resolution declines to
// guess. Thrown by both resolveTicket and parseEvalArms.
export class ResolutionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ResolutionError";
  }
}

const MODEL_PREFIX = "model:";
const EFFORT_PREFIX = "effort:";
const EVAL_ARMS_HEADING = /^#{1,6}\s+eval arms\s*$/i;
const ANY_HEADING = /^#{1,6}\s+\S/;

function isTier(value: string): value is Tier {
  return (TIERS as readonly string[]).includes(value);
}

function isEffort(value: string): value is Effort {
  return (EFFORTS as readonly string[]).includes(value);
}

// Distinct values carried by labels sharing a prefix, preserving first-seen
// order. Two distinct values for the same dimension is a conflict; a repeated
// identical value is not.
function distinctLabelValues(labels: string[], prefix: string): string[] {
  const seen: string[] = [];
  for (const label of labels) {
    if (!label.startsWith(prefix)) continue;
    const value = label.slice(prefix.length).trim();
    if (value.length > 0 && !seen.includes(value)) seen.push(value);
  }
  return seen;
}

// Pick the single value for one dimension, or fall back to the default.
// Refuses on conflicting labels or a value outside the valid set.
function pickDimension<T extends string>(
  labels: string[],
  prefix: string,
  fallback: T,
  dimension: string,
  valid: readonly T[],
): T {
  const values = distinctLabelValues(labels, prefix);
  if (values.length > 1) {
    throw new ResolutionError(
      `conflicting ${dimension} labels: ${values.join(", ")}. A ticket may carry at most one ${prefix} label.`,
    );
  }
  const chosen = values.length === 1 ? values[0]! : fallback;
  if (!(valid as readonly string[]).includes(chosen)) {
    const source = values.length === 1 ? "label" : "default";
    throw new ResolutionError(
      `unknown ${dimension} "${chosen}" (from ${source}). Valid values: ${valid.join(", ")}.`,
    );
  }
  return chosen as T;
}

// Look up a tier's roster entry, or refuse if the roster omits it. Shared by
// every resolver so the refusal message stays identical wherever a tier is
// missing.
function rosterEntryFor(tier: Tier, roster: Roster): RosterEntry {
  const entry = roster[tier];
  if (!entry) {
    throw new ResolutionError(
      `roster has no entry for tier "${tier}". Add it to the roster in sandcastle.config.json (tier -> { provider, model }).`,
    );
  }
  return entry;
}

/**
 * Resolve a ticket's labels + roster into a concrete provider / model / effort.
 * Absence of a `model:` or `effort:` label means the caller's default for that
 * dimension. Refuses (ResolutionError) on conflicting labels, unknown values,
 * or a resolved tier that has no roster entry.
 */
export function resolveTicket(input: ResolveInput): Resolution {
  const { labels, roster, defaults } = input;

  const tier = pickDimension(
    labels,
    MODEL_PREFIX,
    defaults.tier,
    "model tier",
    TIERS,
  );

  const effort = pickDimension(
    labels,
    EFFORT_PREFIX,
    defaults.effort,
    "effort",
    EFFORTS,
  );

  const entry = rosterEntryFor(tier, roster);
  return { tier, effort, provider: entry.provider, model: entry.model };
}

/** Does this ticket carry the `eval` label? */
export function hasEvalLabel(labels: string[]): boolean {
  return labels.includes(EVAL_LABEL);
}

/**
 * The strongest tier the roster provides, by the TIERS ranking. Refuses
 * (ResolutionError) on an empty roster — the judge needs somewhere to run.
 */
export function strongestRosterTier(roster: Roster): Tier {
  for (let i = TIERS.length - 1; i >= 0; i--) {
    const tier = TIERS[i]!;
    if (roster[tier]) return tier;
  }
  throw new ResolutionError(
    "the roster has no tiers; the eval judge needs at least one tier to run at. Add tier -> { provider, model } entries to sandcastle.config.json.",
  );
}

/**
 * Resolve the eval judge: the strongest roster tier, fixed at high effort, so
 * arm comparisons are credible and consistent across evals. Refuses on an
 * empty roster.
 */
export function resolveJudge(roster: Roster): Resolution {
  const tier = strongestRosterTier(roster);
  const entry = rosterEntryFor(tier, roster);
  return { tier, effort: "high", provider: entry.provider, model: entry.model };
}

/**
 * Resolve one eval arm's explicit tier/effort through the roster into a
 * concrete provider / model / effort. Refuses if the arm's tier has no roster
 * entry.
 */
export function resolveArm(arm: EvalArm, roster: Roster): Resolution {
  const entry = rosterEntryFor(arm.tier, roster);
  return {
    tier: arm.tier,
    effort: arm.effort,
    provider: entry.provider,
    model: entry.model,
  };
}

/**
 * The deterministic arm-suffixed branch for one arm, off the ticket's base
 * branch. Distinct tier/effort combos yield distinct branches, so parallel
 * arms never collide.
 */
export function armBranch(baseBranch: string, arm: EvalArm): string {
  return `${baseBranch}-eval-${arm.tier}-${arm.effort}`;
}

/**
 * Match a judge's free-form winner string against the branches it was allowed
 * to pick from, returning the canonical branch name or null if it names none
 * of them.
 *
 * The judge is asked for the branch verbatim, but it writes prose for a living:
 * a stray backtick, a wrapping quote, an `origin/` or `refs/heads/` prefix, or
 * surrounding whitespace should not discard a fully-judged eval. Anything
 * beyond that decoration — a blended name, a branch that was never an arm —
 * still fails, because guessing which arm was meant would corrupt exactly the
 * signal evals exist to produce.
 */
export function matchWinnerBranch(
  winner: string,
  branches: string[],
): string | null {
  const strip = (value: string) =>
    value
      .trim()
      .replace(/^[`'"*\s]+|[`'"*.,\s]+$/g, "")
      .replace(/^(?:refs\/heads\/|origin\/)+/, "");

  const target = strip(winner);
  if (target.length === 0) return null;
  return branches.find((branch) => strip(branch) === target) ?? null;
}

// One arm line: `tier/effort` optionally followed by a hypothesis after a
// separator (em dash, hyphen, or colon). List markers (- or *) are stripped.
const ARM_LINE = /^([A-Za-z0-9]+)\/([A-Za-z0-9]+)(?:\s*[—:-]\s*(\S.*?))?\s*$/;

/**
 * Parse an `## Eval arms` issue-body section into a list of arms. Each arm is a
 * `tier/effort` combo with an optional one-line hypothesis. Refuses
 * (ResolutionError) if the section is missing, has no arm lines, or contains a
 * malformed line or an unknown tier/effort.
 */
export function parseEvalArms(body: string): EvalArm[] {
  const lines = body.split(/\r?\n/);

  const headingIndex = lines.findIndex((line) =>
    EVAL_ARMS_HEADING.test(line.trim()),
  );
  if (headingIndex === -1) {
    throw new ResolutionError(
      'no "## Eval arms" section in the issue body. An eval ticket must list its arms under that heading.',
    );
  }

  const arms: EvalArm[] = [];
  for (let i = headingIndex + 1; i < lines.length; i++) {
    const trimmed = lines[i]!.trim();
    if (trimmed === "") continue;
    if (ANY_HEADING.test(trimmed)) break; // next section ends the arms block

    const stripped = trimmed.replace(/^[-*]\s+/, "");
    const match = stripped.match(ARM_LINE);
    if (!match) {
      throw new ResolutionError(
        `malformed eval arm line: "${trimmed}". Expected "tier/effort" with an optional hypothesis.`,
      );
    }

    const [, tierRaw, effortRaw, hypothesis] = match;
    if (!isTier(tierRaw!)) {
      throw new ResolutionError(
        `unknown model tier "${tierRaw}" in eval arm "${trimmed}". Valid tiers: ${TIERS.join(", ")}.`,
      );
    }
    if (!isEffort(effortRaw!)) {
      throw new ResolutionError(
        `unknown effort "${effortRaw}" in eval arm "${trimmed}". Valid efforts: ${EFFORTS.join(", ")}.`,
      );
    }

    // Arms are identified by their tier/effort combo, and armBranch() is
    // deterministic per combo — two identical arms would race two sandboxes on
    // one branch and make loser cleanup delete the wrong set. Refuse rather
    // than silently dedupe: a repeated combo means the body says something the
    // author did not mean.
    if (arms.some((a) => a.tier === tierRaw && a.effort === effortRaw)) {
      throw new ResolutionError(
        `duplicate eval arm "${tierRaw}/${effortRaw}". Each arm must be a distinct tier/effort combo.`,
      );
    }

    const arm: EvalArm = { tier: tierRaw, effort: effortRaw };
    if (hypothesis) arm.hypothesis = hypothesis;
    arms.push(arm);
  }

  if (arms.length === 0) {
    throw new ResolutionError(
      'the "## Eval arms" section has no arm lines. List each arm as "tier/effort" on its own line.',
    );
  }

  return arms;
}
