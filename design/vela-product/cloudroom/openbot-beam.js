(() => {
  const VERT = `#version 300 es
in vec2 aPosition;
void main() {
  gl_Position = vec4(aPosition, 0.0, 1.0);
}`;
  const FRAG = `#version 300 es
precision highp float;
out vec4 outColor;
uniform vec2 uResolution;
uniform float uDpr;
uniform float uScale;
uniform float uTime;
uniform float uBeamX;
uniform vec4 uSource;
uniform vec3 uWrap;
uniform float uFall;
uniform vec4 uSurface;
uniform float uRadius;
uniform vec3 uIntro;
uniform vec4 uPour;
uniform vec2 uRun;
uniform vec4 uLens;
uniform float uAtmosphere;
uniform vec4 uAmount;
uniform vec4 uShape;
uniform vec4 uExtra;
uniform vec4 uSmoke;
uniform float uSmokeDrift;
uniform vec3 uTint[16];

#define BEAM uTint[0]
#define LEFT uTint[1]
#define RIGHT uTint[2]
#define CYAN uTint[3]
#define PINK uTint[4]
#define LILAC uTint[5]
#define SKY uTint[6]
#define PURPLE uTint[7]
#define DEEP uTint[8]
#define MIST uTint[9]
#define NAVY uTint[10]
#define WARM uTint[11]
#define EMBER uTint[12]
#define WHITE uTint[13]
#define HAZE uTint[14]
#define LAVENDER uTint[15]

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
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

// The fog has no fine octaves, so its clouds are soft and have no sharp threads.
float softFbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
  for (int i = 0; i < 3; i++) {
    v += a * noise(p);
    p = m * p;
    a *= 0.5;
  }
  return v / 0.875;
}

// The smoke shape. detail adds the fine octaves that give threads and wisps; at 0 it is softFbm.
float smokeFbm(vec2 p, float detail) {
  float v = softFbm(p);
  if (detail < 0.01) return v;
  mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
  p = m * m * m * p;
  return (v * 0.875 + detail * (0.0625 * noise(p) + 0.03125 * noise(m * p))) / (0.875 + detail * 0.09375);
}

float sq(float x) {
  return x * x;
}

float roundBox(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

vec3 spill(float e, float fall, float strength, float front, float t, out vec3 glow) {
  glow = vec3(0.0);
  if (strength < 0.0005) return vec3(0.0);
  // uFall stretches the light that runs down the side; the reveal front stays in real pixels.
  float fr = max(fall, 0.0);
  float f = fr / uFall;
  float d = max(e, 0.0);
  float u = d / (6.0 + 0.28 * f);
  // The spill starts softly at the corner, and stays on the edge rather than bulging out.
  float start = smoothstep(-12.0, 18.0, fall);
  // Below the white core, a dimmer coloured glow runs much further down the edge, as on huly.io.
  float fade = max(exp(-sq(max(f - 125.0, 0.0) / 40.0)), 0.14 * smoothstep(60.0, 125.0, f) * exp(-sq(max(f - 170.0, 0.0) / 80.0)));
  // Bright and dim streaks run down the side, so the light visibly moves.
  float streak = 0.55 + 0.9 * noise(vec2(u * 5.0, f / 18.0 - t * 2.4));
  // A beam near the corner gives more than full strength; it must not turn the side into a white wall.
  float bright = pow(min(strength, 1.0), 1.5);
  float soft = sqrt(min(strength, 1.0));
  float warmth = smoothstep(90.0, 130.0, f);
  vec3 coreK = mix(WHITE, WARM, warmth * exp(-d / 9.0));
  // A thin bright line runs down the edge, with coloured light outside it, as on React Bits.
  float core = mix(1.6, 0.8, warmth) * exp(-sq(d / (3.5 + 0.03 * f))) * exp(-sq(max(f - 130.0, 0.0) / 38.0));
  float wall = 3.0 * exp(-sq(d / 1.6)) * exp(-sq(max(f - 100.0, 0.0) / 30.0));
  float bandCenter = 0.7 - 0.3 * smoothstep(30.0, 110.0, f) - 0.3 * smoothstep(110.0, 190.0, f);
  float offset = u - bandCenter;
  float decay = (1.05 - bandCenter) / 1.31;
  float profile = offset < 0.0 ? exp(-sq(offset / 0.3)) : exp(-offset / decay);
  profile *= 1.0 - smoothstep(1.0, 1.4, u);
  vec3 bandA = mix(CYAN, PINK, smoothstep(50.0, 75.0, f));
  bandA = mix(bandA, LILAC, smoothstep(100.0, 125.0, f));
  vec3 bandB = mix(SKY, PURPLE, smoothstep(50.0, 75.0, f));
  vec3 fanK = mix(bandA, bandB, smoothstep(0.06, 0.18, offset));
  fanK = mix(fanK, DEEP, smoothstep(0.22, 0.42, offset));
  vec3 thread = (WHITE * wall + coreK * core) * streak * bright;
  glow += fanK * 2.2 * profile * fade * streak * bright;
  glow += MIST * 0.7 * exp(-max(u - 1.0, 0.0) / 0.9) * smoothstep(0.7, 1.2, u) * fade * soft;
  glow += EMBER * 0.35 * exp(-sq((f - 165.0) / 22.0)) * exp(-d / 25.0) * bright;
  float lag = u * (6.0 + 0.28 * f) * 0.35;
  float reveal = 1.0 - smoothstep(front - 70.0, front + 8.0, fr + lag);
  glow *= start * reveal;
  return thread * start * reveal;
}

vec3 easeLight(vec3 light, float amount) {
  return -log(max(1.0 - amount * (1.0 - exp(-light)), vec3(0.0001)));
}

void main() {
  vec2 frag = vec2(gl_FragCoord.x, uResolution.y * uDpr - gl_FragCoord.y) / uDpr;
  vec2 p = frag / uScale;
  float t = uTime;
  float bx = uBeamX / uScale;
  float sl = uSurface.x / uScale;
  float sy = uSurface.y / uScale;
  float sr = uSurface.z / uScale;
  float sb = uSurface.w / uScale;
  float radius = uRadius / uScale;
  float dx = p.x - bx;
  float adx = abs(dx);
  float h = sy - p.y;
  float hp = max(h, 0.0);
  float above = smoothstep(-0.5, 0.5, h);
  float side = smoothstep(-24.0, 24.0, dx);
  vec3 sideK = mix(mix(LAVENDER, LEFT, smoothstep(40.0, 140.0, adx)), RIGHT, side);
  float beamWidth = uShape.x;
  float flare = uShape.y;
  float spread = 250.0 * uShape.z;
  float reachL = max(bx - sl, 1.0);
  float reachR = max(sr - bx, 1.0);
  float reach = dx < 0.0 ? reachL : reachR;
  float layerLen = clamp(0.29 * reach, 20.0, 100.0) * sqrt(uShape.z);
  float run = uRun.x;
  float bead = exp(-sq((adx - run + 16.0) / (18.0 + 0.04 * run))) * uRun.y;
  // The far side of the beam does not spill, so its light along the top stops well before the corner.
  float farSide = dx < 0.0 ? step(reachR, reachL) : 1.0 - step(reachR, reachL);
  float reachHere = spread * mix(1.0, 0.7, farSide);
  float bloom = uIntro.y;
  vec2 center = vec2((sl + sr) * 0.5, (sy + sb) * 0.5);
  vec2 halfSize = vec2((sr - sl) * 0.5, (sb - sy) * 0.5);
  float sd = roundBox(p - center, halfSize, radius);
  // Nothing is drawn inside the surface, and the app preview covers most of the hero.
  if (sd * uScale < -0.75) {
    outColor = vec4(0.0);
    return;
  }
  float outline = max(sd, 0.0);
  // The light that runs along the top reaches the high haze later, and its front gets softer with
  // height, so the haze grows as a soft curve and not as a vertical wall.
  float runSoft = 60.0 + 1.5 * outline;
  float runMask = 1.0 - smoothstep(run - runSoft, run + 0.25 * runSoft, adx + 0.5 * outline);
  float flowMask = (1.0 - smoothstep(0.75 * reachHere, 1.1 * reachHere, adx)) * runMask;
  float edgeMask = (1.0 - smoothstep(0.8 * reachHere, 1.25 * reachHere, adx)) * runMask;
  float below = max(p.y - sy - radius, 0.0);
  float hug = smoothstep(-0.5, 0.5, sd) * exp(-sq(below / 14.0));
  float hugWide = smoothstep(-0.5, 0.5, sd) * exp(-sq(below / 40.0));

  vec3 col = vec3(0.0);

  float breathe = 1.0 + uExtra.x * (0.12 * sin(t * 1.5708) + 0.08 * (noise(vec2(t * 0.5, 3.0)) - 0.5));

  float front = uIntro.x / uScale;
  float drop = smoothstep(front + 10.0, front - 110.0, p.y);
  float falling = 1.0 - smoothstep(sy - 24.0, sy, front);
  // A source element sits near the top of the canvas, so the beam is at full strength sooner.
  float sourced = uWrap.z;
  float top = mix(mix(0.3, 0.6, sourced), 1.0, smoothstep(0.0, 0.3 * sy, p.y));
  // A wide beam keeps a thin core: only its glow grows, so the beam does not turn into a white column.
  float beamCore = 24.0 * exp(-adx / (2.0 * min(beamWidth, 1.9)));
  float beamHalo = 1.3 * exp(-adx / (6.0 * min(beamWidth, 2.1))) + 0.3 * exp(-adx / (22.0 * beamWidth));
  vec3 haloK = mix(BEAM, sideK, exp(-hp / 80.0));
  // Beside the surface, the glow fades out below its top edge rather than stopping at a line.
  float fadeBelow = exp(-sq(max(-h, 0.0) / 70.0));
  col += (BEAM * beamCore + haloK * beamHalo) * top * fadeBelow * drop;
  col += sideK * 0.16 * exp(-adx / (45.0 * beamWidth)) * top * fadeBelow * drop;
  if (sourced > 0.5) {
    // The beam falls onto the source element, its light runs around the element's edge, and the
    // beam leaves again from the element's bottom edge.
    vec4 sb4 = uSource / uScale;
    vec2 sc = vec2(sb4.x + sb4.z, sb4.y + sb4.w) * 0.5;
    float sdS = roundBox(p - sc, vec2(sb4.z - sb4.x, sb4.w - sb4.y) * 0.5, uWrap.x / uScale);
    float outS = max(sdS, 0.0);
    float struck = smoothstep(sb4.y - 4.0, sb4.y + 12.0, front);
    float around = 1.0 - smoothstep(uWrap.y / uScale - 24.0, uWrap.y / uScale, adx);
    float near = 0.4 + 0.6 * exp(-adx / 28.0);
    // The canvas has fewer pixels than the screen, so the element draws its own sharp rim. The
    // beam adds only soft light, which starts under the element's edge so that no step shows there.
    float wrapLine = 1.4 * exp(-outS / 3.0) * near;
    float wrapHalo = (exp(-outS / 7.0) + 0.35 * exp(-outS / 22.0)) * 1.1 * near;
    col += (mix(sideK, WHITE, 0.6) * wrapLine + sideK * wrapHalo) * struck * around * breathe * smoothstep(-3.0, -1.0, sdS);
    float mouth = exp(-length(vec2(dx / 2.2, max(p.y - sb4.w, 0.0))) / 9.0) * smoothstep(sb4.w - 3.0, sb4.w + 6.0, p.y);
    col += mix(WHITE, BEAM, 0.3) * mouth * 7.0 * drop;
  }
  // The falling tip stays as narrow as the beam's glow. A wide tip spread sideways under the source.
  float head = exp(-sq((p.y - front) / 24.0)) * falling;
  float headWidth = min(beamWidth, 1.9);
  col += mix(WHITE, BEAM, 0.35) * head * (7.0 * exp(-adx / (2.6 * headWidth)) + 0.3 * exp(-adx / (6.0 * headWidth)));

  float psi = adx * (outline + 1.0);
  float rise = max(hp - 17.9 * flare, 0.0);
  float skirtAmp = 16.6 * (exp(-sq(rise / (11.8 * flare))) + 0.3 * exp(-rise / (60.0 * flare)));
  float skirtSpread = mix(134.0, 155.0, side) * breathe * flare * flare * mix(0.35, 1.0, min(bloom, 1.0)) * max(bloom, 1.0);
  float skirt = skirtAmp * exp(-psi / skirtSpread) * exp(-sq(adx / (max(layerLen, 60.0 * sqrt(uShape.z)) * 1.48))) * hug;
  // The tall light along the top thins out before the corners; only the edge line and the spill turn over them.
  float overTop = smoothstep(sl - 10.0, sl + 90.0, p.x) * (1.0 - smoothstep(sr - 90.0, sr + 10.0, p.x));
  col += sideK * skirt * flowMask * overTop * min(bloom, 1.0);

  float layer = 12.0 * exp(-adx / layerLen) * exp(-outline / (0.19 * layerLen)) * (1.0 + 0.4 * bead);
  col += sideK * layer * hug * flowMask * overTop * min(bloom, 1.0);
  float washMask = smoothstep(sl - 60.0, sl + 6.0, p.x) * (1.0 - smoothstep(sr - 6.0, sr + 45.0, p.x));
  float wash = 1.4 * exp(-adx / 100.0) * exp(-outline / 30.0);
  col += mix(BEAM, sideK, 0.25) * wash * hugWide * washMask * flowMask * overTop * min(bloom, 1.0);
  float highWash = 0.7 * exp(-adx / 140.0) * exp(-outline / 60.0);
  col += mix(BEAM, NAVY, 0.5) * highWash * hugWide * washMask * runMask * overTop * min(bloom, 1.0);

  float sideSign = dx < 0.0 ? 0.0 : 1.0;
  float along = 0.5 * log((hp + 1.0) / (adx + 1.0));
  float lineScale = 6.0;
  float lp = log(max(psi, 1.0) / (120.0 * flare * flare)) * lineScale;
  float lineIndex = floor(lp) + sideSign * 97.0;
  float wobble = 0.22 * (noise(vec2(lineIndex * 3.7, t * 0.4 + along)) - 0.5);
  float lf = fract(lp + wobble) - 0.5;
  float gradient = lineScale * length(vec2(hp + 1.0, adx)) / max(psi, 1.0) / uScale;
  float halfWidth = max(0.35 * gradient, 0.0001);
  float drawWidth = max(halfWidth, 0.6 * gradient);
  float lineShape = exp(-lf * lf / (drawWidth * drawWidth)) * (halfWidth / drawWidth);
  float lineOn = step(0.3, hash(vec2(lineIndex, 1.3)));
  float lineFlow = smoothstep(0.25, 0.85, noise(vec2(lineIndex * 5.3 + 2.0, along * 3.0 + t * 1.1)));
  float flareArea = 120.0 * flare * flare;
  float lineEnvelope = smoothstep(0.85 * flareArea, 1.4 * flareArea, psi) * (1.0 - smoothstep(3.2 * flareArea, 7.5 * flareArea, psi));
  lineEnvelope *= exp(-adx / (layerLen * 1.5)) * hug * flowMask * overTop * (1.0 - smoothstep(35.0 * flare, 80.0 * flare, hp));
  float lineBright = mix(0.35, 1.0, hash(vec2(lineIndex, 4.0)));
  col += mix(sideK, WHITE, 0.6) * lineShape * lineOn * lineFlow * lineEnvelope * lineBright * 2.6 * uAmount.w * min(bloom, 1.0);

  float edgeGlow = 22.8 * exp(-sq(adx / (0.6 * reachHere))) * edgeMask * (1.0 + 0.9 * bead);
  // The line turns over the corner and fades out down the side, so it does not stop at a step.
  float onTop = exp(-sq(max(p.y - sy - radius, 0.0) / 50.0));
  float edgeLine = (0.5 * exp(-sq(max(sd, 0.0) / 3.0)) + 0.25 * exp(-max(sd, 0.0) / 10.0)) * edgeGlow * onTop;
  col += mix(sideK, WHITE, 0.6) * edgeLine * min(bloom, 1.0);

  float pour = uShape.w;
  float baseline = exp(-sq(130.0 / 150.0));
  float strengthR = exp(-sq(reachR / (150.0 * uShape.z))) * (1.0 - smoothstep(0.8 * spread, 1.25 * spread, reachR)) / baseline * pour;
  float strengthL = exp(-sq(reachL / (150.0 * uShape.z))) * (1.0 - smoothstep(0.8 * spread, 1.25 * spread, reachL)) / baseline * pour;
  // Only the side nearer the beam spills, as on huly.io.
  strengthR *= step(reachR, reachL);
  strengthL *= 1.0 - step(reachR, reachL);
  float fallY = p.y - sy - 0.3 * radius;
  float rightHalf = smoothstep(center.x - 4.0, center.x + 4.0, p.x);
  float haloR = exp(-length(p - vec2(sr - 0.3 * radius, sy + 0.3 * radius)) / 40.0) * sqrt(strengthR);
  float haloL = exp(-length(p - vec2(sl + 0.3 * radius, sy + 0.3 * radius)) / 40.0) * sqrt(strengthL);
  vec3 glowR;
  vec3 glowL;
  // The spill also touches the top edge, so it waits there until the light along the top has run past.
  // It fades out slowly down the side, and is gone well before the bottom edge.
  float spillMask = runMask * exp(-sq(max(fallY - 0.05 * (sb - sy), 0.0) / (0.28 * (sb - sy))));
  col += spill(sd, fallY, strengthR, uPour.y, t, glowR) * rightHalf * spillMask;
  col += spill(sd, fallY, strengthL, uPour.x, t, glowL) * (1.0 - rightHalf) * spillMask;
  col += easeLight((glowR * rightHalf + NAVY * 0.9 * haloR) * spillMask, uPour.w);
  col += easeLight((glowL * (1.0 - rightHalf) + NAVY * 0.9 * haloL) * spillMask, uPour.z);

  vec2 toLens = frag - uLens.xy;
  float lens = exp(-dot(toLens, toLens) / sq(uLens.z)) * uLens.w;
  col *= 1.0 + 0.35 * lens * hug;
  float fogAmount = uAmount.y * uAtmosphere;
  float near = exp(-adx / 150.0);
  float envelope = near * (0.3 + 0.4 * exp(-hp / 50.0) + 1.2 * exp(-p.y / 120.0));
  if (fogAmount > 0.001 && h > -160.0 && envelope * max(0.25, 6.0 * exp(-adx / 50.0)) > 0.002) {
    // Each visit has its own smoke: puff size, fine detail, edge sharpness, swirl and drift speed.
    float ft = t * uSmokeDrift;
    vec2 fq = vec2(dx / 1.7, h) / (120.0 * uSmoke.x) + vec2(ft * 0.012, -ft * 0.03) + uExtra.zw;
    vec2 fw = vec2(softFbm(fq + vec2(1.7, 9.2) + ft * 0.03), softFbm(fq + vec2(8.3, 2.8) - ft * 0.025));
    float cloud = smokeFbm(fq + uSmoke.w * fw, uSmoke.y);
    float density = smoothstep(mix(0.4, 0.52, uSmoke.z), mix(0.95, 0.66, uSmoke.z), cloud);
    float lit = exp(-adx / 50.0);
    float fogMask = fadeBelow;
    float reached = smoothstep(front + 80.0, front - 40.0, p.y);
    vec3 fog = (HAZE * 0.45 + BEAM * 6.0 * lit * reached) * density * envelope * fogMask;
    float surfaceLit = exp(-adx / (layerLen * 1.3)) * exp(-outline / 28.0) * hugWide * flowMask * washMask * min(bloom, 1.0);
    fog += sideK * 2.2 * surfaceLit * (smoothstep(0.3, 0.85, cloud) - 0.35) * fogMask;
    float smoke = smoothstep(0.2, 0.9, cloud);
    fog += BEAM * 0.35 * exp(-adx / (22.0 * beamWidth)) * (0.35 + 0.9 * smoke) * fogMask * top * drop;
    col += fog * fogAmount * (1.0 + 1.2 * lens);
  }

  float haze = col.b;
  float dotGrid = 1.0 - smoothstep(0.16, 0.32, length(fract(frag / 3.0) - 0.5));
  col += BEAM * dotGrid * clamp(haze, 0.0, 1.5) * (1.0 - smoothstep(1.5, 4.0, haze)) * (0.18 + 0.3 * lens) * uExtra.y;
  if (lens > 0.002) {
    vec2 drift = frag - vec2(0.0, t * 14.0);
    vec2 sparkCell = floor(drift / 7.0);
    vec2 sparkPos = (sparkCell + 0.2 + 0.6 * vec2(hash(sparkCell + 5.1), hash(sparkCell + 2.3))) * 7.0;
    float sparkDist = length(drift - sparkPos);
    float sparkOn = step(0.8, hash(sparkCell + 8.8));
    float sparkTwinkle = 0.5 + 0.5 * sin(t * 5.0 + hash(sparkCell + 3.7) * 6.283);
    col += WHITE * exp(-sparkDist * sparkDist / 0.35) * sparkOn * sparkTwinkle * lens * clamp(haze * 1.5, 0.0, 1.0) * 1.6 * uAmount.z;
  }

  float dustAmount = uAmount.z;
  if (dustAmount > 0.001) {
    float dustLp = log(max(psi, 1.0) / (120.0 * flare * flare)) * 2.2;
    vec2 flowCell = vec2(dustLp / 1.1, (along + t * 0.22) / 0.06);
    vec2 cellId = floor(flowCell) + vec2(sideSign * 131.0, 0.0);
    vec2 cellJitter = vec2(hash(cellId + 1.7), hash(cellId + 8.3));
    vec2 particleFlow = (floor(flowCell) + 0.1 + cellJitter * 0.8) * vec2(1.1, 0.06);
    float particlePsi = 120.0 * flare * flare * exp(particleFlow.x / 2.2);
    float particleAlong = particleFlow.y - t * 0.22;
    float particleX = sqrt(particlePsi * exp(-2.0 * particleAlong)) - 1.0;
    float particleH = sqrt(particlePsi * exp(2.0 * particleAlong)) - 1.0;
    float particleDist = length(vec2(adx - particleX, hp - particleH)) * uScale;
    float particleOn = step(0.62, hash(cellId + 3.1));
    float particleTwinkle = 0.5 + 0.5 * sin(t * 5.0 + hash(cellId + 9.2) * 6.283);
    float dustZone = exp(-sq(log(max(psi, 1.0) / (200.0 * flare * flare)) / 0.55)) * hug * flowMask * smoothstep(0.0, 6.0, outline);
    col += WHITE * exp(-particleDist * particleDist / 0.3) * particleOn * particleTwinkle * dustZone * 3.0 * dustAmount * min(bloom, 1.0);

    vec2 glitterCell = vec2(dx / 3.0, (p.y - t * 26.0) / 9.0);
    vec2 glitterId = floor(glitterCell);
    vec2 glitterJitter = vec2(hash(glitterId + 4.4), hash(glitterId + 2.9));
    vec2 glitterPos = (glitterId + 0.15 + glitterJitter * 0.7) * vec2(3.0, 9.0);
    float glitterDist = length(vec2(dx, p.y - t * 26.0) - glitterPos) * uScale;
    float glitterOn = step(0.7, hash(glitterId + 6.6));
    float glitterTwinkle = 0.5 + 0.5 * sin(t * 6.0 + hash(glitterId + 1.1) * 6.283);
    float glitterZone = exp(-adx / (5.0 * beamWidth)) * smoothstep(1.5, 4.0, adx / beamWidth) * above * smoothstep(25.0, 70.0, hp) * top * drop;
    col += WHITE * exp(-glitterDist * glitterDist / 0.25) * glitterOn * glitterTwinkle * glitterZone * 2.2 * dustAmount * uAtmosphere;
  }

  col *= uAmount.x * (1.0 + uIntro.z * 0.9 * exp(-length(vec2(dx, h * 1.6)) / 70.0));
  col *= smoothstep(-0.75, 0.25, sd * uScale);

  vec3 emit = 1.0 - exp(-col);
  // The dark haze changes slowly, so 8-bit output shows each step as a stripe. Pixel noise of about
  // one step hides the stripes. Empty pixels get no noise.
  float dither = (hash(gl_FragCoord.xy) + hash(gl_FragCoord.xy + 17.31) - 1.0) / 255.0;
  emit = clamp(emit + dither * clamp(max(emit.r, max(emit.g, emit.b)) * 255.0, 0.0, 1.0), 0.0, 1.0);
  outColor = vec4(emit, max(emit.r, max(emit.g, emit.b)));
}`;

  const STOPS = [
    [0, 1], [-11.5, 0.77], [9.5, 0.95], [-53.5, 0.79],
    [39.5, 0.77], [16, 0.81], [-16.3, 0.92], [13.3, 0.96],
    [3, 1.01], [1, 0.68], [1, 1.15], [142.5, 0.51],
    [113.6, 0.28], [-7.5, 0.13], [-2.7, 0.32], [7.5, 0.81],
  ];
  const HUE_RANGE = { from: 30, to: 200 };
  const SATURATION = 0.78;
  const BEAM_COLOR = '#a46cf5';
  const BEAM_POSITION = 0.78;
  const INTRO_DURATION = 7.6;
  const INTRO_DELAY = 2500;
  const MAX_DPR = 1.5;
  const MAX_PIXELS = 1600;
  const MAX_SCALE = 1.25;
  const BEAM_GAP = 56;
  const SURFACE_PAD = 6;

  const SETTINGS = {
    intensity: 1.6,
    beamWidth: 3.2,
    flare: 2.5,
    spread: 3.8,
    spill: 1,
    fall: 2.2,
    fog: 0.3,
    smokeSize: 1,
    smokeDetail: 0,
    smokeEdge: 0,
    smokeSwirl: 0.8,
    smokeDrift: 1,
    dust: 1.4,
    streaks: 0.8,
    dots: 0.8,
    pulse: 1.4,
    speed: 1.8,
    seed: 3,
  };

  const PRESETS = {
    default: SETTINGS,
    soft: {
      ...SETTINGS,
      intensity: 1.3,
      beamWidth: 4.2,
      flare: 3,
      spread: 4.2,
      fall: 1.6,
      fog: 0.5,
      dust: 0.6,
      streaks: 0.4,
      dots: 0.5,
      pulse: 0.8,
      speed: 1.2,
    },
    laser: {
      ...SETTINGS,
      intensity: 1.8,
      beamWidth: 1.2,
      flare: 1.5,
      spread: 3.2,
      fall: 2.6,
      fog: 0.15,
      dust: 1,
      streaks: 1.6,
      dots: 1,
      pulse: 1.6,
      speed: 2.2,
    },
    haze: {
      ...SETTINGS,
      intensity: 1.5,
      beamWidth: 3,
      flare: 2.8,
      spread: 4,
      fall: 2,
      fog: 0.6,
      dust: 1.9,
      streaks: 1,
      dots: 1.2,
      pulse: 1.2,
      speed: 1.6,
      seed: 16,
    },
  };

  const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
  const easeOut = (value) => 1 - (1 - clamp(value, 0, 1)) ** 3;
  const impactCurve = (value) => {
    const damping = 0.62;
    const omega = 8 * Math.sqrt(1 - damping * damping);
    return 1 - Math.exp(-4.96 * value) * (Math.cos(omega * value) + (damping * 8 / omega) * Math.sin(omega * value));
  };

  const clampHue = (hue) => {
    const value = ((hue % 360) + 360) % 360;
    return value <= HUE_RANGE.from || value >= HUE_RANGE.to
      ? value
      : value - HUE_RANGE.from < HUE_RANGE.to - value
        ? HUE_RANGE.from
        : HUE_RANGE.to;
  };

  const rgbToHsl = ([red, green, blue]) => {
    const max = Math.max(red, green, blue);
    const min = Math.min(red, green, blue);
    const delta = max - min;
    let hue = 0;
    if (delta > 0) {
      hue = max === red
        ? ((green - blue) / delta) % 6
        : max === green
          ? (blue - red) / delta + 2
          : (red - green) / delta + 4;
    }
    return [(hue * 60 + 360) % 360, max > 0 ? delta / max : 0];
  };

  const hslToRgb = (hue, saturation) => {
    const sector = (((hue % 360) + 360) % 360) / 60;
    const chroma = saturation;
    const x = chroma * (1 - Math.abs((sector % 2) - 1));
    const lift = 1 - chroma;
    const [red, green, blue] = sector < 1
      ? [chroma, x, 0]
      : sector < 2
        ? [x, chroma, 0]
        : sector < 3
          ? [0, chroma, x]
          : sector < 4
            ? [0, x, chroma]
            : sector < 5
              ? [x, 0, chroma]
              : [chroma, 0, x];
    return [red + lift, green + lift, blue + lift];
  };

  const hexToRgb = (hex) => {
    const value = Number.parseInt(hex.replace('#', ''), 16);
    return [((value >> 16) & 255) / 255, ((value >> 8) & 255) / 255, (value & 255) / 255];
  };

  const createTints = (color, hueSpread, nativeHue = false) => {
    const [hue, saturation] = rgbToHsl(color);
    const tints = new Float32Array(STOPS.length * 3);
    STOPS.forEach(([offset, multiplier], index) => {
      tints.set(
        hslToRgb(
          nativeHue ? hue + offset * hueSpread : clampHue(hue + offset * hueSpread),
          clamp(saturation * multiplier, 0, 1),
        ),
        index * 3,
      );
    });
    return tints;
  };

  const compileShader = (gl, type, source) => {
    const shader = gl.createShader(type);
    if (!shader) return null;
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      gl.deleteShader(shader);
      return null;
    }
    return shader;
  };

  const createProgram = (gl) => {
    const vertex = compileShader(gl, gl.VERTEX_SHADER, VERT);
    const fragment = compileShader(gl, gl.FRAGMENT_SHADER, FRAG);
    const program = gl.createProgram();
    if (!vertex || !fragment || !program) return null;
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.linkProgram(program);
    gl.deleteShader(vertex);
    gl.deleteShader(fragment);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      gl.deleteProgram(program);
      return null;
    }
    return program;
  };

  const getUniforms = (gl, program) => {
    const find = (name) => gl.getUniformLocation(program, name);
    return {
      uResolution: find('uResolution'),
      uDpr: find('uDpr'),
      uScale: find('uScale'),
      uTime: find('uTime'),
      uBeamX: find('uBeamX'),
      uSource: find('uSource'),
      uWrap: find('uWrap'),
      uFall: find('uFall'),
      uSurface: find('uSurface'),
      uRadius: find('uRadius'),
      uIntro: find('uIntro'),
      uPour: find('uPour'),
      uRun: find('uRun'),
      uLens: find('uLens'),
      uAtmosphere: find('uAtmosphere'),
      uAmount: find('uAmount'),
      uShape: find('uShape'),
      uExtra: find('uExtra'),
      uSmoke: find('uSmoke'),
      uSmokeDrift: find('uSmokeDrift'),
      uTint: find('uTint'),
    };
  };

  let active = null;

  const destroy = () => {
    if (!active) return;
    cancelAnimationFrame(active.raf);
    clearTimeout(active.delayTimer);
    active.ro?.disconnect();
    active.io?.disconnect();
    window.removeEventListener('pointermove', active.onPointerMove);
    document.documentElement.removeEventListener('pointerleave', active.onPointerLeave);
    document.removeEventListener('visibilitychange', active.onVisibility);
    active.canvas.removeEventListener('webglcontextlost', active.onContextLost);
    active.source?.removeAttribute('data-beam-source');
    active.source?.removeAttribute('data-beam-struck');
    active.source?.style.removeProperty('--landing-beam-rim');
    active.canvas.removeAttribute('data-source');
    active.gl.deleteBuffer(active.buffer);
    active.gl.deleteVertexArray(active.vao);
    active.gl.deleteProgram(active.program);
    active = null;
  };

  const mount = (root) => {
    destroy();
    const canvas = root.querySelector('.ob-beam-canvas');
    if (!canvas) return;
    const surfaceKey = canvas.dataset.surface || 'preview';
    const field = surfaceKey === 'field';
    const surface = field
      ? null
      : root.querySelector(surfaceKey === 'shape' ? '.ob-energy-shape' : '.ob-preview');
    const origin = canvas.dataset.origin || 'header';
    const sources = {
      header: '.ob-header .ob-btn-primary',
      title: '.ob-title-logo',
      primary: '.ob-download-split',
      secondary: '.ob-actions > .ob-btn-secondary',
    };
    const source = origin === 'none' ? null : root.querySelector(sources[origin] || sources.header);
    const settings = PRESETS[canvas.dataset.settings] || PRESETS.default;
    const beamColor = canvas.dataset.color || BEAM_COLOR;
    const nativeHue = canvas.dataset.native === 'true';
    const beamPosition = Number(canvas.dataset.position ?? BEAM_POSITION);
    const introDuration = Number(canvas.dataset.intro ?? INTRO_DURATION);
    const introRise = Number(canvas.dataset.rise ?? 1.4);
    const introDelay = Number(canvas.dataset.delay ?? INTRO_DELAY);
    const pace = Number(canvas.dataset.pace ?? 1);
    const gl = canvas.getContext('webgl2', {
      alpha: true,
      premultipliedAlpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      failIfMajorPerformanceCaveat: false,
    });
    if ((!field && !surface) || !gl) return;
    const program = createProgram(gl);
    if (!program) return;
    const uniforms = getUniforms(gl, program);
    const vao = gl.createVertexArray();
    const buffer = gl.createBuffer();
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, 'aPosition');
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);

    const tints = createTints(hexToRgb(beamColor), 1, nativeHue);
    const rim = Array.from(tints.subarray(0, 3), (value) => Math.round(value * 255)).join(', ');
    source?.setAttribute('data-beam-source', '');
    source?.style.setProperty('--landing-beam-rim', rim);
    canvas.toggleAttribute('data-source', Boolean(source));

    const state = {
      width: 1,
      height: 1,
      ratio: 1,
      time: 0,
      introTime: 0,
      delayOver: false,
      visible: true,
      struck: false,
      pointer: { x: 0, y: 0, inside: false },
      lens: { x: 0, y: 0, vx: 0, vy: 0, strength: 0, placed: false },
    };
    let raf = 0;
    let last = 0;

    const resize = () => {
      const width = Math.max(1, canvas.clientWidth);
      const height = Math.max(1, canvas.clientHeight);
      const dpr = Math.min(window.devicePixelRatio || 1, MAX_DPR);
      const largest = Math.max(width, height) * dpr;
      const ratio = largest > MAX_PIXELS ? (dpr * MAX_PIXELS) / largest : dpr;
      const pixelWidth = Math.max(1, Math.round(width * ratio));
      const pixelHeight = Math.max(1, Math.round(height * ratio));
      if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
        canvas.width = pixelWidth;
        canvas.height = pixelHeight;
      }
      state.width = width;
      state.height = height;
      state.ratio = canvas.width / width;
    };

    const surfaceRect = () => {
      const canvasBox = canvas.getBoundingClientRect();
      const box = surface.getBoundingClientRect();
      if (box.width > 0) {
        const styles = getComputedStyle(surface);
        const radius = Math.max(
          Number.parseFloat(styles.borderTopLeftRadius) || 0,
          Number.parseFloat(styles.borderTopRightRadius) || 0,
        );
        return {
          left: box.left - canvasBox.left,
          top: box.top - canvasBox.top,
          right: box.right - canvasBox.left,
          bottom: box.bottom - canvasBox.top,
          radius: Math.min(radius, box.width / 2, box.height / 2),
          real: true,
        };
      }
      return {
        left: -state.width * 2,
        top: state.height,
        right: state.width * 3,
        bottom: state.height * 3,
        radius: 0,
        real: false,
      };
    };

    const draw = () => {
      const width = state.width;
      const height = state.height;
      const scale = clamp(Math.min(width / 585, height / 507), 0.2, MAX_SCALE);
      let surfaceBox = field
        ? { left: -width * 2, top: height, right: width * 3, bottom: height * 3, radius: 0, real: false }
        : surfaceRect();
      const canvasBox = canvas.getBoundingClientRect();
      let beamX = beamPosition * width;
      let sourceBox;
      if (source) {
        const box = source.getBoundingClientRect();
        if (box.width > 0) {
          const center = (box.left + box.right) / 2 - canvasBox.left;
          const safe = surfaceBox.radius + SURFACE_PAD;
          if (surfaceBox.real && center >= safe && center >= surfaceBox.left + safe && center <= surfaceBox.right - safe) {
            const radius = Number.parseFloat(getComputedStyle(source).borderTopLeftRadius) || 0;
            sourceBox = {
              left: box.left - canvasBox.left,
              top: box.top - canvasBox.top,
              right: box.right - canvasBox.left,
              bottom: box.bottom - canvasBox.top,
              radius: Math.min(radius, box.width / 2, box.height / 2),
            };
            beamX = center;
          }
        }
      }

      if (surfaceBox.real && surfaceKey !== 'shape') {
        const safe = surfaceBox.radius + SURFACE_PAD;
        if (beamX > surfaceBox.right - safe) {
          gl.clearColor(0, 0, 0, 0);
          gl.clear(gl.COLOR_BUFFER_BIT);
          if (state.struck) {
            source?.removeAttribute('data-beam-struck');
            state.struck = false;
          }
          return;
        }
        if (beamX >= surfaceBox.left) {
          beamX = clamp(beamX, surfaceBox.left + safe, surfaceBox.right - safe);
        } else {
          surfaceBox = {
            left: -width * 2,
            top: height,
            right: width * 3,
            bottom: height * 3,
            radius: 0,
            real: false,
          };
        }
      }

      const startY = -0.15 * height;
      const dropLength = surfaceBox.top - startY + 0.3 * height;
      const intro = state.introTime;
      const settled = intro >= introDuration;
      const front = settled
        ? surfaceBox.top + 0.3 * height
        : startY + dropLength * clamp(intro / introRise, 0, 1) ** 1.3;
      const elapsed = intro - introRise * (Math.max(0, surfaceBox.top - startY) / dropLength) ** (1 / 1.3);
      const bloom = settled ? 1 : elapsed <= 0 ? 0 : impactCurve(elapsed);
      const impact = settled || elapsed <= 0 ? 0 : Math.exp(-elapsed / 0.35) * (1 - Math.exp(-elapsed / 0.05));
      const spillLimit = settings.spread * 250 * 1.3 + 80;
      const runTime = elapsed;
      const run = settled ? 1e5 : elapsed <= 0 ? -40 : spillLimit * (1 - Math.exp(-1.15 * runTime));
      const runStrength = settled || elapsed <= 0 ? 0 : Math.exp(-1.15 * runTime) * (1 - Math.exp(-runTime / 0.08));

      const spillRun = (distance) => {
        if (settled) return [1e3, 1];
        if (elapsed <= 0) return [-distance, 0];
        const traveled = Math.min(distance, spillLimit * 0.97);
        const solved = -Math.log(1 - traveled / spillLimit) / 1.15;
        const remaining = runTime - solved;
        const ramp = clamp((remaining + 0.15) / 0.8, 0, 1);
        const ease = ramp * ramp * ramp * (ramp * (ramp * 6 - 15) + 10);
        return remaining <= 0
          ? [run - traveled, ease]
          : [Math.min(1.15 * (spillLimit - traveled), 320) * remaining + 240 * remaining * remaining, ease];
      };

      const [pourLeft, pourLeftEdge] = spillRun((beamX - surfaceBox.left) / scale);
      const [pourRight, pourRightEdge] = spillRun((surfaceBox.right - beamX) / scale);
      const sourceImpact = sourceBox
        ? introRise * clamp((sourceBox.top - startY) / dropLength, 0, 1) ** (1 / 1.3)
        : 0;
      const wrapLength = settled ? 1e5 : Math.max(0, intro - sourceImpact) * 420;
      const struck = settled || intro >= sourceImpact;
      if (struck !== state.struck) {
        if (struck) source?.setAttribute('data-beam-struck', '');
        else source?.removeAttribute('data-beam-struck');
        state.struck = struck;
      }
      const atmosphere = settled
        ? 1
        : elapsed <= 0
          ? 0.3 * easeOut(intro / introRise)
          : 0.3 + 0.7 * (1 - Math.exp(-1.3 * runTime));

      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.useProgram(program);
      gl.uniform2f(uniforms.uResolution, width, height);
      gl.uniform1f(uniforms.uDpr, state.ratio);
      gl.uniform1f(uniforms.uScale, scale);
      gl.uniform1f(uniforms.uTime, state.time);
      gl.uniform1f(uniforms.uBeamX, beamX);
      gl.uniform4f(
        uniforms.uSource,
        sourceBox?.left ?? 0,
        sourceBox?.top ?? 0,
        sourceBox?.right ?? 0,
        sourceBox?.bottom ?? 0,
      );
      gl.uniform3f(uniforms.uWrap, sourceBox?.radius ?? 0, wrapLength, sourceBox ? 1 : 0);
      gl.uniform1f(uniforms.uFall, settings.fall);
      gl.uniform4f(uniforms.uSurface, surfaceBox.left, surfaceBox.top, surfaceBox.right, surfaceBox.bottom);
      gl.uniform1f(uniforms.uRadius, surfaceBox.radius);
      gl.uniform3f(uniforms.uIntro, front, bloom, impact);
      gl.uniform4f(uniforms.uPour, pourLeft, pourRight, pourLeftEdge, pourRightEdge);
      gl.uniform2f(uniforms.uRun, run, runStrength);
      gl.uniform4f(uniforms.uLens, state.lens.x, state.lens.y, 95 * scale, state.lens.strength);
      gl.uniform1f(uniforms.uAtmosphere, atmosphere);
      gl.uniform4f(uniforms.uAmount, settings.intensity, settings.fog, settings.dust, settings.streaks);
      gl.uniform4f(uniforms.uShape, settings.beamWidth, settings.flare, settings.spread, settings.spill);
      gl.uniform4f(
        uniforms.uExtra,
        settings.pulse,
        settings.dots,
        34.34 + settings.seed * 7.31,
        11.58 + settings.seed * 3.17,
      );
      gl.uniform4f(
        uniforms.uSmoke,
        settings.smokeSize,
        settings.smokeDetail,
        settings.smokeEdge,
        settings.smokeSwirl,
      );
      gl.uniform1f(uniforms.uSmokeDrift, settings.smokeDrift);
      gl.uniform3fv(uniforms.uTint, tints);
      gl.bindVertexArray(vao);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.bindVertexArray(null);
    };

    const loop = (now) => {
      raf = 0;
      if (active) active.raf = 0;
      if (!state.visible || document.hidden) {
        last = 0;
        return;
      }
      if (!last) last = now;
      const delta = Math.min(0.05, (now - last) / 1000) || 1 / 60;
      last = now;
      state.time += delta * settings.speed * pace;
      if (state.delayOver && state.introTime < introDuration) {
        state.introTime = Math.min(introDuration, state.introTime + delta);
      }

      const pointer = state.pointer;
      const lens = state.lens;
      const inside = pointer.inside;
      if (inside && !lens.placed) {
        lens.x = pointer.x;
        lens.y = pointer.y;
        lens.vx = 0;
        lens.vy = 0;
        lens.placed = true;
      }
      const forceX = 120 * (pointer.x - lens.x) - 19 * lens.vx;
      const forceY = 120 * (pointer.y - lens.y) - 19 * lens.vy;
      lens.vx += forceX * delta;
      lens.vy += forceY * delta;
      lens.x += lens.vx * delta;
      lens.y += lens.vy * delta;
      const target = inside ? 1 : 0;
      lens.strength += (target - lens.strength) * (1 - Math.exp(-delta / (target > lens.strength ? 0.3 : 0.55)));
      if (lens.strength < 0.001 && !inside) lens.placed = false;

      draw();
      raf = requestAnimationFrame(loop);
      if (active) active.raf = raf;
    };

    const start = () => {
      if (!raf && state.visible && !document.hidden) {
        last = 0;
        raf = requestAnimationFrame(loop);
        if (active) active.raf = raf;
      }
    };

    const onPointerMove = (event) => {
      const box = canvas.getBoundingClientRect();
      const x = event.clientX - box.left;
      const y = event.clientY - box.top;
      state.pointer.x = x;
      state.pointer.y = y;
      state.pointer.inside = x >= 0 && y >= 0 && x <= box.width && y <= box.height;
      start();
    };

    const onPointerLeave = () => {
      state.pointer.inside = false;
      start();
    };

    const onVisibility = () => {
      if (document.hidden) {
        cancelAnimationFrame(raf);
        raf = 0;
        if (active) active.raf = 0;
        last = 0;
      } else {
        start();
      }
    };

    const onContextLost = () => {
      cancelAnimationFrame(raf);
      raf = 0;
      if (active) active.raf = 0;
    };

    const delayTimer = setTimeout(() => {
      state.delayOver = true;
      start();
    }, introDelay);

    const ro = new ResizeObserver(() => {
      resize();
      start();
    });
    ro.observe(canvas);
    const io = new IntersectionObserver((entries) => {
      state.visible = entries.some((entry) => entry.isIntersecting);
      if (state.visible) start();
    });
    io.observe(canvas);
    window.addEventListener('pointermove', onPointerMove, { passive: true });
    document.documentElement.addEventListener('pointerleave', onPointerLeave);
    document.addEventListener('visibilitychange', onVisibility);
    canvas.addEventListener('webglcontextlost', onContextLost);

    active = {
      raf,
      delayTimer,
      ro,
      io,
      canvas,
      source,
      gl,
      program,
      vao,
      buffer,
      onPointerMove,
      onPointerLeave,
      onVisibility,
      onContextLost,
    };
    resize();
    draw();
    start();
  };

  window.velaBeam = { mount, destroy };
})();
