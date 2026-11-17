// Plan for In-app purchases in this repo. Phases run in order for each area:
// implement, audit, unit tests, device test, vision, smoke.
// Gates on every area: cargo fmt --check, cargo clippy -D warnings, cargo test.
// Statuses reflect what ran (see purchases-run-report.md): device-test needs an
// NSH host that registers StoreKitHost/BillingHost, which lives outside this repo.

export type Phase = "implement" | "audit" | "unit-tests" | "device-test" | "vision" | "smoke";

export interface Area {
  name: string;
  phases: Phase[];
  status: Record<Phase, "not-started" | "done" | "blocked">;
}

export const workflow = {
  repo: "SuperNSH/purchases",
  gates: [
    "cargo fmt --all -- --check",
    "cargo clippy --all-targets -- -D warnings",
    "cargo test --all-targets",
  ],
  areas: [
    {
      name: "purchases",
      phases: ["implement", "audit", "unit-tests", "device-test", "vision", "smoke"],
      status: {
        "implement": "done",
        "audit": "done",
        "unit-tests": "done",
        "device-test": "blocked",
        "vision": "not-started",
        "smoke": "not-started",
      },
    },
  ] satisfies Area[],
};
