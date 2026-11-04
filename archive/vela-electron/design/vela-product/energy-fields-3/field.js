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
  const wave = (n, fn) => Array.from({ length: n }, (_, i) => fn(i / (n - 1), i));

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

  const wash = (x, y, radius, color, alpha = 1, inner = 0.1) => {
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

  const star = (x, y, radius, alpha = 1) => wash(x, y, radius, '#ffffff', alpha, 0.06);

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

  const screenFade = (w, h, stops) => {
    const g = ctx.createLinearGradient(0, 0, 0, h);
    for (const [at, color] of stops) g.addColorStop(at, color);
    ctx.save();
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, w, h);
    ctx.restore();
  };

  // 21 — a warm horizon rising behind the workspace, clear of all copy.
  const drawDawn = (env, time) => {
    const { a, w, h } = env;
    const shelf = a.preview ? a.preview.y : h * 0.62;
    const rise = 240 + Math.sin(time * 0.14) * 16;
    const g = ctx.createLinearGradient(0, shelf - rise, 0, shelf + 30);
    g.addColorStop(0, 'transparent');
    g.addColorStop(0.68, '#ff9a5e22');
    g.addColorStop(1, '#ffb15e4a');
    ctx.save();
    ctx.fillStyle = g;
    ctx.fillRect(0, shelf - rise, w, rise + 30);
    ctx.restore();

    wash(w * 0.5, shelf + 10, w * 0.42, '#ffb15e', 0.2);
    wash(w * 0.06, h * 0.98, w * 0.22, '#ff7a5e', 0.12);
    wash(w * 0.94, h * 0.98, w * 0.22, '#ffd36b', 0.1);
    stroke([{ x: 0, y: shelf }, { x: w, y: shelf }], '#ffd9a0', 1.4, 0.2, 22);

    for (let i = 0; i < 54; i += 1) {
      const x = hash(i) * w;
      const travel = (time * (4 + hash(i + 1) * 9) + hash(i + 2) * rise) % rise;
      const y = shelf - travel;
      const fade = clamp(1 - travel / rise, 0, 1);
      star(x, y, 1.6 + hash(i + 3) * 2.8, fade * 0.32);
    }
  };

  // 22 — a soft aura around the real workspace window only.
  const drawHalo = (env, time) => {
    const { a } = env;
    const p = a.preview;
    if (!p) return;
    const breathe = 1 + Math.sin(time * 0.5) * 0.035;
    wash(p.cx, p.cy, p.w * 0.78 * breathe, '#8fb8ff', 0.2);
    wash(p.cx, p.cy, p.w * 0.52 * breathe, '#9fe8d3', 0.16);

    ctx.save();
    ctx.globalAlpha = 0.4;
    ctx.strokeStyle = '#cfe4ff';
    ctx.lineWidth = 1.2;
    ctx.shadowColor = '#9fe8d3';
    ctx.shadowBlur = 26;
    ctx.beginPath();
    ctx.roundRect(p.x - 6, p.y - 6, p.w + 12, p.h + 12, 18);
    ctx.stroke();
    ctx.restore();

    for (let i = 0; i < 3; i += 1) {
      const t = (time * 0.045 + i / 3) % 1;
      const edge = Math.floor(t * 4);
      const local = (t * 4) % 1;
      const x = edge === 1 ? p.right : edge === 3 ? p.x : lerp(p.x, p.right, local);
      const y = edge === 0 ? p.y : edge === 2 ? p.bottom : lerp(p.y, p.bottom, local);
      star(x, y, 17, 0.42);
    }
  };

  // 23 — slow dust suspended in the page, visible only as faint points.
  const drawDrift = (env, time) => {
    const { w, h } = env;
    wash(w * 0.16, h * 0.12, w * 0.3, '#8fb8ff', 0.05);
    wash(w * 0.86, h * 0.2, w * 0.26, '#c9a1ff', 0.04);
    for (let i = 0; i < 150; i += 1) {
      const x = (hash(i) * w + time * (2.5 + hash(i + 1) * 5)) % w;
      const y = hash(i + 2) * h + Math.sin(time * 0.22 + i) * 9;
      star(x, y, 1.2 + hash(i + 3) * 2.8, 0.16 + hash(i + 4) * 0.22);
    }
    for (let i = 0; i < 5; i += 1) {
      const x = i % 2 ? w * (0.82 + hash(i) * 0.1) : w * (0.06 + hash(i) * 0.1);
      const y = h * (0.12 + hash(i + 2) * 0.6);
      wash(x, y, 55 + hash(i + 3) * 60, i % 2 ? '#c9a1ff' : '#8fb8ff', 0.035);
    }
  };

  // 24 — diagonal light bands drifting slowly, like sun through blinds.
  const drawMarquee = (env, time) => {
    const { w, h } = env;
    const colors = ['#9fe8d3', '#c9a1ff', '#8fb8ff', '#9fe8d3', '#f0a6ff'];
    for (let i = 0; i < 6; i += 1) {
      const band = 90 + hash(i) * 150;
      const offset = ((time * 4 + i * 260) % (w + 700)) - 350;
      const slant = 220;
      const g = ctx.createLinearGradient(offset, 0, offset + band, 0);
      g.addColorStop(0, 'transparent');
      g.addColorStop(0.5, colors[i % colors.length]);
      g.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.045;
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.moveTo(offset, 0);
      ctx.lineTo(offset + band, 0);
      ctx.lineTo(offset + band - slant, h);
      ctx.lineTo(offset - slant, h);
      ctx.closePath();
      ctx.fill();
      ctx.restore();
    }
    wash(w * 0.5, 0, w * 0.7, '#9fe8d3', 0.05);
  };

  // 25 — light traveling around the viewport edge, outside the content column.
  const drawEdge = (env, time) => {
    const { w, h } = env;
    const inset = 9;
    const perimeter = 2 * (w - inset * 2) + 2 * (h - inset * 2);

    ctx.save();
    ctx.globalAlpha = 0.16;
    ctx.strokeStyle = '#9fe8d3';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(inset, inset, w - inset * 2, h - inset * 2, 12);
    ctx.stroke();
    ctx.restore();

    ctx.save();
    ctx.globalAlpha = 0.75;
    ctx.strokeStyle = '#d8fff3';
    ctx.lineWidth = 2;
    ctx.setLineDash([46, perimeter - 46]);
    ctx.lineDashOffset = -time * 26;
    ctx.shadowColor = '#9fe8d3';
    ctx.shadowBlur = 16;
    ctx.beginPath();
    ctx.roundRect(inset, inset, w - inset * 2, h - inset * 2, 12);
    ctx.stroke();
    ctx.restore();

    wash(0, 0, 110, '#9fe8d3', 0.12);
    wash(w, 0, 110, '#8fb8ff', 0.1);
    wash(0, h, 110, '#9fe8d3', 0.1);
    wash(w, h, 110, '#c9a1ff', 0.1);
  };

  // 26 — light runs along the real section dividers instead of across the copy.
  const drawHairline = (env, time) => {
    const { a, w, h } = env;
    const lines = [
      a.header ? a.header.bottom : null,
      a.preview ? a.preview.y : null,
      ...a.sections.map((s) => s.y),
      a.footer ? a.footer.bottom : null,
    ].filter((y) => Number.isFinite(y) && y > 0 && y < h);

    lines.forEach((y, i) => {
      stroke([{ x: 0, y }, { x: w, y }], i % 2 ? '#8fb8ff' : '#9fe8d3', 1, 0.09);
      const x = ((time * 0.1 + i * 0.19) % 1) * (w + 500) - 250;
      const g = ctx.createLinearGradient(x - 170, 0, x + 170, 0);
      g.addColorStop(0, 'transparent');
      g.addColorStop(0.5, i % 2 ? '#cfe4ff' : '#d8fff3');
      g.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.46;
      ctx.fillStyle = g;
      ctx.fillRect(x - 170, y - 0.8, 340, 1.6);
      ctx.restore();
    });

    for (let i = 0; i < 24; i += 1) {
      const x = (i / 24) * w;
      const y = a.preview ? a.preview.y : h * 0.6;
      star(x, y, 2, 0.12);
    }
  };

  // 27 — a low aurora veil confined to the hero's empty upper air.
  const drawAurora = (env, time) => {
    const { a, w, h } = env;
    const floor = a.hero ? a.hero.bottom : h * 0.6;
    wash(w * 0.5, floor * 0.2, w * 0.6, '#0ea5a0', 0.06);
    const colors = ['#9fe8d3', '#8fb8ff', '#c9a1ff', '#9fe8d3', '#6bd6ff'];
    for (let i = 0; i < 6; i += 1) {
      const base = lerp(h * 0.02, floor * 0.72, i / 5);
      const top = wave(36, (t) => ({
        x: t * w,
        y: base + Math.sin(t * 2.4 + time * 0.18 + i) * (38 + i * 6),
      }));
      const bottom = top.map((p, k) => ({
        x: p.x,
        y: p.y + 150 + Math.sin(k * 0.16 + time * 0.26 + i) * 55,
      }));
      ribbon(top, bottom, colors[i % colors.length], 'transparent', 0.075);
    }
    wash(w * 0.3, floor * 0.3, w * 0.3, '#c9a1ff', 0.035);
  };

  // 28 — large out-of-focus lights drifting near the page corners.
  const drawBokeh = (env, time) => {
    const { w, h } = env;
    const spots = [
      { x: 0.08, y: 0.14, r: 128, c: '#8fb8ff' },
      { x: 0.9, y: 0.1, r: 104, c: '#c9a1ff' },
      { x: 0.05, y: 0.62, r: 96, c: '#9fe8d3' },
      { x: 0.94, y: 0.56, r: 120, c: '#f0a6ff' },
      { x: 0.2, y: 0.9, r: 86, c: '#ffd36b' },
      { x: 0.82, y: 0.92, r: 110, c: '#8fb8ff' },
      { x: 0.5, y: 0.04, r: 78, c: '#9fe8d3' },
    ];
    spots.forEach((s, i) => {
      const x = s.x * w + Math.sin(time * 0.16 + i) * 18;
      const y = s.y * h + Math.cos(time * 0.12 + i) * 12;
      const g = ctx.createRadialGradient(x, y, 0, x, y, s.r);
      g.addColorStop(0, s.c);
      g.addColorStop(0.55, s.c);
      g.addColorStop(1, 'transparent');
      ctx.save();
      ctx.globalAlpha = 0.075;
      ctx.fillStyle = g;
      ctx.beginPath();
      ctx.arc(x, y, s.r, 0, TAU);
      ctx.fill();
      ctx.restore();
    });
  };

  // 29 — sparse embers lifting from the bottom corners.
  const drawEmber = (env, time) => {
    const { w, h } = env;
    wash(w * 0.04, h * 1.02, w * 0.26, '#ff7a5e', 0.16);
    wash(w * 0.96, h * 1.02, w * 0.26, '#ffb15e', 0.14);
    for (let i = 0; i < 48; i += 1) {
      const side = i % 2;
      const originX = side ? w * (0.88 + hash(i) * 0.1) : w * (0.02 + hash(i) * 0.1);
      const rise = (time * (10 + hash(i + 1) * 22) + hash(i + 2) * h) % (h * 0.7);
      const x = originX + Math.sin(time * 0.5 + i) * 24;
      const y = h - rise;
      star(x, y, 2.2 + hash(i + 3) * 3.6, (1 - rise / (h * 0.7)) * 0.62);
    }
    for (let i = 0; i < 10; i += 1) {
      const x = (i % 2 ? w * 0.9 : w * 0.1) + (hash(i) - 0.5) * 60;
      const y = h - (h * 0.25) * hash(i + 2);
      wash(x, y, 9, '#ffd36b', 0.1);
    }
  };

  // 30 — a faint scan of light passing down the page, once every few seconds.
  const drawSignal = (env, time) => {
    const { w, h } = env;
    const cycle = 9;
    const t = (time % cycle) / cycle;
    const y = lerp(-120, h + 120, t);
    const fade = Math.sin(t * Math.PI);

    const band = ctx.createLinearGradient(0, y - 90, 0, y + 90);
    band.addColorStop(0, 'transparent');
    band.addColorStop(0.5, '#9fe8d3');
    band.addColorStop(1, 'transparent');
    ctx.save();
    ctx.globalAlpha = 0.055 * fade;
    ctx.fillStyle = band;
    ctx.fillRect(0, y - 90, w, 180);
    ctx.restore();

    const line = ctx.createLinearGradient(0, 0, w, 0);
    line.addColorStop(0, 'transparent');
    line.addColorStop(0.5, '#d8fff3');
    line.addColorStop(1, 'transparent');
    ctx.save();
    ctx.globalAlpha = 0.16 * fade;
    ctx.fillStyle = line;
    ctx.fillRect(0, y, w, 1);
    ctx.restore();

    for (let i = 0; i < 18; i += 1) {
      const x = (i / 18) * w;
      star(x, y + Math.sin(i + time) * 3, 1.8, 0.12 * fade);
    }
  };

  const CONCEPTS = [
    { slug: 'dawn', name: 'Dawn', line: 'Warm light rises from the bottom edge without touching the copy.', draw: drawDawn },
    { slug: 'halo', name: 'Halo', line: 'A soft breathing aura stays around the workspace window only.', draw: drawHalo },
    { slug: 'drift', name: 'Drift', line: 'Faint dust drifts through the page background at a slow pace.', draw: drawDrift },
    { slug: 'marquee', name: 'Marquee', line: 'Wide diagonal bands of light pass behind the page like window blinds.', draw: drawMarquee },
    { slug: 'edge', name: 'Edge', line: 'A current travels around the viewport border, outside the content.', draw: drawEdge },
    { slug: 'hairline', name: 'Hairline', line: 'Light moves along the real section dividers, not through the copy.', draw: drawHairline },
    { slug: 'aurora', name: 'Aurora', line: 'A low aurora veil stays in the empty air above the hero.', draw: drawAurora },
    { slug: 'bokeh', name: 'Bokeh', line: 'Large out-of-focus lights drift near the page corners.', draw: drawBokeh },
    { slug: 'ember', name: 'Ember', line: 'Sparse warm embers lift from the bottom corners.', draw: drawEmber },
    { slug: 'signal', name: 'Signal', line: 'A faint band of light scans the page once every few seconds.', draw: drawSignal },
  ];

  select.innerHTML = CONCEPTS.map((c, i) => `<option value="${c.slug}">${String(i + 21).padStart(2, '0')} — ${c.name}</option>`).join('');
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
    indexLabel.textContent = `${String(current + 21).padStart(2, '0')} / 30`;
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
