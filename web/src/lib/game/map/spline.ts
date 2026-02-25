// ── Catmull-Rom Spline Utility ───────────────────────────────────────────────
// Tessellates an array of waypoints into a smooth polyline using Catmull-Rom
// interpolation. Used by the infra layer to render fiber/cable edges as curves.

/**
 * Minimum squared distance between two points to consider them non-duplicate.
 * Points closer than ~0.0001 degrees (~11m at equator) are treated as duplicates.
 */
const MIN_DIST_SQ = 1e-8;

/**
 * Deduplicate consecutive points that are nearly identical.
 */
function deduplicatePoints(points: [number, number][]): [number, number][] {
    if (points.length <= 1) return points;
    const result: [number, number][] = [points[0]];
    for (let i = 1; i < points.length; i++) {
        const prev = result[result.length - 1];
        const dx = points[i][0] - prev[0];
        const dy = points[i][1] - prev[1];
        if (dx * dx + dy * dy > MIN_DIST_SQ) {
            result.push(points[i]);
        }
    }
    return result;
}

/**
 * Compute Catmull-Rom spline interpolation between P1 and P2.
 *
 * Formula:
 *   q(t) = 0.5 * ((2*P1) + (-P0 + P2)*t + (2*P0 - 5*P1 + 4*P2 - P3)*t^2 + (-P0 + 3*P1 - 3*P2 + P3)*t^3)
 *
 * @param p0 Control point before the span start
 * @param p1 Span start point
 * @param p2 Span end point
 * @param p3 Control point after the span end
 * @param t  Interpolation parameter [0, 1]
 * @returns Interpolated [lon, lat]
 */
function catmullRomPoint(
    p0: [number, number],
    p1: [number, number],
    p2: [number, number],
    p3: [number, number],
    t: number,
): [number, number] {
    const t2 = t * t;
    const t3 = t2 * t;

    const lon =
        0.5 *
        (2 * p1[0] +
            (-p0[0] + p2[0]) * t +
            (2 * p0[0] - 5 * p1[0] + 4 * p2[0] - p3[0]) * t2 +
            (-p0[0] + 3 * p1[0] - 3 * p2[0] + p3[0]) * t3);

    const lat =
        0.5 *
        (2 * p1[1] +
            (-p0[1] + p2[1]) * t +
            (2 * p0[1] - 5 * p1[1] + 4 * p2[1] - p3[1]) * t2 +
            (-p0[1] + 3 * p1[1] - 3 * p2[1] + p3[1]) * t3);

    return [lon, lat];
}

/**
 * Tessellate a series of waypoints into a smooth Catmull-Rom spline polyline.
 *
 * @param waypoints  Ordered [lon, lat] pairs defining the route.
 * @param segmentsPerSpan  Number of linear segments per span (default 10).
 * @returns Tessellated polyline as [lon, lat][] — includes all original waypoints.
 *
 * Edge cases:
 * - 0 or 1 waypoints: returns empty array
 * - 2 waypoints: returns straight line (the two points)
 * - 3+ waypoints: full Catmull-Rom interpolation with mirrored phantom endpoints
 */
export function catmullRomSpline(
    waypoints: [number, number][],
    segmentsPerSpan: number = 10,
): [number, number][] {
    // Deduplicate consecutive near-identical points
    const pts = deduplicatePoints(waypoints);

    if (pts.length <= 1) return [];
    if (pts.length === 2) return [pts[0], pts[1]];

    const result: [number, number][] = [];
    const n = pts.length;

    for (let i = 0; i < n - 1; i++) {
        // Get the 4 control points for this span.
        // For the first span, mirror P0 from P1 across P0 (phantom point).
        // For the last span, mirror P3 from Pn-2 across Pn-1 (phantom point).
        const p0: [number, number] =
            i === 0
                ? [2 * pts[0][0] - pts[1][0], 2 * pts[0][1] - pts[1][1]]
                : pts[i - 1];
        const p1 = pts[i];
        const p2 = pts[i + 1];
        const p3: [number, number] =
            i === n - 2
                ? [2 * pts[n - 1][0] - pts[n - 2][0], 2 * pts[n - 1][1] - pts[n - 2][1]]
                : pts[i + 2];

        // Emit points for this span
        const segments = Math.max(1, segmentsPerSpan);
        for (let j = 0; j < segments; j++) {
            const t = j / segments;
            result.push(catmullRomPoint(p0, p1, p2, p3, t));
        }
    }

    // Always include the final point
    result.push(pts[n - 1]);

    return result;
}
