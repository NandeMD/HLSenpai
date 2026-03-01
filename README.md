
All perfectly GOP-aligned.  
All 720p.  
All adaptive.  

Just how senpai likes it.

---

## 🔥 Features

- 720p multi-bitrate ladder (low / medium / high)
- Strict GOP alignment (2×FPS)
- Independent segments
- CloudFront-ready folder structure
- Proper HLS VOD packaging
- Future DRM-compatible architecture
- No weird transcoding chaos

---

## 🧠 Encoding Philosophy

We believe:

- H.264 is still king for compatibility
- AAC LC is the safest audio choice
- TS segments are the least dramatic option
- `-sc_threshold 0` is sacred
- Keyframes must behave
- Segments must align
- Bitrate ladders should scale gracefully

No cursed defaults.  
No mysterious FFmpeg incantations.  
Just clean, predictable output.

---

## 🛠 Tech Stack

| Layer | Tech |
|-------|------|
| GUI | `iced` |
| Core | Rust |
| Video Engine | `rust-ffmpeg` |
| Codec | libx264 |
| Audio | AAC |
| Output | HLS (TS) |
| Deployment Target | AWS S3 + CloudFront |

---

## 💅 Typical Output Settings

| Variant | Resolution | Video Bitrate | Audio |
|----------|------------|---------------|-------|
| Low | 1280×720 | 1200 kbps | 96k |
| Medium | 1280×720 | 2500 kbps | 128k |
| High | 1280×720 | 4500 kbps | 128k |

All:

- Profile: High
- Level: 4.1
- Pixel format: yuv420p
- Segment duration: 6s
- GOP: 2×FPS

---

## 😈 Why Not Just Use Raw FFmpeg?

Because:

- GUIs are hot.
- Manual CLI flags are error-prone.
- Forgetting `-keyint_min` is a sin.
- You deserve better.

HLSenpai makes the right choices by default.

---

## ⚠️ Requirements

- FFmpeg installed with:
  - libx264
  - AAC encoder
- Rust stable toolchain
- A respectable internet connection
- Good taste

---

## 🧎 A Final Note

HLSenpai does not judge your input videos.  
It simply scales them to 720p and makes them stream-ready.

Gracefully.  
Efficiently.  
A little smugly.
