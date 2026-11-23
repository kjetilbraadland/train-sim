// Train Logistics Puzzle — three.js client.
// Renders the engine's JSON state over WebSocket. No game logic here.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';

// ---------- setup ----------
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x0b0f14);
scene.fog = new THREE.Fog(0x0b0f14, 2500, 6000);

const camera = new THREE.PerspectiveCamera(55, innerWidth / innerHeight, 1, 10000);
camera.position.set(900, 1400, 1800);

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setSize(innerWidth, innerHeight);
renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
document.body.appendChild(renderer.domElement);

const controls = new OrbitControls(camera, renderer.domElement);
controls.target.set(900, 0, 800);
controls.enableDamping = true;

scene.add(new THREE.HemisphereLight(0x9db8e8, 0x1a2333, 1.1));
const dir = new THREE.DirectionalLight(0xffffff, 0.7);
dir.position.set(600, 1200, 400);
scene.add(dir);

// Ground
const ground = new THREE.Mesh(
  new THREE.PlaneGeometry(6000, 4000),
  new THREE.MeshStandardMaterial({ color: 0x14202e, roughness: 1 })
);
ground.rotation.x = -Math.PI / 2;
scene.add(ground);

// Grid for scale
const grid = new THREE.GridHelper(4000, 40, 0x24405c, 0x1b2c3f);
grid.position.set(900, 0.1, 800);
scene.add(grid);

// ---------- world objects ----------
const world = { map: null, edgeMeshes: new Map(), nodeMeshes: new Map(),
                cityMeshes: new Map(), signalMeshes: new Map(), trainMeshes: new Map() };

// ---------- model loading ----------
const gltf = new GLTFLoader();
const modelCache = {};
function loadModel(url) {
  return modelCache[url] ??= new Promise((res, rej) =>
    gltf.load(url, g => res(g.scene), undefined, rej));
}
function spawnModel(url, parent, place) {
  loadModel(url).then(tpl => {
    const m = tpl.clone(true);
    if (place) place(m);
    parent.add(m);
  }).catch(e => console.error(e));
}
function goodIcon(parent, good, x, y, z) {
  spawnModel(`/models/goods_${good.toLowerCase()}.glb`, parent, m => {
    m.scale.setScalar(2.5);
    m.updateMatrixWorld(true);
    const b = new THREE.Box3().setFromObject(m);
    m.position.set(x, y - b.min.y, z);
  });
}

const GOOD_COLORS = { Coal: 0x555555, Steel: 0x8899aa, Food: 0x55cc55, Timber: 0xb08040 };
const GOOD_LABELS = { Coal: 'Coal', Steel: 'Steel', Food: 'Food', Timber: 'Timber' };

function labelSprite(text, color = '#cfe3ff', scale = 60) {
  const c = document.createElement('canvas');
  c.width = 256; c.height = 64;
  const ctx = c.getContext('2d');
  ctx.font = 'bold 36px system-ui';
  ctx.fillStyle = color;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(text, 128, 32);
  const tex = new THREE.CanvasTexture(c);
  const spr = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, depthTest: false }));
  spr.scale.set(scale, scale / 4, 1);
  return spr;
}

function clearWorld() {
  for (const m of [...world.edgeMeshes.values(), ...world.nodeMeshes.values(),
                   ...world.cityMeshes.values(), ...world.signalMeshes.values(),
                   ...world.trainMeshes.values()]) scene.remove(m);
  world.edgeMeshes.clear(); world.nodeMeshes.clear(); world.cityMeshes.clear();
  world.signalMeshes.clear(); world.trainMeshes.clear();
}

// Build static geometry from the map (called once per NewGame).
function buildMap(map) {
  clearWorld();
  world.map = map;
  const nodePos = id => {
    const n = map.nodes.find(n => n.id === id);
    return new THREE.Vector3(n.pos[0], 2, n.pos[1]);
  };

  for (const e of map.edges) {
    const a = nodePos(e.a), b = nodePos(e.b);
    const len = a.distanceTo(b);
    const mesh = new THREE.Mesh(
      new THREE.CylinderGeometry(4, 4, len, 8),
      new THREE.MeshStandardMaterial({ color: 0x3a4d63, roughness: 0.8 })
    );
    mesh.position.copy(a).add(b).multiplyScalar(0.5).setY(2);
    mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), b.clone().sub(a).normalize());
    scene.add(mesh);
    world.edgeMeshes.set(e.id, mesh);
  }

  for (const n of map.nodes) {
    const mesh = new THREE.Mesh(
      new THREE.SphereGeometry(8, 12, 12),
      new THREE.MeshStandardMaterial({ color: 0x6b87a8 })
    );
    mesh.position.set(n.pos[0], 2, n.pos[1]);
    scene.add(mesh);
    world.nodeMeshes.set(n.id, mesh);
  }

  for (const c of map.cities) {
    const g = new THREE.Group();
    const base = new THREE.Mesh(
      new THREE.CylinderGeometry(40, 48, 16, 24),
      new THREE.MeshStandardMaterial({ color: 0x2b6cb0 })
    );
    base.position.y = 8;
    g.add(base);
    // Station model on top of the city block
    spawnModel('/models/station.glb', g, m => { m.position.set(-28, 16, 0); });
    // Supply/demand pips
    const pips = new THREE.Group();
    let i = 0;
    for (const [good] of c.supply) {
      const pip = new THREE.Mesh(new THREE.BoxGeometry(14, 14, 14),
        new THREE.MeshStandardMaterial({ color: GOOD_COLORS[good] }));
      pip.position.set((i - c.supply.length / 2) * 20, 30, -20);
      g.add(pip); i++;
      goodIcon(g, good, pip.position.x, 37, pip.position.z);
    }
    i = 0;
    for (const [good] of c.demand) {
      const pip = new THREE.Mesh(new THREE.BoxGeometry(14, 14, 14),
        new THREE.MeshStandardMaterial({ color: GOOD_COLORS[good], emissive: GOOD_COLORS[good], emissiveIntensity: 0.25 }));
      pip.position.set((i - c.demand.length / 2) * 20, 30, 20);
      g.add(pip); i++;
      goodIcon(g, good, pip.position.x, 37, pip.position.z);
    }
    const label = labelSprite(c.name);
    label.position.y = 55;
    g.add(label);
    const n = map.nodes.find(n => n.id === c.station);
    g.position.set(n.pos[0], 0, n.pos[1]);
    scene.add(g);
    world.cityMeshes.set(c.id, g);
  }
}

