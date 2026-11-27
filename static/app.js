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
const world = { map: null, mapSig: null, edgeMeshes: new Map(), nodeMeshes: new Map(), cityMeshes: new Map(), signalMeshes: new Map(), trainMeshes: new Map() };

// Map node (x, y) -> three.js position (y becomes z, height 2).
const nodePos = n => new THREE.Vector3(n.pos[0], 2, n.pos[1]);

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

// A sprite whose text can be redrawn in place (used for the cargo manifest).
function makeLabel() {
  const c = document.createElement('canvas');
  c.width = 256; c.height = 64;
  const ctx = c.getContext('2d');
  const tex = new THREE.CanvasTexture(c);
  const spr = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, depthTest: false }));
  spr.scale.set(48, 12, 1);
  let last = null;
  spr.setText = txt => {
    if (txt === last) return;
    last = txt;
    ctx.clearRect(0, 0, 256, 64);
    ctx.font = 'bold 30px system-ui';
    ctx.fillStyle = '#ffd9a0';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    ctx.fillText(txt, 128, 32);
    tex.needsUpdate = true;
  };
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
  // edges
  const nodePos = id => {
    const n = map.nodes.find(n => n.id === id);
    return new THREE.Vector3(n.pos[0], 2, n.pos[1]);
  };

  for (const e of map.edges) {
    const a = nodePos(e.a), b = nodePos(e.b);
    const len = a.distanceTo(b);
    const g = new THREE.Group();

    // Raised track bed (dark, factorio-style track foundation)
    g.add(new THREE.Mesh(
      new THREE.CylinderGeometry(4, 4, len, 8),
      new THREE.MeshStandardMaterial({ color: 0x4a4238, roughness: 1 })
    ));

    // Wooden sleepers on top of the bed
    const sleeperMat = new THREE.MeshStandardMaterial({ color: 0x6b4a2a, roughness: 1 });
    const nS = Math.max(2, Math.floor(len / 15));
    for (let i = 0; i < nS; i++) {
      const s = new THREE.Mesh(new THREE.BoxGeometry(9, 1, 1.8), sleeperMat);
      s.position.y = 4.5 + (-len / 2 + 5 + (i * (len - 10)) / (nS - 1)); // 4.5 = bed top + half sleeper
      g.add(s);
    }

    // Two steel rails on top of the sleepers
    const railMat = new THREE.MeshStandardMaterial({ color: 0x9aa4ad, roughness: 0.4, metalness: 0.6 });
    for (const side of [-2.2, 2.2]) {
      const r = new THREE.Mesh(new THREE.BoxGeometry(0.9, 1.2, len), railMat);
      r.position.set(side, 5.5, 0);
      g.add(r);
    }

    g.position.copy(a).add(b).multiplyScalar(0.5).setY(2);
    g.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), b.clone().sub(a).normalize());
    scene.add(g);
    world.edgeMeshes.set(e.id, g);
  }

  for (const n of map.nodes) {
    const mat = new THREE.MeshStandardMaterial({ color: 0x5a5248, roughness: 1 });
    const m = new THREE.Mesh(new THREE.SphereGeometry(8, 14, 10), mat);
    m.position.set(n.pos[0], 2, n.pos[1]);
    m.userData.tag = { kind: 'node', id: n.id };
    scene.add(m);
    world.nodeMeshes.set(n.id, m);
  }

  for (const c of map.cities) {
    const g = new THREE.Group();
    // Flat gray concrete pad (Factorio-style ground)
    const base = new THREE.Mesh(
      new THREE.BoxGeometry(100, 5, 100),
      new THREE.MeshStandardMaterial({ color: 0x8a887e, roughness: 1 })
    );
    base.position.y = 2.5;
    g.add(base);
    // Station model on the pad
    spawnModel('/models/station.glb', g, m => { m.position.set(-28, 5, 0); });
    // Factorio-green fence around the station
    const fenceMat = new THREE.MeshStandardMaterial({ color: 0x5cb85c, roughness: 0.8 });
    const fs = 14;
    for (const [dx, dz] of [[-fs, -fs], [fs, -fs], [-fs, fs], [fs, fs]]) {
      const post = new THREE.Mesh(new THREE.CylinderGeometry(0.6, 0.6, 5, 8), fenceMat);
      post.position.set(-28 + dx, 7.5, dz);
      g.add(post);
    }
    for (const [w, d, px, pz] of [[2 * fs, 1, 0, -fs], [2 * fs, 1, 0, fs], [1, 2 * fs, -fs, 0], [1, 2 * fs, fs, 0]]) {
      const bar = new THREE.Mesh(new THREE.BoxGeometry(w, 0.5, d), fenceMat);
      bar.position.set(-28 + px, 9.8, pz);
      g.add(bar);
    }
    // Supply/demand pips
    const pips = new THREE.Group();
    let i = 0;
    for (const [good] of c.supply) {
      const pip = new THREE.Mesh(new THREE.BoxGeometry(14, 14, 14),
        new THREE.MeshStandardMaterial({ color: GOOD_COLORS[good] }));
      pip.position.set((i - c.supply.length / 2) * 20, 12, -20);
      g.add(pip); i++;
      goodIcon(g, good, pip.position.x, 19, pip.position.z);
    }
    i = 0;
    for (const [good] of c.demand) {
      const pip = new THREE.Mesh(new THREE.BoxGeometry(14, 14, 14),
        new THREE.MeshStandardMaterial({ color: GOOD_COLORS[good], emissive: GOOD_COLORS[good], emissiveIntensity: 0.25 }));
      pip.position.set((i - c.demand.length / 2) * 20, 12, 20);
      g.add(pip); i++;
      goodIcon(g, good, pip.position.x, 19, pip.position.z);
    }
    const label = labelSprite(c.name);
    label.position.y = 34;
    g.add(label);
    const n = map.nodes.find(n => n.id === c.station);
    g.position.set(n.pos[0], 0, n.pos[1]);
    scene.add(g);
    world.cityMeshes.set(c.id, g);
  }
}

