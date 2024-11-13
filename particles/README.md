# Particle/Cloth Simulation

Implementation of [this](https://ocw.mit.edu/courses/6-837-computer-graphics-fall-2012/resources/mit6_837f12_assn3/).

This crate differs significantly from what the assignment specifies -- a lot of it does not make sense without the provided base code, so I opted to freestyle it a bit.

The `run.sh` script will run the program with the provided flag and reset it every 25-ish seconds.

```
bash particles/run.sh [trans/rainbow]
```

To run it directly:

```
cargo run --bin particles [# of compute pass runs per frame] [trans/rainbow] [OPTIONAL # of frames to persist for]
```
