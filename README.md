# Project Studio

This repository contains all the code for my 11th grade (2022-23) computer graphics project studio.

To run any of the sub-packages, consult their READMEs.

All of these packages require:
- Rust/Cargo w/version > ~1.60 (idr exactly)
- Support for Vulkan, Metal, or DirectX 12 that can render; OpenGL ES 3.0 probably won't work cause of all the push constants

I have only tested this on my two Linux machines so YMMV.

I tidied it up a bit but some projects are still incomplete or a little buggy so yk don't expect this to be professional or anything.

Some of the projects don't have controllable cameras, of those that do use WASD to move and the mouse to direct the camera (except the realtime one; its README has specific instructions).

You may notice that the sub-packages use different versions of the libraries that they basically all rely on. This is reflective of the fact they weren't all made at once; I never went back to update the dependencies, because it would require dealing with breaking changes -- a lot of the libraries used are still unstable.
