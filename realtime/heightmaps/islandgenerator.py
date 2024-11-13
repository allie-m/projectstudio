import noise
import numpy as np
import random
import sys

from PIL import Image

args = sys.argv

shape = (int(args[1]), int(args[2]))
scale = 0.3
octaves = 8
persistence = 0.5
lacunarity = 2.0
seed = random.randint(1, 100000)

x_idx = np.linspace(0, 1, shape[0])
y_idx = np.linspace(0, 1, shape[1])
world_x, world_y = np.meshgrid(x_idx, y_idx)

world = np.vectorize(noise.snoise2)(world_x/scale, world_y/scale, octaves=octaves, persistence=persistence, lacunarity=lacunarity, repeatx=shape[0], repeaty=shape[1], base=seed)
world = (world + 1.0) / 2.0

def c(x, z):
    bump = lambda t : max(0.0, 1.0 - t**6)
    return bump(x) * bump(z) * 0.9
    # return (1.0 - (x**2 + z**2) / 2.0)

a = np.array([[c(((x / shape[0]) * 2.0 - 1.0), ((z / shape[1]) * 2.0 - 1.0)) for z in range(shape[1])] for x in range(shape[0])])
b = np.ones(world.shape)
world = np.multiply(world, a)

img = np.floor(world * 255.0).astype(np.uint8)
Image.fromarray(img).save("island.png")
