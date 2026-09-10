import { useFrame } from "@react-three/fiber";
import { useRef } from "react";
import * as THREE from "three";

interface VoiceReactiveOrbProps {
  color: string;
  /** 0..1 live voice amplitude — wire this up to the audio pipeline later. */
  amplitude?: number;
}

/**
 * Deliberately separate from EnergySphere (see NAVEEN AI FRONTEND VISUAL ENGINE
 * REQUIREMENT, section 9). This is the seed of the future SPEAKING visual —
 * it must never be merged into NovaCore.
 */
export function VoiceReactiveOrb({ color, amplitude = 0 }: VoiceReactiveOrbProps) {
  const meshRef = useRef<THREE.Mesh>(null);

  useFrame(({ clock }) => {
    const mesh = meshRef.current;
    if (!mesh) return;

    const time = clock.getElapsedTime();
    const pulse = 1 + amplitude * 0.35 + Math.sin(time * 6) * 0.02 * amplitude;

    mesh.scale.setScalar(pulse);
  });

  return (
    <mesh ref={meshRef}>
      <sphereGeometry args={[0.5, 32, 32]} />
      <meshBasicMaterial
        color={color}
        transparent
        opacity={0.6 + amplitude * 0.4}
        blending={THREE.AdditiveBlending}
        depthWrite={false}
      />
    </mesh>
  );
}
