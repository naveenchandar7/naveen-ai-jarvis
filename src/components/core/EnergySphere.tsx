import { useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";
import { getGlowSpriteTexture } from "./glowSprite";

interface EnergySphereProps {
  coreColor: string;
  glowColor: string;

  /**
   * Optional secondary energy color.
   * Future visual engines can supply their own
   * accent without changing this component.
   */
  secondaryColor?: string;

  /**
   * Optional hot accent color.
   */
  hotColor?: string;

  intensity?: number;
  pulseSpeed?: number;

  /**
   * Controls how visible the internal shell is.
   * Keep low so the sphere never becomes a
   * dominant wireframe globe.
   */
  shellOpacity?: number;
}

const BASE_RADIUS = 0.66;

export function EnergySphere({
  coreColor,
  glowColor,
  secondaryColor = "#7b5cff",
  hotColor = "#ff56d8",
  intensity = 1,
  pulseSpeed = 1,
  shellOpacity = 0.055,
}: EnergySphereProps) {
  const groupRef =
    useRef<THREE.Group>(null);

  const shellRef =
    useRef<THREE.Mesh>(null);

  const haloRef =
    useRef<THREE.Sprite>(null);

  const outerRef =
    useRef<THREE.Sprite>(null);

  const violetRef =
    useRef<THREE.Sprite>(null);

  const hotRef =
    useRef<THREE.Sprite>(null);

  const texture = useMemo(
    () => getGlowSpriteTexture(),
    [],
  );

  /*
   * Icosahedron gives the core a much softer
   * organic silhouette than a plain sphere.
   */
  const shellGeometry = useMemo(
    () =>
      new THREE.IcosahedronGeometry(
        BASE_RADIUS,
        5,
      ),
    [],
  );

  const basePositions = useMemo(
    () => {
      const attribute =
        shellGeometry.getAttribute(
          "position",
        );

      const positions: THREE.Vector3[] =
        [];

      for (
        let i = 0;
        i < attribute.count;
        i += 1
      ) {
        positions.push(
          new THREE.Vector3(
            attribute.getX(i),
            attribute.getY(i),
            attribute.getZ(i),
          ),
        );
      }

      return positions;
    },
    [shellGeometry],
  );

  useFrame(({ clock }) => {
    const time =
      clock.getElapsedTime();

    /*
     * -------------------------------
     * CORE BREATHING
     * -------------------------------
     */

    const breathe =
      1 +
      Math.sin(
        time *
          1.35 *
          pulseSpeed,
      ) *
        0.035 *
        intensity;

    const softPulse =
      1 +
      Math.sin(
        time *
          0.85 *
          pulseSpeed +
          1.3,
      ) *
        0.045 *
        intensity;

    const fastPulse =
      1 +
      Math.sin(
        time *
          2.15 *
          pulseSpeed,
      ) *
        0.028 *
        intensity;

    /*
     * -------------------------------
     * GLOW LAYERS
     * -------------------------------
     */

    if (haloRef.current) {
      haloRef.current.scale.setScalar(
        BASE_RADIUS *
          2.45 *
          softPulse,
      );

      haloRef.current.material.opacity =
        0.25 * intensity;
    }

    if (outerRef.current) {
      outerRef.current.scale.setScalar(
        BASE_RADIUS *
          1.65 *
          breathe,
      );

      outerRef.current.material.opacity =
        0.34 * intensity;
    }

    if (violetRef.current) {
      violetRef.current.scale.setScalar(
        BASE_RADIUS *
          1.08 *
          fastPulse,
      );

      violetRef.current.material.opacity =
        0.20 * intensity;
    }

    if (hotRef.current) {
      /*
       * Important:
       * this stays SMALL.
       * We do NOT want another huge white ball.
       */
      hotRef.current.scale.setScalar(
        BASE_RADIUS *
          0.28 *
          (1 +
            Math.sin(
              time *
                2.4 *
                pulseSpeed,
            ) *
              0.04),
      );

      hotRef.current.material.opacity =
        0.82 * intensity;
    }

    /*
     * -------------------------------
     * ORGANIC SHELL DEFORMATION
     * -------------------------------
     */

    const attribute =
      shellGeometry.getAttribute(
        "position",
      );

    for (
      let i = 0;
      i < basePositions.length;
      i += 1
    ) {
      const base =
        basePositions[i];

      const direction =
        base
          .clone()
          .normalize();

      const wave1 =
        Math.sin(
          time *
            1.1 *
            pulseSpeed +
            direction.x * 4.2 +
            direction.z * 3.4,
        );

      const wave2 =
        Math.cos(
          time *
            0.72 *
            pulseSpeed +
            direction.y * 5.1,
        );

      const wave3 =
        Math.sin(
          time *
            0.48 *
            pulseSpeed +
            direction.x *
              direction.y *
              8,
        );

      const deformation =
        1 +
        (
          wave1 * 0.020 +
          wave2 * 0.010 +
          wave3 * 0.008
        ) *
          intensity;

      const next =
        direction.multiplyScalar(
          BASE_RADIUS *
            deformation,
        );

      attribute.setXYZ(
        i,
        next.x,
        next.y,
        next.z,
      );
    }

    attribute.needsUpdate =
      true;

    /*
     * -------------------------------
     * 3D ROTATION
     * -------------------------------
     */

    if (groupRef.current) {
      groupRef.current.rotation.y =
        time *
        0.045 *
        pulseSpeed;

      groupRef.current.rotation.x =
        Math.sin(
          time * 0.18,
        ) *
        0.018;

      groupRef.current.rotation.z =
        Math.cos(
          time * 0.14,
        ) *
        0.012;
    }

    if (shellRef.current) {
      shellRef.current.rotation.y =
        time * 0.13;

      shellRef.current.rotation.x =
        time * 0.07;
    }
  });

  return (
    <group ref={groupRef}>

      {/* ==================================================
          VERY SUBTLE INTERNAL STRUCTURE

          This is deliberately NOT a visible wireframe globe.
      ================================================== */}

      <mesh
        ref={shellRef}
        geometry={shellGeometry}
      >
        <meshBasicMaterial
          color={coreColor}
          transparent
          opacity={
            shellOpacity *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          wireframe
          blending={
            THREE.AdditiveBlending
          }
        />
      </mesh>

      {/* ==================================================
          OUTER AURA
      ================================================== */}

      <sprite ref={haloRef}>
        <spriteMaterial
          map={texture}
          color={glowColor}
          transparent
          opacity={
            0.25 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </sprite>

      {/* ==================================================
          MAIN ENERGY BODY
      ================================================== */}

      <sprite ref={outerRef}>
        <spriteMaterial
          map={texture}
          color={coreColor}
          transparent
          opacity={
            0.34 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </sprite>

      {/* ==================================================
          SECONDARY VIOLET / MAGENTA ENERGY
      ================================================== */}

      <sprite ref={violetRef}>
        <spriteMaterial
          map={texture}
          color={secondaryColor}
          transparent
          opacity={
            0.20 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </sprite>

      {/* ==================================================
          SMALL HOT ENERGY NUCLEUS
      ================================================== */}

      <sprite ref={hotRef}>
        <spriteMaterial
          map={texture}
          color={hotColor}
          transparent
          opacity={
            0.82 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </sprite>

      {/* Small white center light */}
      <sprite scale={0.085}>
        <spriteMaterial
          map={texture}
          color="#ffffff"
          transparent
          opacity={
            0.65 *
            intensity
          }
          depthWrite={false}
          toneMapped={false}
          blending={
            THREE.AdditiveBlending
          }
        />
      </sprite>

      {/* ==================================================
          ENERGY LIGHT SOURCES
      ================================================== */}

      <pointLight
        color={glowColor}
        intensity={
          3.8 *
          intensity
        }
        distance={5}
      />

      <pointLight
        color={secondaryColor}
        intensity={
          2.1 *
          intensity
        }
        distance={3.5}
        position={[
          0.42,
          0.16,
          0.28,
        ]}
      />

      <pointLight
        color={hotColor}
        intensity={
          1.2 *
          intensity
        }
        distance={2.4}
        position={[
          -0.22,
          0.12,
          0.18,
        ]}
      />

    </group>
  );
}