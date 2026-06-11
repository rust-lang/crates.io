<!--
  @component
  Renders the cuddly Ferris mascot. On devices with a hovering pointer, the
  eye highlights follow the cursor, on touch or reduced-motion they stay put.
-->
<script lang="ts">
  import type { SVGAttributes } from 'svelte/elements';

  import { MediaQuery } from 'svelte/reactivity';

  type Props = SVGAttributes<SVGSVGElement>;

  let { class: className, ...rest }: Props = $props();

  let svgEl: SVGSVGElement;

  let eyes = $state({ left: { x: 0, y: 0 }, right: { x: 0, y: 0 } });

  let reducedMotion = new MediaQuery('prefers-reduced-motion: reduce', false);
  let canHover = new MediaQuery('hover: hover', false);
  let enabled = $derived(!reducedMotion.current && canHover.current);

  function handlePointerMove(event: PointerEvent) {
    // Hybrid devices report `hover: hover` yet still emit touch-origin pointer
    // events. Ignore those so the eyes only track a real hovering pointer.
    if (event.pointerType === 'touch') return;

    eyes = eyeTransforms(svgEl.getBoundingClientRect(), { x: event.clientX, y: event.clientY });
  }

  type Point = Pick<DOMPoint, 'x' | 'y'>;
  type Rect = Pick<DOMRect, 'left' | 'top' | 'width' | 'height'>;

  // SVG viewBox width (must match the `viewBox` on the <svg> below).
  const VIEWBOX_WIDTH = 4417;

  // Natural center of each white highlight in the artwork, in SVG user units.
  // The highlights sit toward the top-left of their sockets.
  const RIGHT_ART: Point = { x: 2654, y: 1651 };
  const LEFT_ART: Point = { x: 1696, y: 1657 };

  // Base shift of each highlight from its artwork position, in SVG user units.
  // This sets the neutral gaze and the point tracking pivots around.
  const RIGHT_REST: Point = { x: 60, y: 130 };
  const LEFT_REST: Point = { x: 100, y: 130 };

  // Resting positions double as the gaze pivots (artwork center + base shift).
  const RIGHT_PIVOT: Point = { x: RIGHT_ART.x + RIGHT_REST.x, y: RIGHT_ART.y + RIGHT_REST.y };
  const LEFT_PIVOT: Point = { x: LEFT_ART.x + LEFT_REST.x, y: LEFT_ART.y + LEFT_REST.y };

  // Max highlight travel from rest, in SVG user units (an ellipse: x ≠ y).
  const TRAVEL: Point = { x: 100, y: 125 };

  // Cursor distance (screen px) at which the highlights reach full travel.
  const SATURATION_PX = 200;

  /**
   * Computes the translate to apply to each eye highlight so the gaze leans
   * toward the cursor.
   *
   * Maps each eye's gaze pivot from SVG user units into screen pixels (matching
   * the cursor's space), leans it toward the cursor via {@link eyeOffset},
   * then composes the lean onto the eye's resting shift.
   *
   * @param rect On-screen bounds of the SVG (from `getBoundingClientRect()`).
   * @param cursor Cursor position, in screen pixels.
   * @returns Per-eye translate from the artwork origin, in SVG user units.
   */
  function eyeTransforms(rect: Rect, cursor: Point): { left: Point; right: Point } {
    let scale = rect.width / VIEWBOX_WIDTH;

    let toScreen = (center: Point): Point => ({
      x: rect.left + center.x * scale,
      y: rect.top + center.y * scale,
    });

    let rightOffset = eyeOffset(toScreen(RIGHT_PIVOT), cursor);
    let leftOffset = eyeOffset(toScreen(LEFT_PIVOT), cursor);

    return {
      right: { x: RIGHT_REST.x + rightOffset.x, y: RIGHT_REST.y + rightOffset.y },
      left: { x: LEFT_REST.x + leftOffset.x, y: LEFT_REST.y + leftOffset.y },
    };
  }

  function eyeOffset(eyeCenter: Point, cursor: Point): Point {
    let dx = cursor.x - eyeCenter.x;
    let dy = cursor.y - eyeCenter.y;
    let distance = Math.hypot(dx, dy);
    if (distance === 0) {
      return { x: 0, y: 0 };
    }

    let magnitude = Math.min(distance / SATURATION_PX, 1);
    return {
      x: (dx / distance) * magnitude * TRAVEL.x,
      y: (dy / distance) * magnitude * TRAVEL.y,
    };
  }