// ---------- dynamic update (per snapshot) ----------
function updateState(s) {
  if (!world.map || world.map.id !== s.map.id) buildMap(s.map);

  // Signals
  for (const sig of s.signals) {
    let m = world.signalMeshes.get(sig.id);
    if (!m) {
      const from = s.map.nodes.find(n => n.id === sig.from);
      m = new THREE.Mesh(
        new THREE.SphereGeometry(10, 12, 12),
        new THREE.MeshStandardMaterial({ color: 0x22cc44, emissive: 0x22cc44, emissiveIntensity: 0.6 })
      );
      m.position.set(from.pos[0], 25, from.pos[1]);
      scene.add(m);
      world.signalMeshes.set(sig.id, m);
    }
    const col = sig.green ? 0x22cc44 : 0xcc2222;
    m.material.color.setHex(col);
    m.material.emissive.setHex(col);
  }

  // Trains
  for (const t of s.trains) {
    let m = world.trainMeshes.get(t.id);
    if (!m) {
      m = new THREE.Group();
      spawnModel('/models/train.glb', m);
      scene.add(m);
      world.trainMeshes.set(t.id, m);
    }
    // Position: interpolate along edge, or sit at node.
    // GLB base is y=0; track top is y=6 (track centre y=2, radius 4).
    let pos;
    if (t.edge != null) {
      const e = s.map.edges.find(e => e.id === t.edge);
      const a = s.map.nodes.find(n => n.id === e.a);
      const b = s.map.nodes.find(n => n.id === e.b);
      const va = new THREE.Vector3(a.pos[0], 6, a.pos[1]);
      const vb = new THREE.Vector3(b.pos[0], 6, b.pos[1]);
      pos = va.clone().lerp(vb, t.progress);
      m.lookAt(vb);
    } else {
      const n = s.map.nodes.find(n => n.id === t.node);
      pos = new THREE.Vector3(n.pos[0], 6, n.pos[1]);
    }
    m.position.copy(pos);
  }

  // HUD
  const st = s;
  document.getElementById('tick').textContent = st.tick;
  document.getElementById('score').textContent = st.score;
  document.getElementById('money').textContent = st.money;
  const green = st.signals.filter(x => x.green).length;
  const red = st.signals.length - green;
  document.getElementById('signals').textContent = `${green}g / ${red}r`;
}

// ---------- websocket ----------
let ws = null;
function connect() {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  ws = new WebSocket(`${proto}://${location.host}/ws`);
  ws.onmessage = ev => {
    try { updateState(JSON.parse(ev.data)); } catch (e) { console.error(e); }
  };
  ws.onclose = () => setTimeout(connect, 1500);
  ws.onerror = () => ws.close();
}
function cmd(obj) { if (ws?.readyState === 1) ws.send(JSON.stringify(obj)); }

// ---------- HUD buttons ----------
let paused = false, speed = 1;
document.getElementById('btn-pause').onclick = () => {
  paused = !paused;
  cmd({ type: 'SetPaused', paused });
  document.getElementById('btn-pause').textContent = paused ? 'Resume' : 'Pause';
};
document.getElementById('btn-speed').onclick = () => {
  speed = speed >= 4 ? 1 : speed * 2;
  cmd({ type: 'SetSpeed', multiplier: speed });
  document.getElementById('btn-speed').textContent = `Speed ${speed}x`;
};
document.getElementById('btn-reset').onclick = () => cmd({ type: 'NewGame', map_id: null });

// ---------- loop ----------
connect();
addEventListener('resize', () => {
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(innerWidth, innerHeight);
});
renderer.setAnimationLoop(() => {
  controls.update();
  renderer.render(scene, camera);
});
