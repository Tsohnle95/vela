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
    // ---------------------------------------------------------------- saved
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

    // ------------------------------------------------------------------ new
    rift: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.28;
  float n = fbm(vec2(uv.y * 2.2 + t * 0.15, 1.7));
  float tear = uv.x - (0.16 * sin(uv.y * 3.0 + t * 0.6) + 0.07 * sin(uv.y * 9.0 - t * 0.9) + (n - 0.5) * 0.22);
  float d = abs(tear);
  float core = exp(-d * 70.0);
  float glow = exp(-d * 8.0);
  vec3 col = vec3(0.004, 0.006, 0.012);
  col += vec3(1.0, 0.42, 0.22) * glow * 0.55;
  col += vec3(1.0, 0.85, 0.7) * core * 1.2;
  col += vec3(0.5, 0.18, 0.9) * exp(-d * 2.2) * 0.18;
  col += vec3(0.1, 0.5, 0.9) * pow(fbm(uv * 3.0 + vec2(0.0, -t * 0.4)), 3.0) * 0.15;
  col += vec3(0.7, 0.8, 1.0) * stars(uv * 1.3 + vec2(0.0, t * 0.02), 16.0, uTime) * 0.5;
  col *= 1.0 - smoothstep(0.6, 1.2, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    tide: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.22;
  vec3 col = vec3(0.003, 0.008, 0.012);
  for (int i = 0; i < 6; i++) {
    float fi = float(i);
    float y = -0.35 + fi * 0.14;
    float w = sin(uv.x * 1.4 + t * 0.5 + fi * 0.9) * 0.09
      + sin(uv.x * 3.3 - t * 0.7 + fi * 1.7) * 0.04
      + (fbm(vec2(uv.x * 1.5 + t * 0.15, fi * 3.0)) - 0.5) * 0.12;
    float band = 1.0 - smoothstep(0.0, 0.05, abs(uv.y - y - w));
    float crest = 1.0 - smoothstep(0.0, 0.012, abs(uv.y - y - w));
    vec3 c = mix(vec3(0.05, 0.35, 0.55), vec3(0.1, 0.9, 0.75), 0.5 + 0.5 * sin(fi * 1.3 + t * 0.2));
    col += c * band * 0.18;
    col += vec3(0.7, 0.95, 1.0) * crest * 0.3;
  }
  col += vec3(0.02, 0.06, 0.12) * exp(-abs(uv.y + 0.1) * 2.0);
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    halo: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.3;
  float r = length(uv);
  float a = atan(uv.y, uv.x);
  vec3 col = vec3(0.004, 0.006, 0.014);
  for (int i = 0; i < 5; i++) {
    float fi = float(i);
    float radius = 0.16 + fi * 0.11 + 0.012 * sin(t * 0.8 + fi);
    float ring = 1.0 - smoothstep(0.0, 0.008, abs(r - radius));
    float pulse = 0.5 + 0.5 * sin(t * 1.1 - fi * 1.3);
    vec3 c = mix(vec3(0.95, 0.72, 0.35), vec3(0.4, 0.6, 1.0), fi / 4.0);
    col += c * ring * (0.25 + 0.3 * pulse);
    col += c * exp(-abs(r - radius) * 22.0) * 0.1 * pulse;
  }
  vec2 orb = vec2(cos(t * 0.4), sin(t * 0.4)) * 0.34;
  col += vec3(1.0, 0.9, 0.75) * exp(-length(uv - orb) * 60.0) * 0.9;
  col += vec3(0.6, 0.4, 1.0) * exp(-r * 4.0) * 0.22;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    ember: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.4;
  vec3 col = vec3(0.012, 0.006, 0.004);
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float scale = 4.0 + fi * 2.8;
    vec2 g = vec2(uv.x * scale, (uv.y - t * (0.35 + fi * 0.2)) * scale);
    vec2 id = floor(g);
    vec2 f = fract(g) - 0.5;
    float h = hash(id + fi * 11.0);
    if (h > 0.4) {
      vec2 off = vec2((hash(id + 3.1) - 0.5) * 0.8, (hash(id + 5.7) - 0.5) * 0.2);
      vec2 q = f - off;
      float head = 1.0 - smoothstep(0.0, 0.024 + 0.03 * h, length(q * vec2(1.0, 1.2)));
      float tail = exp(-abs(q.x) * 110.0) * smoothstep(-0.18, 0.0, q.y) * (1.0 - smoothstep(0.0, 0.03, q.y));
      float flicker = 0.55 + 0.45 * sin(t * 6.0 + h * 20.0);
      vec3 c = mix(vec3(1.0, 0.3, 0.05), vec3(1.0, 0.86, 0.42), h);
      col += c * (head + tail * 0.5) * flicker * (0.45 + 0.55 * h) * (1.1 - uv.y);
    }
  }
  float n = fbm(uv * 2.2 + vec2(t * 0.1, -t * 0.25));
  col += vec3(0.3, 0.1, 0.03) * smoothstep(0.35, 0.95, n) * (0.45 - uv.y) * 0.55;
  col += vec3(1.0, 0.35, 0.1) * exp(-length(uv - vec2(0.0, -0.55)) * 2.4) * 0.4;
  col *= 1.0 - smoothstep(0.55, 1.2, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    mirage: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.2;
  float horizon = -0.02;
  float shimmer = (fbm(vec2(uv.x * 3.0, uv.y * 6.0 - t * 1.2)) - 0.5) * 0.04;
  vec2 q = uv;
  q.x += shimmer * (1.0 - smoothstep(horizon, 0.5, uv.y));
  vec3 col = vec3(0.02, 0.012, 0.01);
  col = mix(vec3(0.02, 0.012, 0.015), vec3(0.35, 0.16, 0.08), smoothstep(horizon, 0.7, q.y));
  float sun = length(vec2(q.x, q.y - 0.12));
  col += vec3(1.0, 0.55, 0.25) * exp(-sun * 6.0) * 0.4;
  col += vec3(1.0, 0.85, 0.6) * exp(-sun * 30.0) * 0.9;
  float bands = sin((q.y - horizon) * 70.0 + fbm(vec2(q.x * 2.0, t)) * 6.0);
  col += vec3(0.9, 0.6, 0.35) * pow(max(0.0, bands), 8.0) * exp(-abs(q.y - horizon) * 7.0) * 0.25;
  if (q.y < horizon) {
    float d = horizon - q.y;
    col = mix(vec3(0.16, 0.07, 0.04), vec3(0.02, 0.012, 0.014), smoothstep(0.0, 0.7, d));
    col += vec3(0.8, 0.45, 0.2) * exp(-d * 12.0) * 0.25 * (0.6 + 0.4 * sin(q.x * 20.0 + d * 30.0 + t));
  }
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    lumen: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.15;
  vec2 light = vec2(-0.45, 0.55);
  vec2 d = uv - light;
  float r = length(d);
  float a = atan(d.y, d.x);
  vec3 col = vec3(0.004, 0.008, 0.012);
  float rays = 0.0;
  for (int i = 0; i < 6; i++) {
    float fi = float(i);
    float n = fbm(vec2(a * 1.6 + fi * 2.0, r * 1.2 - t * 0.2 + fi));
    rays += pow(max(0.0, 0.5 + 0.5 * sin(a * (3.0 + fi) + t * 0.2 + fi)), 10.0) * (0.5 + 0.5 * n);
  }
  rays *= exp(-r * 0.9);
  col += mix(vec3(0.25, 0.8, 0.75), vec3(0.75, 0.95, 1.0), 0.5 + 0.5 * sin(a + t * 0.1)) * rays * 0.16;
  col += vec3(0.8, 1.0, 0.95) * exp(-r * 10.0) * 1.2;
  col += vec3(0.04, 0.16, 0.2) * exp(-r * 1.6) * 0.4;
  col += vec3(0.1, 0.3, 0.3) * pow(fbm(uv * 2.0 + vec2(t * 0.1, 0.0)), 3.0) * 0.15;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    circuit: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.5;
  vec3 col = vec3(0.004, 0.008, 0.01);
  float lines = 0.0;
  float pulses = 0.0;
  for (int i = 0; i < 3; i++) {
    float fi = float(i);
    float scale = 3.0 + fi * 2.2;
    vec2 g = uv * vec2(1.5, 1.0) * scale;
    vec2 id = floor(g);
    vec2 f = fract(g);
    float h = hash(id + fi * 13.0);
    float hx = step(0.45, hash(vec2(id.y + fi * 3.0, 7.0)));
    float vy = step(0.45, hash(vec2(id.x + fi * 5.0, 3.0)));
    float lineH = hx * (1.0 - smoothstep(0.0, 0.04, abs(f.y - 0.5)));
    float lineV = vy * (1.0 - smoothstep(0.0, 0.04, abs(f.x - 0.5)));
    lines += (lineH + lineV) * (0.25 + 0.35 * h);
    float ph = fract(f.x + t * (0.4 + h * 0.5));
    pulses += lineH * smoothstep(0.0, 0.12, ph) * (1.0 - smoothstep(0.12, 0.3, ph));
    float pv = fract(f.y + t * (0.4 + hash(id + 2.0) * 0.5));
    pulses += lineV * smoothstep(0.0, 0.12, pv) * (1.0 - smoothstep(0.12, 0.3, pv));
  }
  col += vec3(0.1, 0.75, 0.55) * lines * 0.5;
  col += vec3(0.6, 1.0, 0.85) * pulses * 0.5;
  col += vec3(0.02, 0.12, 0.1) * exp(-length(uv) * 1.2) * 0.6;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    dunes: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.18;
  vec2 p = uv * 2.0;
  float n = fbm(p * 1.1 + vec2(t * 0.1, -t * 0.05));
  float n2 = fbm(p * 2.2 + vec2(3.1, 5.7) + n * 1.6);
  float h = n * 0.7 + n2 * 0.3;
  float lines = abs(fract(h * 7.0 + p.y * 0.15 + t * 0.5) - 0.5);
  float contour = 1.0 - smoothstep(0.0, 0.035, lines);
  vec3 col = vec3(0.018, 0.012, 0.014);
  col = mix(col, vec3(0.16, 0.08, 0.1), smoothstep(0.2, 0.8, h));
  col += vec3(1.0, 0.6, 0.55) * contour * 0.18;
  col += vec3(0.4, 0.15, 0.25) * exp(-length(uv) * 1.5) * 0.4;
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    bloom: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.25;
  float r = length(uv);
  float a = atan(uv.y, uv.x);
  float petals = 0.5 + 0.5 * sin(a * 8.0 + sin(a * 3.0 + t * 0.4) * 2.0 + t * 0.3);
  float petals2 = 0.5 + 0.5 * sin(a * 13.0 - sin(a * 4.0 - t * 0.3) * 1.6 - t * 0.22);
  float open = 0.22 + 0.07 * sin(t * 0.6);
  vec3 col = vec3(0.005, 0.006, 0.012);
  vec3 c = mix(vec3(1.0, 0.55, 0.85), vec3(0.55, 0.85, 1.0), petals);
  float shape = 1.0 - smoothstep(0.0, 0.075, abs(r - open - 0.11 * petals));
  float shape2 = 1.0 - smoothstep(0.0, 0.11, abs(r - open * 1.75 - 0.08 * petals2));
  col += c * shape2 * 0.12;
  col += c * shape * 0.28;
  col += vec3(1.0, 0.9, 1.0) * exp(-r * 9.0) * 0.5;
  col += vec3(0.5, 0.2, 0.8) * exp(-r * 2.0) * 0.18;
  float n = fbm(uv * 2.0 + vec2(t * 0.1, 0.0));
  col += c * pow(n, 4.0) * 0.16;
  col *= 1.0 - smoothstep(0.55, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    static: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.9;
  vec2 p = gl_FragCoord.xy;
  float grain = hash(p + vec2(floor(t * 30.0) * 131.0, 0.0));
  vec3 col = vec3(grain) * 0.2;
  float scan = 0.5 + 0.5 * sin(p.y * 1.4 + t * 10.0);
  col *= 0.5 + 0.7 * scan;
  float band = exp(-pow((fract(uv.y * 0.5 - t * 0.06) - 0.5) * 6.0, 2.0));
  col += vec3(0.3, 0.62, 0.68) * band * 0.14;
  float glitch = step(0.97, hash(vec2(floor(uv.y * 60.0), floor(t * 3.0))));
  col += vec3(0.55, 0.9, 0.95) * glitch * 0.16;
  col += vec3(0.04, 0.1, 0.12) * exp(-length(uv) * 1.4) * 0.5;
  col *= 1.0 - smoothstep(0.5, 1.1, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    tunnel: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.5;
  float r = length(uv) + 0.0001;
  float a = atan(uv.y, uv.x);
  float z = 1.0 / r;
  float rings = fract(z * 0.6 - t * 0.8);
  float ring = 1.0 - smoothstep(0.0, 0.06, abs(rings - 0.5));
  ring *= smoothstep(0.0, 0.35, r);
  float hue = fract(0.55 + a * 0.12 + z * 0.03);
  vec3 c = hsl2rgb(vec3(hue, 0.75, 0.6));
  vec3 col = vec3(0.004, 0.005, 0.012);
  col += c * ring * 0.6;
  col += c * exp(-r * 3.0) * 0.25;
  float spokes = pow(max(0.0, 0.5 + 0.5 * sin(a * 6.0 + t * 0.3)), 8.0);
  col += vec3(0.3, 0.7, 1.0) * spokes * exp(-r * 1.5) * 0.08;
  col *= smoothstep(0.0, 0.2, r);
  outColor = vec4(col, 1.0);
}`,

    lattice: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.25;
  float a = t * 0.15;
  mat2 R = mat2(cos(a), -sin(a), sin(a), cos(a));
  vec2 p = R * uv * vec2(1.5, 1.0) * 4.0;
  float d1 = abs(fract(p.x + p.y) - 0.5);
  float d2 = abs(fract(p.x - p.y) - 0.5);
  float line = 1.0 - smoothstep(0.0, 0.045, min(d1, d2));
  float node = exp(-(d1 + d2) * 45.0);
  float depth = 0.5 + 0.5 * sin(length(uv) * 3.0 - t * 0.8);
  vec3 tint = mix(vec3(0.2, 0.5, 1.0), vec3(0.8, 0.4, 1.0), depth);
  vec3 col = vec3(0.004, 0.007, 0.014);
  col += tint * line * 0.32;
  col += tint * node * 0.55;
  col += vec3(0.6, 0.9, 1.0) * pow(fbm(uv * 3.0 + vec2(t * 0.1, 0.0)), 6.0) * 0.3;
  col *= 1.0 - smoothstep(0.55, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    comet: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.5;
  vec3 col = vec3(0.003, 0.005, 0.012);
  col += vec3(0.7, 0.8, 1.0) * stars(uv * 1.2, 20.0, uTime) * 0.7;
  vec2 dir = normalize(vec2(-0.7, -1.0));
  vec2 side = vec2(-dir.y, dir.x);
  for (int i = 0; i < 4; i++) {
    float fi = float(i);
    float speed = 0.22 + fi * 0.07;
    float prog = fract(t * speed + fi * 0.29);
    vec2 head = vec2(0.85, 0.85) + dir * prog * 2.1;
    vec2 rel = uv - head;
    float along = dot(rel, dir);
    float lateral = abs(dot(rel, side));
    float body = exp(-lateral * 80.0) * exp(-abs(along) * 10.0);
    float behind = 1.0 - smoothstep(-0.06, 0.02, along);
    float streak = exp(-lateral * 45.0) * exp(-max(0.0, -along) * 4.5) * behind;
    float halo = exp(-lateral * 12.0) * exp(-max(0.0, -along) * 2.2) * behind;
    vec3 tint = mix(vec3(1.0, 0.85, 0.65), vec3(0.6, 0.8, 1.0), fi / 3.0);
    col += tint * body * 1.1;
    col += tint * streak * 0.5;
    col += tint * halo * 0.06;
  }
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    mist: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.08;
  vec2 p = vec2(uv.x * 1.4, uv.y);
  float n1 = fbm(p * 1.2 + vec2(t * 0.2, 0.0));
  float n2 = fbm(p * 2.4 + vec2(-t * 0.15, 1.7));
  float n3 = fbm(p * 4.2 + vec2(t * 0.12, 4.1));
  float bank = smoothstep(0.28, 0.76, n1 * 0.5 + n2 * 0.35 + n3 * 0.15);
  vec3 col = vec3(0.007, 0.011, 0.014);
  col = mix(col, vec3(0.17, 0.22, 0.23), bank * 0.9);
  col += vec3(0.33, 0.44, 0.45) * pow(n2, 2.2) * 0.3;
  col += vec3(0.4, 0.56, 0.56) * exp(-length(uv - vec2(-0.1, 0.05)) * 1.2) * 0.2;
  col += vec3(0.2, 0.31, 0.33) * exp(-length(uv - vec2(0.38, -0.12)) * 2.2) * 0.16;
  col += vec3(0.16, 0.22, 0.24) * exp(-abs(uv.y + 0.28) * 2.6) * 0.18;
  col *= 1.0 - smoothstep(0.6, 1.3, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,

    radiant: `
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  float t = uTime * 0.2;
  vec2 sun = vec2(0.0, 0.05 + sin(t * 0.5) * 0.02);
  vec2 d = uv - sun;
  float r = length(d);
  float a = atan(d.y, d.x);
  vec3 col = vec3(0.006, 0.008, 0.014);
  float rays = pow(max(0.0, 0.5 + 0.5 * sin(a * 12.0 + sin(a * 5.0 - t * 0.6) * 2.0 + t * 0.2)), 14.0);
  col += vec3(1.0, 0.75, 0.4) * rays * exp(-r * 1.4) * 0.22;
  col += vec3(1.0, 0.55, 0.2) * exp(-r * 2.2) * 0.5;
  col += vec3(1.0, 0.95, 0.8) * exp(-r * 12.0) * 1.6;
  for (int i = 1; i <= 3; i++) {
    float fi = float(i);
    vec2 g = sun - d * fi * 0.45;
    col += mix(vec3(0.4, 0.7, 1.0), vec3(1.0, 0.7, 0.4), fract(fi * 0.3))
      * exp(-length(uv - g) * 40.0) * 0.12;
  }
  col *= 1.0 - smoothstep(0.6, 1.25, length(uv * vec2(0.55, 1.0)));
  outColor = vec4(col, 1.0);
}`,
  };

  const EFFECTS = [
    { slug: 'silk', name: 'Silk', line: 'A smoother current.', layout: 'bottom-center', speed: 0.65, saved: true, tone: ['#1b6d63', '#7c6cf0'] },
    { slug: 'prism', name: 'Prism', line: 'A different angle.', layout: 'top-left', speed: 0.6, saved: true, tone: ['#6b3fd0', '#2fb8a0'] },
    { slug: 'vortex', name: 'Vortex', line: 'Everything in orbit.', layout: 'center', speed: 0.7, saved: true, tone: ['#1e6fd6', '#8e3fe0'] },
    { slug: 'nebula', name: 'Nebula', line: 'A field of quiet light.', layout: 'center', speed: 0.5, saved: true, tone: ['#57309e', '#c23b86'] },
    { slug: 'aurora', name: 'Aurora', line: 'A slow sky of light.', layout: 'bottom-left', speed: 0.8, saved: true, tone: ['#1fa98a', '#7f5cf5'] },
    { slug: 'rift', name: 'Rift', line: 'A seam of heat in the dark.', layout: 'left', speed: 0.7, tone: ['#e05a2b', '#7d35d6'] },
    { slug: 'tide', name: 'Tide', line: 'Slow water, layered light.', layout: 'bottom-right', speed: 0.7, tone: ['#176f8f', '#38d6b4'] },
    { slug: 'halo', name: 'Halo', line: 'Rings keeping their distance.', layout: 'center', speed: 0.6, tone: ['#c9a24f', '#5c7fe8'] },
    { slug: 'ember', name: 'Ember', line: 'Warm sparks finding height.', layout: 'bottom-left', speed: 0.85, tone: ['#c4491a', '#f0b24a'] },
    { slug: 'mirage', name: 'Mirage', line: 'A horizon that will not settle.', layout: 'bottom-center', speed: 0.55, tone: ['#b4552a', '#e7b07a'] },
    { slug: 'lumen', name: 'Lumen', line: 'Light through a quiet room.', layout: 'top-right', speed: 0.5, tone: ['#2fa89a', '#bfe6e0'] },
    { slug: 'circuit', name: 'Circuit', line: 'A board awake with signals.', layout: 'top-left', speed: 0.8, tone: ['#128a63', '#8ff0c4'] },
    { slug: 'dunes', name: 'Dunes', line: 'Contours of slow terrain.', layout: 'bottom-right', speed: 0.6, tone: ['#8f5540', '#d9917d'] },
    { slug: 'bloom', name: 'Bloom', line: 'Something opening outward.', layout: 'center', speed: 0.65, tone: ['#b34d8c', '#7ab6e8'] },
    { slug: 'static', name: 'Static', line: 'Signal without a message.', layout: 'bottom-left', speed: 1, tone: ['#3b4a4d', '#aebfc2'] },
    { slug: 'tunnel', name: 'Tunnel', line: 'Forward, without a floor.', layout: 'top-center', speed: 0.9, tone: ['#2c5fd0', '#7e46d8'] },
    { slug: 'lattice', name: 'Lattice', line: 'Structure in slow rotation.', layout: 'bottom-center', speed: 0.55, tone: ['#2d6cbf', '#8f5be0'] },
    { slug: 'comet', name: 'Comet', line: 'Brief light across the field.', layout: 'top-right', speed: 0.8, tone: ['#3f5f9e', '#f0d6a0'] },
    { slug: 'mist', name: 'Mist', line: 'Low weather, moving slowly.', layout: 'bottom-left', speed: 0.5, tone: ['#42575a', '#8aa3a3'] },
    { slug: 'radiant', name: 'Radiant', line: 'A star with nowhere to hide.', layout: 'center', speed: 0.55, tone: ['#d98a2b', '#f6d38a'] },
  ];

  const speedFor = (slug) => (EFFECTS.find((effect) => effect.slug === slug)?.speed ?? 1);

  const compile = (gl, type, source) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Animation shader failed:', gl.getShaderInfoLog(shader));
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
      console.error('Animation program failed:', gl.getProgramInfoLog(program));
      gl.deleteProgram(program);
      return null;
    }
    return program;
  };

  const makeSurface = (gl) => {
    const vao = gl.createVertexArray();
    const buffer = gl.createBuffer();
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
    return { vao, buffer };
  };

  let active = null;

  const destroy = () => {
    if (!active) return;
    cancelAnimationFrame(active.raf);
    window.removeEventListener('resize', active.resize);
    window.removeEventListener('pointermove', active.pointer);
    document.removeEventListener('visibilitychange', active.visibility);
    active.observer?.disconnect();
    active.gl.deleteVertexArray(active.vao);
    active.gl.deleteBuffer(active.buffer);
    active.gl.deleteProgram(active.program);
    active = null;
  };

  const mount = (canvas, slug) => {
    const previousCanvas = active?.canvas || null;
    destroy();
    if (previousCanvas && previousCanvas !== canvas) {
      previousCanvas.getContext('webgl2')?.getExtension('WEBGL_lose_context')?.loseContext();
    }
    if (!canvas || !SHADERS[slug]) return false;
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      failIfMajorPerformanceCaveat: false,
    });
    if (!gl) return false;
    const program = build(gl, HEADER + SHADERS[slug]);
    if (!program) return false;
    const { vao, buffer } = makeSurface(gl);

    const uniforms = {
      resolution: gl.getUniformLocation(program, 'uResolution'),
      time: gl.getUniformLocation(program, 'uTime'),
      pointer: gl.getUniformLocation(program, 'uPointer'),
    };
    const speed = speedFor(slug);
    const startTime = performance.now();
    const pointer = [0.5, 0.5];
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let raf = 0;
    let last = 0;

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      const width = Math.max(1, Math.round((canvas.clientWidth || window.innerWidth) * dpr));
      const height = Math.max(1, Math.round((canvas.clientHeight || window.innerHeight) * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      if (reduced) draw(performance.now());
      else start();
    };

    const draw = (now) => {
      raf = 0;
      if (!active || (document.hidden && !reduced)) return;
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
      if (reduced) return;
      raf = requestAnimationFrame(draw);
      if (active) active.raf = raf;
    };

    const start = () => {
      if (!raf && !document.hidden) {
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
      raf: 0,
      canvas,
      vao,
      buffer,
      program,
      gl,
      resize,
      pointer: onPointer,
      visibility: onVisibility,
      observer: typeof ResizeObserver === 'function' ? new ResizeObserver(resize) : null,
    };
    window.addEventListener('resize', resize);
    window.addEventListener('pointermove', onPointer, { passive: true });
    document.addEventListener('visibilitychange', onVisibility);
    active.observer?.observe(canvas);
    resize();
    start();
    return true;
  };

  let snapshotSurface = null;

  const snapshot = (slug) => {
    if (!SHADERS[slug]) return '';
    if (!snapshotSurface) {
      const canvas = document.createElement('canvas');
      canvas.width = 480;
      canvas.height = 300;
      const gl = canvas.getContext('webgl2', {
        alpha: false,
        antialias: false,
        depth: false,
        stencil: false,
        preserveDrawingBuffer: true,
      });
      if (!gl) return '';
      snapshotSurface = { canvas, gl, ...makeSurface(gl) };
    }
    const { canvas, gl, vao } = snapshotSurface;
    const program = build(gl, HEADER + SHADERS[slug]);
    if (!program) return '';
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(program);
    const resolution = gl.getUniformLocation(program, 'uResolution');
    const time = gl.getUniformLocation(program, 'uTime');
    if (resolution) gl.uniform2f(resolution, canvas.width, canvas.height);
    if (time) gl.uniform1f(time, 9 * speedFor(slug) + 4);
    gl.bindVertexArray(vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.bindVertexArray(null);
    const url = canvas.toDataURL('image/jpeg', 0.84);
    gl.deleteProgram(program);
    return url;
  };

  window.VelaLab = { EFFECTS, SHADERS, mount, destroy, snapshot };
})();
