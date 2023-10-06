# High Quality Realtime Rendering

Idk I was originally basing this off of one of the assignments [here](https://inst.eecs.berkeley.edu/~cs294-13/fa09/assignments/second.pdf)
but ended up kinda freestyling it.

```
cargo run --bin realtime [path/to/heightmap/folder]
```

I kinda gave up on generating or triangulating the heightmaps myself. You'll notice I drifted/downsized a lot from my original concept and that was mainly cause I ran out of time and spun my wheels dealing with the terrain instead of anything else.

Use z and x to zoom in and out and k and l to lower and raise the level of detail.

All credit to [tin-terrain](https://github.com/heremaps/tin-terrain) for generating the meshes I collected into x57y418.

~~Heightmaps will have raw caches generated for them, so they may be loaded faster. These caches have the endianness of the system they are generated on.~~

## Original Concept

The assignment is pretty vague so here are the features I want to implement:
- [x] Terrain w/realistic heightmaps
- ~~Plants/animals/decorations~~
- ~~Realistic water~~
- ~~(Soft) shadows~~
- ~~Clouds/fog~~
- ~~Atmosphere~~
- ~~Random terrain generation?~~
- ~~Lens flaring?~~

~~Probably will not implement fluid dynamics -- maybe for fog?~~

The final product should be like a bird flying through the sky around a beautiful natural landscape. Perhaps I could even set it in a precolumbian San Francisco Bay Area?
