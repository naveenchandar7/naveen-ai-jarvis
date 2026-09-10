import { useFrame } from "@react-three/fiber";
import { useMemo, useRef } from "react";
import * as THREE from "three";

interface ParticleFieldProps {
  particleColor: string;
  count?: number;
  intensity?: number;
  speed?: number;
}

interface Particle {
  radius: number;
  angle: number;
  vertical: number;
  depth: number;
  speed: number;
  phase: number;
}

function createParticles(count: number): Particle[] {
  const particles: Particle[] = [];

  for (let i = 0; i < count; i += 1) {
    particles.push({
      radius: 0.75 + Math.random() * 1.15,
      angle: Math.random() * Math.PI * 2,
      vertical: 0.35 + Math.random() * 0.9,
      depth: -0.55 + Math.random() * 1.1,
      speed: 0.18 + Math.random() * 0.55,
      phase: Math.random() * Math.PI * 2,
    });
  }

  return particles;
}

export function ParticleField({
  particleColor,
  count = 900,
  intensity = 1,
  speed = 1,
}: ParticleFieldProps) {
  const pointsRef = useRef<THREE.Points>(null);

  const particles = useMemo(() => createParticles(count), [count]);
  const positions = useMemo(() => new Float32Array(count * 3), [count]);

  useFrame(({ clock }) => {
    const points = pointsRef.current;
    if (!points) return;

    const time = clock.getElapsedTime();

    particles.forEach((particle, index) => {
      const orbitSpeed = particle.speed * speed;
      const angle = particle.angle + time * orbitSpeed;

      // Slightly elliptical 3D motion, distributed around the orbital system.
      const x = Math.cos(angle) * particle.radius;
      const y = Math.sin(angle) * particle.radius * 0.42 * particle.vertical;
      const z = Math.sin(angle * 1.7 + particle.phase) * particle.depth;

      const driftX = Math.sin(time * 0.35 + particle.phase) * 0.035;
      const driftY = Math.cos(time * 0.28 + particle.phase) * 0.03;
      const driftZ = Math.sin(time * 0.22 + particle.phase) * 0.04;

      positions[index * 3] = x + driftX;
      positions[index * 3 + 1] = y + driftY;
      positions[index * 3 + 2] = z + driftZ;
    });

    const attribute = points.geometry.getAttribute("position");
    attribute.needsUpdate = true;

    points.rotation.y = time * 0.012;
    points.rotation.x = Math.sin(time * 0.12) * 0.025;
  });

  return (
    <points ref={pointsRef}>
      <bufferGeometry>
        <bufferAttribute attach="attributes-position" args={[positions, 3]} />
      </bufferGeometry>

      <pointsMaterial
        color={particleColor}
        size={0.02}
        transparent
        opacity={0.72 * intensity}
        depthWrite={false}
        sizeAttenuation
        toneMapped={false}
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}
