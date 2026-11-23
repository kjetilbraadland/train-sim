"""Shared helpers for train-sim Blender asset scripts.

Conventions:
  - 1 Blender unit = 1 metre (matches the engine's world frame).
  - Assets are built with their FRONT facing +Y and their base at Z=0.
    The glTF exporter (default Y-up) turns that into +Z forward in three.js,
    which is exactly what `m.lookAt(vb)` expects in static/app.js.
  - Game palette (from static/app.js):
      background 0x0b0f14, ground 0x14202e, track 0x3a4d63,
      nodes 0x6b87a8, city 0x2b6cb0, train gold 0xd4a017,
      signal green 0x22cc44 / red 0xcc2222,
      goods: Coal 0x555555, Steel 0x8899aa, Food 0x55cc55, Timber 0xb08040.
"""

import os
import sys

import bpy
from math import radians
from mathutils import Vector

GAME_BG = 0x0B0F14
GAME_GROUND = 0x14202E
GAME_TRACK = 0x3A4D63
GAME_NODE = 0x6B87A8
GAME_CITY = 0x2B6CB0
GOLD = 0xD4A017
GOLD_DARK = 0x8A6A10
CHARCOAL = 0x2A2F3A
STEEL_GREY = 0x8899AA
SIG_GREEN = 0x22CC44
SIG_RED = 0xCC2222
GOOD_COAL = 0x555555
GOOD_STEEL = 0x8899AA
GOOD_FOOD = 0x55CC55
GOOD_TIMBER = 0xB08040


def srgb_to_linear(c):
    v = c / 255.0
    return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4


def hex_color(h):
    return (srgb_to_linear((h >> 16) & 255),
            srgb_to_linear((h >> 8) & 255),
            srgb_to_linear(h & 255), 1.0)


def clear_scene():
    bpy.ops.object.select_all(action="DESELECT")
    for block in (bpy.data.meshes, bpy.data.materials, bpy.data.curves,
                  bpy.data.images, bpy.data.cameras, bpy.data.lights):
        for item in list(block):
            if item.users == 0:
                block.remove(item)
    for obj in list(bpy.data.objects):
        bpy.data.objects.remove(obj, do_unlink=True)


def make_mat(name, hex_color_, roughness=0.6, metallic=0.0,
            emissive=None, emissive_intensity=0.0):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = hex_color(hex_color_)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if "Emission Color" in bsdf.inputs:
        if emissive is not None:
            bsdf.inputs["Emission Color"].default_value = hex_color(emissive)
            bsdf.inputs["Emission Strength"].default_value = emissive_intensity
        else:
            bsdf.inputs["Emission Strength"].default_value = 0.0
    return mat


def cube(name, size, loc, mat=None):
    bpy.ops.mesh.primitive_cube_add(size=2, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
    bpy.ops.object.transform_apply(scale=True)
    if mat:
        obj.data.materials.append(mat)
    return obj


def cylinder(name, r, depth, loc, mat=None, vertices=12,
            rot=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        radius=r, depth=depth, vertices=vertices, location=loc)
    obj = bpy.context.object
    obj.name = name
    obj.rotation_euler = [radians(a) for a in rot]
    if mat:
        obj.data.materials.append(mat)
    return obj


def parent_children(name, children):
    """Join is lossy for our use; parent instead so the glTF keeps hierarchy."""
    root = cube(name, (0.001, 0.001, 0.001), (0, 0, 0))
    root.hide_render = True
    for c in children:
        c.parent = root
        c.matrix_parent_inverse = root.matrix_world.inverted()
    return root


def setup_world():
    world = bpy.context.scene.world
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    bg.inputs[0].default_value = hex_color(GAME_BG)
    bg.inputs[1].default_value = 1.0


def setup_lights():
    """Mimic the game's lighting: hemisphere fill + one directional key."""
    bpy.ops.object.light_add(type="SUN", location=(60, 120, 160))
    sun = bpy.context.object
    sun.name = "KeyLight"
    sun.data.energy = 3.0
    sun.rotation_euler = (radians(45), 0, radians(30))

    bpy.ops.object.light_add(type="AREA", location=(-80, -60, 140))
    fill = bpy.context.object
    fill.name = "FillLight"
    fill.data.energy = 15000
    fill.data.size = 120
    fill.rotation_euler = (radians(20), 0, 0)


def add_camera(name, loc, target=(0, 0, 5), size=72):
    cam_data = bpy.data.cameras.new(name)
    cam_data.lens = 45
    cam_data.sensor_width = size
    cam = bpy.data.objects.new(name, cam_data)
    bpy.context.collection.objects.link(cam)
    cam.location = loc
    direction = Vector(target) - Vector(loc)
    cam.rotation_euler = direction.to_track_quat("Z", "Y").to_euler()
    return cam


def configure_render(width=1024, height=1024, samples=32):
    scene = bpy.context.scene
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    for want in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT", "CYCLES"):
        try:
            scene.render.engine = want
            break
        except Exception:
            continue
    try:
        scene.cycles.samples = samples
        scene.cycles.use_denoising = True
    except Exception:
        pass  # non-cycles engines have no cycles settings
    if scene.render.engine in ("BLENDER_EEVEE", "BLENDER_EEVEE_NEXT"):
        try:
            scene.eevee.taa_render_samples = samples
        except Exception:
            pass


def render_preview(objs, out_path, cam_loc, target=(0, 0, 6), name="cam"):
    for cam in [o for o in bpy.data.objects if o.type == "CAMERA"]:
        bpy.data.objects.remove(cam, do_unlink=True)
    cam = add_camera(name, cam_loc, target)
    bpy.context.scene.camera = cam
    # Hide helper objects (tiny parents) from the render? They're 1mm, harmless.
    bpy.context.view_layer.update()
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    scene = bpy.context.scene
    scene.render.filepath = out_path
    bpy.ops.render.render(write_still=True)


def select(objs):
    bpy.ops.object.select_all(action="DESELECT")
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[-1]


def export_gltf(objs, out_path, name="asset"):
    # Include children so the glTF keeps the hierarchy.
    all_objs = []
    for o in objs:
        all_objs.append(o)
        all_objs.extend(o.children_recursive)
    select(all_objs)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    kwargs = dict(
        filepath=out_path,
        use_selection=True,
        export_apply=True,
        export_animations=False,
        export_yup=True,
    )
    for fmt in ("GLB", "GLTF_BINARY"):  # 5.x renamed the enum
        try:
            bpy.ops.export_scene.gltf(export_format=fmt, **kwargs)
            break
        except TypeError:
            continue
    print(f"EXPORTED {out_path}")


def print_log(msg):
    print(f"[train-sim assets] {msg}")
