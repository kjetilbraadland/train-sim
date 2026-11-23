"""Build the city/station asset for train-sim.

The engine's cities are hub nodes (station nodes) where trains stop to
load/unload. The client currently draws them as blue cylinders; this
replaces that with a proper station: platform + canopy + city buildings.

Footprint: ~75 m wide, track runs along Y at x=0 (track = 8 m wide,
matching the client's 4 m radius cylinders).

Layout (metres, front = +Y):
  platform  x ∈ [ 5.5, 14.5], y ∈ [-14, 14]
  buildings x ∈ [17, 55]
Outputs:
  static/models/station.glb
  assets/previews/station_*.png
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import common  # noqa: E402
from common import (GOLD, GOLD_DARK, CHARCOAL, GAME_CITY, GAME_GROUND,  # noqa: E402
                   clear_scene, make_mat, cube, cylinder,
                   setup_world, setup_lights, configure_render,
                   render_preview, export_gltf)

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PREV = os.path.join(ROOT, "assets", "previews")
OUT = os.path.join(ROOT, "static", "models", "station.glb")
from bpy import data  # noqa: E402

clear_scene()

MAT = {
    "platform": make_mat("st_platform", 0x9AA8B8, roughness=0.85),
    "platform_edge": make_mat("st_platform_edge", 0x6B7889, roughness=0.85),
    "roof": make_mat("st_roof", 0x2E3A4A, roughness=0.7),
    "roof_edge": make_mat("st_roof_edge", GOLD, roughness=0.5),
    "pole": make_mat("st_pole", 0x39404D, roughness=0.6, metallic=0.3),
    "city_blue": make_mat("st_city_blue", GAME_CITY, roughness=0.6),
    "city_warm": make_mat("st_city_warm", 0xB0783C, roughness=0.7),
    "city_dark": make_mat("st_city_dark", 0x243247, roughness=0.8),
    "roof_blue": make_mat("st_roof_blue", 0x1F4E79, roughness=0.6),
    "window": make_mat("st_window", 0xFFD9A0,
                       emissive=0xFFD9A0, emissive_intensity=1.2),
    "sign": make_mat("st_sign", 0x1C2029, roughness=0.7),
    "sign_edge": make_mat("st_sign_edge", GOLD, roughness=0.45),
    "ground_disc": make_mat("st_preview_ground", GAME_GROUND, roughness=0.95),
}

parts = []


def add(obj):
    parts.append(obj)
    return obj


# ---------- platform ----------
# Slab (raised 1.2 m) with a darker edge band
add(cube("platform", (9.0, 28.0, 1.2), (10.0, 0.0, 0.6),
         mat=MAT["platform"]))
add(cube("platform_edge_l", (0.3, 28.0, 0.3), (5.55, 0.0, 1.35),
         mat=MAT["platform_edge"]))
add(cube("platform_edge_r", (0.3, 28.0, 0.3), (14.45, 0.0, 1.35),
         mat=MAT["platform_edge"]))

# Canopy: 4 poles + roof over the middle of the platform
for dy in (-8.0, 8.0):
    for dx in (6.5, 13.5):
        add(cylinder("canopy_pole", 0.35, 4.5, (dx, dy, 2.55),
                    mat=MAT["pole"], vertices=10))
add(cube("canopy_roof", (9.5, 20.0, 0.5), (10.0, 0.0, 5.1),
         mat=MAT["roof"]))
add(cube("canopy_roof_edge", (9.7, 20.2, 0.2), (10.0, 0.0, 4.85),
         mat=MAT["roof_edge"]))

# Bench
add(cube("bench", (1.6, 4.0, 0.5), (12.8, -2.0, 1.65), mat=MAT["city_dark"]))
add(cube("bench_back", (0.3, 4.0, 0.9), (13.6, -2.0, 2.3),
         mat=MAT["city_dark"]))

# Lamp post
add(cylinder("lamp_pole", 0.3, 6.0, (7.0, 10.0, 3.0), mat=MAT["pole"],
            vertices=10))
add(cube("lamp", (1.0, 1.0, 0.6), (7.0, 10.0, 6.3), mat=MAT["window"]))


# ---------- city buildings ----------
def building(x, y, w, d, h, mat, roof_mat, windows=True):
    """Box building + roof; optional emissive window strip."""
    add(cube("bldg", (w, d, h), (x, y, h / 2 + 0.0), mat=mat))
    add(cube("bldg_roof", (w * 1.06, d * 1.06, 0.6),
             (x, y, h + 0.3), mat=roof_mat))
    if windows:
        wh = 1.4
        n = max(1, int(d // 4))
        for i in range(n):
            wy = y - d / 2 + (i + 0.5) * (d / n)
            add(cube("bldg_win", (0.2, 2.0, wh), (x - w / 2 - 0.1, wy,
                                                   h * 0.55), mat=MAT["window"]))


building(24, -6, 9, 16, 14, MAT["city_blue"], MAT["roof_blue"])
building(36, -14, 8, 10, 10, MAT["city_warm"], MAT["roof"])
building(34, 4, 10, 14, 18, MAT["city_dark"], MAT["roof_blue"])
building(47, -4, 12, 12, 9, MAT["city_blue"], MAT["roof"])
building(48, 12, 9, 9, 13, MAT["city_warm"], MAT["roof"])

# Clock tower
add(cube("tower", (6, 6, 22), (20, 16, 11), mat=MAT["city_blue"]))
add(cube("tower_top", (7, 7, 1.2), (20, 16, 22.6), mat=MAT["roof_edge"]))
add(cylinder("tower_light", 0.9, 1.4, (20, 16, 24.0), mat=MAT["window"],
            vertices=12))

# ---------- station sign ----------
for dx in (17.0, 21.0):
    add(cylinder("sign_pole", 0.3, 5.5, (dx, 16.0, 2.75), mat=MAT["pole"],
                vertices=10))
add(cube("sign_board", (6.0, 0.5, 1.6), (19.0, 16.0, 5.8), mat=MAT["sign"]))
add(cube("sign_border", (6.2, 0.55, 0.25), (19.0, 16.0, 6.65),
         mat=MAT["sign_edge"]))

# ---------- preview ground + track hint ----------
bpy.ops.mesh.primitive_cylinder_add(radius=60, depth=1.5, vertices=48,
                                   location=(28, 0, -0.75))
ground = bpy.context.object
ground.data.materials.append(MAT["ground_disc"])
# Track line along Y (the platform sits beside it)
for dy in range(-24, 25, 2):
    add(cube("track_seg_preview", (8, 1.9, 1.2), (0, dy, 0.6),
             mat=MAT["platform"]))

# Exclude preview-only track segments from the export
export_parts = [p for p in parts if not p.name.startswith("track_seg_preview")]

# Export root (empty)
root = data.objects.new("station", None)
bpy.context.collection.objects.link(root)
for p in export_parts:
    p.parent = root
    p.matrix_parent_inverse = root.matrix_world.inverted()

setup_world()
setup_lights()
configure_render(width=1024, height=1024, samples=48)

render_preview(export_parts, os.path.join(PREV, "station_oblique.png"),
              cam_loc=(85, -55, 38), target=(24, 0, 8))
render_preview(export_parts, os.path.join(PREV, "station_platform.png"),
              cam_loc=(-18, -40, 22), target=(10, 0, 6))
render_preview(export_parts, os.path.join(PREV, "station_city.png"),
              cam_loc=(85, 30, 30), target=(36, 8, 10))

export_gltf([root], OUT)
