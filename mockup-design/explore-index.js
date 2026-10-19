const violetHourId = "10-dusk-crescent-detail-violet-hour";
const curatedGroups = [
  ...velaCityWorkflowGroups.map((group) => ({
    ...group,
    variants: velaCityWorkflowExplorations.filter((design) => design.parent === group.seed)
  })),
  ...velaDetailedGroups
    .filter((group) => group.group !== "tidal")
    .map((group) => ({
      ...group,
      title: `${velaExplorations.find((design) => design.id === group.seed).name} · a world of its own.`,
      variants: velaDetailedExplorations.filter((design) => design.parent === group.seed && (group.group !== "crescent" || design.id !== violetHourId))
    }))
];

const groupOrderByFamily = new Map();
for (const group of curatedGroups) {
  const seed = velaExplorations.find((design) => design.id === group.seed);
  const groupIndex = (groupOrderByFamily.get(seed.family) ?? 0) + 1;
  groupOrderByFamily.set(seed.family, groupIndex);
  const cluster = document.createElement("div");
  cluster.className = "iteration-cluster";
  cluster.id = `lineage-${group.group}`;
  cluster.innerHTML = `<div class="cluster-heading"><span>${String(groupIndex).padStart(2, "0")} / ${group.group.toUpperCase()}</span><h3>${group.title}</h3></div><div class="iteration-card-grid"></div>`;
  const grid = cluster.querySelector(".iteration-card-grid");
  for (const design of [seed, ...group.variants]) {
    const card = document.createElement("a");
    const isSeed = design === seed;
    card.className = `iteration-card exploration-card ${isSeed ? "seed-card" : ""} ${design.light ? "scene-light" : "scene-dark"}`;
    card.href = `iterations/explore.html?design=${design.id}`;
    card.style.cssText = `--scene-ink:${design.ink};--scene-accent:${design.accent};--scene-paper:${design.sky};--scene-ground:${design.foreground}`;
    card.innerHTML = `<div class="iteration-preview exploration-preview">${velaAtmosphere(design, "card")}<span>${isSeed ? "SELECTED STARTING POINT" : design.motif.toUpperCase()}</span></div>
      <div class="iteration-card-body"><div><small>${design.family} · ${isSeed ? "FAVORITE" : "NEW LANDSCAPE"}</small><h3>${design.name}</h3><p>${isSeed ? "Saved before this round in the reference archive." : design.note}</p></div><span class="iteration-card-arrow" aria-hidden="true">↗</span></div>`;
    grid.append(card);
  }
  document.querySelector(`#detail-${seed.family}`).append(cluster);
}
