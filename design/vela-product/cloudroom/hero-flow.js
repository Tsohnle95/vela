(() => {
  const TONES = {
    mint: { core: '#ffffff', light: '#eafff7', mid: '#6ee7b7', deep: '#0f9f7a', alt: '#38bdf8' },
    ribbon: { core: '#ffffff', light: '#e0ffff', mid: '#67e8f9', deep: '#0e7490', alt: '#8b5cf6' },
    beam: { core: '#ffffff', light: '#eafff7', mid: '#b3f6d6', deep: '#0f9f7a', alt: '#7dd3fc' },
    aurora: { core: '#ffffff', light: '#ede9fe', mid: '#a78bfa', deep: '#6d28d9', alt: '#2dd4bf' },
    amber: { core: '#fffaf0', light: '#ffedd5', mid: '#fdba74', deep: '#c2410c', alt: '#fb7185' },
    lime: { core: '#f7ffe0', light: '#ecfccb', mid: '#c6ff3d', deep: '#4d7c0f', alt: '#9fd8ff' },
  };

  let active = null;

  const rgba = (hex, alpha) => {
    const value = Number.parseInt(hex.slice(1), 16);
    return `rgba(${(value >> 16) & 255}, ${(value >> 8) & 255}, ${value & 255}, ${alpha})`;
  };

  const random = (min, max) => min + Math.random() * (max - min);

  const makeParticles = (mode) => {
    const count = mode === 'beam' ? 78 : mode === 'pulse' ? 54 : mode === 'grid' ? 28 : mode === 'ribbon' ? 56 : 34;
    return Array.from({ length: count }, () => ({
      x: random(-1, 1),
      y: random(0, 1),
      speed: random(0.012, 0.05),
      phase: random(0, Math.PI * 2),
      size: random(0.8, 2.4),
    }));
  };

  const destroy = () => {
    if (!active) return;
    cancelAnimationFrame(active.raf);
    active.ro?.disconnect();
    active = null;
  };

  const mount = (root) => {
    destroy();
    const canvas = root.querySelector('.ob-flow-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const host = canvas.parentElement;
    const mode = canvas.dataset.flow || 'ribbon';
    const tone = TONES[canvas.dataset.tone] || TONES.mint;
    const cxRatio = Math.max(0.1, Math.min(0.9, Number(canvas.dataset.x || 64) / 100));
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const particles = makeParticles(mode);
    const start = performance.now();
    const state = { raf: 0, ro: null, gone: false };
    let width = 1;
    let height = 1;

    const resize = () => {
      const rect = host.getBoundingClientRect();
      width = Math.max(1, rect.width);
      height = Math.max(1, rect.height);
      canvas.width = Math.round(width * dpr);
      canvas.height = Math.round(height * dpr);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      if (reduced) draw(0);
    };

    const radial = (cx, cy, radius, inner, outer) => {
      const gradient = ctx.createRadialGradient(cx, cy, 0, cx, cy, radius);
      gradient.addColorStop(0, inner);
      gradient.addColorStop(0.55, outer);
      gradient.addColorStop(1, rgba(tone.deep, 0));
      return gradient;
    };

    const drawRibbon = (time) => {
      const cx = width * cxRatio;
      ctx.fillStyle = radial(cx, height * 0.28, Math.max(width, height) * 0.7, rgba(tone.mid, 0.2), rgba(tone.deep, 0.08));
      ctx.fillRect(0, 0, width, height);
      ctx.globalCompositeOperation = 'lighter';

      const layers = 8;
      for (let i = 0; i < layers; i += 1) {
        const offset = (i - (layers - 1) / 2) * 14;
        const amplitude = 18 + i * 5;
        const phase = time * 0.55 + i * 0.8;
        const vertical = ctx.createLinearGradient(0, 0, 0, height);
        vertical.addColorStop(0, rgba(tone.mid, 0));
        vertical.addColorStop(0.22, rgba(tone.alt, 0.09));
        vertical.addColorStop(0.52, rgba(tone.mid, 0.28));
        vertical.addColorStop(0.72, rgba(tone.light, 0.36));
        vertical.addColorStop(1, rgba(tone.mid, 0));
        ctx.beginPath();
        for (let y = -40; y <= height + 40; y += 16) {
          const x = cx + offset + Math.sin(y * 0.005 + phase) * amplitude + Math.sin(y * 0.011 - phase * 0.7) * amplitude * 0.38;
          if (y === -40) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = vertical;
        ctx.lineWidth = 13 + (i % 3) * 9;
        ctx.globalAlpha = 0.9;
        ctx.stroke();
      }

      ctx.globalAlpha = 1;
      ctx.beginPath();
      for (let y = -40; y <= height + 40; y += 16) {
        const x = cx + Math.sin(y * 0.006 + time * 0.8) * 8;
        if (y === -40) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = rgba(tone.light, 0.9);
      ctx.lineWidth = 2.5;
      ctx.shadowColor = rgba(tone.mid, 0.95);
      ctx.shadowBlur = 30;
      ctx.stroke();
      ctx.shadowBlur = 0;

      const flareY = height * 0.62;
      ctx.save();
      ctx.translate(cx, flareY);
      ctx.scale(1, 0.27);
      const flare = ctx.createRadialGradient(0, 0, 0, 0, 0, 280);
      flare.addColorStop(0, rgba(tone.light, 0.34));
      flare.addColorStop(0.24, rgba(tone.mid, 0.22));
      flare.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = flare;
      ctx.beginPath();
      ctx.arc(0, 0, 280, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();

      for (const particle of particles) {
        const y = ((particle.y + time * particle.speed) % 1) * height;
        const x = cx + (particle.x - 0.5) * 230 + Math.sin(time * 0.9 + particle.phase) * 12;
        const alpha = 0.2 + 0.4 * Math.max(0, Math.sin(particle.phase + time * 1.1));
        ctx.fillStyle = rgba(tone.light, alpha);
        ctx.beginPath();
        ctx.arc(x, y, particle.size, 0, Math.PI * 2);
        ctx.fill();
      }
    };

    const drawBeam = (time) => {
      const cx = width * cxRatio;
      ctx.fillStyle = radial(cx, height * 0.18, Math.max(width, height) * 0.68, rgba(tone.mid, 0.18), rgba(tone.deep, 0.06));
      ctx.fillRect(0, 0, width, height);
      ctx.globalCompositeOperation = 'lighter';

      const column = ctx.createLinearGradient(0, 0, 0, height);
      column.addColorStop(0, rgba(tone.mid, 0));
      column.addColorStop(0.12, rgba(tone.mid, 0.06));
      column.addColorStop(0.5, rgba(tone.light, 0.18));
      column.addColorStop(0.82, rgba(tone.mid, 0.08));
      column.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = column;
      ctx.fillRect(cx - 150, 0, 300, height);

      const inner = ctx.createLinearGradient(0, 0, 0, height);
      inner.addColorStop(0, rgba(tone.light, 0));
      inner.addColorStop(0.2, rgba(tone.light, 0.16));
      inner.addColorStop(0.58, rgba(tone.light, 0.3));
      inner.addColorStop(0.85, rgba(tone.light, 0.1));
      inner.addColorStop(1, rgba(tone.light, 0));
      ctx.fillStyle = inner;
      ctx.fillRect(cx - 34, 0, 68, height);

      const core = ctx.createLinearGradient(0, 0, 0, height);
      core.addColorStop(0, rgba(tone.core, 0));
      core.addColorStop(0.14, rgba(tone.core, 0.55));
      core.addColorStop(0.52, rgba(tone.core, 0.95));
      core.addColorStop(0.8, rgba(tone.light, 0.55));
      core.addColorStop(1, rgba(tone.core, 0));
      ctx.fillStyle = core;
      ctx.globalAlpha = 0.8 + Math.sin(time * 2.1) * 0.12;
      ctx.fillRect(cx - 1.5, 0, 3, height);
      ctx.globalAlpha = 1;

      const flareY = height * 0.6;
      const flareScale = 1 + Math.sin(time * 1.25) * 0.06;
      ctx.save();
      ctx.translate(cx, flareY);
      ctx.scale(flareScale, 0.3 * flareScale);
      const flare = ctx.createRadialGradient(0, 0, 0, 0, 0, 300);
      flare.addColorStop(0, rgba(tone.core, 0.5));
      flare.addColorStop(0.18, rgba(tone.mid, 0.34));
      flare.addColorStop(0.55, rgba(tone.mid, 0.1));
      flare.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = flare;
      ctx.beginPath();
      ctx.arc(0, 0, 300, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();

      for (const particle of particles) {
        const y = (((particle.y - time * particle.speed) % 1) + 1) % 1 * height;
        const x = cx + particle.x * 145 + Math.sin(time * 0.7 + particle.phase) * 11;
        const alpha = 0.12 + 0.42 * Math.max(0, Math.sin(particle.phase + time * 1.25));
        ctx.fillStyle = rgba(tone.light, alpha);
        ctx.beginPath();
        ctx.arc(x, y, particle.size, 0, Math.PI * 2);
        ctx.fill();
      }
    };

    const drawCurtain = (time) => {
      const cx = width * cxRatio;
      ctx.fillStyle = radial(width * 0.55, height * 0.12, Math.max(width, height) * 0.72, rgba(tone.mid, 0.16), rgba(tone.deep, 0.06));
      ctx.fillRect(0, 0, width, height);
      ctx.globalCompositeOperation = 'lighter';
      ctx.lineCap = 'round';

      for (let i = 0; i < 4; i += 1) {
        const base = height * (0.12 + i * 0.06) + Math.sin(time * 0.28 + i * 1.1) * 20;
        const horizontal = ctx.createLinearGradient(0, 0, width, 0);
        horizontal.addColorStop(0, rgba(tone.deep, 0));
        horizontal.addColorStop(0.18, rgba(tone.alt, 0.07));
        horizontal.addColorStop(0.5, rgba(tone.mid, 0.18));
        horizontal.addColorStop(0.82, rgba(tone.alt, 0.07));
        horizontal.addColorStop(1, rgba(tone.deep, 0));
        ctx.beginPath();
        for (let x = -40; x <= width + 40; x += 20) {
          const y = base + Math.sin(x * 0.004 + time * 0.5 + i * 1.2) * 40 + Math.sin(x * 0.009 - time * 0.32 + i) * 18;
          if (x === -40) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = horizontal;
        ctx.lineWidth = 62 + i * 22;
        ctx.globalAlpha = 0.8;
        ctx.stroke();
        ctx.lineWidth = 2;
        ctx.globalAlpha = 0.55;
        ctx.strokeStyle = rgba(tone.light, 0.18);
        ctx.stroke();
      }

      ctx.globalAlpha = 1;
      const column = ctx.createLinearGradient(0, 0, 0, height);
      column.addColorStop(0, rgba(tone.light, 0.2));
      column.addColorStop(0.55, rgba(tone.mid, 0.08));
      column.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = column;
      ctx.fillRect(cx - 26, 0, 52, height);

      for (const particle of particles) {
        const x = ((particle.x + 1) / 2 + time * particle.speed * 0.7) % 1 * width;
        const y = ((particle.y + time * particle.speed * 0.3) % 1) * height;
        const alpha = 0.1 + 0.32 * Math.max(0, Math.sin(particle.phase + time * 1.1));
        ctx.fillStyle = rgba(tone.light, alpha);
        ctx.beginPath();
        ctx.arc(x, y, particle.size, 0, Math.PI * 2);
        ctx.fill();
      }
    };

    const drawPulse = (time) => {
      const cx = width * cxRatio;
      ctx.fillStyle = radial(cx, height * 0.28, Math.max(width, height) * 0.7, rgba(tone.mid, 0.22), rgba(tone.deep, 0.08));
      ctx.fillRect(0, 0, width, height);
      ctx.globalCompositeOperation = 'lighter';

      const column = ctx.createLinearGradient(0, 0, 0, height);
      column.addColorStop(0, rgba(tone.mid, 0));
      column.addColorStop(0.2, rgba(tone.mid, 0.1));
      column.addColorStop(0.55, rgba(tone.light, 0.22));
      column.addColorStop(0.88, rgba(tone.mid, 0.08));
      column.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = column;
      ctx.fillRect(cx - 110, 0, 220, height);

      const core = ctx.createLinearGradient(0, 0, 0, height);
      core.addColorStop(0, rgba(tone.core, 0));
      core.addColorStop(0.2, rgba(tone.core, 0.45));
      core.addColorStop(0.65, rgba(tone.core, 0.9));
      core.addColorStop(1, rgba(tone.core, 0));
      ctx.fillStyle = core;
      ctx.fillRect(cx - 1.4, 0, 2.8, height);

      for (let i = 0; i < 5; i += 1) {
        const phase = (time * 0.14 + i / 5) % 1;
        const y = height * (0.03 + phase * 0.94);
        const fade = Math.sin(phase * Math.PI);
        const radius = 150 + 75 * Math.sin(time * 0.8 + i);
        ctx.save();
        ctx.translate(cx, y);
        ctx.scale(1, 0.28);
        const glow = ctx.createRadialGradient(0, 0, 0, 0, 0, radius);
        glow.addColorStop(0, rgba(tone.core, 0.5 * fade));
        glow.addColorStop(0.22, rgba(tone.mid, 0.36 * fade));
        glow.addColorStop(0.62, rgba(tone.mid, 0.1 * fade));
        glow.addColorStop(1, rgba(tone.mid, 0));
        ctx.fillStyle = glow;
        ctx.beginPath();
        ctx.arc(0, 0, radius, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();

        ctx.beginPath();
        ctx.ellipse(cx, y, radius * 0.72, 20, 0, 0, Math.PI * 2);
        ctx.strokeStyle = rgba(tone.light, 0.34 * fade);
        ctx.lineWidth = 1.6;
        ctx.stroke();

        ctx.beginPath();
        ctx.ellipse(cx, y, radius * 0.38, 10, 0, 0, Math.PI * 2);
        ctx.strokeStyle = rgba(tone.core, 0.26 * fade);
        ctx.lineWidth = 1;
        ctx.stroke();
      }

      const flareY = height * 0.68;
      ctx.save();
      ctx.translate(cx, flareY);
      ctx.scale(1, 0.28);
      const flare = ctx.createRadialGradient(0, 0, 0, 0, 0, 230);
      flare.addColorStop(0, rgba(tone.core, 0.3));
      flare.addColorStop(0.3, rgba(tone.mid, 0.2));
      flare.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = flare;
      ctx.beginPath();
      ctx.arc(0, 0, 230, 0, Math.PI * 2);
      ctx.fill();
      ctx.restore();

      for (const particle of particles) {
        const y = (((particle.y - time * particle.speed * 0.7) % 1) + 1) % 1 * height;
        const x = cx + particle.x * 120 + Math.sin(time * 0.6 + particle.phase) * 10;
        const alpha = 0.12 + 0.4 * Math.max(0, Math.sin(particle.phase + time * 1.2));
        ctx.fillStyle = rgba(tone.light, alpha);
        ctx.beginPath();
        ctx.arc(x, y, particle.size, 0, Math.PI * 2);
        ctx.fill();
      }
    };

    const drawGrid = (time) => {
      const cx = width * cxRatio;
      ctx.fillStyle = radial(cx, height * 0.35, Math.max(width, height) * 0.7, rgba(tone.mid, 0.1), rgba(tone.deep, 0.04));
      ctx.fillRect(0, 0, width, height);
      ctx.globalCompositeOperation = 'lighter';

      ctx.strokeStyle = rgba(tone.mid, 0.055);
      ctx.lineWidth = 1;
      for (let i = 0; i <= 8; i += 1) {
        const y = (i / 8) * height;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
      }
      for (let i = 0; i <= 12; i += 1) {
        const x = (i / 12) * width;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
      }

      for (let i = 0; i < 7; i += 1) {
        const y = height * (0.11 + i * 0.125) + Math.sin(time * 0.25 + i) * 8;
        const x = (((time * 0.1 + i * 0.17) % 1.2) * (width + 600)) - 300;
        const move = ctx.createLinearGradient(x - 260, 0, x + 260, 0);
        move.addColorStop(0, rgba(tone.light, 0));
        move.addColorStop(0.5, rgba(tone.mid, 0.34));
        move.addColorStop(1, rgba(tone.light, 0));
        ctx.strokeStyle = move;
        ctx.lineWidth = 1.4;
        ctx.beginPath();
        ctx.moveTo(x - 260, y);
        ctx.lineTo(x + 260, y);
        ctx.stroke();
      }

      for (let i = 0; i < 6; i += 1) {
        const x = width * (0.08 + i * 0.165) + Math.sin(time * 0.2 + i) * 10;
        const y = (((time * 0.08 + i * 0.19) % 1.2) * (height + 600)) - 300;
        const move = ctx.createLinearGradient(0, y - 260, 0, y + 260);
        move.addColorStop(0, rgba(tone.light, 0));
        move.addColorStop(0.5, rgba(tone.mid, 0.26));
        move.addColorStop(1, rgba(tone.light, 0));
        ctx.strokeStyle = move;
        ctx.lineWidth = 1.2;
        ctx.beginPath();
        ctx.moveTo(x, y - 260);
        ctx.lineTo(x, y + 260);
        ctx.stroke();
      }

      const column = ctx.createLinearGradient(0, 0, 0, height);
      column.addColorStop(0, rgba(tone.mid, 0));
      column.addColorStop(0.5, rgba(tone.mid, 0.07));
      column.addColorStop(1, rgba(tone.mid, 0));
      ctx.fillStyle = column;
      ctx.fillRect(cx - 45, 0, 90, height);

      ctx.beginPath();
      for (let x = 0; x <= width; x += 18) {
        const y = height * 0.46 + Math.sin(x * 0.006 + time * 1.1) * 28;
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = rgba(tone.light, 0.18);
      ctx.lineWidth = 1;
      ctx.stroke();

      for (let i = 0; i < 34; i += 1) {
        const x = ((i * 97) % 1000) / 1000 * width;
        const y = ((i * 173) % 1000) / 1000 * height;
        const alpha = 0.06 + 0.4 * Math.max(0, Math.sin(time * 1.1 + i * 0.9));
        ctx.fillStyle = rgba(tone.light, alpha);
        ctx.fillRect(x, y, 2, 2);
      }
    };

    const draw = (time) => {
      if (state.gone) return;
      ctx.clearRect(0, 0, width, height);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = 'source-over';
      if (mode === 'beam') drawBeam(time);
      else if (mode === 'curtain') drawCurtain(time);
      else if (mode === 'pulse') drawPulse(time);
      else if (mode === 'grid') drawGrid(time);
      else drawRibbon(time);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = 'source-over';
    };

    const frame = (now) => {
      if (state.gone) return;
      draw((now - start) / 1000);
      state.raf = requestAnimationFrame(frame);
    };

    state.ro = new ResizeObserver(resize);
    state.ro.observe(host);
    resize();
    if (reduced) draw(0);
    else state.raf = requestAnimationFrame(frame);
    active = state;
  };

  window.velaHeroFlow = { mount, destroy };
})();
