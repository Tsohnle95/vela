// The four independently moving planes remain in the same SVG coordinate space.
// This keeps the landscape aligned when the hero, index card, and interlude crop it differently.
function velaAtmosphere(config, instance = "hero") {
  const id = `${config.id}-${instance}`;
  let seed = [...id].reduce((value, char) => (value * 33 + char.charCodeAt(0)) >>> 0, 5381);
  const random = () => ((seed = (1664525 * seed + 1013904223) >>> 0) / 4294967296);
  const dark = !config.light;
  const starColor = dark ? "#eaf8ef" : "#ffffff";
  const stars = Array.from({ length: dark ? 106 : 27 }, () => {
    const x = Math.round(random() * 1600), y = Math.round(random() * 580);
    const radius = random() > .94 ? 2.4 : .5 + random() * .8;
    return `<circle cx="${x}" cy="${y}" r="${radius.toFixed(1)}" opacity="${(.19 + random() * .6).toFixed(2)}"/>`;
  }).join("");
  const craters = Array.from({ length: config.celestial === "saturn" ? 7 : 13 }, () => {
    const angle = random() * Math.PI * 2, distance = Math.sqrt(random()) * (config.celestial === "saturn" ? 45 : 57);
    return `<circle cx="${(1200 + Math.cos(angle) * distance).toFixed(1)}" cy="${(270 + Math.sin(angle) * distance).toFixed(1)}" r="${(2 + random() * 6).toFixed(1)}" fill="${config.horizon}" opacity=".1"/>`;
  }).join("");
  const satellites = config.celestial === "saturn" ? `<ellipse cx="1200" cy="270" rx="174" ry="49" transform="rotate(-23 1200 270)" fill="none" stroke="${config.tint || config.glow}" stroke-width="14" opacity=".5"/><ellipse cx="1200" cy="270" rx="190" ry="57" transform="rotate(-23 1200 270)" fill="none" stroke="${starColor}" stroke-width="2" opacity=".56"/><g class="rotating-marker"><circle cx="1374" cy="270" r="9" fill="${starColor}"/><path d="M1359 270h30m-15-15v30" stroke="${config.tint || config.glow}" stroke-width="2"/></g>` : "";
  const meteorShower = config.celestial === "meteor" ? `<g class="shooting-star shooting-star-one"><path d="M1300 20 1180 95" fill="none" stroke="url(#meteor-tail-one-${id})" stroke-width="2.1" stroke-linecap="round"/><circle cx="1180" cy="95" r="2.6" fill="${starColor}"/></g><g class="shooting-star shooting-star-two"><path d="M1150 70 1060 140" fill="none" stroke="url(#meteor-tail-two-${id})" stroke-width="1.7" stroke-linecap="round"/><circle cx="1060" cy="140" r="2.1" fill="${config.tint || config.glow}"/></g>` : "";
  const celestial = `<g class="celestial-body" transform="translate(${config.celestial === "saturn" ? -110 : 0} 0)"><circle cx="1200" cy="270" r="238" fill="url(#halo-${id})"/>${satellites}<circle cx="1200" cy="270" r="${config.celestial === "saturn" ? 69 : config.celestial === "planet" ? 91 : 79}" fill="url(#planet-${id})" opacity=".91"/>${craters}${config.celestial === "crescent" ? `<circle cx="1168" cy="243" r="79" fill="${config.sky}" opacity=".94"/>` : ""}${config.celestial === "eclipse" ? `<circle cx="1200" cy="270" r="72" fill="${config.sky}"/><circle cx="1200" cy="270" r="86" fill="none" stroke="${config.glow}" stroke-width="5" opacity=".67"/>` : ""}${config.celestial === "aurora" ? `<path d="M0 347Q278 65 620 241t512-82q216-83 468-38" fill="none" stroke="${config.glow}" stroke-width="113" opacity=".19"/>` : ""}${meteorShower}${config.celestial === "beacon" ? `<path d="M800 598 1490 78" fill="none" stroke="${config.glow}" stroke-width="3" opacity=".32"/>` : ""}</g>`;
  const mountain = config.terrain === "alpine";
  const city = config.terrain === "city";
  const water = config.terrain === "water";
  const distantCity = city ? Array.from({ length: 32 }, (_, index) => {
    const x = index * 51 + Math.round(random() * 12);
    const width = 29 + Math.round(random() * 19);
    const height = 74 + Math.round(random() * 122);
    const top = 748 - height;
    const windows = [];
    for (let row = top + 12; row < 740; row += 15) {
      for (let column = x + 5; column < x + width - 3; column += 9) {
        if (random() > .35) windows.push(`<rect x="${column}" y="${row}" width="2.5" height="4"/>`);
      }
    }
    return `<path d="M${x} 748v-${height}h${width}v${height}z" fill="url(#far-${id})" opacity="${(.56 + random() * .3).toFixed(2)}"/><path d="M${x + 4} ${top + 9}v${height - 22}" stroke="${starColor}" stroke-width="1" opacity=".08"/><g fill="${config.glow}" opacity=".52">${windows.join("")}</g>`;
  }).join("") : "";
  const farTerrain = city
    ? `<g>${distantCity}</g><path d="M0 748h1600v192H0z" fill="url(#far-${id})" opacity=".74"/>`
    : water
      ? `<path d="M0 645q297-82 574 23t541-34q268-89 485-22v328H0z" fill="url(#far-${id})"/><path d="M0 680q370-39 725 22t875-49" fill="none" stroke="${config.glow}" stroke-width="3" opacity=".26"/>`
      : config.terrain === "mist"
        ? `<path d="M0 753q159-176 366-119t363-100q211-142 401-21t470-42v469H0z" fill="url(#far-${id})" opacity=".59"/><path d="M0 719q300-59 590 21t591-36q218-89 419-42" fill="none" stroke="${starColor}" stroke-width="22" opacity=".17"/><path d="M0 775q305-101 556 5t543-7q281-98 501-61" fill="none" stroke="${config.glow}" stroke-width="28" opacity=".16"/>`
      : `<path d="M0 781 48 723l46 19 47-109 30 9 88-135 23 24 43-57 83 77 36-32 54-139 35 21 31-55 48 34 36 72 38-38 71 69 46-49 57-85 29 18 39-59 49 30 71 105 42-39 66 48 48-91 59 12 36-75 39 9 59 92 58-38 52 51 48-84 23 17 43-55 48 41 89 114v337H0z" fill="url(#far-${id})"/><g fill="${config.glow}" opacity=".22"><path d="m251 519 31 12 43-57 44 41-56-19z"/><path d="m475 408 23-28 35 21 31-55 48 34-50 5-27 34-36-15z"/><path d="m838 382 29 18 39-59 49 30-43-6-37 50z"/><path d="m1252 463 36-75 39 9 59 92-55-48-40-22z"/></g>`;
  const cityBackFacade = "M0 790v-98h84v-48h77v95h96v-55h63v-103h77v135h90v-65h71v91h96v-125h80v87h113v-62h73v-105h84v172h109v-78h74v93h101v-117h84v135h106v-88h102v136H0z";
  const cityFrontFacade = "M0 810v-107h107v59h84V653h99v110h84v-70h101v61h87V615h76v139h109v-84h93v96h117V645h84v119h105v-51h93v80h111v-110h83v127H0z";
  const cityGeometry = `<path d="${cityBackFacade}" fill="url(#near-${id})"/><path d="${cityFrontFacade}" fill="${config.foreground}" opacity=".55"/>`;
  const cityWindowClip = city
    ? `<clipPath id="city-facades-${id}" clipPathUnits="userSpaceOnUse"><path d="${cityBackFacade}"/><path d="${cityFrontFacade}"/></clipPath>`
    : "";
  // Roof-run bounds mirror the silhouette paths; rear windows stop at the foreground roofline.
  const backBuildingFacades = [
    [0, 84, 692], [84, 161, 644], [161, 257, 739], [257, 320, 684],
    [320, 397, 581], [397, 487, 716], [487, 558, 651], [558, 654, 742],
    [654, 734, 617], [734, 847, 704], [847, 920, 642], [920, 1004, 537],
    [1004, 1113, 709], [1113, 1187, 631], [1187, 1288, 724], [1288, 1372, 607],
    [1372, 1478, 742], [1478, 1580, 654]
  ];
  const frontBuildingFacades = [
    [0, 107, 703], [107, 191, 762], [191, 290, 653], [290, 374, 763],
    [374, 475, 693], [475, 562, 754], [562, 638, 615], [638, 747, 754],
    [747, 840, 670], [840, 957, 766], [957, 1041, 645], [1041, 1146, 764],
    [1146, 1239, 713], [1239, 1350, 793], [1350, 1433, 683]
  ];
  const visibleBackFacades = backBuildingFacades.flatMap(([left, right, top]) => {
    const edges = [left, right];
    for (const [frontLeft, frontRight] of frontBuildingFacades) {
      if (frontRight > left && frontLeft < right) {
        edges.push(Math.max(left, frontLeft), Math.min(right, frontRight));
      }
    }
    edges.sort((first, second) => first - second);
    return edges.slice(0, -1).flatMap((start, index) => {
      const end = edges[index + 1];
      const midpoint = (start + end) / 2;
      const foreground = frontBuildingFacades.find(([frontLeft, frontRight]) => midpoint >= frontLeft && midpoint < frontRight);
      const bottom = foreground ? Math.min(790, foreground[2]) : 790;
      return bottom > top ? [[start, end, top, bottom]] : [];
    });
  });
  const makeFacadeWindows = (facades, foreground) => {
    const width = foreground ? 4 : 3;
    const height = foreground ? 6 : 5;
    const horizontalGap = foreground ? 7 : 6;
    const verticalGap = foreground ? 9 : 8;
    const horizontalMargin = foreground ? 8 : 6;
    const topMargin = foreground ? 12 : 10;
    const bottomMargin = foreground ? 12 : 10;
    const litChance = foreground ? .64 : .48;
    return facades.flatMap(([left, right, top, bottom]) => {
      const availableWidth = right - left - horizontalMargin * 2;
      const availableHeight = bottom - top - topMargin - bottomMargin;
      const columns = Math.floor((availableWidth + horizontalGap) / (width + horizontalGap));
      const rows = Math.floor((availableHeight + verticalGap) / (height + verticalGap));
      if (columns < 1 || rows < 1) return [];
      const gridWidth = columns * width + (columns - 1) * horizontalGap;
      const gridHeight = rows * height + (rows - 1) * verticalGap;
      const firstX = Math.round(left + horizontalMargin + (availableWidth - gridWidth) / 2);
      const firstY = Math.round(top + topMargin + (availableHeight - gridHeight) / 2);
      const windows = [];
      for (let row = 0; row < rows; row += 1) {
        for (let column = 0; column < columns; column += 1) {
          if (random() > litChance) continue;
          const x = firstX + column * (width + horizontalGap);
          const y = firstY + row * (height + verticalGap);
          const opacity = foreground ? .32 + random() * .42 : .2 + random() * .34;
          windows.push(`<rect x="${x}" y="${y}" width="${width}" height="${height}" fill="${config.glow}" opacity="${opacity.toFixed(2)}"/>`);
        }
      }
      return windows;
    });
  };
  const windowLights = city
    ? [...makeFacadeWindows(visibleBackFacades, false), ...makeFacadeWindows(frontBuildingFacades.map(([left, right, top]) => [left, right, top, 810]), true)].join("")
    : "";
  const coast = water ? `<path d="M0 744q290-64 573 40t583-47q233-70 444-34v237H0z" fill="url(#near-${id})"/><g fill="none" stroke="${config.glow}" opacity=".37"><path d="M0 772q290-64 573 40t583-47q233-70 444-34" stroke-width="2"/><path d="M0 805q290-64 573 40t583-47q233-70 444-34" stroke-width="2"/></g>${Array.from({ length: 55 }, () => `<path d="M${Math.round(random() * 1600)} ${Math.round(766 + random() * 120)}h${Math.round(9 + random() * 65)}" stroke="${config.glow}" stroke-width="${(random() * 2 + .5).toFixed(1)}" opacity="${(.1 + random() * .3).toFixed(2)}"/>`).join("")}` : "";
  const rockLines = Array.from({ length: 54 }, () => {
    const x = Math.round(random() * 1600), y = Math.round(625 + random() * 230);
    return `<path d="M${x} ${y}q${Math.round(13 + random() * 30)} ${Math.round(-8 + random() * 20)} ${Math.round(48 + random() * 73)} ${Math.round(-13 + random() * 25)}" fill="none" stroke="${random() > .5 ? starColor : config.glow}" stroke-width="${(0.8 + random() * 1.5).toFixed(1)}" opacity="${(.07 + random() * .16).toFixed(2)}"/>`;
  }).join("");
  const ridge = !city && !water ? config.terrain === "mist"
    ? `<path d="M0 830q220-79 448 4t482-60q327-100 670-14v180H0z" fill="url(#near-${id})" opacity=".72"/><path d="M0 786q255-73 496 0t520-35q312-99 584-58" fill="none" stroke="${starColor}" stroke-width="24" opacity=".17"/><path d="M0 845q260-55 553 17t547-45q285-80 500-35" fill="none" stroke="${config.glow}" stroke-width="31" opacity=".22"/>`
    : `<path d="M0 835 80 765l42 17 74-142 29 16 48-86 29 27 41-31 33 51 50-22 72-112 36 44 38-19 64 99 53-16 45 60 46-28 70-122 41 25 29-65 37 12 56 103 57-36 51 62 57-33 47 26 56-103 39 13 38-53 46 23 74 102 34-16 50 84 56-18 42 55v240H0z" fill="url(#near-${id})"/><path d="M0 849 80 785l42 17 74-142 29 16 48-86 29 27 41-31 33 51 50-22 72-112 36 44 38-19 64 99 53-16 45 60 46-28 70-122 41 25 29-65 37 12 56 103 57-36 51 62 57-33 47 26 56-103 39 13 38-53 46 23 74 102 34-16 50 84 56-18 42 55" fill="none" stroke="${starColor}" stroke-width="1.3" opacity=".19"/><g fill="${config.foreground}" opacity=".23"><path d="m198 640 75-70 29 27 41-31-69 153-115 65z"/><path d="m499 483 36 44 38-19 64 99-74 59-135 17z"/><path d="m790 501 41 25 29-65 37 12 56 103-129 132-118-40z"/><path d="m1350 540 39 13 38-53 46 23 74 102-101 52-160-14z"/></g>${rockLines}` : "";
  const foliage = city ? "" : Array.from({ length: 85 }, () => {
    const x = Math.round(random() * 1660 - 30), y = Math.round(792 + random() * 150);
    const height = Math.round(11 + random() * (city ? 25 : mountain ? 81 : 52));
    return city
      ? `<path d="M${x} ${y}v-${height}m-8 8 8-7 8 7" fill="none" stroke="${config.horizon}" stroke-width="2" opacity=".45"/>`
      : `<path d="M${x} ${y}v-${height}m-${Math.round(height / 3)} ${Math.round(height * .64)} ${Math.round(height / 3)}-${Math.round(height * .36)} ${Math.round(height / 3)} ${Math.round(height * .36)}m-${Math.round(height * .56)} ${Math.round(-height * .3)} ${Math.round(height * .23)}-${Math.round(height * .3)} ${Math.round(height * .23)} ${Math.round(height * .3)}" fill="none" stroke="${config.horizon}" stroke-width="${height > 54 ? 2.7 : 1.5}" opacity=".54"/>`;
  }).join("");
  const grass = city ? "" : Array.from({ length: 115 }, () => {
    const x = Math.round(random() * 1630 - 15), y = Math.round(873 + random() * 65), h = 8 + random() * 46;
    return `<path d="M${x} ${y}q${Math.round(random() * 24 - 12)}-${Math.round(h / 2)} ${Math.round(random() * 20 - 10)}-${Math.round(h)}" fill="none" stroke="${random() > .5 ? config.horizon : config.glow}" stroke-width="${(1 + random() * 1.5).toFixed(1)}" opacity="${(.19 + random() * .34).toFixed(2)}"/>`;
  }).join("");
  const meteorGradients = config.celestial === "meteor" ? `<linearGradient id="meteor-tail-one-${id}" gradientUnits="userSpaceOnUse" x1="1300" y1="20" x2="1180" y2="95"><stop stop-color="${starColor}" stop-opacity="0"/><stop offset=".65" stop-color="${config.tint || config.glow}" stop-opacity=".42"/><stop offset="1" stop-color="${starColor}" stop-opacity=".98"/></linearGradient><linearGradient id="meteor-tail-two-${id}" gradientUnits="userSpaceOnUse" x1="1150" y1="70" x2="1060" y2="140"><stop stop-color="${starColor}" stop-opacity="0"/><stop offset=".68" stop-color="${config.tint || config.glow}" stop-opacity=".32"/><stop offset="1" stop-color="${starColor}" stop-opacity=".82"/></linearGradient>` : "";
  const defs = `<defs><linearGradient id="sky-${id}" x2=".25" y2="1"><stop stop-color="${config.sky}"/><stop offset=".72" stop-color="${config.horizon}"/><stop offset="1" stop-color="${config.foreground}"/></linearGradient><radialGradient id="planet-${id}" cx=".32" cy=".25" r=".85"><stop stop-color="${starColor}"/><stop offset=".48" stop-color="${config.tint || config.glow}"/><stop offset="1" stop-color="${config.horizon}"/></radialGradient><radialGradient id="halo-${id}" gradientUnits="userSpaceOnUse" cx="1200" cy="270" r="238"><stop stop-color="${config.tint || config.glow}" stop-opacity=".3"/><stop offset=".36" stop-color="${config.tint || config.glow}" stop-opacity=".18"/><stop offset=".72" stop-color="${config.tint || config.glow}" stop-opacity=".055"/><stop offset="1" stop-color="${config.tint || config.glow}" stop-opacity="0"/></radialGradient>${meteorGradients}<linearGradient id="far-${id}" x2=".15" y2="1"><stop stop-color="${config.horizon}"/><stop offset="1" stop-color="${config.foreground}"/></linearGradient><linearGradient id="near-${id}" x2=".45" y2="1"><stop stop-color="${config.horizon}"/><stop offset="1" stop-color="${config.foreground}"/></linearGradient><radialGradient id="vignette-${id}"><stop offset=".45" stop-color="${config.sky}" stop-opacity="0"/><stop offset="1" stop-color="${config.foreground}" stop-opacity=".32"/></radialGradient><filter id="grain-${id}"><feTurbulence type="fractalNoise" baseFrequency=".53" numOctaves="3" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/><feComponentTransfer><feFuncA type="table" tableValues="0 .19"/></feComponentTransfer></filter></defs>`;
  const svg = (plane, content, localDefs = "") => `<svg class="scene-art atmosphere-${plane}" viewBox="0 0 1600 940" preserveAspectRatio="xMidYMid slice" aria-hidden="true" xmlns="http://www.w3.org/2000/svg">${defs}${localDefs}${content}</svg>`;
  return `<div class="atmosphere-scene" aria-hidden="true">
    ${svg("sky", `<rect width="1600" height="940" fill="url(#sky-${id})"/><rect width="1600" height="940" fill="url(#vignette-${id})"/><g fill="${starColor}">${stars}</g>${celestial}`)}
    ${svg("distance", `${farTerrain}<path d="M0 940V605q295 99 622 28t978-12v319H0z" fill="${config.glow}" opacity=".045"/>`)}
    ${svg("terrain", `${city ? cityGeometry + `<g clip-path="url(#city-facades-${id})">${windowLights}</g>` : water ? coast : ridge}<path d="M0 940V847q293-91 653 12t947-50v131H0z" fill="${config.foreground}" opacity=".65"/>${foliage}`, cityWindowClip)}
    ${svg("foreground", `<path d="M0 940v-45q352-50 640-5t509-12q225-41 451 13v49H0z" fill="${config.foreground}"/>${grass}<path fill="#fff" filter="url(#grain-${id})" opacity=".38" d="M0 780h1600v160H0z"/>`)}
    ${svg("grain", `<path fill="#fff" filter="url(#grain-${id})" opacity=".46" d="M0 0h1600v940H0z"/>`)}
  </div>`;
}
