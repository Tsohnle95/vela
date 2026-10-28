(() => {
const HEADER = `#version 300 es
precision highp float;
out vec4 outColor;
uniform vec2 uResolution;
uniform float uTime;
uniform vec2 uPointer;
#define PI 3.14159265359

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
    u.y
  );
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
  for (int i = 0; i < 5; i++) {
    v += a * noise(p);
    p = m * p;
    a *= 0.5;
  }
  return v;
}

float stars(vec2 p, float density, float t) {
  vec2 gv = fract(p * density) - 0.5;
  vec2 id = floor(p * density);
  float h = hash(id);
  vec2 off = (vec2(hash(id + 1.3), hash(id + 2.7)) - 0.5) * 0.7;
  float d = length(gv - off);
  float tw = 0.5 + 0.5 * sin(t * 2.0 + h * 6.283);
  return (1.0 - smoothstep(0.0, 0.03, d)) * (0.3 + 0.7 * h) * tw;
}

vec3 hsl2rgb(vec3 c) {
  vec3 rgb = clamp(abs(mod(c.x * 6.0 + vec3(0.0, 4.0, 2.0), 6.0) - 3.0) - 1.0, 0.0, 1.0);
  return c.z + c.y * (rgb - 0.5) * (1.0 - abs(2.0 * c.z - 1.0));
}
`;

  const VERTEX = `#version 300 es
in vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;

  const SHADERS = {
    aurora: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime;
  vec3 col = vec3(0.004, 0.007, 0.014);
  col += vec3(0.012, 0.035, 0.055) * (0.5 - uv.y);
  col += vec3(0.55, 0.8, 1.0) * stars(uv * 1.1 + vec2(0.0, t * 0.012), 22.0, t) * 0.5;

  for (int i = 0; i < 5; i++) {
    float fi = float(i);
    float y = -0.02 + fi * 0.075
      + 0.16 * sin(uv.x * 1.35 + t * 0.22 + fi * 1.6)
      + 0.08 * sin(uv.x * 3.2 - t * 0.14 + fi);
    float n = fbm(vec2(uv.x * 1.5 + t * 0.07 + fi * 1.7, uv.y * 0.65 - t * 0.05 + fi * 2.1));
    float ribbon = 1.0 - smoothstep(0.0, 0.15, abs(uv.y - y - 0.22 * (n - 0.5)));
    vec3 c = mix(vec3(0.1, 1.0, 0.66), vec3(0.28, 0.55, 1.0), 0.5 + 0.5 * sin(fi * 1.2 + t * 0.11));
    c = mix(c, vec3(1.0, 0.36, 0.78), 0.35 + 0.35 * sin(uv.x * 2.1 + fi));
    col += c * ribbon * 0.18;
  }

  float curtain = fbm(vec2(uv.x * 2.6 + t * 0.1, uv.y * 1.1 - t * 0.18));
  col += vec3(0.08, 0.6, 0.52)
    * (1.0 - smoothstep(0.0, 0.2, abs(uv.y - curtain * 0.35 + 0.08))) * 0.16;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.62, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    ink: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.55;
  vec2 p = uv * 2.2;
  float n1 = fbm(p + vec2(t * 0.14, -t * 0.1));
  float n2 = fbm(p * 1.35 + vec2(4.1, 2.3) + n1 * 2.5 + vec2(-t * 0.08, t * 0.12));
  float n3 = fbm(p * 1.8 + vec2(2.7, 6.1) + n2 * 3.0 + vec2(t * 0.05, -t * 0.06));
  vec3 col = vec3(0.004, 0.008, 0.012);
  float f = 0.5 + 0.5 * sin(n3 * 4.0 + uv.y * 2.0 + t * 0.6);
  col = mix(col, vec3(0.02, 0.3, 0.28), smoothstep(0.2, 0.8, n2));
  col = mix(col, vec3(0.06, 0.85, 0.67), smoothstep(0.45, 0.82, n3));
  col = mix(col, vec3(0.5, 0.32, 0.95), smoothstep(0.55, 0.92, n1 * n3));
  col = mix(col, vec3(1.0, 0.55, 0.35), smoothstep(0.7, 0.98, f) * 0.5);
  col += vec3(0.8, 0.95, 0.9) * pow(max(0.0, n3), 6.0) * 0.55;
  float band = pow(max(0.0, 1.0 - abs(sin(n2 * 8.0 + uv.x * 2.0 + t * 0.5))), 12.0);
  col += vec3(0.55, 0.9, 0.8) * band * 0.06;
  col *= 1.0 - smoothstep(0.6, 1.2, length(uv * vec2(0.58, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    nebula: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.12;
  float n1 = fbm(uv * 1.8 + vec2(t * 0.05, -t * 0.03));
  float n2 = fbm(uv * 3.0 + vec2(4.7, 1.3) - n1 * 2.0 + vec2(-t * 0.02, t * 0.04));
  float core = exp(-length(uv - vec2(0.18 + sin(t * 0.2) * 0.15, -0.05)) * 1.2);
  vec3 col = vec3(0.003, 0.005, 0.014);
  col += vec3(0.22, 0.12, 0.62) * smoothstep(0.2, 0.8, n1) * 0.8;
  col += vec3(0.78, 0.18, 0.55) * smoothstep(0.45, 0.9, n2) * 0.55;
  col += vec3(0.12, 0.68, 0.78) * smoothstep(0.55, 0.95, n1 * n2) * 0.85;
  col += vec3(0.9, 0.8, 1.0) * core * 0.7;
  col += vec3(0.7, 0.85, 1.0) * stars(uv * 1.5 + vec2(0.0, t * 0.01), 18.0, uTime) * 0.9;
  col *= 1.0 - smoothstep(0.6, 1.3, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    caustics: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.55;
  vec2 p = uv * 3.0;
  float warp = fbm(p * 1.2 + vec2(t * 0.1, -t * 0.05));
  p += vec2(warp - 0.5) * 1.15;
  float v = 0.0;
  v += abs(sin(p.x * 2.4 + sin(p.y * 1.8 + t) * 1.5 + t * 0.7));
  v += abs(sin(p.y * 3.1 + sin(p.x * 2.2 - t) * 1.4 - t * 0.5));
  float caustic = pow(1.0 - clamp(v * 0.45, 0.0, 1.0), 5.0);
  vec3 col = vec3(0.004, 0.025, 0.045);
  col += vec3(0.5, 0.9, 1.0) * pow(caustic, 3.0) * 0.55;
  col += vec3(0.08, 0.55, 0.8) * caustic * 0.55;
  col += vec3(0.015, 0.09, 0.14) * (0.6 - uv.y);
  float glint = pow(max(0.0, 1.0 - abs(sin(p.x * 5.0 + t * 1.3) * sin(p.y * 4.0 - t))), 20.0);
  col += vec3(0.3, 0.65, 1.0) * glint * 0.05;
  col *= 1.0 - smoothstep(0.6, 1.2, length(uv * vec2(0.6, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    vortex: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.45;
  float r = length(uv);
  float a = atan(uv.y, uv.x);
  vec3 col = vec3(0.003, 0.005, 0.011);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float ang = a + fi * 2.094 + t * 0.16;
    float n = fbm(vec2(ang * 1.2 + t * 0.1, r * 3.4 - t * 0.22 + fi));
    float arm = sin(ang * 3.0 + r * 16.0 - t * 1.8 + n * 4.0);
    float glow = pow(max(0.0, 1.0 - abs(arm)), 8.0) * exp(-r * 1.5);
    vec3 c = mix(vec3(0.2, 0.7, 1.0), vec3(0.8, 0.3, 1.0), 0.5 + 0.5 * sin(fi * 2.0 + r * 3.0));
    col += c * glow * 0.45;
  }
  col += vec3(0.4, 0.8, 1.0) * exp(-r * 9.0) * 0.9;
  col += vec3(0.05, 0.16, 0.3) * exp(-r * 2.0) * 0.45;
  col += vec3(0.65, 0.85, 1.0) * stars(uv * 2.0 + vec2(t * 0.01, 0.0), 14.0, uTime) * 0.6;
  col *= 1.0 - smoothstep(0.3, 1.1, r);
  outColor = vec4(col, 1.0);
}`,

    prism: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.18;
  float n = fbm(uv * 2.0 + vec2(t * 0.1, -t * 0.05));
  float bands = sin((uv.x * 0.6 + uv.y * 0.4 + n * 0.35 + t * 0.1) * 9.0);
  float hue = fract(0.52 + uv.x * 0.25 + n * 0.35 + bands * 0.04 + t * 0.03);
  vec3 base = hsl2rgb(vec3(hue, 0.85, 0.55));
  vec3 col = vec3(0.004, 0.006, 0.01);
  float ribbon = 1.0 - smoothstep(0.0, 0.24, abs(uv.y - 0.3 * uv.x - 0.15 + n * 0.25));
  col += base * ribbon * 0.42;
  col += hsl2rgb(vec3(fract(hue + 0.12), 0.9, 0.7)) * pow(max(0.0, 1.0 - abs(bands)), 12.0) * 0.12;
  float ribbon2 = 1.0 - smoothstep(0.0, 0.16, abs(uv.y + 0.25 + 0.2 * sin(uv.x * 2.0 + t * 0.2) + n * 0.2));
  col += hsl2rgb(vec3(fract(hue + 0.5), 0.8, 0.6)) * ribbon2 * 0.34;
  col += hsl2rgb(vec3(fract(hue + 0.3), 0.8, 0.65)) * pow(max(0.0, n), 5.0) * 0.25;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    smoke: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.12;
  vec2 p = uv * 2.0;
  vec2 q = vec2(fbm(p + vec2(t * 0.05, -t * 0.03)), fbm(p + vec2(5.2, 1.3) - t * 0.02));
  float n = fbm(p + 3.0 * q + vec2(0.0, -t * 0.1));
  float n2 = fbm(p * 2.0 + 2.0 * q + vec2(0.0, -t * 0.2));
  vec3 col = vec3(0.006, 0.008, 0.011);
  col = mix(col, vec3(0.05, 0.085, 0.095), smoothstep(0.3, 0.8, n));
  col = mix(col, vec3(0.14, 0.23, 0.23), smoothstep(0.55, 0.9, n2));
  col += vec3(0.32, 0.55, 0.55) * pow(max(0.0, n2), 4.0) * 0.28;
  vec2 light = vec2(0.45 + sin(t * 0.1) * 0.1, 0.35);
  col += vec3(0.18, 0.45, 0.45) * exp(-length(uv - light) * 2.2) * 0.22;
  col *= 1.0 - smoothstep(0.55, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    silk: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.3;
  vec3 col = vec3(0.004, 0.008, 0.011);
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    float y0 = -0.24 + fi * 0.16;
    float wave = sin(uv.x * 2.2 + t * 0.4 + fi * 1.4) * 0.12
      + sin(uv.x * 4.1 - t * 0.3 + fi) * 0.05
      + (fbm(vec2(uv.x * 2.0 + t * 0.1, fi)) - 0.5) * 0.15;
    float band = 1.0 - smoothstep(0.0, 0.12, abs(uv.y - y0 - wave));
    float sheen = 0.5 + 0.5 * sin(uv.x * 5.0 + t * 0.8 + fi * 2.0 + uv.y * 4.0);
    vec3 c = mix(vec3(0.1, 0.8, 0.65), vec3(0.55, 0.4, 1.0), 0.5 + 0.5 * sin(fi * 1.7 + t * 0.2));
    col += c * band * (0.2 + 0.3 * sheen);
  }
  col += vec3(0.2, 0.5, 0.5) * exp(-pow((uv.y - 0.2) / 0.5, 2.0)) * 0.12;
  col *= 1.0 - smoothstep(0.55, 1.2, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    grid: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.4;
  vec3 col = vec3(0.003, 0.003, 0.009);
  float horizon = -0.04;
  col += vec3(0.05, 0.01, 0.1) * (0.5 + uv.y);
  col += vec3(0.9, 0.7, 1.0) * stars(vec2(uv.x * 1.2, uv.y * 1.2) + vec2(0.0, t * 0.01), 20.0, uTime)
    * 0.3 * (1.0 - smoothstep(horizon - 0.05, horizon + 0.1, uv.y));
  vec2 sun = vec2(0.0, 0.12);
  float sunD = length(uv - sun);
  float sunMask = 1.0 - smoothstep(0.19, 0.21, sunD);
  col += mix(vec3(1.0, 0.2, 0.6), vec3(1.0, 0.8, 0.3), smoothstep(-0.1, 0.25, uv.y)) * sunMask * 0.7;
  if (uv.y < horizon) {
    float d = horizon - uv.y;
    float z = 0.1 / max(d, 0.001);
    vec2 g = vec2(uv.x * z * 1.6, z * 0.7 + t * 0.8);
    vec2 cell = abs(fract(g) - 0.5);
    float lines = 1.0 - smoothstep(0.0, 0.035, min(cell.x, cell.y));
    float fade = exp(-d * 2.4);
    col += vec3(0.1, 0.8, 1.0) * lines * fade * 0.9;
    col += vec3(0.9, 0.2, 0.8) * lines * fade * 0.25;
    col += vec3(0.4, 0.1, 0.6) * exp(-d * 10.0) * 0.5;
  }
  col *= 1.0 - 0.06 * step(0.5, fract(gl_FragCoord.y * 0.5));
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    particles: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.4;
  vec3 col = vec3(0.003, 0.005, 0.011);
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    float scale = 5.0 + fi * 3.0;
    vec2 g = uv * scale;
    vec2 id = floor(g);
    vec2 f = fract(g) - 0.5;
    float h = hash(id + fi * 7.3);
    vec2 h2 = vec2(hash(id + 1.7), hash(id + 8.3));
    vec2 off = (h2 - 0.5) * 0.7;
    off.x += sin(t * (0.2 + h * 0.4) + id.y * 0.7) * 0.15;
    off.y += cos(t * (0.15 + h * 0.3) + id.x * 0.6) * 0.15;
    float d = length(f - off);
    float glow = 1.0 - smoothstep(0.0, 0.06, d);
    float tw = 0.5 + 0.5 * sin(t * 2.0 + h * 6.283);
    vec3 c = mix(vec3(0.5, 0.8, 1.0), vec3(0.8, 0.4, 1.0), h2.x);
    col += c * glow * tw * (0.12 + 0.18 * h);
  }
  float n = fbm(uv * 2.0 + vec2(t * 0.05, -t * 0.03));
  float streak = pow(max(0.0, 1.0 - abs(sin(uv.x * 5.0 + n * 3.0 + t * 0.4))), 20.0);
  col += vec3(0.08, 0.28, 0.5) * streak * 0.035;
  col += vec3(0.08, 0.15, 0.3) * exp(-length(uv) * 0.8) * 0.25;
  col *= 1.0 - smoothstep(0.6, 1.2, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,
  };

  const SPEEDS = { aurora: 0.8, ink: 0.65, nebula: 0.5, caustics: 0.9, vortex: 0.7, prism: 0.6, smoke: 0.55, silk: 0.65, grid: 0.7, particles: 0.75 };
  let active = null;

  const compile = (gl, type, source) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Vela background shader failed:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  };

  const build = (gl, source) => {
    const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX);
    const fragment = compile(gl, gl.FRAGMENT_SHADER, source);
    if (!vertex || !fragment) return null;
    const program = gl.createProgram();
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.bindAttribLocation(program, 0, 'aPosition');
    gl.linkProgram(program);
    gl.deleteShader(vertex);
    gl.deleteShader(fragment);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Vela background program failed:', gl.getProgramInfoLog(program));
      gl.deleteProgram(program);
      return null;
    }
    return program;
  };

  const destroy = () => {
    if (!active) return;
    cancelAnimationFrame(active.raf);
    window.removeEventListener('resize', active.resize);
    window.removeEventListener('pointermove', active.pointer);
    document.removeEventListener('visibilitychange', active.visibility);
    active.gl.deleteVertexArray(active.vao);
    active.gl.deleteBuffer(active.buffer);
    active.gl.deleteProgram(active.program);
    active = null;
  };

  const mount = (canvas, slug) => {
    destroy();
    if (!canvas || !SHADERS[slug]) return;
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      failIfMajorPerformanceCaveat: false,
    });
    if (!gl) return;
    const program = build(gl, HEADER + SHADERS[slug]);
    if (!program) return;

    const vao = gl.createVertexArray();
    const buffer = gl.createBuffer();
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);

    const uniforms = {
      resolution: gl.getUniformLocation(program, 'uResolution'),
      time: gl.getUniformLocation(program, 'uTime'),
      pointer: gl.getUniformLocation(program, 'uPointer'),
    };
    const speed = SPEEDS[slug] || 1;
    const startTime = performance.now();
    const pointer = [0.5, 0.5];
    let raf = 0;
    let last = 0;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      const width = Math.max(1, Math.round(window.innerWidth * dpr));
      const height = Math.max(1, Math.round(window.innerHeight * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      last = 0;
      start();
    };

    const draw = (now) => {
      raf = 0;
      if (!active || document.hidden) return;
      if (!last) last = now;
      last = now;
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(program);
      if (uniforms.resolution) gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
      if (uniforms.time) gl.uniform1f(uniforms.time, (now - startTime) / 1000 * speed);
      if (uniforms.pointer) gl.uniform2f(uniforms.pointer, pointer[0], pointer[1]);
      gl.bindVertexArray(vao);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindVertexArray(null);
      raf = requestAnimationFrame(draw);
      if (active) active.raf = raf;
    };

    const start = () => {
      if (!raf) {
        last = 0;
        raf = requestAnimationFrame(draw);
        if (active) active.raf = raf;
      }
    };

    const onPointer = (event) => {
      pointer[0] = event.clientX / window.innerWidth;
      pointer[1] = event.clientY / window.innerHeight;
    };

    const onVisibility = () => {
      if (document.hidden) {
        cancelAnimationFrame(raf);
        raf = 0;
      } else {
        start();
      }
    };

    active = {
      raf,
      vao,
      buffer,
      program,
      gl,
      resize,
      pointer: onPointer,
      visibility: onVisibility,
    };
    window.addEventListener('resize', resize);
    window.addEventListener('pointermove', onPointer, { passive: true });
    document.addEventListener('visibilitychange', onVisibility);
    resize();
    start();
  };

  window.velaBackground = { mount, destroy };
})();
