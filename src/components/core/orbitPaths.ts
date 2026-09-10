import * as THREE from "three";

export interface OrbitDefinition {
  radius: number;
  flatten: number;
  tilt: [number, number, number];
  speed: number;
  opacity: number;
}

export const ORBITS: OrbitDefinition[] = [
  { radius: 0.85, flatten: 0.9, tilt: [0.18, 0.05, 0.0], speed: 0.24, opacity: 0.26 },
  { radius: 1.02, flatten: 0.85, tilt: [-0.12, 0.08, 0.06], speed: -0.17, opacity: 0.2 },
  { radius: 1.2, flatten: 0.92, tilt: [0.22, -0.06, -0.04], speed: 0.12, opacity: 0.15 },
];

export function getOrbitPoint(orbit: OrbitDefinition, angle: number): THREE.Vector3 {
  const point = new THREE.Vector3(
    Math.cos(angle) * orbit.radius,
    Math.sin(angle) * orbit.radius * orbit.flatten,
    Math.sin(angle * 1.35) * orbit.radius * 0.25,
  );

  point.applyEuler(new THREE.Euler(orbit.tilt[0], orbit.tilt[1], orbit.tilt[2]));

  return point;
}

export function buildOrbitCurve(orbit: OrbitDefinition): THREE.CatmullRomCurve3 {
  const points: THREE.Vector3[] = [];
  const steps = 160;

  for (let i = 0; i <= steps; i += 1) {
    const angle = (i / steps) * Math.PI * 2;
    points.push(getOrbitPoint(orbit, angle));
  }

  return new THREE.CatmullRomCurve3(points, true, "centripetal", 0.15);
}
