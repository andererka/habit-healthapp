import { useEffect, useState } from "react";

import { mediaSrc } from "../lib/media";
import type { Media } from "../types";

/**
 * Renders an exercise's image or video.
 *
 * Videos autoplay muted and loop: an exercise demo is a silent illustration you
 * copy, not something you sit and watch, and an unmuted autoplay in a window
 * that appears unannounced would be startling.
 */
export function MediaView({ media }: { media: Media }) {
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let current = true;
    setFailed(false);
    mediaSrc(media).then((resolved) => {
      if (current) setSrc(resolved);
    });
    return () => {
      current = false;
    };
  }, [media]);

  if (failed) {
    return <p className="media-missing">That file is missing from the media folder.</p>;
  }

  if (!src) return null;

  if (media.kind === "video") {
    return (
      <video
        className="media"
        src={src}
        autoPlay
        muted
        loop
        playsInline
        onError={() => setFailed(true)}
      />
    );
  }

  return <img className="media" src={src} alt="" onError={() => setFailed(true)} />;
}
