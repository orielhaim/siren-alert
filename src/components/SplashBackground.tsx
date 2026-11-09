import { useEffect, useRef } from "react";
import createGlobe from "cobe";
export function SplashBackground() {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    let globe: ReturnType<typeof createGlobe>;
    let frame = 0;
    let phi = 4.15;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    try {
      globe = createGlobe(canvas, {
        devicePixelRatio: Math.min(devicePixelRatio, 2),
        width: 900,
        height: 900,
        phi,
        theta: 0.24,
        dark: 1,
        diffuse: 1.8,
        mapSamples: 18000,
        mapBrightness: 4.5,
        baseColor: [0.17, 0.16, 0.16],
        markerColor: [0.95, 0.35, 0.28],
        glowColor: [0.16, 0.08, 0.075],
        markers: [{ location: [31.8, 35.2], size: 0.055 }],
        opacity: 0.9,
      });
    } catch {
      return;
    }
    const animate = () => {
      if (!document.hidden && !reduced.matches) {
        phi += 0.0012;
        globe.update({ phi });
      }
      frame = requestAnimationFrame(animate);
    };
    if (!reduced.matches) frame = requestAnimationFrame(animate);
    return () => {
      cancelAnimationFrame(frame);
      globe.destroy();
    };
  }, []);
  return (
    <div className="splash-background" aria-hidden="true">
      <div className="splash-haze" />
      <canvas ref={ref} />
      <div className="splash-vignette" />
    </div>
  );
}
