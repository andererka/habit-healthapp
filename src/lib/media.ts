import { convertFileSrc, invoke } from "@tauri-apps/api/core";

import type { Media } from "../types";

/**
 * Turn a stored media reference into something the webview can load.
 *
 * The file may live in the user's media directory or inside the app bundle
 * (the illustrations shipped with the default exercises), and only the Rust
 * side knows where the bundle is — hence the round trip rather than building
 * the path here.
 */
export async function mediaSrc(media: Media): Promise<string> {
  const path = await invoke<string>("media_path", { src: media.src });
  return convertFileSrc(path);
}