// ---------- dynamic update (per snapshot) ----------
let lastState = null;
function updateState(s) {
  const prev = lastState;
  lastState = s;
  if (prev && s.money > prev.money) onDelivery(s, prev, s.money - prev.money);
  // Rebuild the map when the graph changes: player-built track adds nodes
  // and edges with new ids, while the map id itself never changes.
  const sig = s.map.nodes.map(n => n.id).join(',') + '|' + s.map.edges.map(e => e.id).join(',');
  if (!world.map || world.mapSig !== sig) {
    clearGhosts();
    buildMap(s.map);
    world.map = s.map;
    world.mapSig = sig;
  }

  // Signals
  for (const sig of s.signals) {
    let m = world.signalMeshes.get(sig.id);
    if (!m) {
      const from = s.map.nodes.find(n => n.id === sig.from);
      const to = s.map.nodes.find(n => n.id === sig.to);
      m = new THREE.Group();
      // Factorio-style signal: small post beside the track, cube head, glowing lamp.
      const dir = new THREE.Vector3(to.pos[0] - from.pos[0], 0, to.pos[1] - from.pos[1]).normalize();
      const side = new THREE.Vector3(-dir.z, 0, dir.x);
      const pos = new THREE.Vector3(from.pos[0], 0, from.pos[1]).addScaledVector(side, 9);
      const post = new THREE.Mesh(
        new THREE.CylinderGeometry(0.8, 0.8, 9, 8),
        new THREE.MeshStandardMaterial({ color: 0x2a2f36, roughness: 0.9 })
      );
      post.position.y = 4.5;
      m.add(post);
      const head = new THREE.Mesh(
        new THREE.BoxGeometry(3, 3, 3),
        new THREE.MeshStandardMaterial({ color: 0x22262c, roughness: 0.7 })
      );
      head.position.y = 9.5;
      m.add(head);
      const lamp = new THREE.Mesh(
        new THREE.SphereGeometry(1.4, 12, 12),
        new THREE.MeshStandardMaterial({ color: 0x22cc44, emissive: 0x22cc44, emissiveIntensity: 0.9 })
      );
      lamp.position.y = 12;
      m.add(lamp);
      m.userData.lamp = lamp;
      m.position.copy(pos);
      scene.add(m);
      world.signalMeshes.set(sig.id, m);
    }
    const col = sig.green ? 0x22cc44 : 0xcc2222;
    m.userData.lamp.material.color.setHex(col);
    m.userData.lamp.material.emissive.setHex(col);
  }

  // Trains
  for (const t of s.trains) {
    let m = world.trainMeshes.get(t.id);
    if (!m) {
      const model = new THREE.Group();
      spawnModel('/models/train.glb', model);
      m = model;
      const lbl = makeLabel();
      lbl.position.y = 14;
      m.add(lbl);
      m.userData.label = lbl;
      m.userData.tag = { kind: 'train', id: t.id };
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
    // Cargo manifest above the train: what it's carrying, and load/capacity.
    const parts = t.cargo.map(([g, q]) => `${g} ×${q}`);
    const loaded = t.cargo.reduce((a, [, q]) => a + q, 0);
    m.userData.label.setText(
      `${parts.length ? parts.join('   ') : '— empty —'}   ${loaded}/${t.capacity}`
    );
  }

  // Drop meshes for trains/signals that no longer exist (e.g. after NewGame).
  for (const [id, mesh] of world.trainMeshes)
    if (!s.trains.some(t => t.id === id)) { mesh.removeFromParent(); world.trainMeshes.delete(id); }
  for (const [id, mesh] of world.signalMeshes)
    if (!s.signals.some(x => x.id === id)) { mesh.removeFromParent(); world.signalMeshes.delete(id); }

  // HUD
  const st = s;
  document.getElementById('tick').textContent = st.tick;
  setTween(document.getElementById('score'), st.score);
  setTween(document.getElementById('money'), st.money);
  const green = st.signals.filter(x => x.green).length;
  const red = st.signals.length - green;
  document.getElementById('signals').textContent = `${green}g / ${red}r`;
  // Throughput: units delivered in the trailing 10 s window. The engine
  // sends score_window (score at the start of each of the last 100 ticks);
  // the client only does the display math.
  const win = st.score_window;
  const tp = win && win.length === 100 ? (st.score - win[0]) / 10 : 0;
  document.getElementById('throughput').innerHTML =
    `${tp.toFixed(1)} <small style="font-weight:400">u/10s</small>`;
  if (paused) {
    document.getElementById('pause-money').textContent = st.money;
    document.getElementById('pause-score').textContent = st.score;
  }
}

// ---------- feedback: toasts, sound, floating text ----------
const toastsEl = document.getElementById('toasts');
function toast(msg, kind = 'info') {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = msg;
  toastsEl.appendChild(el);
  while (toastsEl.children.length > 4) toastsEl.firstChild.remove();
  setTimeout(() => {
    el.classList.add('out');
    setTimeout(() => el.remove(), 350);
  }, 2400);
}

// Tiny synthesized blips (no audio assets). Context is created lazily on the
// first user gesture to satisfy browser autoplay policy.
const sfx = {
  ctx: null,
  ensure() {
    if (!this.ctx) {
      const AC = window.AudioContext || window.webkitAudioContext;
      if (AC) this.ctx = new AC();
    }
    if (this.ctx && this.ctx.state === 'suspended') this.ctx.resume();
    return this.ctx;
  },
  tone(freq, dur, type, vol, delay = 0) {
    const c = this.ensure();
    if (!c) return;
    const t0 = c.currentTime + delay;
    const o = c.createOscillator(), g = c.createGain();
    o.type = type;
    o.frequency.value = freq;
    g.gain.setValueAtTime(0.0001, t0);
    g.gain.exponentialRampToValueAtTime(vol, t0 + 0.012);
    g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    o.connect(g);
    g.connect(c.destination);
    o.start(t0);
    o.stop(t0 + dur + 0.05);
  },
  click() { this.tone(660, 0.07, 'triangle', 0.1); },
  build() { this.tone(520, 0.09, 'sine', 0.14); this.tone(780, 0.12, 'sine', 0.11, 0.07); },
  error() { this.tone(180, 0.18, 'square', 0.07); },
  cash() { this.tone(660, 0.09, 'sine', 0.13); this.tone(880, 0.09, 'sine', 0.13, 0.08); this.tone(1100, 0.16, 'sine', 0.11, 0.16); },
};
renderer.domElement.addEventListener('pointerdown', () => sfx.ensure(), { once: true });

function rejectBuild() { sfx.error(); toast('Not enough funds', 'bad'); }
const COSTS = { track: 20, signal: 50, train: 200 };
function canAfford(cost) {
  if (lastState && lastState.money < cost) { rejectBuild(); return false; }
  return true;
}

let deliveryCount = 0;
function onDelivery(s, prev, delta) {
  deliveryCount += 1;
  sfx.cash();
  const scoreDelta = s.score - prev.score;
  toast(`+${delta} funds · delivery complete${scoreDelta > 0 ? ` · +${scoreDelta} score` : ''}`, 'gold');
  // Pop a floating +N above the station the train just unloaded at.
  const t = s.trains.find(t => t.edge == null && s.map.cities.some(c => c.station === t.node));
  const n = t && s.map.nodes.find(n => n.id === t.node);
  if (n) addFloatText(new THREE.Vector3(n.pos[0], 14, n.pos[1]), `+${delta}`);
}

// Floating world text: rises and fades over ~1.3 s.
const floatTexts = [];
function addFloatText(pos, text, color = '#ffd9a0') {
  const c = document.createElement('canvas');
  c.width = 128; c.height = 64;
  const ctx = c.getContext('2d');
  ctx.font = 'bold 34px system-ui';
  ctx.fillStyle = color;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(text, 64, 32);
  const tex = new THREE.CanvasTexture(c);
  const spr = new THREE.Sprite(new THREE.SpriteMaterial({ map: tex, depthTest: false, transparent: true }));
  spr.scale.set(40, 20, 1);
  spr.position.copy(pos);
  scene.add(spr);
  floatTexts.push({ spr, t: 0 });
}
function updateFloatTexts(dt) {
  for (let i = floatTexts.length - 1; i >= 0; i--) {
    const f = floatTexts[i];
    f.t += dt / 1.3;
    f.spr.position.y += dt * 35;
    f.spr.material.opacity = Math.max(0, 1 - f.t);
    if (f.t >= 1) {
      scene.remove(f.spr);
      f.spr.material.map.dispose();
      f.spr.material.dispose();
      floatTexts.splice(i, 1);
    }
  }
}

// Tween displayed counters (money/score) to their target values.
const tweens = new Map();
function setTween(el, target) {
  const cur = parseFloat(el.textContent) || 0;
  if (cur === target) { el.textContent = String(target); return; }
  tweens.set(el, { from: cur, to: target, t: 0 });
}
function updateTween(dt) {
  for (const [el, w] of tweens) {
    w.t = Math.min(1, w.t + dt / 0.4);
    const e = 1 - Math.pow(1 - w.t, 3);
    el.textContent = String(Math.round(w.from + (w.to - w.from) * e));
    if (w.t >= 1) tweens.delete(el);
  }
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
function setPaused(p) {
  if (paused === p) return;
  paused = p;
  cmd({ type: 'SetPaused', paused });
  const b = document.getElementById('btn-pause');
  b.textContent = p ? 'Resume' : 'Pause';
  b.classList.toggle('paused', p);
  document.getElementById('screen-pause').classList.toggle('show', p);
}
document.getElementById('btn-pause').onclick = () => { sfx.click(); setPaused(!paused); };
document.getElementById('btn-resume').onclick = () => { sfx.click(); setPaused(false); };
document.getElementById('btn-pause-reset').onclick = () => document.getElementById('btn-reset').click();
document.getElementById('btn-start').onclick = () => {
  sfx.click();
  document.getElementById('screen-start').classList.remove('show');
};
document.getElementById('btn-speed').onclick = () => {
  speed = speed >= 4 ? 1 : speed * 2;
  cmd({ type: 'SetSpeed', multiplier: speed });
  document.getElementById('btn-speed').textContent = `Speed ${speed}x`;
};
document.getElementById('btn-reset').onclick = () => {
  setMode('select');
  if (paused) setPaused(false);
  cmd({ type: 'NewGame', map_id: null });
  document.getElementById('screen-start').classList.add('show');
  toast('New game', 'good');
};
document.getElementById('btn-loop').onclick = () => {
  // The engine parks a train forever when its route is exhausted. Give the
  // train a multi-lap out-and-back route along the north line so it keeps
  // running (and auto-trading) instead of stopping.
  const t = lastState?.trains[0];
  if (!t) return;
  const circuit = [0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0];
  let i = circuit.indexOf(t.node);
  if (i < 0) i = 0;
  const lap = circuit.slice(i).concat(circuit.slice(0, i + 1));
  const route = [];
  for (let l = 0; l < 8; l++) route.push(...lap);
  cmd({ type: 'SetRoute', train_id: t.id, route });
};
// ---------- loop ----------
connect();
addEventListener('resize', () => {
  camera.aspect = innerWidth / innerHeight;
  camera.updateProjectionMatrix();
  renderer.setSize(innerWidth, innerHeight);
});
let lastFrame = performance.now();
renderer.setAnimationLoop(() => {
  const now = performance.now();
  const dt = Math.min(0.05, (now - lastFrame) / 1000);
  lastFrame = now;
  controls.update();
  updateFloatTexts(dt);
  updateTween(dt);
  renderer.render(scene, camera);
});

// ---------- build & route UI ----------
const ui = {
  mode: 'select',   // select | track | signal | train | route
  buildFrom: null,  // anchor node id for track/signal ghosts
  ghosts: null,     // THREE.Group of ghost previews
  routeTrain: null, // selected train id (route mode)
  route: [],        // node ids, starting at the train's node
  routeLine: null,  // route preview group
};

const raycaster = new THREE.Raycaster();
const pointerNdc = new THREE.Vector2();

const MODE_HINTS = {
  select: 'Pick a tool above, then click the map · Esc cancels',
  track: 'Track: click a node, then a green direction to lay 300 m track (20)',
  signal: 'Signal: click a node, then a green neighbour to place a signal (50)',
  train: 'Train: click any node to buy a train there (200)',
  route: 'Route: click a train, then track nodes; Confirm sends the plan',
};

function setHint(text) { document.getElementById('hint').textContent = text; }

function setMode(mode) {
  ui.mode = mode;
  clearGhosts();
  if (mode !== 'route') { ui.routeTrain = null; ui.route = []; clearRoutePreview(); }
  document.querySelectorAll('#hotbar .tool').forEach(b =>
    b.classList.toggle('active', b.dataset.mode === mode));
  document.getElementById('context').classList.toggle('route', mode === 'route');
  setHint(MODE_HINTS[mode]);
  updateModeStatus();
}

function updateModeStatus() {
  let extra = '';
  if (ui.mode === 'route')
    extra = ui.routeTrain == null
      ? ' — click a train'
      : ` — train ${ui.routeTrain} · ${ui.route.length} node${ui.route.length === 1 ? '' : 's'}`;
  document.getElementById('mode-status').innerHTML = `Mode: <b>${ui.mode}</b>${extra}`;
}

document.querySelectorAll('#hotbar .tool').forEach(b =>
  b.onclick = () => { sfx.click(); setMode(ui.mode === b.dataset.mode ? 'select' : b.dataset.mode); });

const KEY_MODES = { '1': 'track', '2': 'signal', '3': 'train', '4': 'route' };
addEventListener('keydown', ev => {
  if (KEY_MODES[ev.key]) {
    const m = KEY_MODES[ev.key];
    sfx.click();
    setMode(ui.mode === m ? 'select' : m);
  } else if (ev.key === 'Escape') {
    setMode('select');
  } else if (ev.key === ' ') {
    ev.preventDefault();
    // Avoid the keyup also activating a focused button (e.g. a hotbar tool).
    if (document.activeElement?.tagName === 'BUTTON') document.activeElement.blur();
    document.getElementById('btn-pause').click();
  } else if (ev.key === 'r' || ev.key === 'R') {
    document.getElementById('btn-reset').click();
  }
});

// -- hover: node highlight + detail panel --------------------------------
const nodeHighlight = new THREE.Mesh(
  new THREE.TorusGeometry(12, 1, 8, 32),
  new THREE.MeshBasicMaterial({ color: 0x9db8e8, transparent: true, opacity: 0.8 })
);
nodeHighlight.rotation.x = Math.PI / 2;
nodeHighlight.visible = false;
scene.add(nodeHighlight);

const detailEl = document.getElementById('detail');
const detailTitle = document.getElementById('detail-title');
const detailRows = document.getElementById('detail-rows');
function showDetail(title, rows) {
  detailTitle.textContent = title;
  detailRows.innerHTML = rows.map(([k, v]) =>
    `<div class="row"><span>${k}</span><b>${v}</b></div>`).join('');
  detailEl.classList.add('show');
}
function hideDetail() { detailEl.classList.remove('show'); }

renderer.domElement.addEventListener('pointermove', ev => {
  pointerNdc.set((ev.clientX / innerWidth) * 2 - 1, -(ev.clientY / innerHeight) * 2 + 1);
  raycaster.setFromCamera(pointerNdc, camera);
  const targets = [...world.nodeMeshes.values(), ...world.trainMeshes.values()];
  let tag = null;
  for (const h of raycaster.intersectObjects(targets, true)) {
    let o = h.object;
    while (o && !o.userData?.tag) o = o.parent;
    if (o) { tag = o.userData.tag; break; }
  }
  if (tag?.kind === 'node') {
    const n = nodeById(tag.id);
    if (n) {
      nodeHighlight.visible = true;
      nodeHighlight.position.set(n.pos[0], 3, n.pos[1]);
    }
    const edges = world.map.edges.filter(e => e.a === tag.id || e.b === tag.id).length;
    showDetail(`Node ${tag.id}`, [['Connected edges', edges]]);
  } else if (tag?.kind === 'train') {
    nodeHighlight.visible = false;
    const t = lastState?.trains.find(x => x.id === tag.id);
    if (t) {
      const loaded = t.cargo.reduce((a, [, q]) => a + q, 0);
      const cargo = t.cargo.map(([g, q]) => `${g}×${q}`).join(' ') || '—';
      showDetail(`Train ${t.id}`, [
        ['Position', t.edge != null ? `edge ${t.edge}` : `node ${t.node}`],
        ['Cargo', `${cargo}  (${loaded}/${t.capacity})`],
      ]);
    }
  } else {
    nodeHighlight.visible = false;
    hideDetail();
  }
});

// -- ghost previews --------------------------------------------------------
function clearGhosts() {
  if (ui.ghosts) {
    scene.remove(ui.ghosts);
    ui.ghosts.traverse(o => { o.geometry?.dispose(); o.material?.dispose(); });
    ui.ghosts = null;
  }
  ui.buildFrom = null;
}

// Mirrors engine::map::grid_dir_step (ADR 0004 / 0015): 0=E 1=W 2=N 3=S
// 4=NE 5=NW 6=SE 7=SW; diagonals move 300 m per axis.
function gridStep(dir) {
  const m = [[1,0],[-1,0],[0,1],[0,-1],[1,1],[-1,1],[1,-1],[-1,-1]][dir];
  return m ? [m[0] * 300, m[1] * 300] : null;
}

function nodeById(id) { return world.map?.nodes?.find(n => n.id === id); }

function nearNode(pos, eps = 25) {
  return world.map?.nodes.find(n => Math.hypot(n.pos[0] - pos[0], n.pos[1] - pos[1]) <= eps);
}

function ghostMaterial(ok) {
  return new THREE.MeshBasicMaterial({ color: ok ? 0x44cc66 : 0x885533, transparent: true, opacity: 0.45 });
}

function showTrackGhosts(nodeId) {
  clearGhosts();
  const from = nodeById(nodeId);
  if (!from) return;
  const fp = nodePos(from);
  const g = new THREE.Group();
  for (let d = 0; d < 8; d++) {
    const st = gridStep(d);
    if (!st) continue;
    const tp = new THREE.Vector3(fp.x + st[0], 2, fp.z + st[1]);
    const near = nearNode([fp.x + st[0], fp.z + st[1]]);
    const edgeExists = near && world.map.edges.some(
      e => (e.a === nodeId && e.b === near.id) || (e.b === nodeId && e.a === near.id));
    const ok = !edgeExists;
    const grp = new THREE.Group();
    grp.userData.tag = { kind: 'ghost-track', dir: d, ok };
    const bed = new THREE.Mesh(new THREE.CylinderGeometry(4, 4, fp.distanceTo(tp), 10), ghostMaterial(ok));
    bed.position.copy(fp).lerp(tp, 0.5);
    bed.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), tp.clone().sub(fp).normalize());
    const cap = new THREE.Mesh(new THREE.SphereGeometry(8, 12, 8), ghostMaterial(ok));
    cap.position.copy(tp);
    grp.add(bed, cap);
    g.add(grp);
  }
  scene.add(g);
  ui.ghosts = g;
  ui.buildFrom = nodeId;
}

