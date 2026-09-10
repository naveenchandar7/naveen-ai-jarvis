import { useTheme } from "../../hooks/useTheme";
import {
  visualProfiles,
  type AssistantVisualState,
} from "../../config/visualProfiles";

import { CoreGlow } from "./CoreGlow";
import { EnergySphere } from "./EnergySphere";
import { NeuralPlexus } from "./NeuralPlexus";
import { NodeSystem } from "./NodeSystem";
import { OrbitSystem } from "./OrbitSystem";
import { ParticleField } from "./ParticleField";

interface NovaCoreProps {
  intensity?: number;
  particleDensity?: number;
  pulseSpeed?: number;
  rotationSpeed?: number;
  visualState?: AssistantVisualState;
}

export function NovaCore({
  intensity = 1,
  particleDensity = 1,
  pulseSpeed = 1,
  rotationSpeed = 1,
  visualState = "idle",
}: NovaCoreProps) {
  const theme = useTheme();

  /*
   * Visual profile is the single source of truth
   * for neural behavior.
   *
   * Later:
   * idle → listening → thinking → speaking
   * and eventually:
   * neural HUD → JARVIS HUD → other visual skins
   */
  const profile =
    visualProfiles[visualState];

  return (
    <group>
      {/* =========================================
          CENTRAL ENERGY CORE
      ========================================= */}

      <EnergySphere
  coreColor={theme.coreColor}
  glowColor={theme.coreGlow}
  secondaryColor={
    profile.palette.accent
  }
  hotColor={
    profile.palette.hot
  }
  intensity={
    intensity *
    profile.intensity
  }
  pulseSpeed={
    pulseSpeed *
    profile.speed
  }
/>

      {/* =========================================
          DYNAMIC NEURAL FIELD
      ========================================= */}

      <NeuralPlexus
        coreHotColor={
          theme.nodeColor
        }
        edgeColor={
          theme.particleColor
        }

        palette={
          profile.palette
        }

        intensity={
          intensity *
          profile.intensity
        }

        pointCount={
          Math.round(
            190 *
            profile.density,
          )
        }

        density={
          profile.density
        }

        activity={
          profile.activity
        }

        speed={
          pulseSpeed *
          profile.speed
        }
      />

      {/* =========================================
          SECONDARY ORBIT STRUCTURE

          Keep subtle.
          Neural field is the main visual.
      ========================================= */}

      <OrbitSystem
        orbitColor={
          theme.orbitColor
        }
        intensity={
          intensity * 0.08
        }
        rotationSpeed={
          rotationSpeed *
          profile.speed
        }
      />

      {/* =========================================
          ACTIVE TRAVELLING NODES
      ========================================= */}

      <NodeSystem
        nodeColor={
          theme.nodeColor
        }
        intensity={
          intensity *
          profile.intensity *
          0.75
        }
        speed={
          pulseSpeed *
          profile.speed
        }
      />

      {/* =========================================
          BACKGROUND PARTICLES
      ========================================= */}

      <ParticleField
        particleColor={
          theme.particleColor
        }
        count={Math.round(
          320 *
          particleDensity *
          profile.density,
        )}
        intensity={
          intensity *
          profile.intensity *
          0.5
        }
        speed={
          pulseSpeed *
          profile.speed *
          0.5
        }
      />

      {/* =========================================
          BLOOM / GLOW
      ========================================= */}

      <CoreGlow
        intensity={
          intensity *
          profile.intensity
        }
      />
    </group>
  );
}