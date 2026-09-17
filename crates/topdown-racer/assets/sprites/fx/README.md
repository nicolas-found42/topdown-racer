# Driving FX assets

`smoke.png` and `dust.png` are original 16×16 RGBA pixel silhouettes made for
this project (#55), under the repository's asset policy. No external game's
art is used. They replace the former Kenney-derived flat square imports;
`Kenney-License.txt` remains as provenance of that earlier import only.

Both stamps use a stepped union of four ellipses (centres/radii in texels:
6,8/5,5; 9,5/4,4; 10,10/4,4; 4,6/3,3). Pixels outside the union are transparent;
the outer band has half alpha. Smoke uses Rival White (237,241,244), maximum
alpha 192; dust uses Dirt Light (216,154,94), maximum alpha 160. Runtime opacity
is capped at 45% and fades over the existing lifetime. This leaves road edges
and nearby rivals legible when the two cues overlap.

Each stamp remains exactly two world units at 8 texels/unit with nearest
sampling, no growing puff animation, and unchanged trigger/cadence/lifetime.
Audio is original procedural PCM synthesized by `audio.rs`/`feedback.rs`.
