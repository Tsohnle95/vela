(() => {
  const HEADER = `#version 300 es
precision highp float;
out vec4 outColor;
uniform vec2 uResolution;
uniform float uTime;
uniform float uReveal;
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

float castHead(vec2 uv, vec2 origin, vec2 target, float delay) {
  vec2 d = target - origin;
  float len = max(length(d), 0.001);
  vec2 dir = d / len;
  vec2 rel = uv - origin;
  float along = dot(rel, dir) / len;
  float across = abs(dot(rel, vec2(-dir.y, dir.x)));
  float head = clamp((uTime - delay) / 0.75, 0.0, 1.0);
  float pulse = exp(-pow((along - head) / 0.09, 2.0)) * exp(-across * across * 26.0);
  float isLive = smoothstep(delay, delay + 0.08, uTime) * (1.0 - smoothstep(delay + 1.0, delay + 1.7, uTime));
  return pulse * isLive;
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
  col *= 1.0 - smoothstep(0.62, 1.25, length(uv * vec2(0.55, 1.0)));
  col *= uReveal;
  outColor = vec4(col, 1.0);
}
`;

  const SCENES = {
    cathedral: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.68, 0.78);
  vec2 tg = vec2(0.0, -0.26);
  float cone = beamCone(uv, o, tg, 0.15, 0.36);
  float core = beamCone(uv, o, tg, 0.012, 0.09);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 warm = vec3(1.0, 0.87, 0.66);
  vec3 col = vec3(0.006, 0.007, 0.01);
  col += warm * cone * 0.15;
  col += warm * core * 0.8;
  float bands = 0.5 + 0.5 * sin(across * 34.0 + t * 0.35);
  col += mix(vec3(0.5, 0.75, 1.0), vec3(1.0, 0.82, 0.45), bands) * cone * 0.05;
  vec2 dp = vec2(across * 9.0, along * 12.0 - t * 0.45);
  col += warm * stars(dp, 2.0, t * 0.8) * cone * 0.85;
  col += vec3(1.0, 0.75, 0.45) * exp(-length(uv - tg) * 7.0) * 0.16;
  col += warm * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    searchlight: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.7, 0.8);
  vec2 tg = vec2(0.02 + sin(t * 0.3) * 0.04, -0.26);
  float cone = beamCone(uv, o, tg, 0.06, 0.14);
  float core = beamCone(uv, o, tg, 0.014, 0.05);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 cool = vec3(0.78, 0.94, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.011);
  col += cool * cone * 0.18;
  col += cool * core * 0.82;
  float q = along * 10.0 - t * 2.8;
  float dash = smoothstep(0.0, 0.08, fract(q)) * (1.0 - smoothstep(0.08, 0.24, fract(q)));
  float gate = step(0.4, hash(vec2(floor(q), 9.0)));
  float stream = exp(-across * across * 180.0) * dash * gate;
  col += cool * stream * 0.95;
  col += cool * exp(-across * across * 22.0) * cone * 0.08;
  col += cool * exp(-length(uv - o) * 10.0) * 0.35;
  col += vec3(0.5, 0.8, 1.0) * exp(-length(uv - tg) * 8.0) * 0.2;
  col += cool * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'aurora-fall': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.62, 0.82);
  vec2 tg = vec2(0.02, -0.27);
  float cone = beamCone(uv, o, tg, 0.13, 0.3);
  float core = beamCone(uv, o, tg, 0.01, 0.05);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  float n1 = fbm(vec2(across * 4.0 + t * 0.1, along * 6.0 - t * 0.6));
  float n2 = fbm(vec2(across * 9.0 - t * 0.15, along * 3.0 + 3.0));
  vec3 mint = vec3(0.1, 0.95, 0.68);
  vec3 violet = vec3(0.55, 0.4, 1.0);
  vec3 c = mix(mint, violet, 0.5 + 0.5 * sin(along * 4.0 + t * 0.25));
  vec3 col = vec3(0.004, 0.007, 0.012);
  col += c * cone * (0.12 + 0.55 * pow(n1, 2.0));
  col += c * core * 0.75;
  col += c * pow(max(0.0, n2 - 0.2), 2.0) * cone * 0.4;
  float filament = pow(max(0.0, 1.0 - abs(fract(along * 7.0 - t * 1.2) - 0.5) * 2.0), 10.0);
  col += c * filament * exp(-across * across * 220.0) * 0.55;
  float q = along * 9.0 - t * 1.5;
  float dash = smoothstep(0.0, 0.06, fract(q)) * (1.0 - smoothstep(0.06, 0.2, fract(q)));
  float gate = step(0.45, hash(vec2(floor(q), 4.0)));
  col += c * exp(-across * across * 110.0) * dash * gate * 0.85;
  col += c * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'golden-hour': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.7, 0.82);
  vec2 tg = vec2(0.0, -0.29);
  float cone = beamCone(uv, o, tg, 0.16, 0.42);
  float core = beamCone(uv, o, tg, 0.014, 0.08);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 gold = vec3(1.0, 0.76, 0.42);
  vec3 col = vec3(0.02, 0.012, 0.006);
  col += vec3(0.16, 0.08, 0.02) * smoothstep(-0.5, 0.35, uv.y);
  col += gold * cone * 0.18;
  col += gold * core * 0.9;
  vec2 g1 = vec2(across * 11.0, along * 13.0 - t * 0.5);
  vec2 id1 = floor(g1);
  vec2 f1 = fract(g1) - 0.5;
  float h1 = hash(id1 + 3.0);
  float mote1 = (1.0 - smoothstep(0.0, 0.08, length(f1))) * step(0.5, h1);
  col += mix(vec3(1.0, 0.42, 0.1), vec3(1.0, 0.92, 0.62), h1) * mote1 * cone * 1.4;
  vec2 g2 = vec2(across * 23.0, along * 25.0 - t * 0.8);
  vec2 id2 = floor(g2);
  vec2 f2 = fract(g2) - 0.5;
  float h2 = hash(id2 + 8.0);
  float mote2 = (1.0 - smoothstep(0.0, 0.1, length(f2))) * step(0.72, h2);
  col += vec3(1.0, 0.7, 0.3) * mote2 * cone * 0.9;
  col += gold * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    moonbeam: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.62, 0.88);
  vec2 tg = vec2(-0.02, -0.26);
  float cone = beamCone(uv, o, tg, 0.06, 0.22);
  float core = beamCone(uv, o, tg, 0.01, 0.04);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 cool = vec3(0.62, 0.8, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.012);
  col += cool * cone * 0.2;
  col += cool * core * 0.95;
  float fog = fbm(vec2(along * 3.0 + t * 0.08, across * 5.0 - t * 0.12));
  col += cool * fog * cone * 0.22;
  vec2 dp = vec2(across * 12.0, along * 16.0 - t * 0.3);
  col += cool * stars(dp, 2.5, t * 1.6) * cone * 1.1;
  col += vec3(0.75, 0.88, 1.0) * exp(-length(uv - o) * 7.0) * 0.4;
  col += vec3(0.4, 0.6, 0.95) * exp(-length(uv - tg) * 8.0) * 0.18;
  col += cool * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    datafall: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.58, 0.9);
  vec2 tg = vec2(0.0, -0.28);
  float cone = beamCone(uv, o, tg, 0.12, 0.28);
  float core = beamCone(uv, o, tg, 0.01, 0.05);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 cyan = vec3(0.25, 0.95, 0.85);
  vec3 col = vec3(0.003, 0.008, 0.01);
  col += cyan * cone * 0.12;
  col += cyan * core * 0.8;
  float lane = floor(across * 24.0);
  float h = hash(vec2(lane, 7.0));
  float q = along * 11.0 - t * (1.2 + h * 1.1);
  float dash = smoothstep(0.0, 0.06, fract(q)) * (1.0 - smoothstep(0.06, 0.18, fract(q)));
  float gate = step(0.45, h);
  col += cyan * exp(-across * across * 120.0) * dash * gate * 1.1;
  float grid = 1.0 - smoothstep(0.0, 0.012, abs(fract(across * 24.0) - 0.5));
  col += cyan * grid * cone * 0.12;
  float scan = exp(-pow((along - fract(t * 0.15)) * 10.0, 2.0));
  col += cyan * scan * cone * 0.18;
  col += cyan * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'solar-flare': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.58, 0.74);
  vec2 tg = vec2(-0.02, -0.28);
  float cone = beamCone(uv, o, tg, 0.18, 0.36);
  float core = beamCone(uv, o, tg, 0.018, 0.1);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 hot = vec3(1.0, 0.68, 0.32);
  vec3 col = vec3(0.01, 0.006, 0.004);
  col += hot * cone * 0.16;
  col += vec3(1.0, 0.9, 0.7) * core * 0.9;
  float shimmer = fbm(vec2(uv.x * 9.0, uv.y * 6.0 - t * 1.6));
  col += hot * pow(shimmer, 3.0) * cone * 0.25;
  float q = along * 9.0 - t * 3.0;
  float dash = smoothstep(0.0, 0.05, fract(q)) * (1.0 - smoothstep(0.05, 0.18, fract(q)));
  float gate = step(0.55, hash(vec2(floor(q), 5.0)));
  col += vec3(1.0, 0.85, 0.55) * exp(-across * across * 120.0) * dash * gate * 1.0;
  for (int i = 1; i <= 3; i++) {
    float fi = float(i);
    vec2 p = o + (tg - o) * fi * 0.28;
    vec3 tint = i == 1 ? vec3(1.0, 0.55, 0.25) : (i == 2 ? vec3(1.0, 0.8, 0.45) : vec3(0.7, 0.85, 1.0));
    col += tint * exp(-length(uv - p) * 34.0) * 0.14;
  }
  col += hot * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'prism-split': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.66, 0.82);
  vec2 tg = vec2(0.0, -0.27);
  float cone = beamCone(uv, o, tg, 0.13, 0.3);
  float core = beamCone(uv, o, tg, 0.016, 0.06);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 col = vec3(0.004, 0.005, 0.01);
  col += vec3(0.9, 0.95, 1.0) * cone * 0.1;
  col += vec3(1.0) * core * 0.55;
  float split = smoothstep(0.35, 0.75, along);
  vec2 tgR = tg + vec2(-0.18, 0.0);
  vec2 tgG = tg;
  vec2 tgB = tg + vec2(0.18, 0.0);
  col += vec3(1.0, 0.3, 0.35) * beamCone(uv, o, tgR, 0.06, 0.16) * split * 0.65;
  col += vec3(0.3, 1.0, 0.55) * beamCone(uv, o, tgG, 0.06, 0.16) * split * 0.65;
  col += vec3(0.35, 0.55, 1.0) * beamCone(uv, o, tgB, 0.06, 0.16) * split * 0.65;
  float q = along * 8.0 - t * 1.6;
  float dash = smoothstep(0.0, 0.06, fract(q)) * (1.0 - smoothstep(0.06, 0.2, fract(q)));
  float gate = step(0.4, hash(vec2(floor(q), 4.0)));
  vec3 hue = mix(vec3(1.0, 0.35, 0.4), vec3(0.4, 0.7, 1.0), 0.5 + 0.5 * sin(along * 7.0 + t * 0.4));
  col += hue * exp(-across * across * 110.0) * dash * gate * 0.9;
  col += vec3(1.0, 0.95, 0.9) * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    cascade: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.18, 0.95);
  vec2 tg = vec2(0.04, -0.3);
  float cone = beamCone(uv, o, tg, 0.13, 0.26);
  float core = beamCone(uv, o, tg, 0.018, 0.07);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 blue = vec3(0.55, 0.82, 1.0);
  vec3 col = vec3(0.004, 0.007, 0.013);
  col += blue * cone * 0.16;
  col += blue * core * 0.82;
  float lane = floor(across * 20.0);
  float h = hash(vec2(lane, 2.0));
  float q = along * 16.0 - t * (2.0 + h * 1.2);
  float dash = smoothstep(0.0, 0.06, fract(q)) * (1.0 - smoothstep(0.06, 0.2, fract(q)));
  float gate = step(0.35, h);
  col += blue * exp(-across * across * 90.0) * dash * gate * 1.5;
  float mist = fbm(uv * 4.0 + vec2(t * 0.1, -t * 0.08));
  col += blue * pow(mist, 3.0) * exp(-length(uv - tg) * 5.0) * 0.35;
  col += vec3(0.9, 0.97, 1.0) * exp(-abs(length(uv - tg) - 0.06) * 18.0) * 0.35;
  col += blue * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'halo-shaft': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.62, 0.8);
  vec2 tg = vec2(0.0, -0.27);
  float cone = beamCone(uv, o, tg, 0.08, 0.2);
  float core = beamCone(uv, o, tg, 0.01, 0.04);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 gold = vec3(1.0, 0.78, 0.42);
  vec3 blue = vec3(0.45, 0.72, 1.0);
  vec3 col = vec3(0.004, 0.006, 0.011);
  col += mix(blue, gold, 0.5 + 0.5 * sin(along * 4.0 + t * 0.2)) * cone * 0.16;
  col += gold * core * 0.85;
  float rings = pow(max(0.0, 1.0 - abs(fract(along * 10.0 - t * 0.8) - 0.5) * 2.0), 10.0);
  col += gold * rings * exp(-across * across * 150.0) * 0.6;
  float halo = 1.0 - smoothstep(0.0, 0.012, abs(length(uv - tg) - 0.17));
  col += mix(blue, gold, 0.5) * halo * 0.22;
  col += vec3(0.9, 0.8, 0.55) * exp(-length(uv - tg) * 8.0) * 0.2;
  col += gold * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    'meteor-stream': `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.7, 0.86);
  vec2 tg = vec2(-0.02, -0.3);
  float cone = beamCone(uv, o, tg, 0.06, 0.18);
  float core = beamCone(uv, o, tg, 0.008, 0.04);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 col = vec3(0.003, 0.006, 0.014);
  col += vec3(0.75, 0.85, 1.0) * stars(uv * 1.6 + vec2(t * 0.01, 0.0), 22.0, t) * 0.75;
  col += vec3(0.7, 0.85, 1.0) * cone * 0.12;
  col += vec3(0.9, 0.95, 1.0) * core * 0.85;
  float lane = floor(across * 26.0);
  float h = hash(vec2(lane, 3.0));
  float q = along * 7.0 - t * (1.5 + h * 1.2);
  float dash = smoothstep(0.0, 0.04, fract(q)) * (1.0 - smoothstep(0.04, 0.14, fract(q)));
  float gate = step(0.45, h);
  float laneGlow = exp(-pow(fract(across * 26.0) - 0.5, 2.0) * 90.0);
  col += mix(vec3(0.5, 0.7, 1.0), vec3(1.0, 0.92, 0.75), h) * laneGlow * dash * gate * 1.3;
  col += vec3(1.0, 0.9, 0.7) * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    sanctuary: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.72, 0.9);
  vec2 tg = vec2(0.0, -0.28);
  float cone = beamCone(uv, o, tg, 0.19, 0.42);
  float core = beamCone(uv, o, tg, 0.012, 0.08);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 teal = vec3(0.35, 0.85, 0.78);
  vec3 col = vec3(0.004, 0.008, 0.01);
  col += teal * cone * 0.14;
  col += teal * core * 0.7;
  float fog = fbm(vec2(across * 3.0 + t * 0.05, along * 4.0 - t * 0.15));
  col += teal * fog * cone * 0.25;
  col += vec3(0.5, 0.9, 0.85) * beamCone(uv, o + vec2(0.08, 0.0), tg + vec2(0.12, 0.0), 0.05, 0.2) * 0.3;
  col += vec3(0.3, 0.7, 0.7) * beamCone(uv, o + vec2(0.16, 0.02), tg + vec2(0.22, 0.02), 0.035, 0.18) * 0.2;
  vec2 dp = vec2(across * 8.0, along * 11.0 - t * 0.35);
  col += teal * stars(dp, 2.0, t * 1.1) * cone * 0.8;
  col += vec3(0.6, 0.9, 0.85) * exp(-length(uv - tg) * 6.0) * 0.16;
  col += teal * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    pulse: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.6, 0.82);
  vec2 tg = vec2(0.0, -0.27);
  float cone = beamCone(uv, o, tg, 0.09, 0.24);
  float core = beamCone(uv, o, tg, 0.012, 0.05);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 violet = vec3(0.62, 0.4, 1.0);
  vec3 cyan = vec3(0.35, 0.9, 1.0);
  vec3 col = vec3(0.004, 0.005, 0.012);
  col += violet * cone * 0.14;
  col += mix(violet, cyan, 0.5 + 0.5 * sin(along * 6.0 + t * 0.5)) * core * 0.9;
  float pulse = pow(max(0.0, 1.0 - abs(fract(along * 8.0 - t * 1.4) - 0.5) * 2.0), 12.0);
  col += mix(violet, cyan, 0.5 + 0.5 * sin(along * 6.0 + t)) * pulse * exp(-across * across * 140.0) * 0.8;
  float q = along * 12.0 - t * 2.2;
  float dash = smoothstep(0.0, 0.05, fract(q)) * (1.0 - smoothstep(0.05, 0.16, fract(q)));
  float gate = step(0.5, hash(vec2(floor(q), 6.0)));
  col += cyan * exp(-across * across * 280.0) * dash * gate * 1.0;
  col += violet * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    starfall: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(-0.6, 0.9);
  vec2 tg = vec2(0.0, -0.29);
  float cone = beamCone(uv, o, tg, 0.11, 0.3);
  float core = beamCone(uv, o, tg, 0.01, 0.05);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 indigo = vec3(0.5, 0.62, 1.0);
  vec3 col = vec3(0.003, 0.005, 0.015);
  col += vec3(0.7, 0.8, 1.0) * stars(uv * 1.8 + vec2(0.0, t * 0.01), 20.0, t) * 0.8;
  col += indigo * cone * 0.12;
  col += vec3(0.9, 0.95, 1.0) * core * 0.85;
  vec2 dp = vec2(across * 9.0, along * 12.0 - t * 0.7);
  col += indigo * stars(dp, 2.5, t * 2.0) * cone * 1.3;
  col += vec3(1.0) * pow(stars(dp, 1.4, t * 1.6), 2.0) * cone * 1.2;
  col += indigo * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,

    emberlight: `
