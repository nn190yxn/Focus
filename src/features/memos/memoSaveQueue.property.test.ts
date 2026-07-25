import fc from "fast-check";
import { describe, expect, it } from "vitest";

import { LatestMemoSaveQueue } from "./memoSaveQueue";

type QueueEvent = { kind: "edit"; value: string } | { kind: "complete" };

function validatesCriteria(criteria: string[]): string {
  return `validates criteria ${criteria.join(", ")}`;
}

describe("Property M1: latest memo draft is retained", () => {
  it(`${validatesCriteria(["2.2", "2.3", "2.4", "2.5"])} across interleaved edits and save completions`, () => {
    const eventArbitrary = fc.array(
      fc.oneof(
        fc.record({ kind: fc.constant("edit" as const), value: fc.string({ maxLength: 40 }) }),
        fc.record({ kind: fc.constant("complete" as const) }),
      ),
      { minLength: 1, maxLength: 80 },
    ).filter((events) => events.some((event) => event.kind === "edit"));

    fc.assert(fc.property(eventArbitrary, (events: QueueEvent[]) => {
      const queue = new LatestMemoSaveQueue<string>();
      const started: string[] = [];
      let latestDraft = "";

      for (const event of events) {
        if (event.kind === "edit") {
          latestDraft = event.value;
          const initial = queue.enqueue(event.value);
          if (initial !== null) started.push(initial);
        } else if (queue.isActive()) {
          const next = queue.complete();
          if (next !== null) started.push(next);
        }
      }

      while (queue.isActive()) {
        const next = queue.complete();
        if (next !== null) started.push(next);
      }

      expect(started.at(-1)).toBe(latestDraft);
    }), { numRuns: 128 });
  });
});
