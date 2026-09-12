import { Canvas } from "@react-three/fiber";
import { Suspense } from "react";
import * as THREE from "three";
import { NovaCore } from "./NovaCore";
import type { AssistantState } from "../../config/assistantVisualProfiles";

interface NovaCoreSceneProps {
  intensity?: number;
  particleDensity?: number;
  pulseSpeed?: number;
  rotationSpeed?: number;
  visualState?: AssistantState;
}

export function NovaCoreScene({
  intensity = 1,
  particleDensity = 1,
  pulseSpeed = 1,
  rotationSpeed = 1,
  visualState = "idle",
}: NovaCoreSceneProps) {
  return (
    <div className="nova-core-scene">
      <Canvas
        camera={{ position: [0, 0, 5.0], fov: 42 }}
        dpr={[1, 2]}
        gl={{
          antialias: true,
          alpha: true,
          powerPreference: "high-performance",
          // Default ACES tone mapping compresses/desaturates the additive-blended
          // bright colors down to gray. This scene wants raw, saturated glow.
          toneMapping: THREE.NoToneMapping,
        }}
      >
        <Suspense fallback={null}>
          <ambientLight intensity={0.08} />

          <NovaCore
            intensity={intensity}
            particleDensity={particleDensity}
            pulseSpeed={pulseSpeed}
            rotationSpeed={rotationSpeed}
            visualState={visualState}
          />
        </Suspense>
      </Canvas>
    </div>
  );
}
