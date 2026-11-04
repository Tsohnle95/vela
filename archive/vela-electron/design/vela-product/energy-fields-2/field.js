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
    brand: rectOf('.ob-brand'),
    title: rectOf('.ob-title'),
    logo: rectOf('.ob-title-logo'),
    preview: rectOf('.ob-preview'),
    hero: rectOf('.ob-hero'),
    download: rectOf('.ob-header-actions .ob-btn-primary'),
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

  const wash = (x, y, radius, color, alpha = 1, inner = 0.14) => {
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

  const star = (x, y, radius, alpha = 1) => wash(x, y, radius, '#ffffff', alpha, 0.08);

  const wave = (n, fn) => Array.from({ length: n }, (_, i) => fn(i / (n - 1), i));

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

  const poly = (points, colorA, colorB, alpha) => {
    if (!points || points.length < 3) return;
    const minY = Math.min(...points.map((p) => p.y));
    const maxY = Math.max(...points.map((p) => p.y));
    const g = ctx.createLinearGradient(0, minY, 0, Math.max(maxY, minY + 1));
    g.addColorStop(0, colorA);
    g.addColorStop(1, colorB);
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (const p of points.slice(1)) ctx.lineTo(p.x, p.y);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  };

  const previewTop = (a, h) => (a.preview ? a.preview.y : h * 0.66);
  const previewX = (a, w) => (a.preview ? a.preview.cx : w / 2);

  // 12 — luminous fabric drifting across the page in deep folds.
  const drawSilk = (env, time) => {
    const { a, w, h } = env;
    wash(previewX(a, w), h * 0.35, w * 0.7, '#8b5cf6', 0.14);
    const colors = ['#9fe8d3', '#c9a1ff', '#8fb8ff', '#f0a6ff', '#d8fff3'];
    for (let i = 0; i < 7; i += 1) {
      const base = h * (0.08 + i * 0.11);
      const top = wave(60, (t) => ({
        x: t * w,
        y: base + Math.sin(t * 4.2 + time * 0.22 + i * 1.3) * (46 + i * 7) + Math.sin(t * 9.5 - time * 0.16 + i) * 16,
      }));
      const bottom = top.map((p, k) => ({
        x: p.x,
        y: p.y + 110 + Math.sin(k * 0.18 + time * 0.34 + i * 0.9) * 42 + i * 8,
      }));
      ribbon(top, bottom, colors[i % colors.length], 'transparent', 0.1);
      stroke(top, colors[i % colors.length], 1.4, 0.28, 12);
      const fold = top.map((p, k) => ({ x: p.x, y: p.y + 40 + Math.sin(k * 0.3 + time * 0.5) * 24 }));
      stroke(fold, colors[(i + 2) % colors.length], 0.8, 0.14);
      for (let k = 0; k < 2; k += 1) {
        const p = pick(top, (time * 0.18 + k * 0.5 + i * 0.08) % 1);
        star(p.x, p.y, 13, 0.42);
      }
    }
    wash(w * 0.5, previewTop(a, h), w * 0.45, '#c9a1ff', 0.16);
  };

  // 13 — a gravity well bends a page-wide spacetime grid into the workspace.
  const drawGravity = (env, time) => {
    const { a, w, h } = env;
    const cx = previewX(a, w);
    const cy = a.preview ? a.preview.y + a.preview.h * 0.26 : h * 0.6;
    const strength = a.preview ? a.preview.w * 0.4 : w * 0.3;
    wash(cx, cy, w * 0.55, '#6d5cff', 0.16);
    wash(cx, cy, strength * 0.8, '#cfe4ff', 0.14);

    const bend = (x, y) => {
      const dx = cx - x;
      const dy = cy - y;
      const d = Math.hypot(dx, dy) + 1;
      const k = strength / (strength + d * 1.4);
      return { x: x + dx * k * 0.32, y: y + dy * k * 0.32 };
    };

    for (let x = -w * 0.2; x < w * 1.2; x += 54) {
      const pts = wave(44, (t) => bend(x, t * h * 1.1 - h * 0.05));
      const near = Math.abs(x - cx) < w * 0.3 ? 0.3 : 0.1;
      stroke(pts, '#8fb8ff', 1, near);
    }
    for (let y = -h * 0.2; y < h * 1.2; y += 54) {
      const pts = wave(50, (t) => bend(t * w * 1.1 - w * 0.05, y));
      const near = Math.abs(y - cy) < h * 0.3 ? 0.3 : 0.1;
      stroke(pts, '#8fb8ff', 1, near);
    }

    for (let arm = 0; arm < 9; arm += 1) {
      const pts = [];
      for (let i = 0; i <= 44; i += 1) {
        const t = i / 44;
        const r = strength * (0.55 + (1 - t) * 1.7);
        const angle = (arm / 9) * TAU + t * 4.4 + time * 0.16;
        pts.push({ x: cx + Math.cos(angle) * r, y: cy + Math.sin(angle) * r * 0.62 });
      }
      stroke(pts, arm % 2 ? '#c9a1ff' : '#8fb8ff', 1.3, 0.22, 10);
      const p = pick(pts, (time * 0.3 + arm * 0.1) % 1);
      star(p.x, p.y, 12, 0.5);
    }

    ctx.save();
    ctx.globalAlpha = 0.28;
    ctx.strokeStyle = '#8fb8ff';
    ctx.lineWidth = 1.2;
    ctx.setLineDash([26, 18]);
    ctx.lineDashOffset = -time * 45;
    ctx.beginPath();
    ctx.ellipse(cx, cy, strength * 1.02, strength * 0.72, 0, 0, TAU);
    ctx.stroke();
    ctx.restore();

    ctx.save();
    ctx.globalAlpha = 0.65;
    ctx.strokeStyle = '#d8fff3';
    ctx.lineWidth = 2;
    ctx.shadowColor = '#d8fff3';
    ctx.shadowBlur = 30;
    ctx.beginPath();
    ctx.ellipse(cx, cy, strength * 0.62, strength * 0.5, 0, 0, TAU);
    ctx.stroke();
    ctx.restore();

    for (let i = 0; i < 24; i += 1) {
      const t = (time * 0.12 + i / 24) % 1;
      const angle = t * TAU * 2 + i;
      const r = strength * (0.62 + t * 1.1);
      const p = { x: cx + Math.cos(angle) * r, y: cy + Math.sin(angle) * r * 0.7 };
      star(p.x, p.y, 8, 0.5 * (1 - t));
      stroke([p, { x: cx + Math.cos(angle + 0.5) * r * 0.82, y: cy + Math.sin(angle + 0.5) * r * 0.58 }], '#c9a1ff', 1, 0.2);
    }
    wash(cx, cy, strength * 0.35, '#05060a', 0.3, 0.6);
  };

  // 14 — a supernova shockwave and debris sweeping the whole page.
  const drawSupernova = (env, time) => {
    const { a, w, h } = env;
    const origin = a.title || a.logo;
    const cx = origin ? origin.cx : w / 2;
    const cy = origin ? origin.cy : h * 0.3;
    const span = Math.hypot(w, h) * 0.62;
    const cycle = (time % 7) / 7;
    wash(cx, cy, span * 0.7, '#ff7ab8', 0.18);
    wash(cx, cy, 120, '#fff4cf', 0.6);

    for (let ring = 0; ring < 3; ring += 1) {
      const phase = (cycle + ring / 3) % 1;
      const radius = phase * span;
      const alpha = (1 - phase) ** 2 * 0.6;
      ctx.save();
      ctx.globalAlpha = alpha;
      ctx.strokeStyle = ring === 1 ? '#ffd36b' : '#ff7ab8';
      ctx.lineWidth = 2 + (1 - phase) * 6;
      ctx.shadowColor = '#ff7ab8';
      ctx.shadowBlur = 24;
      ctx.beginPath();
      ctx.arc(cx, cy, radius, 0, TAU);
      ctx.stroke();
      ctx.restore();
    }

    for (let i = 0; i < 90; i += 1) {
      const angle = hash(i) * TAU;
      const phase = (cycle * 1.4 + hash(i + 2)) % 1;
      const distance = phase * span * 1.15;
      const x = cx + Math.cos(angle) * distance;
      const y = cy + Math.sin(angle) * distance * 0.62;
      const len = 8 + (1 - phase) * 46;
      stroke([{ x: x - Math.cos(angle) * len, y: y - Math.sin(angle) * len * 0.62 }, { x, y }], i % 3 ? '#ffb15e' : '#c9a1ff', 1 + hash(i + 5) * 2, (1 - phase) * 0.5);
      if (i % 6 === 0) star(x, y, 7, (1 - phase) * 0.5);
    }

    if (a.preview) {
      wash(a.preview.cx, a.preview.y, 170, '#ff7ab8', 0.28);
      stroke([{ x: a.preview.x, y: a.preview.y }, { x: a.preview.right, y: a.preview.y }], '#ffd36b', 2, 0.28, 18);
    }
  };

  // 15 — a beam strikes a prism behind the title and fans into spectra across the page.
  const drawPrism = (env, time) => {
    const { a, w, h } = env;
    const origin = a.logo || a.title;
    const ox = origin ? origin.cx : w / 2;
    const oy = origin ? origin.cy : h * 0.28;
    const px = a.title ? a.title.cx : w / 2;
    const py = a.title ? a.title.bottom + 90 : h * 0.5;
    wash(px, py, w * 0.5, '#8fb8ff', 0.14);

    const prism = [
      { x: px - 66, y: py + 46 },
      { x: px, y: py - 52 },
      { x: px + 66, y: py + 46 },
    ];
    stroke([...prism, prism[0]], '#e8f4ff', 2, 0.7, 20);
    poly(prism, '#ffffff', '#8fb8ff', 0.16);

    const beam = [origin ? { x: ox, y: oy + 30 } : { x: w / 2, y: 0 }, { x: px, y: py }];
    stroke(beam, '#ffffff', 9, 0.4, 30);
    stroke(beam, '#e8f4ff', 2, 0.95, 10);

    const spectrum = ['#ff6b8a', '#ffb15e', '#ffe08a', '#9fe8d3', '#6bd6ff', '#8fb8ff', '#c9a1ff'];
    spectrum.forEach((color, i) => {
      const spread = (i - 3) / 3;
      const end = { x: px + spread * w * 0.62, y: h * (0.9 - Math.abs(spread) * 0.12) };
      const shaft = [
        { x: px - 8, y: py + 26 },
        { x: end.x - 34, y: end.y },
        { x: end.x + 34, y: end.y },
        { x: px + 8, y: py + 26 },
      ];
      poly(shaft, color, 'transparent', 0.17);
      stroke([{ x: px, y: py + 20 }, end], color, 2.6, 0.55, 16);
      const p = pick([{ x: px, y: py }, end], (time * 0.4 + i * 0.1) % 1);
      star(p.x, p.y, 12, 0.5);
    });

    for (let i = 0; i < 30; i += 1) {
      const t = hash(i);
      star(px + (hash(i + 3) - 0.5) * 120, py + (t - 0.5) * 100, 2.5, 0.3);
    }
  };

  // 16 — a double helix of photons descends the page into the workspace.
  const drawHelix = (env, time) => {
    const { a, w, h } = env;
    const cx = previewX(a, w);
    const bottom = a.preview ? a.preview.y + a.preview.h * 0.3 : h * 0.8;
    wash(cx, h * 0.4, w * 0.45, '#6d5cff', 0.16);
    const radiusAt = (t) => 24 + t * Math.min(w * 0.26, 230);
    const strands = [0, Math.PI];
    for (const phase of strands) {
      const pts = wave(70, (t) => ({
        x: cx + Math.sin(t * TAU * 2.4 + time * 0.7 + phase) * radiusAt(t),
        y: lerp(-50, bottom, t),
      }));
      const color = phase ? '#9fe8d3' : '#c9a1ff';
      stroke(pts, color, 10, 0.14, 30);
      stroke(pts, color, 2.2, 0.7, 12);
      for (let k = 0; k < 2; k += 1) {
        const p = pick(pts, (time * 0.28 + k * 0.5) % 1);
        star(p.x, p.y, 18, 0.6);
      }
    }
    for (let i = 0; i < 22; i += 1) {
      const t = i / 22;
      const x1 = cx + Math.sin(t * TAU * 2.4 + time * 0.7) * radiusAt(t);
      const x2 = cx + Math.sin(t * TAU * 2.4 + time * 0.7 + Math.PI) * radiusAt(t);
      const y = lerp(-50, bottom, t);
      const glow = 0.5 + 0.5 * Math.sin(time * 1.4 + i);
      stroke([{ x: x1, y }, { x: x2, y }], '#8fb8ff', 1, 0.06 + glow * 0.18);
    }
    wash(cx, bottom, 130, '#c9f7e8', 0.3);
    stroke([{ x: a.preview ? a.preview.x : 0, y: bottom }, { x: a.preview ? a.preview.right : w, y: bottom }], '#c9f7e8', 1.6, 0.3, 16);
  };

  // 17 — a magnetar at the logo throws dipole field loops across the page.
  const drawMagnetar = (env, time) => {
    const { a, w, h } = env;
    const origin = a.logo || a.title;
    const cx = origin ? origin.cx : w / 2;
    const cy = origin ? origin.cy : h * 0.3;
    wash(cx, cy, Math.min(w, h) * 0.6, '#8b5cf6', 0.18);
    wash(cx, cy, 70, '#ffffff', 0.5);

    for (let side = -1; side <= 1; side += 2) {
      for (let loop = 0; loop < 6; loop += 1) {
        const spread = 170 + loop * 90;
        const height = 130 + loop * 62;
        const pts = [];
        for (let i = 0; i <= 40; i += 1) {
          const t = i / 40;
          const bulge = Math.sin(t * Math.PI);
          pts.push({
            x: cx + side * bulge * spread + Math.sin(t * TAU + time * 0.3 + loop) * 6,
            y: cy + (t - 0.5) * height * 2 * 0.32 + Math.sin(t * Math.PI * 2 + time * 0.4) * 10,
          });
        }
        const color = loop % 2 ? '#8fb8ff' : '#c9a1ff';
        stroke(pts, color, 1.2, 0.22, 10);
        const p = pick(pts, (time * 0.3 + loop * 0.13 + (side > 0 ? 0 : 0.17)) % 1);
        star(p.x, p.y, 10, 0.55);
      }
    }

    stroke([{ x: cx, y: cy }, { x: cx, y: -80 }], '#e8f4ff', 8, 0.16, 30);
    stroke([{ x: cx, y: cy }, { x: cx, y: -80 }], '#e8f4ff', 1.6, 0.6, 12);
    stroke([{ x: cx, y: cy }, { x: cx, y: h + 80 }], '#c9a1ff', 10, 0.1, 30);
    stroke([{ x: cx, y: cy }, { x: cx, y: h + 80 }], '#c9a1ff', 2, 0.4, 12);

    for (let i = 0; i < 40; i += 1) {
      const t = (time * 0.5 + hash(i)) % 1;
      const side = i % 2 ? 1 : -1;
      const spread = 170 + (i % 6) * 90;
      const y = cy + (t - 0.5) * 240;
      star(cx + side * Math.sin(t * Math.PI) * spread, y, 3, 0.4 * (1 - t * 0.6));
    }
  };

  // 18 — geysers erupt from behind the workspace and rain light back down.
  const drawGeyser = (env, time) => {
    const { a, w, h } = env;
    const ground = previewTop(a, h);
    const vents = [0.2, 0.42, 0.61, 0.8];
    wash(w / 2, ground, w * 0.5, '#6ee7b7', 0.18);
    vents.forEach((fx, i) => {
      const x = w * fx;
      const phase = (time * 0.34 + i * 0.22) % 1;
      const burst = Math.sin(phase * Math.PI);
      const color = i % 2 ? '#9fe8d3' : '#8fb8ff';

      for (let y = ground; y > ground - h * 0.45; y -= 34) {
        const lift = (ground - y) / (h * 0.45);
        const sway = Math.sin(lift * 6 + time * 1.4 + i) * 16 * lift;
        wash(x + sway, y, 26 + lift * 42, color, 0.05 * burst);
      }

      for (let k = 0; k < 100; k += 1) {
        const seed = i * 31 + k * 3.7;
        const t = (time * (0.4 + hash(seed) * 0.25) + hash(seed + 1)) % 1;
        const height = (h * 0.34 + i * 24) * burst * (0.45 + hash(seed + 2) * 0.75);
        const drift = Math.sin(t * Math.PI) * (hash(seed + 3) - 0.5) * (180 + hash(seed + 4) * 220);
        const px = x + drift + Math.sin(time * 1.1 + seed) * 8;
        const py = ground - 4 * height * t * (1 - t);
        const fade = Math.sin(t * Math.PI);
        star(px, py, 1.8 + hash(seed + 5) * 4.6, fade * 0.8);
        if (k % 14 === 0) {
          stroke([{ x: px, y: py }, { x: px + (hash(seed + 6) - 0.5) * 16, y: py + 14 + hash(seed + 7) * 22 }], color, 1, fade * 0.3);
        }
      }

      const peak = ground - h * 0.4 * burst;
      wash(x, peak, 90 * burst + 20, color, burst * 0.14);
      wash(x, ground, 60 + burst * 40, color, 0.24 + burst * 0.18);
    });
    for (let i = 0; i < 70; i += 1) {
      const x = hash(i) * w;
      const y = ground - 40 - hash(i + 4) * h * 0.4;
      star(x, y, 2.4, 0.16);
    }
  };

  // 19 — light floods upward from the bottom of the page and submerges the interface.
  const drawFlood = (env, time) => {
    const { a, w, h } = env;
    const surface = previewTop(a, h) - 40 + Math.sin(time * 0.5) * 10;
    wash(w / 2, h, w * 0.7, '#0ea5b7', 0.2);
    wash(w / 2, surface, w * 0.55, '#9fe8d3', 0.24);

    const top = wave(60, (t) => ({
      x: t * w,
      y: surface + Math.sin(t * 7 + time * 0.9) * 12 + Math.sin(t * 15 - time * 0.5) * 5,
    }));
    const bottom = [{ x: w, y: h }, { x: 0, y: h }];
    poly([...top, ...bottom], '#9fe8d3', '#082f3a', 0.16);
    stroke(top, '#d8fff3', 2, 0.6, 20);

    for (let i = 0; i < 10; i += 1) {
      const y = surface + 30 + i * 36;
      const pts = wave(30, (t) => ({
        x: t * w,
        y: y + Math.sin(t * 5 + time * 0.7 + i * 0.9) * 10,
      }));
      stroke(pts, i % 2 ? '#6bd6ff' : '#9fe8d3', 1, 0.14);
    }
    for (let i = 0; i < 8; i += 1) {
      const pts = wave(40, (t) => ({
        x: w * (i / 7) + Math.sin(t * 5.2 + time * 0.6 + i) * 46,
        y: lerp(surface + 12, h + 30, t),
      }));
      stroke(pts, i % 2 ? '#6bd6ff' : '#d8fff3', 1, 0.09);
    }
    wash(w / 2, surface + 30, w * 0.42, '#c9f7e8', 0.16);

    for (let i = 0; i < 80; i += 1) {
      const x = hash(i) * w;
      const rise = (time * (10 + hash(i + 1) * 26) + hash(i + 2) * h) % h;
      const y = h - rise;
      if (y > surface) star(x, y, 2 + hash(i + 3) * 4, 0.28);
    }

    for (let i = 0; i < 8; i += 1) {
      const x = w * (0.1 + i * 0.1);
      const pts = wave(20, (t) => ({
        x: x + Math.sin(t * 4 + time * 0.5 + i) * 20,
        y: lerp(surface, h * 0.1, t),
      }));
      stroke(pts, '#6bd6ff', 1.2, 0.1);
    }
  };

  // 20 — the real interface becomes a page-scale constellation.
  const drawConstellation = (env, time) => {
    const { a, w, h } = env;
    wash(w / 2, h * 0.4, w * 0.6, '#3b2f7a', 0.18);
    const nodes = [
      a.brand, a.download, a.logo, a.title,
      a.preview && { x: a.preview.x, y: a.preview.y, cx: a.preview.x, cy: a.preview.y },
      a.preview && { x: a.preview.right, y: a.preview.y, cx: a.preview.right, cy: a.preview.y },
      a.preview && { x: a.preview.x, y: a.preview.bottom, cx: a.preview.x, cy: a.preview.bottom },
      a.preview && { x: a.preview.right, y: a.preview.bottom, cx: a.preview.right, cy: a.preview.bottom },
      a.preview && { x: a.preview.cx, y: a.preview.cy, cx: a.preview.cx, cy: a.preview.cy },
      a.sections[0], a.sections[1], a.footer,
    ].filter(Boolean).map((r) => ({ x: r.cx ?? r.x, y: r.cy ?? r.y }));

    const links = [[0, 1], [0, 2], [2, 3], [1, 3], [3, 4], [3, 5], [4, 5], [4, 6], [5, 7], [6, 7], [5, 8], [6, 8], [6, 9], [7, 9], [9, 10], [10, 11], [9, 11]];
    links.forEach(([i, j], k) => {
      const p1 = nodes[i];
      const p2 = nodes[j];
      if (!p1 || !p2) return;
      const pts = [p1, { x: lerp(p1.x, p2.x, 0.5), y: lerp(p1.y, p2.y, 0.5) + Math.sin(k + time * 0.2) * 18 }, p2];
      stroke(pts, k % 2 ? '#8fb8ff' : '#c9a1ff', 1.4, 0.32, 12);
      const pulse = pick(pts, (time * 0.22 + k * 0.09) % 1);
      star(pulse.x, pulse.y, 9, 0.6);
    });

    nodes.forEach((p, i) => {
      star(p.x, p.y, 16 + (i % 3) * 6, 0.75);
      stroke([{ x: p.x - 12, y: p.y }, { x: p.x + 12, y: p.y }], '#d8fff3', 1, 0.3);
      stroke([{ x: p.x, y: p.y - 12 }, { x: p.x, y: p.y + 12 }], '#d8fff3', 1, 0.3);
    });

    for (let i = 0; i < 180; i += 1) {
      const x = (hash(i) * w + time * (0.6 + hash(i + 1) * 1.4)) % w;
      const y = hash(i + 2) * h;
      star(x, y, 1.6 + hash(i + 3) * 2.4, 0.14 + hash(i + 4) * 0.3);
    }
  };

  const CONCEPTS = [
    { slug: 'nebula-storm', name: 'Nebula Storm', line: 'The nebula grows weather: lightning moves through the galaxy clouds.', draw: null, external: 'nebula-storm.html' },
    { slug: 'silk', name: 'Silk', line: 'Luminous fabric drifts across the page and folds through the interface.', draw: drawSilk },
    { slug: 'gravity', name: 'Gravity', line: 'A gravity well bends a page-wide spacetime grid into the workspace.', draw: drawGravity },
    { slug: 'supernova', name: 'Supernova', line: 'A shockwave and debris sweep the entire page from behind the hero.', draw: drawSupernova },
    { slug: 'prism', name: 'Prism', line: 'A beam strikes a prism behind the title and fans into spectra across the page.', draw: drawPrism },
    { slug: 'helix', name: 'Helix', line: 'Two intertwined strands of photons descend the page into the app.', draw: drawHelix },
    { slug: 'magnetar', name: 'Magnetar', line: 'A magnetar at the logo throws dipole field loops across the page.', draw: drawMagnetar },
    { slug: 'geyser', name: 'Geyser', line: 'Geysers erupt from behind the workspace and rain light back down.', draw: drawGeyser },
    { slug: 'flood', name: 'Flood', line: 'Light floods upward from the bottom of the page and submerges the interface.', draw: drawFlood },
    { slug: 'constellation', name: 'Constellation', line: 'The real interface becomes a page-scale constellation of light.', draw: drawConstellation },
  ];

  select.innerHTML = CONCEPTS.map((c, i) => `<option value="${c.slug}">${String(i + 11).padStart(2, '0')} — ${c.name}</option>`).join('');
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
    if (concept.external) {
      location.href = concept.external;
      return;
    }
    select.value = concept.slug;
    indexLabel.textContent = `${String(current + 11).padStart(2, '0')} / 20`;
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
    const concept = CONCEPTS[current];
    if (concept.draw) {
      ctx.save();
      ctx.globalCompositeOperation = 'lighter';
      concept.draw({ a: anchors(), w: innerWidth, h: innerHeight }, time);
      ctx.restore();
    }
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
