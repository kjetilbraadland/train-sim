"""Build the train consist (locomotive + 2 freight wagons) for train-sim.

Layout (metres, front = +Y, wheel base at Z=0):
  locomotive  y ∈ [-12, +10]  (22 m)
  wagon 1     y ∈ [-30, -13]  (17 m)
  wagon 2     y ∈ [-48, -31]  (17 m)
  total ≈ 58 m — matches the placeholder in static/app.js.

Outputs:
  static/models/train.glb
  assets/previews/train_three_quarter.png
  assets/previews/train_side.png
  assets/previews/train_front.png
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common  # noqa: E402
from common import (GOLD, GOLD_DARK, CHARCOAL, GAME_GROUND,  # noqa: E402
                   GOOD_COAL, GOOD_STEEL, GOOD_FOOD, GOOD_TIMBER,
                   clear_scene, make_mat, cube, cylinder,
                   setup_world, setup_lights, configure_render,
                   render_preview, export_gltf)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PREV = os.path.join(ROOT, "assets", "previews")
OUT = os.path.join(ROOT, "static", "models", "train.glb")

import bpy  # noqa: E402
from bpy import data, ops  # noqa: E402


# Clear BEFORE creating materials (clear_scene removes unused material
# blocks, which would delete freshly created ones otherwise).
clear_scene()

# ---------- materials ----------
MAT = {
    "gold": make_mat("train_gold", GOLD, roughness=0.45),
    "gold_dark": make_mat("train_gold_dark", GOLD_DARK, roughness=0.5),
    "charcoal": make_mat("train_charcoal", CHARCOAL, roughness=0.7),
    "steel": make_mat("train_steel", 0x39404D, roughness=0.5, metallic=0.4),
    "dark": make_mat("train_dark", 0x1C2029, roughness=0.8),
    "light": make_mat("train_headlight", 0xFFFFFF,
                      emissive=0xFFFFFF, emissive_intensity=2.0),
    "coal": make_mat("good_coal", GOOD_COAL, roughness=0.9),
    "steelc": make_mat("good_steel", GOOD_STEEL, roughness=0.4, metallic=0.7),
    "food": make_mat("good_food", GOOD_FOOD, roughness=0.8),
    "timber": make_mat("good_timber", GOOD_TIMBER, roughness=0.8),
}

parts = []  # everything that goes into the export


def add(obj):
    parts.append(obj)
    return obj


def bogie(y):
    """One 4-wheel bogie under a 5 m wide body, wheel radius 0.9."""
    base = add(cube(f"bogie_{y}", (3.0, 2.4, 0.8), (0, y, 0.5),
                    mat=MAT["steel"]))
    for dx in (-1.35, 1.35):
        for dy in (-0.7, 0.7):
            add(cylinder("wheel", 0.9, 0.35, (dx, y + dy, 0.9),
                        mat=MAT["dark"], rot=(0, 90, 0)))
    return base


def locomotive():
    # Body: z 0.8..4.0, y -12..+10
    add(cube("loco_body", (5.0, 22.0, 3.2), (0, -1.0, 2.4), mat=MAT["gold"]))
    # Roof
    add(cube("loco_roof", (4.2, 18.0, 0.4), (0, -1.0, 4.2),
             mat=MAT["gold_dark"]))
    # Cab: raised block over the front third
    add(cube("loco_cab", (4.4, 6.0, 1.3), (0, 5.0, 4.95), mat=MAT["gold_dark"]))
    # Front window band
    add(cube("loco_front_glass", (3.4, 0.3, 1.1), (0, 10.05, 3.1),
             mat=MAT["dark"]))
    # Cab windows (front + side)
    add(cube("loco_cab_glass_f", (3.0, 0.3, 0.8), (0, 8.05, 4.9),
             mat=MAT["dark"]))
    add(cube("loco_cab_glass_l", (0.3, 2.6, 0.8), (-2.25, 5.0, 4.9),
             mat=MAT["dark"]))
    add(cube("loco_cab_glass_r", (0.3, 2.6, 0.8), (2.25, 5.0, 4.9),
             mat=MAT["dark"]))
    # Side window strip
    add(cube("loco_glass_l", (0.25, 9.0, 0.8), (-2.55, -4.0, 3.0),
             mat=MAT["dark"]))
    add(cube("loco_glass_r", (0.25, 9.0, 0.8), (2.55, -4.0, 3.0),
             mat=MAT["dark"]))
    # Gold trim stripe along the body
    add(cube("loco_stripe_l", (0.15, 21.0, 0.35), (-2.55, -1.0, 1.5),
             mat=MAT["gold_dark"]))
    add(cube("loco_stripe_r", (0.15, 21.0, 0.35), (2.55, -1.0, 1.5),
             mat=MAT["gold_dark"]))
    # Headlights (emissive) at the nose corners
    for dx in (-1.7, 1.7):
        add(cube(f"headlight_{dx}", (0.55, 0.35, 0.55), (dx, 10.15, 3.4),
                 mat=MAT["light"]))
    # Bogies
    bogie(-8.0)
    bogie(5.5)


def wagon(y0, y1, crate_colors):
    """Open-top freight wagon with low walls and a cargo stack."""
    length = y1 - y0
    yc = (y0 + y1) / 2
    # Body: z 0.9..3.1
    add(cube("wagon_body", (4.6, length, 2.2), (0, yc, 2.0),
             mat=MAT["charcoal"]))
    # Gold trim band
    add(cube("wagon_band_t", (4.7, length, 0.25), (0, yc, 3.05),
             mat=MAT["gold"]))
    add(cube("wagon_band_b", (4.7, length, 0.25), (0, yc, 1.0),
             mat=MAT["gold"]))
    # Low walls (4 rails, z 3.1..4.0)
    wall_h, wall_t = 0.9, 0.25
    for dx in (-2.2, 2.2):
        add(cube("wagon_wall_side", (wall_t, length, wall_h),
                 (dx, yc, 3.1 + wall_h / 2), mat=MAT["charcoal"]))
    for dy in (-length / 2, length / 2):
        add(cube("wagon_wall_end", (4.6, wall_t, wall_h),
                 (0, yc + dy, 3.1 + wall_h / 2), mat=MAT["charcoal"]))
    # Cargo stack: 3 crates per wagon, one per good colour
    for i, mat in enumerate(crate_colors):
        cx = -1.6 + i * 1.6
        add(cylinder(f"crate_{i}", 0.9, 1.6, (cx, yc, 3.9), mat=mat,
                    vertices=8))
        add(cube(f"crate_top_{i}", (1.3, 1.3, 0.5), (cx, yc, 4.95), mat=mat))
    # Bogies
    bogie(y0 + 2.5)
    bogie(y1 - 2.5)


# ---------- build ----------
# Ground disc for previews (not exported)
bpy.ops.mesh.primitive_cylinder_add(radius=75, depth=1.5, vertices=48,
                                    location=(0, -19, -0.75))
ground = bpy.context.object
ground.name = "preview_ground"
gm = make_mat("preview_ground", GAME_GROUND, roughness=0.95)
ground.data.materials.append(gm)

locomotive()
wagon(-30, -13, [MAT["coal"], MAT["steelc"], MAT["food"]])
wagon(-48, -31, [MAT["timber"], MAT["coal"], MAT["steelc"]])

# Single export root (an empty)
root = data.objects.new("train", None)
bpy.context.collection.objects.link(root)
for p in parts:
    p.parent = root
    p.matrix_parent_inverse = root.matrix_world.inverted()

setup_world()
setup_lights()
configure_render(width=1024, height=1024, samples=48)

# ---------- previews ----------
render_preview(parts, os.path.join(PREV, "train_three_quarter.png"),
              cam_loc=(52, 55, 30), target=(0, -18, 5))
render_preview(parts, os.path.join(PREV, "train_side.png"),
              cam_loc=(75, -19, 22), target=(0, -19, 5))
render_preview(parts, os.path.join(PREV, "train_front.png"),
              cam_loc=(0, 75, 16), target=(0, 2, 6))

# ---------- export ----------
export_gltf([root], OUT)
