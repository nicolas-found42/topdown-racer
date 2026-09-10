# Driving FX assets

`smoke.png` and `dust.png` derive from `PNG (Transparent)/smoke_08.png`
in Kenney's [Particle Pack](https://kenney.nl/assets/particle-pack), CC0.
The original grant is retained in `Kenney-License.txt`.

Import uses FFmpeg once, before committing the processed assets:

```sh
ffmpeg -i smoke_08.png -vf 'scale=16:16:flags=area,lutrgb=r=237:g=241:b=244:a=if(lt(val\,48)\,0\,192)' -frames:v 1 smoke.png
ffmpeg -i smoke_08.png -vf 'scale=16:16:flags=area,lutrgb=r=216:g=154:b=94:a=if(lt(val\,48)\,0\,160)' -frames:v 1 dust.png
```

RGB uses the project palette's Rival White and Dirt Light. Alpha is reduced
to transparent/one translucent value; the runtime only fades opacity. Each
16-texel stamp is exactly two world units at 8 texels/unit, with nearest
sampling and no growing/rescaled puff animation.
