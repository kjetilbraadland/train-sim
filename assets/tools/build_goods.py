"""Build the four goods/cargo icons for train-sim.

Matches the engine's Goods enum (engine/src/state.rs):
  Coal   -> 0x555555
  Steel  -> 0x8899AA
  Food   -> 0x55CC55
  Timber -> 0xB08040

Each icon is ~4 m tall, base at z=0, on a small base disc. Icons are
laid out in a row for previewing and exported as separate glTF files:

  static/models/goods_coal.glb
  static/models/goods_steel.glb
  static/models/goods_food.glb
  static/models/goods_timber.glb
  assets/previews/goods_all.png
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import common  # noqa: E402
from common import (GOOD_COAL, GOOD_STEEL, GOOD_FOOD, GOOD_TIMBER,  # noqa: E402
                   CHARCOAL, GAME_GROUND, STEEL_GREY,
                   clear_scene, make_mat, cube, cylinder,
                   setup_world, setup_lights, configure_render,
                   render_preview, export_gltf)
from bpy import data  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PREV = os.path.join(ROOT, "assets", "previews")
OUT = os.path.join(ROOT, "static", "models")

clear_scene()

MAT = {
    "coal": make_mat("g_coal", GOOD_COAL, roughness=0.9),
    "coal_dark": make_mat("g_coal_dark", 0x333333, roughness=0.95),
    "steel": make_mat("g_steel", GOOD_STEEL, roughness=0.35, metallic=0.7),
    "steel_dark": make_mat("g_steel_dark", 0x667788, roughness=0.4,
                          metallic=0.6),
    "food": make_mat("g_food", GOOD_FOOD, roughness=0.7),
    "food_dark": make_mat("g_food_dark", 0x3A993A, roughness=0.8),
    "timber": make_mat("g_timber", GOOD_TIMBER, roughness=0.75),
    "timber_dark": make_mat("g_timber_dark", 0x8A6430, roughness=0.8),
    "pad": make_mat("g_pad", 0x1A2533, roughness=0.9),
    "ground_disc": make_mat("g_preview_ground", GAME_GROUND, roughness=0.95),
}

icons = {}


def new_icon(name, x):
    """Create the icon root Empty at position (x, 0, 0)."""
    root = data.objects.new(name, None)
    bpy.context.collection.objects.link(root)
    root.location = (x, 0, 0)
    icons[name] = root
    return root


def add_to(root, obj):
    # Parts are built in world coords around the origin; the icon root
    # sits at (x, 0, 0) with no rotation, so plain parenting (identity
    # parent inverse) re-expresses their transforms relative to the root.
    obj.parent = root


def pad(icon, r=2.6):
    """Base disc the icon sits on."""
    cyl = cylinder("pad", r, 0.25, (0, 0, 0.125), mat=MAT["pad"],
                  vertices=24)
    add_to(icon, cyl)
    return cyl


# ---------- coal: a lumpy pile ----------
def build_coal(x):
    icon = new_icon("goods_coal", x)
    pad(icon)
    bpy.ops.mesh.primitive_cone_add(radius1=1.9, radius2=0.4, depth=1.2,
                                   location=(0, 0, 1.0), vertices=12)
    add_to(icon, bpy.context.object)
    lumps = [(-0.7, 0.4, 0.9, 0.75), (0.6, -0.4, 0.95, 0.7),
            (0.1, 0.1, 1.7, 0.65), (-0.4, -0.5, 1.5, 0.5),
            (0.5, 0.5, 1.6, 0.45)]
    for i, (lx, ly, lz, lr) in enumerate(lumps):
        bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=1, radius=lr,
                                             location=(lx, ly, lz))
        o = bpy.context.object
        o.scale = (1, 1, 0.85)
        o.rotation_euler[2] = i * 0.7
        o.data.materials.append(MAT["coal"])
        add_to(icon, o)
    return icon


# ---------- steel: an I-beam standing + 2 flat beams ----------
def build_steel(x):
    icon = new_icon("goods_steel", x)
    pad(icon)

    def i_beam(loc, vertical=True):
        if vertical:
            f = cube("flange_b", (1.6, 0.6, 0.35), (loc[0], loc[1], 0.35),
                    mat=MAT["steel"])
            w = cube("web", (0.35, 0.6, 2.3), loc, mat=MAT["steel_dark"])
            t = cube("flange_t", (1.6, 0.6, 0.35),
                    (loc[0], loc[1], loc[2] * 2 + 0.35), mat=MAT["steel"])
            add_to(icon, f); add_to(icon, w); add_to(icon, t)
        else:
            f1 = cube("fl1", (0.35, 1.6, 0.35), (loc[0], loc[1], loc[2]),
                    mat=MAT["steel"])
            w = cube("web", (0.35, 0.35, 3.4),
                    (loc[0], loc[1], loc[2] + 0.1), mat=MAT["steel_dark"])
            f2 = cube("fl2", (0.35, 1.6, 0.35),
                     (loc[0], loc[1], loc[2] + 0.2), mat=MAT["steel"])
            add_to(icon, f1); add_to(icon, w); add_to(icon, f2)

    i_beam((-0.9, -0.9, 0.725), vertical=True)
    i_beam((0.9, 0.9, 2.2), vertical=False)
    i_beam((0.9, 0.9, 3.0), vertical=False)
    return icon


# ---------- food: a grain sack ----------
def build_food(x):
    icon = new_icon("goods_food", x)
    pad(icon)
    bpy.ops.mesh.primitive_uv_sphere_add(radius=1.5, segments=16,
                                       ring_count=12)
    body = bpy.context.object
    body.scale = (1, 0.75, 1.15)
    body.location = (0, 0, 1.7)
    body.data.materials.append(MAT["food"])
    add_to(icon, body)
    bpy.ops.mesh.primitive_cylinder_add(radius=0.45, depth=0.5,
                                     location=(0, 0, 3.45), vertices=12)
    neck = bpy.context.object
    neck.data.materials.append(MAT["food_dark"])
    add_to(icon, neck)
    bpy.ops.mesh.primitive_torus_add(major_radius=0.55, minor_radius=0.18,
                                   location=(0, 0, 3.15))
    knot = bpy.context.object
    knot.data.materials.append(MAT["food_dark"])
    add_to(icon, knot)
    # a small crate at the feet
    add_to(icon, cube("crate", (1.4, 1.4, 1.4), (1.1, 0.9, 0.725),
                    mat=MAT["food_dark"]))
    return icon


# ---------- timber: a stacked log pile ----------
def build_timber(x):
    icon = new_icon("goods_timber", x)
    pad(icon)

    def log(loc, mat, r=0.5, length=4.0):
        cyl = cylinder("log", r, length, loc, mat=mat, vertices=12,
                      rot=(90, 0, 0))
        add_to(icon, cyl)

    # bottom row of 3
    log((-1.1, 0.5, 0.725 + 0.5), MAT["timber"])
    log((1.1, 0.5, 0.725 + 0.5), MAT["timber"])
    log((0.0, -0.8, 0.725 + 0.5), MAT["timber"])
    # top row of 2
    log((-0.55, 0.5, 0.725 + 1.5), MAT["timber_dark"])
    log((0.55, 0.5, 0.725 + 1.5), MAT["timber_dark"])
    return icon


build_coal(-13)
build_steel(-4.5)
build_food(4.5)
build_timber(13)

# ---------- preview ground ----------
bpy.ops.mesh.primitive_cylinder_add(radius=45, depth=1.5, vertices=48,
                                   location=(0, 0, -0.75))
ground = bpy.context.object
ground.data.materials.append(MAT["ground_disc"])

setup_world()
setup_lights()
configure_render(width=1024, height=1024, samples=48)

render_preview([], os.path.join(PREV, "goods_all.png"),
              cam_loc=(0, -42, 22), target=(0, 0, 2))

for name in ("goods_coal", "goods_steel", "goods_food", "goods_timber"):
    root = icons[name]
    keep = root.location.copy()
    root.location = (0, 0, 0)  # export each icon centred on its origin
    export_gltf([root], os.path.join(OUT, name + ".glb"))
    root.location = keep
