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
  const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
  const lerp = (a, b, t) => a + (b - a) * t;
  const hash = (n) => {
    const value = Math.sin(n * 127.1 + 311.7) * 43758.5453;
    return value - Math.floor(value);
  };

  const rectOf = (selector) => {
    const el = document.querySelector(selector);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) return null;
    return { x: r.left, y: r.top, w: r.width, h: r.height, cx: r.left + r.width / 2, cy: r.top + r.height / 2, right: r.right, bottom: r.bottom };
  };

  const anchors = () => ({
    header: rectOf('.ob-header'),
    brand: rectOf('.ob-brand'),
    nav: rectOf('.ob-nav'),
    download: rectOf('.ob-header-actions .ob-btn-primary'),
    hero: rectOf('.ob-hero'),
    title: rectOf('.ob-title'),
    logo: rectOf('.ob-title-logo'),
    description: rectOf('.ob-description'),
    preview: rectOf('.ob-preview'),
    sections: Array.from(document.querySelectorAll('.ob-section, .ob-features, .ob-flow, .ob-faq')).map((el) => {
      const r = el.getBoundingClientRect();
      return { x: r.left, y: r.top, w: r.width, h: r.height, cx: r.left + r.width / 2, cy: r.top + r.height / 2, right: r.right, bottom: r.bottom };
    }).filter((r) => r.w > 0),
    footer: rectOf('.ob-footer'),
  });

  const sample = (points, t) => {
    if (!points.length) return null;
    if (points.length === 1) return points[0];
    const lengths = [];
    let total = 0;
    for (let i = 1; i < points.length; i += 1) {
      const dx = points[i].x - points[i - 1].x;
      const dy = points[i].y - points[i - 1].y;
      const length = Math.hypot(dx, dy);
      lengths.push(length);
      total += length;
    }
    const target = clamp(t, 0, 1) * total;
    let run = 0;
    for (let i = 0; i < lengths.length; i += 1) {
      if (run + lengths[i] >= target) {
        const k = (target - run) / Math.max(lengths[i], 0.001);
        return { x: lerp(points[i].x, points[i + 1].x, k), y: lerp(points[i].y, points[i + 1].y, k) };
      }
      run += lengths[i];
    }
    return points[points.length - 1];
  };

  const line = (points, color, width, alpha = 1) => {
    if (points.length < 2) return;
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.strokeStyle = color;
    ctx.lineWidth = width;
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.beginPath();
    ctx.moveTo(points[0].x, points[0].y);
    for (let i = 1; i < points.length; i += 1) ctx.lineTo(points[i].x, points[i].y);
    ctx.stroke();
    ctx.restore();
  };

  const glow = (point, color, radius, alpha = 1) => {
    if (!point) return;
    ctx.save();
    ctx.globalAlpha = alpha;
    const gradient = ctx.createRadialGradient(point.x, point.y, 0, point.x, point.y, radius);
    gradient.addColorStop(0, color);
    gradient.addColorStop(0.2, color);
    gradient.addColorStop(1, 'transparent');
    ctx.fillStyle = gradient;
    ctx.beginPath();
    ctx.arc(point.x, point.y, radius, 0, TAU);
    ctx.fill();
    ctx.restore();
  };

  const pulse = (points, t, color, size = 2, trail = 12) => {
    if (!points.length) return;
    for (let i = trail; i >= 0; i -= 1) {
      const p = sample(points, t - i * 0.006);
      if (!p) continue;
      const k = 1 - i / (trail + 1);
      ctx.save();
      ctx.globalAlpha = k * 0.65;
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(p.x, p.y, size * k + 0.4, 0, TAU);
      ctx.fill();
      ctx.restore();
    }
    const head = sample(points, t);
    if (head) glow(head, color, size * 6, 0.55);
  };

  const path = (...points) => points.filter(Boolean);

  const node = (rect, color) => {
    if (!rect) return null;
    glow({ x: rect.cx, y: rect.cy }, color, 18, 0.2);
    ctx.save();
    ctx.fillStyle = color;
    ctx.globalAlpha = 0.65;
    ctx.beginPath();
    ctx.arc(rect.cx, rect.cy, 2, 0, TAU);
    ctx.fill();
    ctx.restore();
    return { x: rect.cx, y: rect.cy };
  };

  // 01 — energy routes through the page structure and real navigation controls.
  const drawCircuit = (env, time) => {
    const { brand, nav, download, title, preview, sections, footer } = env.a;
    if (!brand || !download || !title || !preview) return;
    const trunk = path(
      { x: brand.cx, y: brand.cy },
      { x: nav?.cx || brand.cx + 90, y: brand.cy },
      { x: download.cx, y: download.cy },
      { x: title.cx, y: title.cy },
      { x: preview.cx, y: preview.y + 2 },
      { x: preview.cx, y: preview.bottom },
      sections[0] ? { x: sections[0].cx, y: sections[0].y + 16 } : null,
      footer ? { x: footer.cx, y: footer.cy } : null,
    );
    line(trunk, '#9fe8d3', 1, 0.12);
    for (const point of [brand, nav, download, title, preview, footer]) {
      if (point) node(point, '#9fe8d3');
    }
    for (let i = 0; i < 3; i += 1) {
      pulse(trunk, (time * 0.08 + i / 3) % 1, i === 1 ? '#b9a1ff' : '#9fe8d3', 2.2, 18);
    }
    const navLinkRects = Array.from(document.querySelectorAll('.ob-nav a')).map((el) => el.getBoundingClientRect()).filter((r) => r.width);
    for (const r of navLinkRects) {
      const branch = [{ x: nav?.cx || brand.cx, y: brand.cy }, { x: r.left + r.width / 2, y: r.top + r.height / 2 }];
      line(branch, '#9fe8d3', 1, 0.09);
      pulse(branch, (time * 0.16 + r.left * 0.001) % 1, '#9fe8d3', 1.2, 8);
    }
  };

  // 02 — energy moves down the workspace sidebar and branches into file rows.
  const drawSidebar = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const x = p.x + p.w * 0.085;
    const top = p.y + p.h * 0.18;
    const bottom = p.y + p.h * 0.86;
    const conduit = [{ x, y: top }, { x, y: bottom }];
    line(conduit, '#6ee7b7', 1.2, 0.22);
    const rows = [0.34, 0.45, 0.56, 0.67, 0.78];
    rows.forEach((f, i) => {
      const y = p.y + p.h * f;
      const branch = [{ x, y }, { x: p.x + p.w * (0.18 + i * 0.012), y }];
      line(branch, '#6ee7b7', 1, 0.14);
      pulse(branch, (time * 0.3 + i * 0.17) % 1, '#6ee7b7', 1.4, 8);
      if (i === 3) glow({ x: p.x + p.w * 0.19, y }, '#b8ffe2', 12, 0.35);
    });
    for (let i = 0; i < 3; i += 1) pulse(conduit, (time * 0.2 + i / 3) % 1, '#6ee7b7', 1.8, 16);
  };

  // 03 — a scanline crosses the editor and collects at its active tab.
  const drawScan = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const x = p.x + p.w * 0.19;
    const y = p.y + p.h * 0.1;
    const w = p.w * 0.53;
    const h = p.h * 0.78;
    const progress = (time * 0.16) % 1;
    const scanY = y + h * progress;
    const gradient = ctx.createLinearGradient(x, scanY, x + w, scanY);
    gradient.addColorStop(0, 'transparent');
    gradient.addColorStop(0.2, '#6bd6ff');
    gradient.addColorStop(0.5, '#ffffff');
    gradient.addColorStop(0.8, '#6bd6ff');
    gradient.addColorStop(1, 'transparent');
    ctx.save();
    ctx.globalAlpha = 0.55;
    ctx.strokeStyle = gradient;
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(x, scanY);
    ctx.lineTo(x + w, scanY);
    ctx.stroke();
    ctx.restore();
    for (let i = 0; i < 24; i += 1) {
      const px = x + hash(i * 2.3) * w;
      const py = scanY + (hash(i * 4.1) - 0.5) * 18;
      const alpha = 0.15 + 0.5 * hash(i * 5.7);
      glow({ x: px, y: py }, '#bdeeff', 2.2, alpha);
    }
    const tab = { x: p.x + p.w * 0.32, y: p.y + p.h * 0.035 };
    glow(tab, '#6bd6ff', 14, 0.3);
    pulse([{ x, y: scanY }, { x: tab.x, y: tab.y }], (time * 0.3) % 1, '#6bd6ff', 1.6, 10);
  };

  // 04 — particles climb the agent conversation from the prompt upward.
  const drawThread = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const x = p.x + p.w * 0.84;
    const points = [];
    for (let i = 0; i <= 20; i += 1) {
      const k = i / 20;
      points.push({
        x: x + Math.sin(k * Math.PI * 3 + time * 0.4) * p.w * 0.012,
        y: p.y + p.h * (0.9 - k * 0.72),
      });
    }
    line(points, '#f0a6ff', 1, 0.15);
    for (const f of [0.24, 0.4, 0.56, 0.72]) glow({ x, y: p.y + p.h * f }, '#f0a6ff', 16, 0.16);
    for (let i = 0; i < 4; i += 1) pulse(points, (time * 0.11 + i / 4) % 1, i % 2 ? '#f0a6ff' : '#8fb8ff', 1.6, 18);
    const prompt = { x: p.x + p.w * 0.84, y: p.y + p.h * 0.9 };
    glow(prompt, '#ffffff', 12, 0.3);
  };

  // 05 — energy leaves the changes rail and runs across changed-file rows.
  const drawDiff = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const rail = { x: p.x + p.w * 0.075, y: p.y + p.h * 0.52 };
    const rows = [0.4, 0.5, 0.6, 0.7];
    rows.forEach((f, i) => {
      const end = { x: p.x + p.w * (0.22 + i * 0.035), y: p.y + p.h * f };
      const pathPoints = [rail, { x: p.x + p.w * 0.13, y: end.y }, end];
      line(pathPoints, i % 2 ? '#ffd36b' : '#9fe8d3', 1, 0.13);
      pulse(pathPoints, (time * 0.22 + i * 0.2) % 1, i % 2 ? '#ffd36b' : '#9fe8d3', 1.5, 9);
      for (let d = 0; d < 3; d += 1) {
        const t = (time * 0.28 + d * 0.12 + i * 0.1) % 1;
        const drop = { x: end.x + 8 + d * 5, y: end.y + t * 34 };
        glow(drop, i % 2 ? '#ffd36b' : '#9fe8d3', 1.8, (1 - t) * 0.55);
      }
    });
    glow(rail, '#ffd36b', 18, 0.22);
  };

  // 06 — a falling stream feeds the terminal prompt and splashes upward.
  const drawTerminal = (env, time) => {
    const p = env.a.preview;
    const title = env.a.title;
    if (!p || !title) return;
    const target = { x: p.x + p.w * 0.28, y: p.y + p.h * 0.91 };
    const source = { x: title.cx, y: title.bottom };
    const stream = [];
    for (let i = 0; i <= 16; i += 1) {
      const k = i / 16;
      stream.push({ x: lerp(source.x, target.x, k) + Math.sin(k * 14 + time * 2.2) * 8 * (1 - k), y: lerp(source.y, target.y, k) });
    }
    line(stream, '#6ee7b7', 1.2, 0.18);
    for (let i = 0; i < 4; i += 1) pulse(stream, (time * 0.55 + i / 4) % 1, '#6ee7b7', 1.6, 12);
    ctx.save();
    ctx.globalAlpha = 0.16;
    ctx.strokeStyle = '#6ee7b7';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.arc(target.x, target.y, 30 + (time * 30) % 30, 0, TAU);
    ctx.stroke();
    ctx.restore();
    for (let i = 0; i < 6; i += 1) {
      const k = (time * 0.5 + i / 6) % 1;
      glow({ x: target.x + (hash(i) - 0.5) * 80, y: target.y - k * 70 }, '#b8ffe2', 2, (1 - k) * 0.4);
    }
  };

  // 07 — the app window itself becomes a continuous perimeter channel.
  const drawHalo = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const inset = 3;
    const radius = 14;
    ctx.save();
    ctx.globalAlpha = 0.3;
    ctx.strokeStyle = '#8fb8ff';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(p.x + inset, p.y + inset, p.w - inset * 2, p.h - inset * 2, radius);
    ctx.stroke();
    ctx.restore();
    ctx.save();
    ctx.globalAlpha = 0.8;
    ctx.strokeStyle = '#cfe4ff';
    ctx.lineWidth = 1.8;
    ctx.setLineDash([18, 150]);
    ctx.lineDashOffset = -time * 65;
    ctx.beginPath();
    ctx.roundRect(p.x + inset, p.y + inset, p.w - inset * 2, p.h - inset * 2, radius);
    ctx.stroke();
    ctx.restore();
    for (let i = 0; i < 3; i += 1) {
      const t = (time * 0.06 + i / 3) % 1;
      const perimeter = 2 * (p.w + p.h);
      const d = t * perimeter;
      let x = p.x;
      let y = p.y;
      if (d < p.w) x = p.x + d;
      else if (d < p.w + p.h) { x = p.right; y = p.y + (d - p.w); }
      else if (d < p.w * 2 + p.h) { x = p.right - (d - p.w - p.h); y = p.bottom; }
      else { x = p.x; y = p.bottom - (d - p.w * 2 - p.h); }
      glow({ x, y }, '#cfe4ff', 15, 0.4);
    }
  };

  // 08 — the real prompt box becomes a channel; suggestions branch upward.
  const drawPrompt = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const input = { x: p.x + p.w * 0.83, y: p.y + p.h * 0.91 };
    const source = env.a.title;
    if (source) {
      const pathPoints = [{ x: source.cx, y: source.bottom }, { x: input.x, y: input.y }];
      line(pathPoints, '#8fb8ff', 1, 0.1);
      pulse(pathPoints, (time * 0.3) % 1, '#8fb8ff', 1.4, 14);
    }
    glow(input, '#8fb8ff', 20, 0.3);
    const chips = [
      { x: p.x + p.w * 0.73, y: p.y + p.h * 0.76 },
      { x: p.x + p.w * 0.83, y: p.y + p.h * 0.76 },
      { x: p.x + p.w * 0.93, y: p.y + p.h * 0.76 },
    ];
    chips.forEach((chip, i) => {
      const branch = [input, chip];
      line(branch, '#8fb8ff', 1, 0.1);
      pulse(branch, (time * 0.4 + i * 0.23) % 1, '#bdeeff', 1.2, 8);
      glow(chip, '#8fb8ff', 9, 0.2);
    });
  };

  // 09 — a bridge of energy connects the editor tab to the agent tab.
  const drawBridge = (env, time) => {
    const p = env.a.preview;
    if (!p) return;
    const left = { x: p.x + p.w * 0.3, y: p.y + p.h * 0.055 };
    const right = { x: p.x + p.w * 0.78, y: p.y + p.h * 0.055 };
    const mid = { x: (left.x + right.x) / 2, y: p.y - 14 };
    const arc = [];
    for (let i = 0; i <= 20; i += 1) {
      const t = i / 20;
      const x = lerp(left.x, right.x, t);
      const y = lerp(left.y, right.y, t) - Math.sin(t * Math.PI) * 28;
      arc.push({ x, y });
    }
    line(arc, '#ffd36b', 1.2, 0.14);
    pulse(arc, (time * 0.5) % 1, '#ffd36b', 1.5, 12);
    pulse(arc.slice().reverse(), (time * 0.5 + 0.5) % 1, '#6bd6ff', 1.5, 12);
    glow(left, '#ffd36b', 10, 0.3);
    glow(right, '#6bd6ff', 10, 0.3);
    line([left, { x: left.x, y: left.y + 30 }], '#ffd36b', 1, 0.08);
    line([right, { x: right.x, y: right.y + 30 }], '#6bd6ff', 1, 0.08);
    glow(mid, '#ffffff', 8, 0.15);
  };

  // 10 — light erupts from the real logo and falls in a fountain over the hero.
  const drawFountain = (env, time) => {
    const logo = env.a.logo || env.a.brand;
    const title = env.a.title;
    const p = env.a.preview;
    if (!logo || !title || !p) return;
    const source = { x: logo.cx, y: logo.cy };
    for (let i = 0; i < 12; i += 1) {
      const phase = (time * 0.35 + i / 12) % 1;
      const spread = (hash(i) - 0.5) * 0.7;
      const x = lerp(source.x, title.cx + spread * title.w * 0.35, phase);
      const y = lerp(source.y, title.cy, phase) - Math.sin(phase * Math.PI) * (40 + hash(i + 3) * 80);
      glow({ x, y }, i % 3 === 0 ? '#b9a1ff' : '#9fe8d3', 2.2, (1 - phase) * 0.45);
    }
    line([source, { x: title.cx, y: title.cy }, { x: p.cx, y: p.y }], '#9fe8d3', 1, 0.08);
    for (let i = 0; i < 3; i += 1) {
      const t = (time * 0.1 + i / 3) % 1;
      const p1 = { x: source.x, y: source.cy };
      const p2 = sample([source, { x: title.cx, y: title.cy }, { x: p.cx, y: p.y }], t);
      if (p2) pulse([p1, p2], t, '#9fe8d3', 1.5, 10);
    }
    glow({ x: p.cx, y: p.y }, '#9fe8d3', 20, 0.2);
  };

  const CONCEPTS = [
    { slug: 'circuit-rail', name: 'Circuit Rail', line: 'Energy routes through the real navigation, actions and page sections.', draw: drawCircuit },
    { slug: 'sidebar-conduit', name: 'Sidebar Conduit', line: 'A vertical channel feeds the file rows in the real workspace preview.', draw: drawSidebar },
    { slug: 'editor-scan', name: 'Editor Sweep', line: 'A scanline crosses the editor and collects at the real active tab.', draw: drawScan },
    { slug: 'agent-thread', name: 'Agent Thread', line: 'Photons climb the conversation from the prompt through the message stack.', draw: drawThread },
    { slug: 'diff-runoff', name: 'Diff Runoff', line: 'Energy leaves the changes rail and runs across changed-file rows.', draw: drawDiff },
    { slug: 'terminal-pour', name: 'Terminal Pour', line: 'The beam becomes a falling feed that pours into the terminal prompt.', draw: drawTerminal },
    { slug: 'window-halo', name: 'Window Halo', line: 'The app window itself becomes a continuous perimeter channel.', draw: drawHalo },
    { slug: 'prompt-channel', name: 'Prompt Channel', line: 'Energy channels into the real prompt box and branches into suggestions.', draw: drawPrompt },
    { slug: 'tab-bridge', name: 'Tab Bridge', line: 'An arc of energy connects the editor tab to the agent tab.', draw: drawBridge },
    { slug: 'logo-fountain', name: 'Logo Fountain', line: 'Light erupts from the real logo and falls over the hero into the preview.', draw: drawFountain },
  ];

  select.innerHTML = CONCEPTS.map((concept, i) => `<option value="${concept.slug}">${String(i + 1).padStart(2, '0')} — ${concept.name}</option>`).join('');
  let current = Math.max(0, CONCEPTS.findIndex((concept) => concept.slug === params.get('concept')));
  let start = 0;
  let raf = 0;

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
    const concept = CONCEPTS[current];
    const time = (now - start) / 1000;
    resizeIfNeeded();
    ctx.clearRect(0, 0, innerWidth, innerHeight);
    ctx.save();
    ctx.globalCompositeOperation = 'lighter';
    concept.draw({ a: anchors(), w: innerWidth, h: innerHeight }, time);
    ctx.restore();
    raf = requestAnimationFrame(frame);
  };

  let lastWidth = 0;
  let lastHeight = 0;
  const resizeIfNeeded = () => {
    if (innerWidth !== lastWidth || innerHeight !== lastHeight) {
      lastWidth = innerWidth;
      lastHeight = innerHeight;
      resize();
    }
  };

  prev.addEventListener('click', () => render(current - 1));
  next.addEventListener('click', () => render(current + 1));
  select.addEventListener('change', () => {
    const target = CONCEPTS.findIndex((concept) => concept.slug === select.value);
    if (target >= 0) render(target);
  });
  window.addEventListener('keydown', (event) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (/^(INPUT|SELECT|TEXTAREA|BUTTON|A|OPTION)$/.test(event.target.tagName)) return;
    if (event.key === 'ArrowLeft') { event.preventDefault(); render(current - 1); }
    if (event.key === 'ArrowRight') { event.preventDefault(); render(current + 1); }
  });
  window.addEventListener('popstate', () => {
    const target = CONCEPTS.findIndex((concept) => concept.slug === new URLSearchParams(location.search).get('concept'));
    if (target >= 0) render(target, false);
  });

  render(current, false);
  raf = requestAnimationFrame(frame);
})();