function showSignalGhosts(nodeId) {
  clearGhosts();
  const from = nodeById(nodeId);
  if (!from) return;
  const fp = nodePos(from);
  const g = new THREE.Group();
  for (const e of world.map.edges) {
    if (e.a !== nodeId && e.b !== nodeId) continue;
    const toId = e.a === nodeId ? e.b : e.a;
    const to = nodeById(toId);
    if (!to) continue;
    const tp = nodePos(to);
    // A block-section signal sits ~9 m beside the `from` end, facing `to`.
    const dir3 = tp.clone().sub(fp).normalize();
    const perp = new THREE.Vector3(-dir3.z, 0, dir3.x);
    const already = lastState?.signals?.some(
      s => s.edge === e.id && s.from === nodeId && s.to === toId);
    const grp = new THREE.Group();
    grp.userData.tag = { kind: 'ghost-signal', edge: e.id, ok: !already };
    const lamp = new THREE.Mesh(new THREE.SphereGeometry(3, 12, 8), ghostMaterial(!already));
    lamp.position.copy(fp).add(perp.multiplyScalar(9));
    lamp.position.setY(5);
    grp.add(lamp);
    g.add(grp);
  }
  scene.add(g);
  ui.ghosts = g;
  ui.buildFrom = nodeId;
}

