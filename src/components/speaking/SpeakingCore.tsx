import { useTheme } from "../../hooks/useTheme";
import { VoiceReactiveOrb } from "./VoiceReactiveOrb";

interface SpeakingCoreProps {
  amplitude?: number;
}

/**
 * TODO (future increment): this is a placeholder speaking visual so the
 * SPEAKING state has somewhere real to render. It intentionally does NOT
 * reuse NovaCore's geometry — give it its own visual language once the
 * voice pipeline (V0 in the NAVEEN AI roadmap) is wired up.
 */
export function SpeakingCore({ amplitude = 0 }: SpeakingCoreProps) {
  const theme = useTheme();

  return (
    <group>
      <VoiceReactiveOrb color={theme.coreColor} amplitude={amplitude} />
      <pointLight color={theme.coreGlow} intensity={4} distance={5} />
    </group>
  );
}
