(() => {
  const page = window.velaMeet.build({ beam: false });
  const site = document.getElementById('site');
  site.innerHTML = page;

  const canvas = document.getElementById('energy-canvas');
  const ctx = canvas.getContext('2d');
  const select = document.getElementById('energy-select');
  const indexLabel = document.getElementById('energy-index');
  const nameLabel = document.getElementById('energy-name');
  const lineLabel = document.getElementById('energy-line');
  const prev = document.getElementById('energy-prev');
  const next = document.getElementById('energy-next');
  const params = new URLSearchParams(location.search);

  const TAU = Math.PI * 2;
  const lerp = (a, b, t) => a + (b - a) * t;
  const clamp = (v, a, b) => Math.min(b, Math.max(a, v));
  const hash = (n) => {
    const v = Math.sin(n * 127.1 + 311.7) * 43758.5453;
    return v - Math.floor(v);
  };
  const pick = (points, t) => points[clamp(Math.floor(t * (points.length - 1)), 0, points.length - 1)];

  const rectOf = (selector) => {
    const el = document.querySelector(selector);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return null;
    return { x: r.left, y: r.top, w: r.width, h: r.height, cx: r.left + r.width / 2, cy: r.top + r.height / 2, right: r.right, bottom: r.bottom };
  };

  const anchors = () => ({
    header: rectOf('.ob-header'),
    title: rectOf('.ob-title'),
    logo: rectOf('.ob-title-logo'),
    preview: rectOf('.ob-preview'),
    hero: rectOf('.ob-hero'),
    sections: Array.from(document.querySelectorAll('.ob-section, .ob-features, .ob-flow, .ob-faq')).map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.left, y: r.top, w: r.width, h: r.height, cx: r.left + r.width / 2, cy: r.top + r.height / 2, right: r.right, bottom: r.bottom };
    }).filter((r) => r.w > 0),
    footer: rectOf('.ob-footer'),
  });

  const path = (points) => {
    if (!points || points.length < 2) return false;
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 1; i < points.length; i += 1) ctx.lineTo(points[i].x, points[i].y);
    return true;
  };

  const stroke = (points, color, width, alpha = 1, blur = 0) => {
    if (!path(points)) return;
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    if (blur) {
      ctx.shadowColor = color;
      ctx.shadowBlur = blur;
    }
    ctx.stroke();
    ctx.restore();
  };

  const wash = (x, y, radius, color, alpha = 1, inner = 0.16) => {
    if (!Number.isFinite(x) || !Number.isFinite(y) || radius <= 0) return;
    ctx.save();
    ctx.globalAlpha = alpha;
    const g = ctx.createRadialGradient(x, y, 0, x, y, radius);
    g.addColorStop(0, color);
    g.addColorStop(inner, color);
    g.addColorStop(1, 'transparent');
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, TAU);
    ctx.fill();
    ctx.restore();
  };

  const core = (x, y, radius, alpha = 1) => {
    wash(x, y, radius, '#ffffff', alpha, 0.1);
  };

  const wave = (n, fn) => Array.from({ length: n }, (_, i) => fn(i / (n - 1), i));

  // A filled ribbon between a top and bottom edge.
  const ribbon = (top, bottom, colorA, colorB, alpha) => {
    if (top.length < 2 || bottom.length < 2) return;
    const minY = Math.min(...top.map((p) => p.y), ...bottom.map((p) => p.y));
    const maxY = Math.max(...top.map((p) => p.y), ...bottom.map((p) => p.y));
    const g = ctx.createLinearGradient(0, minY, 0, Math.max(maxY, minY + 1));
    g.addColorStop(0, colorA);
    g.addColorStop(1, colorB);
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.moveTo(top[0].x, top[0].y);
    for (const p of top.slice(1)) ctx.lineTo(p.x, p.y);
    for (let i = bottom.length - 1; i >= 0; i -= 1) ctx.lineTo(bottom[i].x, bottom[i].y);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  };

  const previewTop = (a, h) => (a.preview ? a.preview.y : h * 0.66);
  const previewX = (a, w) => (a.preview ? a.preview.cx : w / 2);

  // 01 — a full-page waterfall with a lit shelf and runoff where it meets the workspace.
  const drawCascade = (env, time) => {
    const { a, w, h } = env;
    const shelf = previewTop(a, h);
    wash(previewX(a, w), -h * 0.1, w * 0.9, '#9fe8d3', 0.2);
    wash(previewX(a, w), shelf, w * 0.55, '#c9a1ff', 0.18);

    for (let i = 0; i < 13; i += 1) {
      const baseX = ((i + 0.5) / 13) * w;
      const hue = i % 3 === 0 ? '#9fe8d3' : i % 3 === 1 ? '#8fb8ff' : '#c9a1ff';
      const pts = wave(34, (t) => ({
        x: baseX + Math.sin(t * 5.4 + time * 0.4 + i) * (26 + (1 - t) * 46),
        y: lerp(-70, shelf + 30, t),
      }));
      ctx.save();
      ctx.globalAlpha = 0.44;
      ctx.strokeStyle = hue;
      ctx.lineWidth = 1.6 + (i % 4) * 1.1;
      ctx.lineCap = 'round';
      ctx.shadowColor = hue;
      ctx.shadowBlur = 16;
      path(pts);
      ctx.stroke();
      ctx.restore();

      for (let k = 0; k < 3; k += 1) {
        const t = (time * (0.24 + i * 0.005) + k / 3) % 1;
        const p = pick(pts, t);
        core(p.x, p.y, 12, 0.7);
      }
    }

    wash(previewX(a, w), shelf, w * 0.42, '#d8fff3', 0.4);
    stroke([{ x: 0, y: shelf }, { x: w, y: shelf }], '#d8fff3', 2.4, 0.7, 20);
    for (let i = 0; i < 40; i += 1) {
      const x = hash(i) * w;
      const len = 70 + hash(i + 2) * 260;
      stroke([{ x, y: shelf }, { x, y: shelf + len }], i % 3 ? '#9fe8d3' : '#c9a1ff', 1, 0.14);
    }
    for (let i = 0; i < 90; i += 1) {
      const x = (hash(i) * w + time * (26 + hash(i + 1) * 70)) % w;
      const y = (hash(i + 4) * shelf + time * (40 + hash(i + 3) * 90)) % Math.max(shelf, 1);
      core(x, y, 3 + hash(i + 6) * 5, 0.2);
    }
  };

  // 02 — a vertical tear through the page with light spilling sideways across the interface.
  const drawRift = (env, time) => {
    const { a, w, h } = env;
    const cx = previewX(a, w);
    const crack = [];
    for (let i = 0; i <= 30; i += 1) {
      const t = i / 30;
      crack.push({
        x: cx + Math.sin(t * 13 + time * 0.5) * 12 + (hash(i) - 0.5) * 16,
        y: t * h,
      });
    }
    wash(cx, h * 0.45, w * 0.42, '#c9a1ff', 0.26);
    wash(cx, h * 0.45, w * 0.16, '#ffffff', 0.22);
    stroke(crack, '#c9a1ff', 34, 0.16, 40);
    stroke(crack, '#e8f4ff', 7, 0.5, 22);
    stroke(crack, '#ffffff', 2, 0.95, 10);

    for (let i = 1; i < 16; i += 1) {
      const t = i / 16;
      const y = t * h;
      const dir = i % 2 ? -1 : 1;
      const len = 60 + hash(i) * (w * 0.32);
      const shard = [
        { x: cx, y },
        { x: cx + dir * len * 0.4, y: y + (hash(i + 2) - 0.5) * 60 },
        { x: cx + dir * len, y: y + (hash(i + 3) - 0.5) * 100 },
      ];
      stroke(shard, i % 3 ? '#8fb8ff' : '#e8f4ff', 1 + hash(i + 5) * 2, 0.3);
      core(shard[2].x, shard[2].y, 14, 0.3);
    }

    for (let i = 0; i < 5; i += 1) {
      const t = (time * 0.32 + i / 5) % 1;
      const y = t * h;
      const beam = ctx.createLinearGradient(cx - w * 0.5, y, cx + w * 0.5, y);
      beam.addColorStop(0, 'transparent');
      beam.addColorStop(0.5, '#d8fff3');
      beam.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.24;
      ctx.fillStyle = beam;
      ctx.fillRect(0, y - 1.5, w, 3);
      ctx.restore();
    }
  };

  // 03 — a huge spiral above the workspace funnels energy down into the app.
  const drawVortex = (env, time) => {
    const { a, w, h } = env;
    const cx = previewX(a, w);
    const cy = a.preview ? a.preview.y - h * 0.28 : h * 0.4;
    const eye = Math.min(w, h) * 0.03;
    wash(cx, cy, Math.min(w, h) * 0.55, '#8fb8ff', 0.2);
    wash(cx, cy, eye * 9, '#e8f4ff', 0.36);

    for (let arm = 0; arm < 7; arm += 1) {
      const pts = [];
      const steps = 70;
      for (let i = 0; i <= steps; i += 1) {
        const t = i / steps;
        const radius = eye + t * (Math.min(w, h) * (0.34 + arm * 0.014));
        const angle = t * 10.4 + time * (0.55 + arm * 0.03) + arm * (TAU / 7);
        pts.push({ x: cx + Math.cos(angle) * radius, y: cy + Math.sin(angle) * radius * 0.62 });
      }
      const color = arm % 2 ? '#8fb8ff' : '#c9a1ff';
      for (let i = 0; i < pts.length - 1; i += 2) {
        const t = i / (pts.length - 1);
        stroke([pts[i], pts[i + 1], pts[Math.min(i + 2, pts.length - 1)]], color, 1 + t * 5, 0.1 + t * 0.4, 10);
      }
      const p = pick(pts, (time * 0.3 + arm * 0.14) % 1);
      core(p.x, p.y, 16, 0.7);
    }

    if (a.preview) {
      const top = a.preview.y;
      const beam = ctx.createLinearGradient(0, cy, 0, top);
      beam.addColorStop(0, '#e8f4ff');
      beam.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.5;
      ctx.fillStyle = beam;
      ctx.beginPath();
      ctx.moveTo(cx - eye, cy);
      ctx.lineTo(cx - 54, top);
      ctx.lineTo(cx + 54, top);
      ctx.lineTo(cx + eye, cy);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      wash(cx, top, 130, '#c9f7e8', 0.55);
      stroke([{ x: a.preview.x, y: top }, { x: a.preview.right, y: top }], '#c9f7e8', 2, 0.4, 18);
    }

    for (let i = 0; i < 60; i += 1) {
      const angle = hash(i) * TAU + time * (0.4 + hash(i + 1));
      const radius = eye + ((hash(i + 2) * Math.min(w, h) * 0.4 + time * 40) % (Math.min(w, h) * 0.4));
      core(cx + Math.cos(angle) * radius, cy + Math.sin(angle) * radius * 0.62, 2.5, 0.3);
    }
  };

  // 04 — a page-scale eclipse behind the hero with a corona reaching past the content.
  const drawEclipse = (env, time) => {
    const { a, w, h } = env;
    const cx = a.title ? a.title.cx : w / 2;
    const cy = a.title ? a.title.cy : h * 0.32;
    const radius = Math.min(w * 0.24, 300);
    wash(cx, cy, radius * 3.1, '#ffb15e', 0.34);
    wash(cx, cy, radius * 1.6, '#ffd36b', 0.5);

    ctx.save();
    const disk = ctx.createRadialGradient(cx, cy, radius * 0.2, cx, cy, radius);
    disk.addColorStop(0, '#1a1206');
    disk.addColorStop(0.86, '#1a1206');
    disk.addColorStop(0.94, '#ffd36b');
    disk.addColorStop(1, 'transparent');
    ctx.fillStyle = disk;
    ctx.beginPath();
    ctx.arc(cx, cy, radius, 0, TAU);
    ctx.fill();
    ctx.restore();

    for (let i = 0; i < 56; i += 1) {
      const angle = (i / 56) * TAU + time * 0.05;
      const inner = radius * (1.02 + hash(i) * 0.06);
      const outer = radius * (1.55 + hash(i + 2) * 0.9);
      const color = i % 3 === 0 ? '#fff4cf' : '#ffd36b';
      stroke([
        { x: cx + Math.cos(angle) * inner, y: cy + Math.sin(angle) * inner },
        { x: cx + Math.cos(angle + 0.012) * outer, y: cy + Math.sin(angle + 0.012) * outer },
      ], color, 1 + hash(i + 1) * 3.4, 0.28, 12);
    }

    for (let i = 0; i < 9; i += 1) {
      const t = (time * 0.12 + i / 9) % 1;
      const angle = t * TAU;
      const length = radius * (1.1 + hash(i + 4) * 2.2);
      const dir = { x: Math.cos(angle), y: Math.sin(angle) };
      const beam = ctx.createRadialGradient(cx, cy, radius, cx, cy, length);
      beam.addColorStop(0, '#fff4cf');
      beam.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.2;
      ctx.fillStyle = beam;
      ctx.beginPath();
      ctx.moveTo(cx + dir.x * radius, cy + dir.y * radius);
      ctx.lineTo(cx + Math.cos(angle + 0.03) * length, cy + Math.sin(angle + 0.03) * length);
      ctx.lineTo(cx + Math.cos(angle - 0.03) * length, cy + Math.sin(angle - 0.03) * length);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    }

    if (a.preview) {
      const shaft = ctx.createLinearGradient(0, cy, 0, a.preview.y);
      shaft.addColorStop(0, '#ffd36b');
      shaft.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.2;
      ctx.fillStyle = shaft;
      ctx.beginPath();
      ctx.moveTo(cx - radius * 0.5, cy);
      ctx.lineTo(a.preview.x + a.preview.w * 0.2, a.preview.y);
      ctx.lineTo(a.preview.right - a.preview.w * 0.2, a.preview.y);
      ctx.lineTo(cx + radius * 0.5, cy);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      wash(a.preview.cx, a.preview.y, 180, '#ffd36b', 0.3);
    }
  };

  // 05 — aurora curtains spanning the whole page and pooling on the workspace shelf.
  const drawCurtain = (env, time) => {
    const { a, w, h } = env;
    const shelf = previewTop(a, h);
    wash(w / 2, 0, w * 0.8, '#9fe8d3', 0.16);

    for (let i = 0; i < 9; i += 1) {
      const offset = i / 9;
      const top = wave(40, (t) => ({
        x: t * w,
        y: h * 0.06 + Math.sin(t * 4.6 + time * 0.4 + i) * 34 + offset * 90,
      }));
      const bottom = top.map((p, k) => ({
        x: p.x,
        y: p.y + 150 + Math.sin(k * 0.5 + time * 0.7 + i) * 60 + offset * 120,
      }));
      const colorA = i % 3 === 0 ? '#9fe8d3' : i % 3 === 1 ? '#8fb8ff' : '#c9a1ff';
      ribbon(top, bottom, colorA, 'transparent', 0.14);
      stroke(top, colorA, 1.6, 0.35, 10);
      const p = pick(top, (time * 0.18 + i * 0.1) % 1);
      core(p.x, p.y, 14, 0.45);
    }

    wash(w / 2, shelf, w * 0.5, '#9fe8d3', 0.3);
    stroke([{ x: 0, y: shelf }, { x: w, y: shelf }], '#c9f7e8', 2, 0.55, 18);
    for (let i = 0; i < 60; i += 1) {
      const x = hash(i) * w;
      const y = shelf - 70 + Math.sin(time * 2 + i) * 22;
      core(x, y, 2 + hash(i + 3) * 4, 0.24);
    }
  };

  // 06 — a branching storm bolt that strikes the workspace and lights the page.
  const drawLightning = (env, time) => {
    const { a, w, h } = env;
    const cycle = Math.floor(time / 2.6);
    const phase = (time % 2.6) / 2.6;
    const flash = Math.max(0, Math.sin(phase * Math.PI * 2)) ** 6;
    const ground = previewTop(a, h);
    wash(w / 2, ground * 0.5, w * 0.8, '#8fb8ff', 0.14 + flash * 0.2);
    if (flash > 0.02) wash(w / 2, ground, w * 0.5, '#d8fff3', flash * 0.22);

    const strikeX = w * (0.3 + hash(cycle * 3 + 1) * 0.4);
    const buildBolt = (x0, y0, x1, y1, depth) => {
      const pts = [];
      for (let i = 0; i <= 22; i += 1) {
        const t = i / 22;
        const spread = (1 - Math.abs(t - 0.5) * 2) * 44;
        pts.push({
          x: lerp(x0, x1, t) + (hash(cycle * 17 + i + depth * 5) - 0.5) * spread,
          y: lerp(y0, y1, t),
        });
      }
      stroke(pts, '#8fb8ff', 12, 0.12 + flash * 0.2, 34);
      stroke(pts, '#e8f4ff', 3.4, 0.3 + flash * 0.6, 18);
      stroke(pts, '#ffffff', 1.4, 0.35 + flash * 0.65, 8);
      if (depth < 2) {
        for (let b = 0; b < 3; b += 1) {
          const start = pick(pts, 0.25 + hash(cycle + b + depth) * 0.4);
          const dir = hash(cycle + b + depth + 3) > 0.5 ? 1 : -1;
          buildBolt(start.x, start.y, start.x + dir * (80 + hash(b + depth) * 220), Math.min(ground, start.y + 140 + hash(b + 7) * 220), depth + 1);
        }
      }
      return pts;
    };

    const bolt = buildBolt(strikeX, -30, strikeX + (hash(cycle) - 0.5) * 240, ground, 0);
    const end = bolt[bolt.length - 1];
    core(end.x, end.y, 90, 0.25 + flash * 0.5);
    for (let i = 0; i < 3; i += 1) {
      const k = 1 - i / 4;
      ctx.save();
      ctx.globalAlpha = flash * 0.35 * k;
      ctx.strokeStyle = '#d8fff3';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.ellipse(end.x, end.y, 90 + i * 70 + phase * 60, 18 + i * 12 + phase * 10, 0, 0, TAU);
      ctx.stroke();
      ctx.restore();
    }
    for (let i = 0; i < 26; i += 1) {
      const t = Math.pow(hash(i + cycle), 0.6);
      const p = pick(bolt, t);
      core(p.x, p.y, 6 + hash(i) * 10, flash * 0.4);
    }
  };

  // 07 — broad horizontal tides rolling the whole viewport.
  const drawTide = (env, time) => {
    const { a, w, h } = env;
    const shelf = previewTop(a, h);
    wash(w / 2, h * 0.4, w * 0.8, '#8fb8ff', 0.12);

    for (let i = 0; i < 6; i += 1) {
      const base = lerp(h * 0.18, shelf + 60, i / 5);
      const top = wave(56, (t) => ({
        x: t * w,
        y: base + Math.sin(t * 6.4 + time * (0.6 + i * 0.05) + i * 0.8) * (14 + i * 6),
      }));
      const bottom = top.map((p) => ({ x: p.x, y: p.y + 150 + i * 26 }));
      const colorA = i % 2 ? '#9fe8d3' : '#8fb8ff';
      ribbon(top, bottom, colorA, 'transparent', 0.12 + i * 0.01);
      stroke(top, colorA, 1.6 + i * 0.6, 0.4, 14);
      for (let k = 0; k < 2; k += 1) {
        const p = pick(top, (time * 0.3 + k * 0.5 + i * 0.1) % 1);
        core(p.x, p.y, 16, 0.5);
      }
    }

    wash(a.preview ? a.preview.cx : w / 2, shelf, w * 0.4, '#c9f7e8', 0.24);
    for (let i = 0; i < 90; i += 1) {
      const x = (hash(i) * w + time * (20 + hash(i + 1) * 60)) % w;
      const y = shelf + 20 + (hash(i + 2) - 0.5) * 70;
      core(x, y, 2 + hash(i + 3) * 4, 0.2);
    }
  };

  // 08 — a large portal opens behind the workspace and feeds light into it.
  const drawPortal = (env, time) => {
    const { a, w, h } = env;
    const cx = previewX(a, w);
    const cy = a.preview ? a.preview.y - h * 0.19 : h * 0.42;
    const rx = Math.min(w * 0.34, 470);
    const ry = Math.min(h * 0.25, 260);
    wash(cx, cy, rx * 1.5, '#c9a1ff', 0.24);

    ctx.save();
    const hole = ctx.createRadialGradient(cx, cy, 0, cx, cy, rx);
    hole.addColorStop(0, '#05070c');
    hole.addColorStop(0.55, '#0a0f1a');
    hole.addColorStop(0.85, '#c9a1ff');
    hole.addColorStop(1, 'transparent');
    ctx.globalAlpha = 0.9;
    ctx.fillStyle = hole;
    ctx.beginPath();
    ctx.ellipse(cx, cy, rx, ry, 0, 0, TAU);
    ctx.fill();
    ctx.restore();

    for (let i = 0; i < 7; i += 1) {
      const k = 1 - i / 8;
      ctx.save();
      ctx.globalAlpha = 0.5 * k;
      ctx.strokeStyle = i % 2 ? '#c9a1ff' : '#9fe8d3';
      ctx.lineWidth = 1.5 + k * 4;
      ctx.shadowColor = '#c9a1ff';
      ctx.shadowBlur = 18;
      ctx.beginPath();
      ctx.ellipse(cx, cy, rx * (1 + i * 0.07), ry * (1 + i * 0.07), 0, 0, TAU);
      ctx.stroke();
      ctx.restore();
    }

    for (let i = 0; i < 34; i += 1) {
      const angle = (i / 34) * TAU;
      const p = { x: cx + Math.cos(angle + time * 0.14) * rx, y: cy + Math.sin(angle + time * 0.14) * ry };
      core(p.x, p.y, 8, 0.5);
      stroke([p, { x: lerp(p.x, cx, 0.25), y: lerp(p.y, cy, 0.25) }], '#c9a1ff', 1.2, 0.2);
    }

    if (a.preview) {
      const beam = ctx.createLinearGradient(0, cy + ry, 0, a.preview.y);
      beam.addColorStop(0, '#e8f4ff');
      beam.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.5;
      ctx.fillStyle = beam;
      ctx.beginPath();
      ctx.moveTo(cx - rx * 0.55, cy + ry * 0.6);
      ctx.lineTo(a.preview.cx - a.preview.w * 0.3, a.preview.y);
      ctx.lineTo(a.preview.cx + a.preview.w * 0.3, a.preview.y);
      ctx.lineTo(cx + rx * 0.55, cy + ry * 0.6);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      wash(a.preview.cx, a.preview.y, 170, '#c9f7e8', 0.45);
    }
  };

  // 09 — a diagonal photon storm sweeping the whole viewport.
  const drawRain = (env, time) => {
    const { a, w, h } = env;
    const shelf = previewTop(a, h);
    wash(w * 0.5, h * 0.3, w * 0.75, '#8fb8ff', 0.16);
    for (let i = 0; i < 260; i += 1) {
      const speed = 170 + hash(i) * 380;
      const x = (hash(i + 3) * (w + 500) + time * speed) % (w + 500) - 250;
      const y = (hash(i + 7) * (h + 300) + time * speed * 1.5) % (h + 300) - 150;
      const len = 16 + hash(i + 8) * 64;
      const color = i % 5 ? '#8fb8ff' : '#ffffff';
      stroke([{ x, y }, { x: x - len * 0.5, y: y + len }], color, 0.7 + hash(i + 6) * 1.8, 0.2 + hash(i + 9) * 0.5);
      if (i % 11 === 0) core(x, y, 8, 0.35);
    }
    wash(w / 2, shelf, w * 0.5, '#d8fff3', 0.34);
    stroke([{ x: 0, y: shelf }, { x: w, y: shelf }], '#d8fff3', 2.2, 0.5, 20);
    for (let i = 0; i < 80; i += 1) {
      const x = hash(i + 20) * w;
      const y = shelf + Math.sin(time * 2.4 + i) * 14;
      core(x, y, 2 + hash(i + 1) * 3, 0.3);
    }
  };

  // 10 — a page-scale bloom opening from the logo and channeling into the interface.
  const drawBloom = (env, time) => {
    const { a, w, h } = env;
    const origin = a.logo || a.title;
    const cx = origin ? origin.cx : w / 2;
    const cy = origin ? origin.cy : h * 0.3;
    const scale = 1 + 0.12 * Math.sin(time * 0.8);
    const radius = Math.min(w * 0.44, 520) * scale;
    wash(cx, cy, radius * 1.5, '#c9a1ff', 0.26);
    wash(cx, cy, 78, '#ffffff', 0.3);

    for (let i = 0; i < 18; i += 1) {
      const angle = (i / 18) * TAU + time * 0.04;
      const color = i % 2 ? '#c9a1ff' : '#9fe8d3';
      const petal = wave(24, (t) => {
        const width = Math.sin(t * Math.PI) * 0.24;
        return {
          x: cx + Math.cos(angle + width * Math.sin(t * 5)) * radius * t,
          y: cy + Math.sin(angle + width * Math.sin(t * 5)) * radius * t * 0.62,
        };
      });
      const left = petal.map((p, k) => ({
        x: p.x + Math.cos(angle + Math.PI / 2) * Math.sin((k / 23) * Math.PI) * 30,
        y: p.y + Math.sin(angle + Math.PI / 2) * Math.sin((k / 23) * Math.PI) * 22,
      }));
      const right = petal.map((p, k) => ({
        x: p.x + Math.cos(angle - Math.PI / 2) * Math.sin((k / 23) * Math.PI) * 30,
        y: p.y + Math.sin(angle - Math.PI / 2) * Math.sin((k / 23) * Math.PI) * 22,
      }));
      ribbon(left, right, color, 'transparent', 0.16);
      stroke(petal, color, 1.5, 0.34, 10);
      const p = petal[Math.floor(((time * 0.2 + i * 0.06) % 1) * (petal.length - 1))];
      core(p.x, p.y, 12, 0.4);
    }

    if (a.preview) {
      const beam = ctx.createLinearGradient(0, cy, 0, a.preview.y);
      beam.addColorStop(0, '#c9f7e8');
      beam.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.36;
      ctx.fillStyle = beam;
      ctx.beginPath();
      ctx.moveTo(cx - 40, cy);
      ctx.lineTo(a.preview.cx - a.preview.w * 0.22, a.preview.y);
      ctx.lineTo(a.preview.cx + a.preview.w * 0.22, a.preview.y);
      ctx.lineTo(cx + 40, cy);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
      wash(a.preview.cx, a.preview.y, 150, '#c9f7e8', 0.35);
    }

    for (let i = 0; i < 50; i += 1) {
      const angle = hash(i) * TAU;
      const r = 70 + ((hash(i + 1) * radius + time * 30) % radius);
      core(cx + Math.cos(angle) * r, cy + Math.sin(angle) * r * 0.62, 2 + hash(i + 2) * 4, 0.25);
    }
  };

  const CONCEPTS = [
    { slug: 'cascade', name: 'Cascade', line: 'A full-page waterfall pours through the hero and breaks across the workspace.', draw: drawCascade },
    { slug: 'rift', name: 'Rift', line: 'A vertical tear opens the page and spills light across the entire interface.', draw: drawRift },
    { slug: 'vortex', name: 'Vortex', line: 'A page-scale spiral funnels the whole field down into the app.', draw: drawVortex },
    { slug: 'eclipse', name: 'Eclipse', line: 'A huge eclipse burns behind the hero and casts a corona past the content.', draw: drawEclipse },
    { slug: 'curtain', name: 'Curtain', line: 'Aurora curtains hang across the full width and pool at the workspace shelf.', draw: drawCurtain },
    { slug: 'lightning', name: 'Lightning', line: 'A branching storm bolt crosses the page and strikes the workspace.', draw: drawLightning },
    { slug: 'tide', name: 'Tide', line: 'Broad tides roll horizontally across the whole viewport.', draw: drawTide },
    { slug: 'portal', name: 'Portal', line: 'A vast portal opens behind the app and pours light into it.', draw: drawPortal },
    { slug: 'photon-rain', name: 'Photon Rain', line: 'A diagonal photon storm sweeps the page and gathers on the workspace.', draw: drawRain },
    { slug: 'bloom', name: 'Bloom', line: 'A page-scale bloom opens from the logo and channels into the interface.', draw: drawBloom },
  ];

  select.innerHTML = CONCEPTS.map((c, i) => `<option value="${c.slug}">${String(i + 1).padStart(2, '0')} — ${c.name}</option>`).join('');
  let current = Math.max(0, CONCEPTS.findIndex((c) => c.slug === params.get('concept')));
  let start = 0;
  let lastWidth = 0;
  let lastHeight = 0;

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    canvas.width = Math.max(1, Math.round(innerWidth * dpr));
    canvas.height = Math.max(1, Math.round(innerHeight * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };

  const render = (nextIndex, push = true) => {
    current = (nextIndex + CONCEPTS.length) % CONCEPTS.length;
    const concept = CONCEPTS[current];
    select.value = concept.slug;
    indexLabel.textContent = `${String(current + 1).padStart(2, '0')} / ${String(CONCEPTS.length).padStart(2, '0')}`;
    nameLabel.textContent = concept.name;
    lineLabel.textContent = concept.line;
    document.title = `Vela — ${concept.name}`;
    const url = new URL(location.href);
    url.searchParams.set('concept', concept.slug);
    if (push) history.pushState(null, '', url);
    else history.replaceState(null, '', url);
    start = performance.now();
  };

  const frame = (now) => {
    if (innerWidth !== lastWidth || innerHeight !== lastHeight) {
      lastWidth = innerWidth;
      lastHeight = innerHeight;
      resize();
    }
    ctx.clearRect(0, 0, innerWidth, innerHeight);
    const time = Math.max(0, (now - start) / 1000);
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    CONCEPTS[current].draw({ a: anchors(), w: innerWidth, h: innerHeight }, time);
    ctx.restore();
    requestAnimationFrame(frame);
  };

  prev.addEventListener('click', () => render(current - 1));
  next.addEventListener('click', () => render(current + 1));
  select.addEventListener('change', () => {
    const target = CONCEPTS.findIndex((c) => c.slug === select.value);
    if (target >= 0) render(target);
  });
  window.addEventListener('keydown', (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (/^(INPUT|SELECT|TEXTAREA|BUTTON|A|OPTION)$/.test(event.target.tagName)) return;
    if (event.key === 'ArrowLeft') { event.preventDefault(); render(current - 1); }
    if (event.key === 'ArrowRight') { event.preventDefault(); render(current + 1); }
  });
  window.addEventListener('popstate', () => {
    const target = CONCEPTS.findIndex((c) => c.slug === new URLSearchParams(location.search).get('concept'));
    if (target >= 0) render(target, false);
  });

  render(current, false);
  requestAnimationFrame(frame);
})();