// -- picking ---------------------------------------------------------------
let downXY = null;
renderer.domElement.addEventListener('pointerdown', ev => { downXY = [ev.clientX, ev.clientY]; });
renderer.domElement.addEventListener('pointerup', ev => {
  if (ev.button !== 0 || !downXY) return;
  const moved = Math.hypot(ev.clientX - downXY[0], ev.clientY - downXY[1]);
  downXY = null;
  if (moved > 6) return; // camera drag, not a click
  handleClick(pick(ev));
});

function pick(ev) {
  pointerNdc.set((ev.clientX / innerWidth) * 2 - 1, -(ev.clientY / innerHeight) * 2 + 1);
  raycaster.setFromCamera(pointerNdc, camera);
  const targets = [...world.nodeMeshes.values(), ...world.trainMeshes.values()];
  if (ui.ghosts) targets.push(ui.ghosts);
  for (const h of raycaster.intersectObjects(targets, true)) {
    let o = h.object;
    while (o && !o.userData?.tag) o = o.parent;
    if (o) return o.userData.tag;
  }
  return null;
}

function handleClick(tag) {
  if (!tag) {
    if (ui.mode === 'track' || ui.mode === 'signal') clearGhosts();
    return;
  }
  switch (tag.kind) {
    case 'node': return onNodeClick(tag.id);
    case 'train': return onTrainClick(tag.id);
    case 'ghost-track':
      if (!tag.ok) return;
      if (!canAfford(COSTS.track)) return;
      cmd({ type: 'BuildTrack', from: ui.buildFrom, dir: tag.dir });
      clearGhosts();
      sfx.build();
      toast(`−${COSTS.track} funds · track built`);
      setHint('Built a 300 m track segment (−20)');
      return;
    case 'ghost-signal':
      if (!tag.ok) return;
      if (!canAfford(COSTS.signal)) return;
      cmd({ type: 'BuildSignal', edge: tag.edge, from: ui.buildFrom });
      clearGhosts();
      sfx.build();
      toast(`−${COSTS.signal} funds · signal placed`);
      setHint('Placed a block-section signal (−50)');
      return;
  }
}

