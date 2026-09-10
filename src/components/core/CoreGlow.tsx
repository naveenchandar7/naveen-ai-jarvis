import { Bloom, EffectComposer } from "@react-three/postprocessing";
import { BlendFunction, KernelSize } from "postprocessing";

interface CoreGlowProps {
  intensity?: number;
}

export function CoreGlow({ intensity = 1 }: CoreGlowProps) {
  return (
    <EffectComposer multisampling={0}>
      <Bloom
        intensity={1.3 * intensity}
        luminanceThreshold={0.08}
        luminanceSmoothing={0.4}
        mipmapBlur
        kernelSize={KernelSize.LARGE}
        blendFunction={BlendFunction.ADD}
      />
    </EffectComposer>
  );
}
