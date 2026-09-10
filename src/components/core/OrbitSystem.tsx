import { useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";
import { buildOrbitCurve, ORBITS } from "./orbitPaths";

interface OrbitSystemProps {
  orbitColor: string;
  intensity?: number;
  rotationSpeed?: number;
}

export function OrbitSystem({ orbitColor, intensity = 1, rotationSpeed = 1 }: OrbitSystemProps) {
  const groupRef = useRef<THREE.Group>(null);

  const orbitGeometries = useMemo(
    () => ORBITS.map((orbit) => ({ orbit, curve: buildOrbitCurve(orbit) })),
    [],
  );

  useFrame(({ clock }) => {
    if (!groupRef.current) return;
    // Steady single-axis spin only — keeps the rings reading as clean
    // concentric circles instead of wobbling into lopsided ellipses.
    groupRef.current.rotation.y = clock.getElapsedTime() * 0.035 * rotationSpeed;
  });

  return (
    <group ref={groupRef}>
      {orbitGeometries.map(({ orbit, curve }, index) => {
        const geometry = new THREE.TubeGeometry(curve, 160, 0.0028, 6, true);

        return (
          <mesh key={index} geometry={geometry}>
            <meshBasicMaterial
              color={orbitColor}
              transparent
              opacity={orbit.opacity * intensity}
              depthWrite={false}
              toneMapped={false}
              blending={THREE.AdditiveBlending}
            />
          </mesh>
        );
      })}
    </group>
  );
}
