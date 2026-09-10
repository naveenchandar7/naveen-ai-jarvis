import { useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";
import { getGlowSpriteTexture } from "./glowSprite";
import { buildOrbitCurve, ORBITS } from "./orbitPaths";

interface NodeSystemProps {
  nodeColor: string;
  intensity?: number;
  speed?: number;
  /** How many nodes ride each orbital path. */
  nodesPerOrbit?: number;
}

interface Node {
  orbitIndex: number;
  curve: THREE.CatmullRomCurve3;
  progress: number;
  travelSpeed: number;
  baseSize: number;
  blinking: boolean;
  blinkSpeed: number;
  phase: number;
}

function createNodes(nodesPerOrbit: number): Node[] {
  const nodes: Node[] = [];

  ORBITS.forEach((orbit, orbitIndex) => {
    const curve = buildOrbitCurve(orbit);

    for (let i = 0; i < nodesPerOrbit; i += 1) {
      nodes.push({
        orbitIndex,
        curve,
        progress: Math.random(),
        travelSpeed: Math.abs(orbit.speed) * (0.7 + Math.random() * 0.6),
        baseSize: 0.05 + Math.random() * 0.035,
        blinking: Math.random() < 0.45,
        blinkSpeed: 1.2 + Math.random() * 2.2,
        phase: Math.random() * Math.PI * 2,
      });
    }
  });

  return nodes;
}

export function NodeSystem({
  nodeColor,
  intensity = 1,
  speed = 1,
  nodesPerOrbit = 3,
}: NodeSystemProps) {
  const groupRef = useRef<THREE.Group>(null);
  const spriteRefs = useRef<(THREE.Sprite | null)[]>([]);

  const nodes = useMemo(() => createNodes(nodesPerOrbit), [nodesPerOrbit]);
  const texture = useMemo(() => getGlowSpriteTexture(), []);

  useFrame(({ clock }) => {
    const time = clock.getElapsedTime();

    // Mirror OrbitSystem's steady single-axis rotation so nodes stay glued to the rings.
    if (groupRef.current) {
      groupRef.current.rotation.y = time * 0.035;
    }

    nodes.forEach((node, index) => {
      const orbit = ORBITS[node.orbitIndex];
      const direction = orbit.speed >= 0 ? 1 : -1;

      const t =
        (node.progress + time * node.travelSpeed * speed * direction * 0.05 + 1) % 1;

      const point = node.curve.getPointAt(((t % 1) + 1) % 1);

      const sprite = spriteRefs.current[index];
      if (!sprite) return;

      sprite.position.copy(point);

      const blink = node.blinking
        ? 0.5 + Math.sin(time * node.blinkSpeed + node.phase) * 0.5
        : 0.85 + Math.sin(time * 0.6 + node.phase) * 0.15;

      const scale = node.baseSize * (0.7 + blink * 0.6);
      sprite.scale.setScalar(scale);

      const mat = sprite.material as THREE.SpriteMaterial;
      mat.opacity = Math.max(0.25, blink) * intensity;
    });
  });

  return (
    <group ref={groupRef}>
      {nodes.map((_node, index) => (
        <sprite
          key={index}
          ref={(sprite) => {
            spriteRefs.current[index] = sprite;
          }}
        >
          <spriteMaterial
            map={texture}
            color={nodeColor}
            transparent
            opacity={0.9}
            depthWrite={false}
            toneMapped={false}
            blending={THREE.AdditiveBlending}
          />
        </sprite>
      ))}
    </group>
  );
}
