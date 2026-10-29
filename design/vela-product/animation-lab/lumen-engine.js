(() => {
  const HEADER = `#version 300 es
precision highp float;
out vec4 outColor;
uniform vec2 uResolution;
uniform float uTime;
uniform float uReveal;
uniform float uCast;
uniform float uImpact;
uniform float uSpread;
uniform vec2 uCastOrigin;
uniform float uHitFrac;
uniform vec2 uPointer;
uniform vec4 uHit;
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

float beamCone(vec2 uv, vec2 origin, vec2 target, float halfWidth, float softness) {
  vec2 d = target - origin;
  float len = max(length(d), 0.001);
  vec2 dir = d / len;
  vec2 rel = uv - origin;
  float along = dot(rel, dir) / len;
  float across = abs(dot(rel, vec2(-dir.y, dir.x)));
  float body = 1.0 - smoothstep(halfWidth, halfWidth + softness, across);
  float reach = smoothstep(-0.06, 0.12, along) * (1.0 - smoothstep(1.0, 1.34, along));
  return body * reach;
}

float beamAlong(vec2 uv, vec2 origin, vec2 target) {
  vec2 d = target - origin;
  float len = max(length(d), 0.001);
  return dot(uv - origin, d / len) / len;
}

float beamAcross(vec2 uv, vec2 origin, vec2 target) {
  vec2 d = target - origin;
  vec2 dir = d / max(length(d), 0.001);
  return abs(dot(uv - origin, vec2(-dir.y, dir.x)));
}

vec2 beamOrigin(float x, float y) {
  float aspect = uResolution.x / max(uResolution.y, 1.0);
  return vec2(x * clamp(aspect / 1.44, 0.34, 1.0), y);
}

vec2 hitPoint(float fraction) {
  return vec2(uHit.x + (fraction - 0.5) * uHit.z * 2.0, uHit.y + uHit.w);
}

float spotCone(vec2 uv, vec2 origin, vec2 target, float sourceRadius, float targetRadius, float softness) {
  vec2 d = target - origin;
  float len = max(length(d), 0.001);
  vec2 dir = d / len;
  vec2 rel = uv - origin;
  float along = dot(rel, dir) / len;
  float across = abs(dot(rel, vec2(-dir.y, dir.x)));
  float radius = mix(sourceRadius, targetRadius, clamp(along, 0.0, 1.0));
  float body = 1.0 - smoothstep(radius, radius + softness, across);
  float reach = smoothstep(-0.05, 0.12, along) * (1.0 - smoothstep(1.0, 1.2, along));
  return body * reach;
}

float topSpill(vec2 uv, vec2 hit, float halfWidth, float thickness, float reach) {
  float dx = abs(uv.x - hit.x);
  float line = exp(-abs(uv.y - hit.y) * thickness);
  float spread = 1.0 - smoothstep(0.0, max(halfWidth, 0.05) * reach * (0.18 + 0.82 * uSpread), dx);
  return line * spread * uImpact;
}

float sideSpill(vec2 uv, vec2 hit, float halfWidth, float side, float t, float width, float falloff) {
  float down = hit.y - uv.y;
  float edgeX = uHit.x + side * (halfWidth + 0.012)
    + 0.018 * sin(down * 6.0 - t * 1.4 + side * 2.0) * smoothstep(0.0, 0.4, down);
  float spread = max(width, 0.02) + max(down, 0.0) * 0.12;
  float x = (uv.x - edgeX) / spread;
  float edge = exp(-x * x);
  float flow = smoothstep(-0.02, 0.06, down) * exp(-max(down, 0.0) * falloff);
  float ripple = 0.7 + 0.3 * sin(down * 28.0 - t * 2.2 + side * 1.7);
  float breakup = 0.6 + 0.4 * fbm(vec2(x * 2.0 + side * 2.0, down * 7.0 - t * 1.2));
  return edge * flow * ripple * breakup * uImpact * smoothstep(0.65, 0.95, uSpread) * 1.35;
}

float impact(vec2 uv, vec2 hit, float rx, float ry) {
  vec2 d = (uv - hit) / vec2(max(rx, 0.01), max(ry, 0.01));
  return exp(-dot(d, d)) * uImpact;
}

float sideSheet(vec2 uv, vec2 hit, float halfWidth, float side, float t) {
  float down = hit.y - uv.y;
  float edgeX = uHit.x + side * (halfWidth + 0.012)
    + 0.018 * sin(down * 6.0 - t * 1.4 + side * 2.0) * smoothstep(0.0, 0.4, down);
  float width = 0.03 + max(down, 0.0) * 0.2;
  float x = (uv.x - edgeX) / width;
  float edge = exp(-x * x);
  float core = exp(-x * x * 18.0) * 0.55;
  float flow = smoothstep(-0.02, 0.06, down) * exp(-max(down, 0.0) * 1.3);
  float ripple = 0.7 + 0.3 * sin(down * 30.0 - t * 2.2 + x * 1.8 + side);
  float breakup = 0.55 + 0.45 * fbm(vec2(x * 1.8 + side * 2.0, down * 7.0 - t * 1.2));
  float dropPhase = fract(down * 9.0 - t * 1.8 + side * 0.3);
  float drops = smoothstep(0.0, 0.06, dropPhase) * (1.0 - smoothstep(0.06, 0.16, dropPhase));
  return (edge + core + edge * drops * 0.8) * flow * ripple * breakup * uImpact * smoothstep(0.65, 0.95, uSpread) * 1.6;
}

float collisionLight(vec2 uv, vec2 hit, float halfWidth, float t) {
  float top = topSpill(uv, hit, halfWidth, 45.0, 1.34);
  float sides = sideSheet(uv, hit, halfWidth, -1.0, t) + sideSheet(uv, hit, halfWidth, 1.0, t);
  float bloom = impact(uv, hit, 0.3, 0.08);
  return top * 0.9 + sides * 0.85 + bloom * 0.3;
}
`;

  const VERTEX = `#version 300 es
in vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;

  const MAIN = `
vec3 scene(vec2 uv, float t);
void main() {
  vec2 uv = (gl_FragCoord.xy - 0.5 * uResolution) / uResolution.y;
  vec3 col = scene(uv, uTime);
  vec2 castSource = beamOrigin(uCastOrigin.x, uCastOrigin.y);
  vec2 hitCenter = hitPoint(uHitFrac);
  float distanceFromSource = length(uv - castSource) / max(length(hitCenter - castSource), 0.001);
  float front = 1.0 - smoothstep(uCast, uCast + 0.12, distanceFromSource);
  col *= front;
  col *= 1.0 - smoothstep(0.62, 1.25, length(uv * vec2(0.55, 1.0)));
  col *= uReveal;
  outColor = vec4(col, 1.0);
}
`;

  const SCENES = {
    cathedral: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.72, 0.82);
  vec2 hit = hitPoint(0.42);
  float cone = spotCone(uv, o, hit, 0.02, 0.34, 0.32);
  float core = spotCone(uv, o, hit, 0.006, 0.09, 0.15);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 warm = vec3(1.0, 0.88, 0.68);
  vec3 col = vec3(0.006, 0.007, 0.01);
  float fog = fbm(vec2(across * 2.2, along * 2.6 - t * 0.08));
  col += warm * cone * (0.15 + 0.2 * fog);
  col += warm * core * 0.45;
  col += warm * pow(fog, 3.0) * cone * 0.25;
  vec2 dp = vec2(across * 9.0, along * 11.0 - t * 0.5);
  col += warm * stars(dp, 2.0, t * 0.7) * cone * 0.7;
  col += warm * topSpill(uv, hit, uHit.z, 55.0, 1.05) * 0.5;
  col += warm * sideSpill(uv, hit, uHit.z, -1.0, t, 0.05, 2.2) * 0.5;
  col += warm * sideSpill(uv, hit, uHit.z, 1.0, t, 0.05, 2.2) * 0.5;
  col += warm * impact(uv, hit, 0.22, 0.07) * 0.3;
  col += warm * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,

    searchlight: `
vec3 scene(vec2 uv, float t) {
  float frac = 0.5 + 0.26 * sin(t * 0.35);
  vec2 hit = hitPoint(frac);
  vec2 trail = hitPoint(frac - 0.08);
  vec2 o = beamOrigin(0.72, 0.84);
  float cone = spotCone(uv, o, hit, 0.07, 0.34, 0.36);
  float core = spotCone(uv, o, hit, 0.02, 0.1, 0.18);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 cool = vec3(0.78, 0.94, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.011);
  float fog = fbm(vec2(across * 3.0, along * 2.0 - t * 0.1));
  col += cool * cone * (0.14 + fog * 0.12);
  col += cool * core * 0.42;
  float q = along * 12.0 - t * 2.6;
  float dash = smoothstep(0.0, 0.07, fract(q)) * (1.0 - smoothstep(0.07, 0.22, fract(q)));
  float gate = step(0.4, hash(vec2(floor(q), 9.0)));
  col += cool * exp(-across * across * 120.0) * dash * gate * 0.55;
  col += cool * spotCone(uv, o, trail, 0.03, 0.26, 0.3) * 0.1;
  col += cool * topSpill(uv, hit, uHit.z, 70.0, 0.4) * 0.55;
  col += cool * sideSpill(uv, hit, uHit.z, -1.0, t, 0.04, 3.0) * 0.38;
  col += cool * sideSpill(uv, hit, uHit.z, 1.0, t, 0.04, 3.0) * 0.38;
  col += cool * impact(uv, hit, 0.1, 0.05) * 0.35;
  col += cool * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    'aurora-fall': `
vec3 scene(vec2 uv, float t) {
  vec2 hit = hitPoint(0.52);
  float width = max(uHit.z * 1.5, 0.3);
  float sway = 0.08 * sin(uv.y * 4.0 + t * 0.35) + 0.025 * sin(uv.y * 13.0 - t * 0.8);
  float dx = abs(uv.x + sway - hit.x) / width;
  float vertical = smoothstep(hit.y - 0.02, hit.y + 0.62, uv.y);
  float fold = 0.5 + 0.5 * sin(dx * 13.0 + t * 0.5 + fbm(vec2(uv.y * 3.0, t * 0.2)) * 4.0);
  float curtain = (1.0 - smoothstep(0.35, 1.0, dx)) * vertical;
  vec3 mint = vec3(0.1, 0.95, 0.68);
  vec3 violet = vec3(0.55, 0.4, 1.0);
  vec3 c = mix(mint, violet, 0.5 + 0.5 * sin(hit.x * 4.0 + uv.y * 3.0 + t * 0.2));
  vec3 col = vec3(0.004, 0.007, 0.012);
  col += c * curtain * (0.08 + 0.22 * fold);
  col += c * pow(fold, 4.0) * curtain * 0.2;
  float q = uv.y * 9.0 + t * 0.8;
  col += c * exp(-abs(fract(q) - 0.5) * 20.0) * curtain * 0.08;
  col += c * topSpill(uv, hit, uHit.z, 50.0, 1.1) * 0.4;
  col += c * sideSpill(uv, hit, uHit.z, -1.0, t, 0.06, 1.8) * 0.45;
  col += c * sideSpill(uv, hit, uHit.z, 1.0, t, 0.06, 1.8) * 0.45;
  col += c * impact(uv, hit, 0.35, 0.06) * 0.2;
  col += c * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    'golden-hour': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.85, 0.02);
  vec2 hit = hitPoint(0.22);
  float cone = spotCone(uv, o, hit, 0.05, 0.26, 0.32);
  float core = spotCone(uv, o, hit, 0.01, 0.08, 0.15);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 gold = vec3(1.0, 0.76, 0.42);
  vec3 col = vec3(0.012, 0.007, 0.004);
  col += gold * cone * 0.18;
  col += gold * core * 0.35;
  col += vec3(1.0, 0.82, 0.48) * exp(-length((uv - o) * vec2(1.0, 1.3)) * 8.0) * 0.5;
  float dx = uv.x - hit.x;
  float reach = 1.0 - smoothstep(0.0, uHit.z * 2.4, max(dx, 0.0));
  col += gold * exp(-abs(uv.y - hit.y) * 40.0) * reach * 0.6;
  col += gold * sideSpill(uv, hit, uHit.z, 1.0, t, 0.07, 1.8) * 0.5;
  col += gold * sideSpill(uv, hit, uHit.z, -1.0, t, 0.05, 2.4) * 0.15;
  col += gold * impact(uv, hit, 0.3, 0.06) * 0.25;
  col += gold * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,

    moonbeam: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.72, 0.88);
  vec2 hit = hitPoint(0.58);
  float cone = spotCone(uv, o, hit, 0.05, 0.5, 0.5);
  float core = spotCone(uv, o, hit, 0.02, 0.16, 0.3);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 cool = vec3(0.62, 0.8, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.012);
  float fog = fbm(vec2(across * 3.0, along * 2.0 - t * 0.1));
  col += cool * cone * (0.1 + 0.12 * fog);
  col += cool * core * 0.3;
  float grid = 1.0 - smoothstep(0.0, 0.02, abs(fract(across * 30.0) - 0.5));
  col += cool * grid * cone * 0.05;
  vec2 dp = vec2(across * 12.0, along * 16.0 - t * 0.3);
  col += cool * stars(dp, 2.5, t * 1.4) * cone * 0.7;
  col += cool * exp(-length(uv - o) * 9.0) * 0.28;
  col += cool * topSpill(uv, hit, uHit.z, 38.0, 1.2) * 0.45;
  col += cool * sideSpill(uv, hit, uHit.z, -1.0, t, 0.09, 1.4) * 0.4;
  col += cool * sideSpill(uv, hit, uHit.z, 1.0, t, 0.09, 1.4) * 0.4;
  col += cool * impact(uv, hit, 0.4, 0.09) * 0.3;
  col += cool * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    datafall: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.62, 0.92);
  vec2 hit = hitPoint(0.48);
  float cone = spotCone(uv, o, hit, 0.02, 0.4, 0.25);
  float core = spotCone(uv, o, hit, 0.006, 0.06, 0.12);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 cyan = vec3(0.25, 0.95, 0.85);
  vec3 col = vec3(0.003, 0.008, 0.01);
  float gx = abs(fract(across * 26.0) - 0.5);
  float gy = abs(fract(along * 18.0 - t * 0.5) - 0.5);
  float grid = 1.0 - smoothstep(0.0, 0.03, min(gx, gy));
  col += cyan * cone * (0.08 + grid * 0.3);
  col += cyan * core * 0.45;
  float lane = floor(across * 24.0);
  float h = hash(vec2(lane, 7.0));
  float q = along * 11.0 - t * (1.2 + h * 1.1);
  float dash = smoothstep(0.0, 0.06, fract(q)) * (1.0 - smoothstep(0.06, 0.18, fract(q)));
  col += cyan * exp(-across * across * 160.0) * dash * step(0.45, h) * 0.8;
  float stripes = 0.55 + 0.45 * step(0.5, fract(uv.x * 38.0 + t * 0.1));
  col += cyan * topSpill(uv, hit, uHit.z, 60.0, 1.0) * stripes * 0.5;
  col += cyan * sideSpill(uv, hit, uHit.z, -1.0, t, 0.05, 2.0) * stripes * 0.4;
  col += cyan * sideSpill(uv, hit, uHit.z, 1.0, t, 0.05, 2.0) * stripes * 0.4;
  col += cyan * impact(uv, hit, 0.3, 0.06) * 0.25;
  col += cyan * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    'solar-flare': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.6, 0.82);
  vec2 hit = hitPoint(0.56);
  float cone = spotCone(uv, o, hit, 0.03, 0.16, 0.22);
  float core = spotCone(uv, o, hit, 0.008, 0.055, 0.1);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 hot = vec3(1.0, 0.68, 0.32);
  vec3 col = vec3(0.01, 0.006, 0.004);
  col += hot * cone * 0.18;
  col += vec3(1.0, 0.92, 0.72) * core * 0.62;
  float streak = exp(-abs(uv.y - hit.y) * 70.0) * exp(-abs(uv.x - hit.x) * 1.8);
  col += hot * streak * 0.55;
  float sx = exp(-abs(uv.x - hit.x) * 70.0) * exp(-abs(uv.y - hit.y) * 3.0);
  col += vec3(1.0, 0.9, 0.65) * sx * 0.3;
  for (int i = 1; i <= 3; i++) {
    float fi = float(i);
    vec2 p = o + (hit - o) * fi * 0.28;
    vec3 tint = i == 1 ? vec3(1.0, 0.55, 0.25) : (i == 2 ? vec3(1.0, 0.8, 0.45) : vec3(0.7, 0.85, 1.0));
    col += tint * exp(-length(uv - p) * 34.0) * 0.12;
  }
  col += hot * topSpill(uv, hit, uHit.z, 55.0, 0.9) * 0.5;
  col += hot * sideSpill(uv, hit, uHit.z, -1.0, t, 0.035, 2.6) * 0.35;
  col += hot * sideSpill(uv, hit, uHit.z, 1.0, t, 0.035, 2.6) * 0.35;
  col += vec3(1.0, 0.85, 0.6) * impact(uv, hit, 0.16, 0.04) * 0.35;
  col += hot * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,

    'prism-split': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.72, 0.86);
  vec2 h0 = hitPoint(0.18);
  vec2 h1 = hitPoint(0.34);
  vec2 h2 = hitPoint(0.50);
  vec2 h3 = hitPoint(0.66);
  vec2 h4 = hitPoint(0.82);
  vec3 red = vec3(1.0, 0.3, 0.35);
  vec3 green = vec3(0.3, 1.0, 0.55);
  vec3 blue = vec3(0.35, 0.55, 1.0);
  vec3 amber = vec3(1.0, 0.7, 0.25);
  vec3 violet = vec3(0.7, 0.35, 1.0);
  vec3 col = vec3(0.004, 0.005, 0.01);
  col += red * spotCone(uv, o, h0, 0.02, 0.06, 0.1) * 0.4;
  col += green * spotCone(uv, o, h1, 0.02, 0.06, 0.1) * 0.4;
  col += blue * spotCone(uv, o, h2, 0.02, 0.08, 0.12) * 0.5;
  col += amber * spotCone(uv, o, h3, 0.02, 0.06, 0.1) * 0.4;
  col += violet * spotCone(uv, o, h4, 0.02, 0.06, 0.1) * 0.4;
  col += vec3(0.9, 0.95, 1.0) * spotCone(uv, o, h2, 0.01, 0.03, 0.06) * 0.45;
  col += red * topSpill(uv, h0, uHit.z, 60.0, 0.35) * 0.3;
  col += green * topSpill(uv, h1, uHit.z, 60.0, 0.35) * 0.3;
  col += blue * topSpill(uv, h2, uHit.z, 60.0, 0.4) * 0.35;
  col += amber * topSpill(uv, h3, uHit.z, 60.0, 0.35) * 0.3;
  col += violet * topSpill(uv, h4, uHit.z, 60.0, 0.35) * 0.3;
  col += violet * sideSpill(uv, h4, uHit.z, 1.0, t, 0.05, 2.0) * 0.35;
  col += red * sideSpill(uv, h0, uHit.z, -1.0, t, 0.05, 2.0) * 0.35;
  col += blue * impact(uv, h2, 0.25, 0.06) * 0.25;
  col += vec3(1.0, 0.85, 0.7) * collisionLight(uv, h2, uHit.z, t) * 0.45;
  return col;
}`,

    cascade: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.3, 0.78);
  vec2 hit = hitPoint(0.44);
  float cone = spotCone(uv, o, hit, 0.02, 0.42, 0.3);
  float core = spotCone(uv, o, hit, 0.008, 0.07, 0.14);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 blue = vec3(0.55, 0.82, 1.0);
  vec3 col = vec3(0.004, 0.007, 0.013);
  col += vec3(1.0, 0.86, 0.62) * exp(-length((uv - o) * vec2(1.0, 1.0)) * 18.0) * 0.3;
  col += blue * cone * 0.14;
  col += blue * core * 0.4;
  float lane = floor(across * 20.0);
  float h = hash(vec2(lane, 2.0));
  float q = along * 16.0 - t * (2.0 + h * 1.2);
  float dash = smoothstep(0.0, 0.05, fract(q)) * (1.0 - smoothstep(0.05, 0.18, fract(q)));
  col += blue * exp(-across * across * 120.0) * dash * step(0.35, h) * 0.9;
  float mist = fbm(uv * 4.0 + vec2(t * 0.1, -t * 0.08));
  col += blue * pow(mist, 3.0) * exp(-length(uv - hit) * 5.0) * 0.25;
  col += blue * topSpill(uv, hit, uHit.z, 45.0, 1.15) * 0.45;
  col += blue * sideSpill(uv, hit, uHit.z, -1.0, t, 0.06, 2.0) * 0.4;
  col += blue * sideSpill(uv, hit, uHit.z, 1.0, t, 0.06, 2.0) * 0.4;
  col += vec3(0.9, 0.97, 1.0) * impact(uv, hit, 0.4, 0.08) * 0.28;
  col += blue * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    'halo-shaft': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.64, 0.84);
  vec2 hit = hitPoint(0.6);
  float cone = spotCone(uv, o, hit, 0.02, 0.36, 0.28);
  float core = spotCone(uv, o, hit, 0.006, 0.08, 0.15);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 gold = vec3(1.0, 0.78, 0.42);
  vec3 blue = vec3(0.45, 0.72, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.011);
  float rings = pow(max(0.0, 1.0 - abs(sin(across * 40.0 + along * 8.0 - t * 1.5))), 10.0);
  col += mix(blue, gold, 0.5 + 0.5 * sin(along * 4.0 + t * 0.2)) * cone * (0.1 + rings * 0.28);
  col += gold * core * 0.35;
  float topRings = 0.6 + 0.4 * sin((uv.x - hit.x) * 50.0 - t * 2.0);
  col += gold * topSpill(uv, hit, uHit.z, 50.0, 1.0) * topRings * 0.45;
  col += gold * sideSpill(uv, hit, uHit.z, -1.0, t, 0.05, 2.0) * 0.4;
  col += gold * sideSpill(uv, hit, uHit.z, 1.0, t, 0.05, 2.0) * 0.4;
  col += mix(blue, gold, 0.5) * impact(uv, hit, 0.3, 0.07) * 0.3;
  col += gold * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,

    'meteor-stream': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.7, 0.9);
  vec2 hit = hitPoint(0.55);
  float cone = spotCone(uv, o, hit, 0.02, 0.4, 0.32);
  float core = spotCone(uv, o, hit, 0.008, 0.08, 0.16);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 col = vec3(0.003, 0.006, 0.014);
  col += vec3(0.7, 0.8, 1.0) * stars(uv * 1.8 + vec2(0.0, t * 0.01), 20.0, t) * 0.6;
  col += vec3(0.75, 0.85, 1.0) * cone * 0.1;
  col += vec3(0.9, 0.95, 1.0) * core * 0.35;
  float lane = floor(across * 26.0);
  float h = hash(vec2(lane, 3.0));
  float q = along * 9.0 - t * (1.5 + h * 1.1);
  float dash = smoothstep(0.0, 0.04, fract(q)) * (1.0 - smoothstep(0.04, 0.16, fract(q)));
  float laneGlow = exp(-pow(fract(across * 26.0) - 0.5, 2.0) * 90.0);
  col += mix(vec3(0.5, 0.7, 1.0), vec3(1.0, 0.92, 0.75), h) * laneGlow * dash * step(0.45, h) * 1.1;
  col += vec3(0.8, 0.9, 1.0) * topSpill(uv, hit, uHit.z, 45.0, 1.1) * 0.45;
  col += vec3(0.6, 0.8, 1.0) * sideSpill(uv, hit, uHit.z, -1.0, t, 0.05, 2.0) * 0.42;
  col += vec3(0.6, 0.8, 1.0) * sideSpill(uv, hit, uHit.z, 1.0, t, 0.05, 2.0) * 0.42;
  col += vec3(1.0, 0.9, 0.7) * impact(uv, hit, 0.35, 0.07) * 0.3;
  col += vec3(0.8, 0.9, 1.0) * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    sanctuary: `
vec3 scene(vec2 uv, float t) {
  vec2 oL = beamOrigin(-0.8, 0.88);
  vec2 oR = beamOrigin(0.8, 0.88);
  vec2 hL = hitPoint(0.34);
  vec2 hR = hitPoint(0.66);
  float coneL = spotCone(uv, oL, hL, 0.02, 0.28, 0.3);
  float coneR = spotCone(uv, oR, hR, 0.02, 0.28, 0.3);
  float coreL = spotCone(uv, oL, hL, 0.006, 0.06, 0.12);
  float coreR = spotCone(uv, oR, hR, 0.006, 0.06, 0.12);
  vec3 teal = vec3(0.35, 0.85, 0.78);
  vec3 warm = vec3(1.0, 0.78, 0.5);
  vec3 col = vec3(0.004, 0.008, 0.01);
  col += teal * coneL * 0.16;
  col += warm * coneR * 0.14;
  col += teal * coreL * 0.35;
  col += warm * coreR * 0.3;
  float fog = fbm(vec2(uv.x * 3.0 + t * 0.05, uv.y * 4.0 - t * 0.1));
  col += mix(teal, warm, 0.5) * fog * (coneL + coneR) * 0.16;
  col += teal * topSpill(uv, hL, uHit.z, 50.0, 0.5) * 0.4;
  col += warm * topSpill(uv, hR, uHit.z, 50.0, 0.5) * 0.4;
  col += teal * sideSpill(uv, hL, uHit.z, -1.0, t, 0.05, 2.0) * 0.4;
  col += warm * sideSpill(uv, hR, uHit.z, 1.0, t, 0.05, 2.0) * 0.4;
  col += mix(teal, warm, 0.5) * impact(uv, mix(hL, hR, 0.5), 0.3, 0.08) * 0.2;
  col += teal * collisionLight(uv, hL, uHit.z, t) * 0.35;
  col += warm * collisionLight(uv, hR, uHit.z, t) * 0.35;
  return col;
}`,

    pulse: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.0, 0.95);
  vec2 hit = hitPoint(0.5);
  float beat = 0.62 + 0.38 * sin(t * 3.2);
  float cone = spotCone(uv, o, hit, 0.02, 0.42, 0.3);
  float core = spotCone(uv, o, hit, 0.008, 0.08, 0.15);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 violet = vec3(0.62, 0.4, 1.0);
  vec3 cyan = vec3(0.35, 0.9, 1.0);
  vec3 col = vec3(0.004, 0.005, 0.012);
  col += violet * cone * 0.16 * beat;
  col += mix(violet, cyan, 0.5 + 0.5 * sin(along * 6.0 + t * 0.5)) * core * 0.45 * beat;
  float rings = pow(max(0.0, 1.0 - abs(fract(along * 7.0 - t * 1.8) - 0.5) * 2.0), 12.0);
  col += cyan * rings * exp(-across * across * 80.0) * 0.5;
  col += violet * topSpill(uv, hit, uHit.z, 45.0, 1.15) * beat * 0.55;
  col += cyan * sideSpill(uv, hit, uHit.z, -1.0, t, 0.06, 2.0) * beat * 0.45;
  col += cyan * sideSpill(uv, hit, uHit.z, 1.0, t, 0.06, 2.0) * beat * 0.45;
  col += cyan * impact(uv, hit, 0.35, 0.08) * beat * 0.35;
  col += mix(violet, cyan, 0.5) * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,

    starfall: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.62, 0.9);
  vec2 hit = hitPoint(0.48);
  float cone = spotCone(uv, o, hit, 0.02, 0.44, 0.35);
  float core = spotCone(uv, o, hit, 0.008, 0.08, 0.15);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 indigo = vec3(0.5, 0.62, 1.0);
  vec3 col = vec3(0.003, 0.005, 0.015);
  col += vec3(0.7, 0.8, 1.0) * stars(uv * 1.8 + vec2(0.0, t * 0.01), 20.0, t) * 0.85;
  col += indigo * cone * 0.18;
  col += vec3(0.9, 0.95, 1.0) * core * 0.65;
  vec2 dp = vec2(across * 9.0, along * 12.0 - t * 0.7);
  col += indigo * stars(dp, 2.5, t * 2.0) * cone * 1.1;
  float sparkle = pow(stars(dp * 0.8, 2.0, t * 1.4), 2.0);
  col += vec3(1.0) * sparkle * cone * 0.9;
  col += indigo * topSpill(uv, hit, uHit.z, 45.0, 1.1) * 0.45;
  col += indigo * sideSpill(uv, hit, uHit.z, -1.0, t, 0.06, 2.0) * 0.42;
  col += indigo * sideSpill(uv, hit, uHit.z, 1.0, t, 0.06, 2.0) * 0.42;
  col += vec3(0.9, 0.95, 1.0) * impact(uv, hit, 0.32, 0.07) * 0.25;
  col += indigo * collisionLight(uv, hit, uHit.z, t) * 0.5;
  return col;
}`,

    emberlight: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.56, 0.86);
  vec2 hit = hitPoint(0.55);
  float cone = spotCone(uv, o, hit, 0.02, 0.38, 0.32);
  float core = spotCone(uv, o, hit, 0.008, 0.07, 0.14);
  float along = beamAlong(uv, o, hit);
  float across = beamAcross(uv, o, hit);
  vec3 warm = vec3(1.0, 0.5, 0.24);
  vec3 col = vec3(0.012, 0.005, 0.004);
  float smoke = fbm(vec2(across * 3.0 + t * 0.06, along * 4.0 - t * 0.18));
  col += warm * cone * (0.12 + 0.2 * smoke);
  col += vec3(1.0, 0.82, 0.55) * core * 0.45;
  vec2 g = vec2(uv.x * 8.0, uv.y * 9.0 + t * 0.8);
  vec2 id = floor(g);
  vec2 f = fract(g) - 0.5;
  float h = hash(id + 3.0);
  float ember = (1.0 - smoothstep(0.0, 0.08, length(f))) * step(0.55, h);
  col += mix(vec3(1.0, 0.25, 0.05), vec3(1.0, 0.85, 0.45), h) * ember * (cone + 0.1) * 1.5;
  float broken = 0.65 + 0.35 * sin((uv.x - hit.x) * 45.0 + fbm(vec2(uv.x * 6.0, t * 0.3)) * 4.0);
  col += warm * topSpill(uv, hit, uHit.z, 45.0, 1.1) * broken * 0.55;
  col += warm * sideSpill(uv, hit, uHit.z, -1.0, t, 0.06, 2.0) * 0.42;
  col += warm * sideSpill(uv, hit, uHit.z, 1.0, t, 0.06, 2.0) * 0.42;
  col += warm * impact(uv, hit, 0.32, 0.07) * 0.3;
  col += warm * collisionLight(uv, hit, uHit.z, t) * 0.55;
  return col;
}`,
  };

  const VARIANTS = [
    { slug: 'cathedral', name: 'Cathedral', line: 'A warm shaft for a quiet room.', toneA: '#c9a45e', toneB: '#f0ddb8', angle: '45deg', y: '14%', brightness: '1.04', opacity: '0.22', style: 'soft', hit: 0.42 , castOrigin: [-0.72, 0.82] },
    { slug: 'searchlight', name: 'Searchlight', line: 'A moving spot, finding the object.', toneA: '#7fa8c9', toneB: '#bcd9ee', angle: '135deg', y: '16%', brightness: '1.03', opacity: '0.18', style: 'sweep', hit: 0.5 , castOrigin: [0.72, 0.84] },
    { slug: 'aurora-fall', name: 'Aurora Fall', line: 'A curtain of colour ending on the glass.', toneA: '#2fb89d', toneB: '#8c6bf0', angle: '90deg', y: '10%', brightness: '1.03', opacity: '0.20', style: 'curtain', hit: 0.52 , castOrigin: [0.0, 0.95] },
    { slug: 'golden-hour', name: 'Golden Hour', line: 'A low sun raking across the object.', toneA: '#d18a3d', toneB: '#ffdf9e', angle: '0deg', y: '12%', brightness: '1.05', opacity: '0.22', style: 'grazing', hit: 0.22 , castOrigin: [-0.85, 0.02] },
    { slug: 'moonbeam', name: 'Moonbeam', line: 'A soft cool wash from above.', toneA: '#6f8fc9', toneB: '#d3e7ff', angle: '135deg', y: '12%', brightness: '1.03', opacity: '0.18', style: 'soft', hit: 0.58 , castOrigin: [0.72, 0.88] },
    { slug: 'datafall', name: 'Datafall', line: 'A grid of signals landing on the screen.', toneA: '#2fb7b0', toneB: '#b8fff2', angle: '45deg', y: '12%', brightness: '1.03', opacity: '0.20', style: 'grid', hit: 0.48 , castOrigin: [-0.62, 0.92] },
    { slug: 'solar-flare', name: 'Solar Flare', line: 'A hard burst with nowhere to hide.', toneA: '#e08b3a', toneB: '#ffd9a0', angle: '135deg', y: '10%', brightness: '1.04', opacity: '0.18', style: 'flare', hit: 0.56 , castOrigin: [0.6, 0.82] },
    { slug: 'prism-split', name: 'Prism Split', line: 'A fan of colours striking the surface.', toneA: '#b06fd8', toneB: '#ffe3c0', angle: '45deg', y: '12%', brightness: '1.03', opacity: '0.18', style: 'prism', hit: 0.5 , castOrigin: [-0.72, 0.86] },
    { slug: 'cascade', name: 'Cascade', line: 'A pendant of light emptying onto the object.', toneA: '#5a90c9', toneB: '#d8f4ff', angle: '90deg', y: '14%', brightness: '1.03', opacity: '0.20', style: 'cascade', hit: 0.44 , castOrigin: [-0.3, 0.78] },
    { slug: 'halo-shaft', name: 'Halo Shaft', line: 'Rings of light breaking on the top edge.', toneA: '#b89a5c', toneB: '#cfe0ff', angle: '135deg', y: '12%', brightness: '1.03', opacity: '0.20', style: 'rings', hit: 0.6 , castOrigin: [0.64, 0.84] },
    { slug: 'meteor-stream', name: 'Meteor Stream', line: 'A shower of light with a wide impact.', toneA: '#5d75b8', toneB: '#ffe8c2', angle: '135deg', y: '14%', brightness: '1.03', opacity: '0.18', style: 'streaks', hit: 0.55 , castOrigin: [0.7, 0.9] },
    { slug: 'sanctuary', name: 'Sanctuary', line: 'Two soft sources meeting on the object.', toneA: '#3b8f87', toneB: '#bdeee6', angle: '90deg', y: '12%', brightness: '1.04', opacity: '0.22', style: 'twin', hit: 0.5 , castOrigin: [0.0, 0.9] },
    { slug: 'pulse', name: 'Pulse', line: 'A steady beam with a measured heartbeat.', toneA: '#7b5ec9', toneB: '#8fd8ff', angle: '90deg', y: '12%', brightness: '1.03', opacity: '0.18', style: 'pulse', hit: 0.5 , castOrigin: [0.0, 0.95] },
    { slug: 'starfall', name: 'Starfall', line: 'A soft beam carrying a sky of sparks.', toneA: '#6b78c9', toneB: '#dbe9ff', angle: '45deg', y: '14%', brightness: '1.03', opacity: '0.18', style: 'stars', hit: 0.48 , castOrigin: [-0.62, 0.9] },
    { slug: 'emberlight', name: 'Emberlight', line: 'A smoky warm cast with embers in the beam.', toneA: '#c96632', toneB: '#ffc98c', angle: '135deg', y: '12%', brightness: '1.05', opacity: '0.22', style: 'embers', hit: 0.55 , castOrigin: [0.56, 0.86] },
  ];

  const compile = (gl, type, source) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('Lumen shader failed:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  };

  const build = (gl, sceneSource) => {
    const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX);
    const fragment = compile(gl, gl.FRAGMENT_SHADER, HEADER + MAIN + sceneSource);
    if (!vertex || !fragment) return null;
    const program = gl.createProgram();
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.bindAttribLocation(program, 0, 'aPosition');
    gl.linkProgram(program);
    gl.deleteShader(vertex);
    gl.deleteShader(fragment);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('Lumen program failed:', gl.getProgramInfoLog(program));
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
    document.removeEventListener('visibilitychange', active.visibility);
    active.observer?.disconnect();
    active.gl.deleteVertexArray(active.vao);
    active.gl.deleteBuffer(active.buffer);
    active.gl.deleteProgram(active.program);
    active = null;
  };

  const mount = (canvas, slug, options = {}) => {
    const previousCanvas = active?.canvas || null;
    destroy();
    if (previousCanvas && previousCanvas !== canvas) {
      previousCanvas.getContext('webgl2')?.getExtension('WEBGL_lose_context')?.loseContext();
    }
    if (!canvas || !SCENES[slug]) return false;
    const gl = canvas.getContext('webgl2', {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      failIfMajorPerformanceCaveat: false,
    });
    if (!gl) return false;
    const program = build(gl, SCENES[slug]);
    if (!program) return false;
    const { vao, buffer } = makeSurface(gl);

    const uniforms = {
      resolution: gl.getUniformLocation(program, 'uResolution'),
      time: gl.getUniformLocation(program, 'uTime'),
      reveal: gl.getUniformLocation(program, 'uReveal'),
      cast: gl.getUniformLocation(program, 'uCast'),
      impact: gl.getUniformLocation(program, 'uImpact'),
      spread: gl.getUniformLocation(program, 'uSpread'),
      castOrigin: gl.getUniformLocation(program, 'uCastOrigin'),
      hitFrac: gl.getUniformLocation(program, 'uHitFrac'),
      hit: gl.getUniformLocation(program, 'uHit'),
      pointer: gl.getUniformLocation(program, 'uPointer'),
    };
    const delay = options.delay ?? 0.5;
    const startTime = performance.now();
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    let raf = 0;

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
      const elapsed = reduced ? delay + 3 : (now - startTime) / 1000;
      const cast = reduced ? 1.6 : Math.max(0, Math.min(1.6, (elapsed - delay) / 0.7));
      const impact = reduced ? 1 : Math.max(0, Math.min(1, (cast - 0.9) / 0.15));
      const spread = reduced ? 1 : Math.max(0, Math.min(1, (elapsed - delay - 0.7) / 0.7));
      const reveal = reduced || elapsed >= delay ? 1 : 0;
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(program);
      if (uniforms.resolution) gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
      if (uniforms.time) gl.uniform1f(uniforms.time, reduced ? 8 : elapsed);
      if (uniforms.reveal) gl.uniform1f(uniforms.reveal, reveal);
      if (uniforms.cast) gl.uniform1f(uniforms.cast, cast);
      if (uniforms.impact) gl.uniform1f(uniforms.impact, impact);
      if (uniforms.spread) gl.uniform1f(uniforms.spread, spread);
      const castOrigin = active?.castOrigin || [0.0, 0.9];
      if (uniforms.castOrigin) gl.uniform2f(uniforms.castOrigin, castOrigin[0], castOrigin[1]);
      if (uniforms.hitFrac) gl.uniform1f(uniforms.hitFrac, active?.hitFrac ?? 0.5);
      const hit = active?.hit || [0.0, -0.12, 0.34, 0.18];
      if (uniforms.hit) gl.uniform4f(uniforms.hit, hit[0], hit[1], hit[2], hit[3]);
      if (uniforms.pointer) gl.uniform2f(uniforms.pointer, 0.5, 0.5);
      gl.bindVertexArray(vao);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindVertexArray(null);
      if (reduced) return;
      raf = requestAnimationFrame(draw);
      if (active) active.raf = raf;
    };

    const start = () => {
      if (!raf && !document.hidden) {
        raf = requestAnimationFrame(draw);
        if (active) active.raf = raf;
      }
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
      hit: options.hit || [0.0, -0.12, 0.34, 0.18],
      hitFrac: options.hitFrac ?? 0.5,
      castOrigin: options.castOrigin || [0.0, 0.9],
      vao,
      buffer,
      program,
      gl,
      resize,
      visibility: onVisibility,
      observer: typeof ResizeObserver === 'function' ? new ResizeObserver(resize) : null,
    };
    window.addEventListener('resize', resize);
    document.addEventListener('visibilitychange', onVisibility);
    active.observer?.observe(canvas);
    resize();
    start();
    return true;
  };

  const setHit = (canvas, hit) => {
    if (!active || active.canvas !== canvas || !Array.isArray(hit)) return;
    active.hit = hit;
  };

  let snapshotSurface = null;

  const snapshot = (slug) => {
    if (!SCENES[slug]) return '';
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
    const program = build(gl, SCENES[slug]);
    if (!program) return '';
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(program);
    const resolution = gl.getUniformLocation(program, 'uResolution');
    const time = gl.getUniformLocation(program, 'uTime');
    const reveal = gl.getUniformLocation(program, 'uReveal');
    const cast = gl.getUniformLocation(program, 'uCast');
    const impact = gl.getUniformLocation(program, 'uImpact');
    const spread = gl.getUniformLocation(program, 'uSpread');
    const castOrigin = gl.getUniformLocation(program, 'uCastOrigin');
    const hitFrac = gl.getUniformLocation(program, 'uHitFrac');
    const hit = gl.getUniformLocation(program, 'uHit');
    if (resolution) gl.uniform2f(resolution, canvas.width, canvas.height);
    if (time) gl.uniform1f(time, 8.0);
    if (reveal) gl.uniform1f(reveal, 1.0);
    if (cast) gl.uniform1f(cast, 1.6);
    if (impact) gl.uniform1f(impact, 1.0);
    if (spread) gl.uniform1f(spread, 1.0);
    if (castOrigin) gl.uniform2f(castOrigin, 0.0, 0.9);
    if (hitFrac) gl.uniform1f(hitFrac, 0.5);
    if (hit) gl.uniform4f(hit, 0.0, -0.12, 0.34, 0.18);
    gl.bindVertexArray(vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.bindVertexArray(null);
    const url = canvas.toDataURL('image/jpeg', 0.84);
    gl.deleteProgram(program);
    return url;
  };

  window.LumenLab = { VARIANTS, mount, destroy, snapshot, setHit };
})();
