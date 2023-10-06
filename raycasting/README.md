# Raycasting and Raytracing

Implementation of [these](https://ocw.mit.edu/courses/6-837-computer-graphics-fall-2012/resources/mit6_837f12_assn4/) [two](https://ocw.mit.edu/courses/6-837-computer-graphics-fall-2012/resources/mit6_837f12_assn5/) assignments. Implementation of the latter is incomplete -- refraction, jitter, and noise are unimplemented, so scenes 11, 12, and 13 will not render as expected (or just not render).

```
cargo run --bin raycasting [width] [height] [max bounces] [path/to/scene] > path/to/output.png
```

The width and height parameters must be multiples of 256.
