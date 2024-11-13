# Curves & Surfaces

Implementation of [this](https://ocw.mit.edu/courses/6-837-computer-graphics-fall-2012/resources/mit6_837f12_assn2/).

```
cargo run --bin hierarchicalmodeling [path/to/Model] [load obj?]
```

Note that you do NOT pipe the file contents into this executible, instead you give an argument specifying the path to the model's skel, obj, and attach files. Give no extension in the run command.

The optional boolean option notes whether to load the model's obj file. Defaults to true for now.

The false case is unimplemented and will crash if toggled.