function onNodeClick(nodeId) {
  if (ui.mode === 'track') {
    if (ui.buildFrom === nodeId) return clearGhosts();
    showTrackGhosts(nodeId);
    setHint(`Anchor node ${nodeId} — click a green direction (Esc cancels)`);
  } else if (ui.mode === 'signal') {
    showSignalGhosts(nodeId);
    setHint(`Anchor node ${nodeId} — click a green neighbour (Esc cancels)`);
  } else if (ui.mode === 'train') {
    if (!canAfford(COSTS.train)) return;
    cmd({ type: 'BuyTrain', node: nodeId });
    sfx.build();
    setHint(`Sent: buy train at node ${nodeId} (200)`);
  } else if (ui.mode === 'route') {
    if (ui.routeTrain == null) return setHint('Click a train first to start the route');
    if (ui.route[ui.route.length - 1] !== nodeId) {
      ui.route.push(nodeId);
      updateModeStatus();
      rebuildRoutePreview();
    }
  }
}

function onTrainClick(trainId) {
  if (ui.mode !== 'route') return;
  const t = lastState?.trains?.find(x => x.id === trainId);
  if (!t) return;
  ui.routeTrain = trainId;
  ui.route = [t.node];
  updateModeStatus();
  rebuildRoutePreview();
  setHint(`Train ${trainId} selected — click track nodes, then Confirm`);
}

