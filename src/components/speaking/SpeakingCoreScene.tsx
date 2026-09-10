import { Canvas } from "@react-three/fiber";
import { Suspense } from "react";
import { SpeakingCore } from "./SpeakingCore";

interface SpeakingCoreSceneProps {
  amplitude?: number;
}

export function SpeakingCoreScene({ amplitude = 0 }: SpeakingCoreSceneProps) {
  return (
    <div className="nova-core-scene">
      <Canvas
        camera={{ position: [0, 0, 5.6], fov: 42 }}
        dpr={[1, 2]}
        gl={{ antialias: true, alpha: true, powerPreference: "high-performance" }}
      >
        <Suspense fallback={null}>
          <ambientLight intensity={0.08} />
          <SpeakingCore amplitude={amplitude} />
        </Suspense>
      </Canvas>
    </div>
  );
}