</script>

<svelte:window onpointermove={enabled ? handlePointerMove : undefined} />

<!-- eslint-disable unicorn/prefer-https -- SVG namespace identifier, not a fetchable URL -->
<svg
  bind:this={svgEl}
  class={['ferris', className]}
  aria-hidden="true"
  version="1.1"
  viewBox="0 0 4417 3259"
  xmlns="http://www.w3.org/2000/svg"
  {...rest}
>
  <path
    d="m2359.9 1822.5-1832.6-96.478s-234.55 174.8-50.238 474.68c3.3542 15.588 349.38 453.49 434.63 482.35 36.015 12.189 5.8234-152.17-54.795-347.87 359.07 93.819 824.05 150.51 1332.3 150.51 588.2 0 1118.7-75.841 1493.7-197.39-76.159 208.91-123.8 406.37-89.369 400.86 72.954-11.692 388.12-378.98 391.85-393.85 171.83-262.07-13.805-483.35-13.805-483.35z"
    fill="#8f1e1c"
  />
  <path
    d="m2208.7 1.3086c-22.225 0-42.933 12-54.541 31.717l-115.55 194.64c-18.158 1.5667-36.342 3.4674-54.4 5.5508l-149.62-167.38c-15.15-16.913-37.695-24.472-59.449-20.01-21.896 4.55-39.889 20.509-47.639 42.297l-76.674 213.82c-17.75 5.325-35.337 10.946-52.879 16.713l-177.68-133.36c-18.067-13.608-41.584-16.479-62.15-7.5332-20.496 8.8625-35.255 28.264-38.805 51.105l-35.254 225.48c-16.308 8.8208-32.455 17.892-48.555 27.154l-199.34-94.779c-20.242-9.6458-43.871-7.658-62.309 5.2754-18.521 12.85-29.342 34.857-28.5 58.02l7.6465 228.81c-14.188 11.833-28.224 23.909-42.133 36.139l-213.66-52.533c-21.713-5.2708-44.457 1.4624-60.17 17.846-15.813 16.442-22.216 40.128-17.137 62.766l50.328 222.88c-11.675 14.446-23.23 29.046-34.555 43.838l-219.39-7.9707c-22.146-0.6542-43.28 10.404-55.65 29.721-12.396 19.221-14.272 43.837-5.0469 65.033l90.893 207.93c-8.8292 16.7-17.6 33.533-26.037 50.625l-216.18 36.688c-21.963 3.7-40.476 19.026-49.018 40.459-8.4875 21.488-5.732 45.971 7.2305 64.812l125.02 181.08c-6.9247 3.2107-13.659 6.777-20.188 10.678-1.925-0.4458-3.854-0.8788-5.7832-1.2246 0 0-431.8 211.97-185.68 636.07 2.3208 21.404 491.04 648.62 628.29 696.38 89.829 31.258-82.964-536-239.1-939.96 4.3748-4.9716 9.4885-10.078 14.898-15.254 66.563 27.156 177.45 68.579 328.37 115.76 27.918 157.62 121.8 433.68 437.02 552.25-30.399 199.25 119.98 411.27 361.01 492.91 117.17 39.692 236.05 41.872 338.45 13.363-153.8-152.1-258.32-339.17-258.32-339.17s230.01 30.396 544.46 38.15c27.542-197.96-122.42-407.47-361.67-488.51-189.08-64.045-382.62-30.472-504.09 72.26-95.697-19.22-232.9-80.167-298.56-267.47 567.83 144.73 1432.8 261.9 2316.3-4.4629-53.734 157.7-157.77 227.19-246.74 257.52-124.73-93.845-316.64-114.91-497.78-39.957-221.36 91.588-352.65 295.97-323.43 482.69 174.78-2.7167 362.25-14.046 544.73-40.492 0 0-107.66 174.01-266.07 325.88 106.53 27.496 230.76 19.865 350.32-29.602 228.06-94.362 360.7-308.47 320.51-499.62 278.99-135.69 356.87-403.56 378.6-546.45 86.175-33.629 172.29-70.191 258.04-112.11 12.782-2.2934 24.813-5.8805 36.32-10.299 13.629 7.9068 26.293 15.748 35.699 23.678-94.533 422.63-201.68 1014.5-117.47 970.26 128.65-67.579 518.98-760.3 518.11-781.81 146.02-368.73-125.8-534.38-235.14-584.29l116.14-168.26c13.042-18.842 15.707-43.325 7.207-64.812-8.5-21.375-27.042-36.709-49-40.459l-216.19-36.688c-8.3667-17.092-17.151-33.925-25.959-50.625l90.822-207.93c9.3459-21.196 7.3697-45.812-5.0137-65.033-12.338-19.317-33.292-30.6-55.621-29.721l-219.4 7.9707c-11.363-14.792-22.901-29.45-34.676-43.838l50.42-222.88c5.1042-22.638-1.3417-46.324-17.096-62.766-15.675-16.383-38.453-23.162-60.107-17.846l-213.71 52.533c-13.838-12.229-27.911-24.305-42.141-36.139l7.6992-228.81c0.825-23.163-9.9698-45.17-28.516-58.02-18.525-12.933-42.056-14.888-62.318-5.2754l-199.36 94.779c-16.058-9.2083-32.201-18.333-48.518-27.154l-35.228-225.48c-3.5709-22.842-18.272-42.243-38.859-51.105-20.512-8.9083-44.032-6.0335-62.045 7.5332l-177.75 133.36c-17.5-5.8167-35.079-11.388-52.883-16.713l-76.609-213.82c-7.7917-21.788-25.754-37.797-47.658-42.297-21.829-4.4625-44.292 3.0973-59.43 20.01l-149.61 167.38c-18.092-2.0833-36.187-3.9841-54.346-5.5508l-115.51-194.64c-11.708-19.717-32.375-31.717-54.65-31.717z"
    fill="#e43a25"
  />
  <path
    d="m2534.6 1569.8s200.65-219.7 401.31 0c0 0 157.67 292.97 0 439.43 0 0-257.99 205.07-401.31 0 0 0-172-161.13 0-439.43z"
    fill="#030404"
  />
  <path
    d="m2766.9 1651.3c0 85.554-50.429 154.91-112.63 154.91-62.183 0-112.61-69.358-112.61-154.91 0-85.55 50.429-154.92 112.61-154.92 62.196 0 112.63 69.367 112.63 154.92z"
    fill="#fff"
    data-test-eye
    transform={`translate(${eyes.right.x} ${eyes.right.y})`}
  />
  <path
    d="m1639 1509.8s344.2-152.4 438.14 187.76c0 0 98.408 396.48-282.65 418.85 0 0-485.93-93.592-155.49-606.61z"
    fill="#030404"
  />
  <path
    d="m1811.9 1657c0 88.221-52 159.78-116.15 159.78-64.129 0-116.14-71.554-116.14-159.78 0-88.242 52.012-159.78 116.14-159.78 64.154 0 116.15 71.533 116.15 159.78z"
    fill="#fff"
    data-test-eye
    transform={`translate(${eyes.left.x} ${eyes.left.y})`}
  />
</svg>

<!-- eslint-enable unicorn/prefer-https -->

<style>
  .ferris {
    display: block;
  }
</style>