// -- route preview ---------------------------------------------------------
function clearRoutePreview() {
  if (ui.routeLine) {
    scene.remove(ui.routeLine);
    ui.routeLine.traverse(o => { o.geometry?.dispose(); o.material?.dispose(); });
    ui.routeLine = null;
  }
}

function rebuildRoutePreview() {
  clearRoutePreview();
  if (ui.route.length < 2) return;
  const g = new THREE.Group();
  for (let i = 0; i + 1 < ui.route.length; i++) {
    const a = nodeById(ui.route[i]), b = nodeById(ui.route[i + 1]);
    if (!a || !b) continue;
    const pa = nodePos(a), pb = nodePos(b);
    const linked = world.map.edges.some(
      e => (e.a === a.id && e.b === b.id) || (e.b === a.id && e.a === b.id));
    g.add(new THREE.Line(
      new THREE.BufferGeometry().setFromPoints([pa.clone().setY(9), pb.clone().setY(9)]),
      new THREE.LineBasicMaterial({ color: linked ? 0x44ff88 : 0xff5544 })));
  }
  scene.add(g);
  ui.routeLine = g;
}

document.getElementById('btn-route-confirm').onclick = () => {
  const t = lastState?.trains?.find(x => x.id === ui.routeTrain);
  if (!t) return;
  // The train may have moved since we started; the engine requires the route
  // to start at its current node, so re-anchor it.
  const route = [t.node];
  for (const n of ui.route) if (n !== route[route.length - 1]) route.push(n);
  if (route.length < 2) return setHint('Route needs at least one hop');
  sfx.click();
  cmd({ type: 'SetRoute', train_id: ui.routeTrain, route });
  setMode('select');
  setHint(`Sent route for train ${t.id} (${route.length} nodes)`);
};