vec3 scene(vec2 uv, float t) {
  vec2 o = beamOrigin(0.6, 0.85);
  vec2 tg = vec2(0.0, -0.27);
  float cone = beamCone(uv, o, tg, 0.11, 0.32);
  float core = beamCone(uv, o, tg, 0.012, 0.06);
  float along = beamAlong(uv, o, tg);
  float across = beamAcross(uv, o, tg);
  vec3 warm = vec3(1.0, 0.5, 0.24);
  vec3 col = vec3(0.012, 0.005, 0.004);
  col += warm * cone * 0.16;
  col += vec3(1.0, 0.82, 0.55) * core * 0.85;
  float smoke = fbm(vec2(uv.x * 3.0 + t * 0.08, uv.y * 4.0 - t * 0.18));
  col += warm * pow(smoke, 2.0) * cone * 0.35;
  vec2 g = vec2(uv.x * 8.0, uv.y * 11.0 + t * 0.9);
  vec2 id = floor(g);
  vec2 f = fract(g) - 0.5;
  float h = hash(id + 3.0);
  vec2 off = vec2((hash(id + 1.0) - 0.5) * 0.8, 0.0);
  float d = length((f - off) * vec2(1.0, 1.5));
  float ember = (1.0 - smoothstep(0.0, 0.09, d)) * step(0.55, h);
  col += mix(vec3(1.0, 0.25, 0.05), vec3(1.0, 0.85, 0.45), h) * ember * cone * 1.8;
  col += warm * exp(-length(uv - tg) * 7.0) * 0.22;
  col += warm * castHead(uv, o, tg, 0.5) * 2.0;
  return col;
}`,
  };

  const VARIANTS = [
    { slug: 'cathedral', name: 'Cathedral', line: 'A warm shaft for a quiet room.', toneA: '#c9a45e', toneB: '#f0ddb8', angle: '45deg', x: '56%', y: '14%', brightness: '1.1', opacity: '0.46' },
    { slug: 'searchlight', name: 'Searchlight', line: 'A cold finger finding the dark.', toneA: '#7fa8c9', toneB: '#bcd9ee', angle: '135deg', x: '44%', y: '16%', brightness: '1.06', opacity: '0.34' },
    { slug: 'aurora-fall', name: 'Aurora Fall', line: 'Colour falling with the light.', toneA: '#2fb89d', toneB: '#8c6bf0', angle: '45deg', x: '58%', y: '18%', brightness: '1.09', opacity: '0.44' },
    { slug: 'golden-hour', name: 'Golden Hour', line: 'Late sun on the workspace.', toneA: '#d18a3d', toneB: '#ffdf9e', angle: '45deg', x: '54%', y: '14%', brightness: '1.12', opacity: '0.5' },
    { slug: 'moonbeam', name: 'Moonbeam', line: 'A little moonlight, channelled.', toneA: '#6f8fc9', toneB: '#d3e7ff', angle: '135deg', x: '44%', y: '16%', brightness: '1.08', opacity: '0.42' },
    { slug: 'datafall', name: 'Datafall', line: 'The signal arrives as light.', toneA: '#2fb7b0', toneB: '#b8fff2', angle: '45deg', x: '54%', y: '16%', brightness: '1.1', opacity: '0.44' },
    { slug: 'solar-flare', name: 'Solar Flare', line: 'Too bright to look away from.', toneA: '#e08b3a', toneB: '#ffd9a0', angle: '135deg', x: '44%', y: '12%', brightness: '1.08', opacity: '0.38' },
    { slug: 'prism-split', name: 'Prism Split', line: 'One beam, three colours.', toneA: '#b06fd8', toneB: '#ffe3c0', angle: '45deg', x: '50%', y: '16%', brightness: '1.08', opacity: '0.4' },
    { slug: 'cascade', name: 'Cascade', line: 'Light that falls like water.', toneA: '#5a90c9', toneB: '#d8f4ff', angle: '90deg', x: '50%', y: '18%', brightness: '1.1', opacity: '0.42' },
    { slug: 'halo-shaft', name: 'Halo Shaft', line: 'Rings around a steady stream.', toneA: '#b89a5c', toneB: '#cfe0ff', angle: '135deg', x: '44%', y: '16%', brightness: '1.09', opacity: '0.44' },
    { slug: 'meteor-stream', name: 'Meteor Stream', line: 'A storm of small arrivals.', toneA: '#5d75b8', toneB: '#ffe8c2', angle: '135deg', x: '44%', y: '18%', brightness: '1.1', opacity: '0.44' },
    { slug: 'sanctuary', name: 'Sanctuary', line: 'Light with room to breathe.', toneA: '#3b8f87', toneB: '#bdeee6', angle: '45deg', x: '56%', y: '14%', brightness: '1.1', opacity: '0.46' },
    { slug: 'pulse', name: 'Pulse', line: 'Energy in measured beats.', toneA: '#7b5ec9', toneB: '#8fd8ff', angle: '135deg', x: '44%', y: '16%', brightness: '1.1', opacity: '0.44' },
    { slug: 'starfall', name: 'Starfall', line: 'Stars caught in the current.', toneA: '#6b78c9', toneB: '#dbe9ff', angle: '45deg', x: '56%', y: '18%', brightness: '1.08', opacity: '0.42' },
    { slug: 'emberlight', name: 'Emberlight', line: 'Warm light, embers and all.', toneA: '#c96632', toneB: '#ffc98c', angle: '135deg', x: '44%', y: '16%', brightness: '1.12', opacity: '0.5' },
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
      const elapsed = reduced ? delay + 2 : (now - startTime) / 1000;
      const reveal = reduced ? 1 : Math.max(0, Math.min(1, (elapsed - delay) / 0.55));
      const smoothReveal = reveal * reveal * (3 - 2 * reveal);
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(program);
      if (uniforms.resolution) gl.uniform2f(uniforms.resolution, canvas.width, canvas.height);
      if (uniforms.time) gl.uniform1f(uniforms.time, reduced ? 8 : elapsed);
      if (uniforms.reveal) gl.uniform1f(uniforms.reveal, smoothReveal);
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
    if (resolution) gl.uniform2f(resolution, canvas.width, canvas.height);
    if (time) gl.uniform1f(time, 8.0);
    if (reveal) gl.uniform1f(reveal, 1.0);
    gl.bindVertexArray(vao);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.bindVertexArray(null);
    const url = canvas.toDataURL('image/jpeg', 0.84);
    gl.deleteProgram(program);
    return url;
  };

  window.LumenLab = { VARIANTS, mount, destroy, snapshot };
})();
