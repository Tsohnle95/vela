(() => {
  const site = document.getElementById('site');
  site.innerHTML = window.velaMeet.build({ effect: 'nebulaStorm', beam: false });
  const galaxy = site.querySelector('.ob-effect-canvas');
  window.VelaLab.mount(galaxy, 'nebulaStorm');

  const canvas = document.getElementById('storm-canvas');
  const ctx = canvas.getContext('2d');
  const TAU = Math.PI * 2;
  const lerp = (a, b, t) => a + (b - a) * t;
  const hash = (n) => {
    const v = Math.sin(n * 127.1 + 311.7) * 43758.5453;
    return v - Math.floor(v);
  };

  const cells = [
    { x: 0.2, y: 0.22, radius: 0.15, period: 7.4, offset: 1.2, seed: 3 },
    { x: 0.8, y: 0.25, radius: 0.16, period: 9.6, offset: 4.6, seed: 11 },
  ];

  let width = 0;
  let height = 0;
  let dpr = 1;

  const resize = () => {
    dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    width = window.innerWidth;
    height = window.innerHeight;
    canvas.width = Math.max(1, Math.round(width * dpr));
    canvas.height = Math.max(1, Math.round(height * dpr));
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  };

  const storm = (current, time) => {
    const age = (time + current.offset) % current.period;
    const progress = age / current.period;
    const flash = progress < 0.085 ? Math.sin((progress / 0.085) * Math.PI) : 0;
    if (flash <= 0.015) return;

    const cycle = Math.floor((time + current.offset) / current.period) + current.seed * 37;
    const rnd = (n) => hash(cycle * 3.1 + n);

    const radius = Math.min(width, height) * current.radius;
    const cx = width * current.x + (rnd(1) - 0.5) * radius * 0.4;
    const cy = height * current.y + (rnd(2) - 0.5) * radius * 0.3;

    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, radius * 1.05, 0, TAU);
    ctx.clip();
    ctx.globalCompositeOperation = 'lighter';

    const haze = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius * 1.2);
    haze.addColorStop(0, `rgba(198, 192, 255, ${0.22 * flash})`);
    haze.addColorStop(0.45, `rgba(138, 118, 255, ${0.09 * flash})`);
    haze.addColorStop(1, 'transparent');
    ctx.fillStyle = haze;
    ctx.beginPath();
    ctx.arc(cx, cy, radius * 1.2, 0, TAU);
    ctx.fill();

    const bolt = (startX, startY, endX, endY, seed, alpha, widthScale) => {
      const points = [];
      for (let i = 0; i <= 12; i += 1) {
        const t = i / 12;
        const jitter = (hash(seed * 7.3 + i * 13.7) - 0.5) * radius * 0.22 * (1 - t * 0.35);
        points.push({
          x: lerp(startX, endX, t) + jitter,
          y: lerp(startY, endY, t) + (hash(seed + i * 2.1) - 0.5) * 5,
        });
      }
      ctx.save();
      ctx.strokeStyle = `rgba(208, 210, 255, ${alpha * flash})`;
      ctx.lineWidth = widthScale;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.shadowColor = 'rgba(140, 140, 255, 0.9)';
      ctx.shadowBlur = 12;
      ctx.beginPath();
      ctx.moveTo(points[0].x, points[0].y);
      for (const point of points.slice(1)) ctx.lineTo(point.x, point.y);
      ctx.stroke();
      ctx.restore();
      return points;
    };

    const main = bolt(
      cx - radius * (0.3 + rnd(10) * 0.25),
      cy - radius * (0.2 + rnd(11) * 0.25),
      cx + radius * (0.3 + rnd(12) * 0.25),
      cy + radius * (0.25 + rnd(13) * 0.3),
      cycle + 1,
      0.5,
      1.1,
    );

    for (let i = 0; i < 3; i += 1) {
      const fork = main[4 + i * 2];
      if (!fork) continue;
      const dir = i % 2 ? 1 : -1;
      bolt(
        fork.x,
        fork.y,
        fork.x + dir * radius * (0.18 + rnd(20 + i) * 0.24),
        fork.y + radius * (0.08 + rnd(24 + i) * 0.24),
        cycle + 9 + i,
        0.22,
        0.65,
      );
    }

    for (let i = 0; i < 7; i += 1) {
      const angle = rnd(40 + i) * TAU;
      const distance = radius * (0.2 + rnd(50 + i) * 0.7);
      const x = cx + Math.cos(angle) * distance;
      const y = cy + Math.sin(angle) * distance;
      const spark = ctx.createRadialGradient(x, y, 0, x, y, 8);
      spark.addColorStop(0, `rgba(220, 220, 255, ${0.35 * flash})`);
      spark.addColorStop(1, 'transparent');
      ctx.fillStyle = spark;
      ctx.beginPath();
      ctx.arc(x, y, 8, 0, TAU);
      ctx.fill();
    }

    ctx.restore();
  };

  const frame = (now) => {
    if (window.innerWidth !== width || window.innerHeight !== height) resize();
    ctx.clearRect(0, 0, width, height);
    const time = now / 1000;
    for (const cell of cells) storm(cell, time);
    requestAnimationFrame(frame);
  };

  document.title = 'Vela — Nebula Storm';
  resize();
  window.addEventListener('resize', resize);
  requestAnimationFrame(frame);
})();