document.getElementById('btn-route-clear').onclick = () => {
  ui.routeTrain = null;
  ui.route = [];
  clearRoutePreview();
  updateModeStatus();
};

// ---------- test hook ----------
window.__ts = {
  nodeScreen(id, y = 3) {
    const n = nodeById(id);
    const v = new THREE.Vector3(n.pos[0], y, n.pos[1]).project(camera);
    return { x: (v.x + 1) / 2 * innerWidth, y: (-v.y + 1) / 2 * innerHeight };
  },
  ghostScreen(fromId, d) {
    const f = nodeById(fromId);
    const st = gridStep(d);
    const v = new THREE.Vector3(f.pos[0] + st[0], 3, f.pos[1] + st[1]).project(camera);
    return { x: (v.x + 1) / 2 * innerWidth, y: (-v.y + 1) / 2 * innerHeight };
  },
  trainScreen(id, y = 6) {
    const m = world.trainMeshes.get(id);
    if (!m) return null;
    const v = m.position.clone().project(camera);
    return { x: (v.x + 1) / 2 * innerWidth, y: (-v.y + 1) / 2 * innerHeight };
  },
  state: () => lastState,
  mode: () => ui.mode,
  ghostDirs() {
    if (!ui.ghosts) return [];
    return ui.ghosts.children.map(c => c.userData.tag)
      .filter(t => t?.kind === 'ghost-track').map(t => ({ dir: t.dir, ok: t.ok }));
  },
  cities: () => lastState?.map?.cities?.map(c => ({ id: c.id, name: c.name, station: c.station })) ?? [],
  floats: () => floatTexts.length,
  deliveries: () => deliveryCount,
  paused: () => paused,
};

