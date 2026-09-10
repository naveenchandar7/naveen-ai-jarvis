import * as THREE from "three";

let cachedTexture: THREE.CanvasTexture | null = null;

/**
 * A soft white radial-gradient sprite. Tint it via material.color —
 * additive blending does the rest. Cached so every caller shares one
 * GPU texture instead of generating duplicates.
 */
export function getGlowSpriteTexture(): THREE.CanvasTexture {
  if (cachedTexture) return cachedTexture;

  const size = 128;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;

  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("2D context unavailable for glow sprite generation");

  const gradient = ctx.createRadialGradient(
    size / 2,
    size / 2,
    0,
    size / 2,
    size / 2,
    size / 2,
  );

  gradient.addColorStop(0, "rgba(255,255,255,1)");
  gradient.addColorStop(0.25, "rgba(255,255,255,0.85)");
  gradient.addColorStop(0.55, "rgba(255,255,255,0.25)");
  gradient.addColorStop(1, "rgba(255,255,255,0)");

  ctx.fillStyle = gradient;
  ctx.fillRect(0, 0, size, size);

  const texture = new THREE.CanvasTexture(canvas);
  texture.needsUpdate = true;

  cachedTexture = texture;
  return texture;
}
