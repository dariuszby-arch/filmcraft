# FilmCraft DW2 — True 10-bit SDR HEVC (experimental)

Branch: `feature/hevc-true10-dw2`. DW1 `v0.20.1-DW1` remains unmodified and published.

## Change
- For H.265/HEVC Main 10 only, the existing compositor keeps linear premultiplied RGBA f32 through rendering, placement, overlays and export effects.
- Once the final image is ready, RGB is encoded through the same continuous sRGB-like transfer curve used by FilmCraft's SDR monitor and converted once to little-endian `rgb48le` (16-bit RGB/channel).
- FFmpeg converts RGB48 to `yuv420p10le` (libx265) or `p010le` (Intel Quick Sync) and encodes HEVC Main 10 with SDR BT.709 metadata.
- Main 8, non-HEVC and existing HDR paths retain DW1 behavior.
- The export video limiter operates on the high-precision encoded RGB values before final 16-bit quantisation, not on RGBA8.
- Linux / Windows GitHub Actions build branches support this experimental implementation; compare the two releases with actual 10-bit gradient and moving footage.

## Important scope
- This fixes the **RGBA8 bottleneck after rendering**, not the precision of every original source/effect. 8-bit media remains 8-bit at its origin, and some overlays or decoding stages may still be 8-bit.
- SDR uses the project's current sRGB-like display transfer curve while signaling BT.709 in the output; calibrating an end-to-end BT.709 transfer workflow would require a separate color-management review.
- GPU export uses an Rgba16Float accumulator where available; CPU uses float32. It does not implement HDR10/PQ/HLG, new color spaces, or advanced 10-bit monitor output.
- Higher memory bandwidth and CPU conversion work are expected at 4K due to RGB48 (6 vs 4 bytes/pixel).
- **This branch is experimental until build, gradient, color, speed and playback tests pass.** No changes to the DW1 published release.

Verify with:
```bash
ffprobe -v error -select_streams v:0 -show_entries stream=codec_name,profile,pix_fmt,color_primaries,color_transfer,color_space -of default=nw=1 video.mp4
```
