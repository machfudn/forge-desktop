import type { CommitInfo } from "../types/git";

export interface GraphSegment {
  fromLane: number;
  toLane: number;
  colorIndex: number;
  kind: "vertical" | "branch-in" | "merge-out";
}

export interface GraphRowLayout {
  lane: number;
  colorIndex: number;
  maxLanes: number;
  incomingLanes: number[]; // lanes passing from top to node center
  outgoingLanes: number[]; // lanes passing from node center to bottom
  segments: GraphSegment[];
}

export const GRAPH_COLORS = [
  "#58a6ff",
  "#3fb950",
  "#bc8cff",
  "#f0883e",
  "#f778ba",
  "#79c0ff",
  "#d29922",
  "#56d364",
];

/**
 * Computes GitKraken-style multi-lane rail coordinates for an ordered list of commits
 * (topological/chronological newest-first).
 */
export function computeCommitGraph(commits: CommitInfo[]): GraphRowLayout[] {
  const activeSlots: Array<string | null> = [];
  const laneColors: number[] = [];
  let nextColor = 0;

  const allocColor = () => {
    const c = nextColor % GRAPH_COLORS.length;
    nextColor++;
    return c;
  };

  return commits.map((commit) => {
    let lane = activeSlots.indexOf(commit.sha);
    const incomingLanes: number[] = [];
    for (let i = 0; i < activeSlots.length; i++) {
      if (activeSlots[i] !== null) incomingLanes.push(i);
    }

    if (lane === -1) {
      lane = activeSlots.indexOf(null);
      if (lane === -1) {
        lane = activeSlots.length;
        activeSlots.push(commit.sha);
        laneColors[lane] = allocColor();
      } else {
        activeSlots[lane] = commit.sha;
        laneColors[lane] = laneColors[lane] ?? allocColor();
      }
    }

    const nodeColor = laneColors[lane] ?? 0;
    const segments: GraphSegment[] = [];

    // Any secondary active slots pointing to this commit converge into `lane`
    for (let i = 0; i < activeSlots.length; i++) {
      if (i !== lane && activeSlots[i] === commit.sha) {
        segments.push({
          fromLane: i,
          toLane: lane,
          colorIndex: laneColors[i] ?? nodeColor,
          kind: "branch-in",
        });
        activeSlots[i] = null;
      }
    }

    const parents = commit.parents ?? [];
    if (parents.length === 0) {
      activeSlots[lane] = null;
    } else {
      // First parent continues on the same lane
      activeSlots[lane] = parents[0];
      segments.push({
        fromLane: lane,
        toLane: lane,
        colorIndex: nodeColor,
        kind: "vertical",
      });

      // Additional parents (merge commit) either connect to existing slot or allocate new lane
      for (let pIdx = 1; pIdx < parents.length; pIdx++) {
        const parentSha = parents[pIdx];
        let pLane = activeSlots.indexOf(parentSha);
        if (pLane === -1) {
          pLane = activeSlots.indexOf(null);
          if (pLane === -1) {
            pLane = activeSlots.length;
            activeSlots.push(parentSha);
          } else {
            activeSlots[pLane] = parentSha;
          }
          laneColors[pLane] = allocColor();
        }
        segments.push({
          fromLane: lane,
          toLane: pLane,
          colorIndex: laneColors[pLane] ?? nodeColor,
          kind: "merge-out",
        });
      }
    }

    // Pass-through rails for all other active slots
    const outgoingLanes: number[] = [];
    for (let i = 0; i < activeSlots.length; i++) {
      if (activeSlots[i] !== null) {
        outgoingLanes.push(i);
        if (i !== lane && !segments.some((s) => s.fromLane === i && s.toLane === i)) {
          segments.push({
            fromLane: i,
            toLane: i,
            colorIndex: laneColors[i] ?? 0,
            kind: "vertical",
          });
        }
      }
    }

    // Trim trailing null slots to keep rail width compact
    while (activeSlots.length > 0 && activeSlots[activeSlots.length - 1] === null) {
      activeSlots.pop();
    }

    const maxLanes = Math.max(
      1,
      lane + 1,
      ...incomingLanes.map((n) => n + 1),
      ...outgoingLanes.map((n) => n + 1)
    );

    return {
      lane,
      colorIndex: nodeColor,
      maxLanes,
      incomingLanes,
      outgoingLanes,
      segments,
    };
  });
}
