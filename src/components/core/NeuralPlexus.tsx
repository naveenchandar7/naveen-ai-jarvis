import { useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";

interface NeuralPalette {
  primary: string;
  secondary: string;
  accent: string;
  hot: string;
}

interface NeuralPlexusProps {
  /*
   * Legacy props are kept so the existing NovaCore
   * does not break while we migrate the visual engine.
   */
  coreHotColor?: string;
  edgeColor?: string;

  /*
   * New future-proof visual controls.
   */
  palette?: NeuralPalette;
  intensity?: number;
  pointCount?: number;
  density?: number;
  activity?: number;
  speed?: number;
}

interface ParticleSeed {
  position: THREE.Vector3;
  velocity: THREE.Vector3;
  phase: number;
  energy: number;
}

const MIN_RADIUS = 0.58;
const MAX_RADIUS = 1.42;
const CONNECT_DISTANCE = 0.44;

const DEFAULT_PALETTE: NeuralPalette = {
  primary: "#00e5ff",
  secondary: "#3c8dff",
  accent: "#8b5cff",
  hot: "#ff4fd8",
};

/**
 * One-time particle initialization.
 *
 * We deliberately keep this isolated from React state.
 * The neural field is a real-time simulation, not UI state.
 */
function createParticleSeeds(
  count: number,
): ParticleSeed[] {
  const seeds: ParticleSeed[] = [];

  for (
    let i = 0;
    i < count;
    i += 1
  ) {
    const theta =
      Math.random() *
      Math.PI *
      2;

    const phi =
      Math.acos(
        2 * Math.random() - 1,
      );

    /*
     * Bias some particles toward the core.
     * This makes the neural mass feel volumetric
     * instead of becoming a hollow wire sphere.
     */
    const normalized =
      Math.pow(
        Math.random(),
        1.55,
      );

    const radius =
      MIN_RADIUS +
      normalized *
        (MAX_RADIUS - MIN_RADIUS);

    const position =
      new THREE.Vector3(
        radius *
          Math.sin(phi) *
          Math.cos(theta),

        radius *
          Math.cos(phi) *
          0.84,

        radius *
          Math.sin(phi) *
          Math.sin(theta),
      );

    const velocity =
      new THREE.Vector3(
        (Math.random() - 0.5) *
          0.12,

        (Math.random() - 0.5) *
          0.12,

        (Math.random() - 0.5) *
          0.12,
      );

    seeds.push({
      position,
      velocity,
      phase:
        Math.random() *
        Math.PI *
        2,
      energy:
        0.35 +
        Math.random() *
          0.65,
    });
  }

  return seeds;
}

/**
 * Blend the visual palette according to the
 * particle's distance from the core.
 *
 * Core:
 * hot / magenta
 *
 * Middle:
 * violet / blue
 *
 * Outer:
 * cyan
 */
function getParticleColor(
  seed: ParticleSeed,
  palette: NeuralPalette,
) {
  const hot =
    new THREE.Color(
      palette.hot,
    );

  const accent =
    new THREE.Color(
      palette.accent,
    );

  const secondary =
    new THREE.Color(
      palette.secondary,
    );

  const primary =
    new THREE.Color(
      palette.primary,
    );

  const radius =
    seed.position.length();

  const t =
    THREE.MathUtils.clamp(
      (radius - MIN_RADIUS) /
        (MAX_RADIUS -
          MIN_RADIUS),
      0,
      1,
    );

  if (t < 0.3) {
    return hot
      .clone()
      .lerp(
        accent,
        t / 0.3,
      );
  }

  if (t < 0.65) {
    return accent
      .clone()
      .lerp(
        secondary,
        (t - 0.3) /
          0.35,
      );
  }

  return secondary
    .clone()
    .lerp(
      primary,
      (t - 0.65) /
        0.35,
    );
}

export function NeuralPlexus({
  coreHotColor,
  edgeColor,

  palette,

  intensity = 1,
  pointCount = 190,
  density = 1,
  activity = 0.65,
  speed = 1,
}: NeuralPlexusProps) {
  const groupRef =
    useRef<THREE.Group>(null);

  /*
   * Keep old props working until NovaCore
   * is migrated to the new palette API.
   */
  const resolvedPalette =
    useMemo<NeuralPalette>(
      () => ({
        ...DEFAULT_PALETTE,

        ...(palette ?? {}),

        /*
         * Backward compatibility:
         * old coreHotColor becomes hot
         * old edgeColor becomes primary.
         */
        hot:
          palette?.hot ??
          coreHotColor ??
          DEFAULT_PALETTE.hot,

        primary:
          palette?.primary ??
          edgeColor ??
          DEFAULT_PALETTE.primary,
      }),
      [
        palette,
        coreHotColor,
        edgeColor,
      ],
    );

  /*
   * Density is a visual parameter,
   * while pointCount remains the hard upper limit.
   */
  const effectivePointCount =
    Math.max(
      40,
      Math.round(
        pointCount *
          THREE.MathUtils.clamp(
            density,
            0.35,
            1.75,
          ),
      ),
    );

  const seeds = useMemo(
    () =>
      createParticleSeeds(
        effectivePointCount,
      ),
    [effectivePointCount],
  );

  const geometryData =
    useMemo(() => {
      const pointsGeo =
        new THREE.BufferGeometry();

      pointsGeo.setAttribute(
        "position",
        new THREE.BufferAttribute(
          new Float32Array(
            effectivePointCount *
              3,
          ),
          3,
        ),
      );

      pointsGeo.setAttribute(
        "color",
        new THREE.BufferAttribute(
          new Float32Array(
            effectivePointCount *
              3,
          ),
          3,
        ),
      );

      /*
       * Maximum possible pair count.
       * Allocated once so animation never
       * creates new arrays every frame.
       */
      const maxSegments =
        (
          effectivePointCount *
          (effectivePointCount - 1)
        ) / 2;

      const lineGeo =
        new THREE.BufferGeometry();

      lineGeo.setAttribute(
        "position",
        new THREE.BufferAttribute(
          new Float32Array(
            maxSegments *
              2 *
              3,
          ),
          3,
        ),
      );

      lineGeo.setAttribute(
        "color",
        new THREE.BufferAttribute(
          new Float32Array(
            maxSegments *
              2 *
              3,
          ),
          3,
        ),
      );

      lineGeo.setDrawRange(
        0,
        0,
      );

      return {
        pointsGeo,
        lineGeo,
      };
    }, [
      effectivePointCount,
    ]);

  useFrame(
    ({ clock }, delta) => {
      const group =
        groupRef.current;

      if (!group) return;

      const time =
        clock.getElapsedTime();

      const hot =
        new THREE.Color(
          resolvedPalette.hot,
        );

      const accent =
        new THREE.Color(
          resolvedPalette.accent,
        );

      
      const secondary =
        new THREE.Color(
          resolvedPalette.secondary,
        );

      /*
       * --------------------------------------------------
       * PARTICLE MOTION
       * --------------------------------------------------
       */

      seeds.forEach(
        (seed) => {
          /*
           * Gentle orbital drift.
           * Activity increases movement,
           * but never turns the system chaotic.
           */
          const drift =
            1 +
            activity * 1.7;

          seed.position.addScaledVector(
            seed.velocity,
            delta *
              speed *
              drift,
          );

          /*
           * Soft radial breathing.
           */
          const currentRadius =
            seed.position.length();

          const breathing =
            Math.sin(
              time *
                (0.45 +
                  activity *
                    0.55) +
                seed.phase,
            );

          const targetRadius =
            THREE.MathUtils.clamp(
              currentRadius +
                breathing *
                  0.0018 *
                  activity,
              MIN_RADIUS,
              MAX_RADIUS,
            );

          if (
            currentRadius >
            0.0001
          ) {
            seed.position.setLength(
              targetRadius,
            );
          }

          /*
           * Bounce inside the neural volume.
           */
          if (
            currentRadius >
              MAX_RADIUS ||
            currentRadius <
              MIN_RADIUS
          ) {
            seed.velocity.multiplyScalar(
              -1,
            );

            seed.position.setLength(
              THREE.MathUtils.clamp(
                currentRadius,
                MIN_RADIUS,
                MAX_RADIUS,
              ),
            );
          }

          /*
           * Slowly varying energy value.
           * This drives node brightness and
           * travelling neural activity.
           */
          seed.energy =
            0.4 +
            0.35 *
              Math.sin(
                time *
                  (1.3 +
                    activity) +
                  seed.phase,
              ) +
            0.25 *
              activity;
        },
      );

      /*
       * --------------------------------------------------
       * POINT POSITIONS + COLORS
       * --------------------------------------------------
       */

      const pointPositions =
        geometryData.pointsGeo.getAttribute(
          "position",
        ) as THREE.BufferAttribute;

      const pointColors =
        geometryData.pointsGeo.getAttribute(
          "color",
        ) as THREE.BufferAttribute;

      seeds.forEach(
        (seed, index) => {
          pointPositions.setXYZ(
            index,
            seed.position.x,
            seed.position.y,
            seed.position.z,
          );

          /*
           * Base spatial color.
           */
          const base =
            getParticleColor(
              seed,
              resolvedPalette,
            );

          /*
           * Active nodes briefly
           * move toward hot white/magenta.
           */
          const activation =
            THREE.MathUtils.clamp(
              seed.energy *
                activity,
              0,
              1,
            );

          const finalColor =
            base
              .clone()
              .lerp(
                hot,
                activation *
                  0.42,
              );

          /*
           * Keep a small number of
           * nodes distinctly bright.
           */
          if (
            index % 19 === 0
          ) {
            finalColor.lerp(
              new THREE.Color(
                "#ffffff",
              ),
              0.28 *
                activity,
            );
          }

          pointColors.setXYZ(
            index,
            finalColor.r,
            finalColor.g,
            finalColor.b,
          );
        },
      );

      pointPositions.needsUpdate =
        true;

      pointColors.needsUpdate =
        true;

      /*
       * --------------------------------------------------
       * DYNAMIC CONNECTION GRAPH
       * --------------------------------------------------
       */

      const linePositions =
        geometryData.lineGeo.getAttribute(
          "position",
        ) as THREE.BufferAttribute;

      const lineColors =
        geometryData.lineGeo.getAttribute(
          "color",
        ) as THREE.BufferAttribute;

      let vertexIndex = 0;

      for (
        let i = 0;
        i < seeds.length;
        i += 1
      ) {
        for (
          let j = i + 1;
          j < seeds.length;
          j += 1
        ) {
          const distance =
            seeds[i].position.distanceTo(
              seeds[j].position,
            );

          /*
           * Activity opens the network slightly.
           * Idle state stays tighter.
           */
          const connectDistance =
            CONNECT_DISTANCE +
            activity * 0.08;

          if (
            distance >=
            connectDistance
          ) {
            continue;
          }

          const a =
            seeds[i].position;

          const b =
            seeds[j].position;

          linePositions.setXYZ(
            vertexIndex,
            a.x,
            a.y,
            a.z,
          );

          linePositions.setXYZ(
            vertexIndex + 1,
            b.x,
            b.y,
            b.z,
          );

          /*
           * Each connection gets a
           * smoothly animated activity pulse.
           */
          const phase =
            (
              i * 0.173 +
              j * 0.071
            ) %
            (Math.PI * 2);

          const flow =
            (
              Math.sin(
                time *
                  (2.2 +
                    activity *
                      2.8) +
                  phase,
              ) +
              1
            ) /
            2;

          const baseA =
            getParticleColor(
              seeds[i],
              resolvedPalette,
            );

          const baseB =
            getParticleColor(
              seeds[j],
              resolvedPalette,
            );

          const connectionA =
            baseA
              .clone()
              .lerp(
                accent,
                flow *
                  0.45 *
                  activity,
              );

          const connectionB =
            baseB
              .clone()
              .lerp(
                secondary,
                flow *
                  0.38,
              );

          /*
           * Hot energy packets.
           */
          if (
            flow >
              0.88 &&
            activity >
              0.45
          ) {
            connectionA.lerp(
              hot,
              0.28,
            );

            connectionB.lerp(
              hot,
              0.22,
            );
          }

          lineColors.setXYZ(
            vertexIndex,
            connectionA.r,
            connectionA.g,
            connectionA.b,
          );

          lineColors.setXYZ(
            vertexIndex + 1,
            connectionB.r,
            connectionB.g,
            connectionB.b,
          );

          vertexIndex += 2;
        }
      }

      geometryData.lineGeo.setDrawRange(
        0,
        vertexIndex,
      );

      linePositions.needsUpdate =
        true;

      lineColors.needsUpdate =
        true;

      /*
       * --------------------------------------------------
       * WHOLE FIELD ROTATION
       * --------------------------------------------------
       */

      group.rotation.y +=
        delta *
        (0.035 +
          activity *
            0.025) *
        speed;

      group.rotation.x =
        Math.sin(
          time * 0.12,
        ) * 0.018;

      group.rotation.z =
        Math.cos(
          time * 0.09,
        ) * 0.012;
    },
  );

  return (
    <group ref={groupRef}>
      {/* ------------------------------------------
          Dynamic neural links
      ------------------------------------------- */}

      <lineSegments
        geometry={
          geometryData.lineGeo
        }
        frustumCulled={false}
      >
        <lineBasicMaterial
          vertexColors
          transparent
          opacity={
            0.22 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </lineSegments>

      {/* ------------------------------------------
          Neural nodes
      ------------------------------------------- */}

      <points
        geometry={
          geometryData.pointsGeo
        }
        frustumCulled={false}
      >
        <pointsMaterial
          size={0.021}
          vertexColors
          transparent
          opacity={
            0.82 *
            intensity
          }
          depthWrite={false}
          sizeAttenuation
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </points>
    </group>
  );
}