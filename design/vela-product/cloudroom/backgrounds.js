(() => {
  const { build } = window.velaMeet;
  const effects = [
    { id: 29, name: 'Aurora', slug: 'aurora' },
    { id: 30, name: 'Ink', slug: 'ink' },
    { id: 31, name: 'Nebula', slug: 'nebula' },
    { id: 32, name: 'Caustics', slug: 'caustics' },
    { id: 33, name: 'Vortex', slug: 'vortex' },
    { id: 34, name: 'Prism', slug: 'prism' },
    { id: 35, name: 'Smoke', slug: 'smoke' },
    { id: 36, name: 'Silk', slug: 'silk' },
    { id: 37, name: 'Grid', slug: 'grid' },
    { id: 38, name: 'Particles', slug: 'particles' },
  ];

  for (const effect of effects) {
    window.velaDirections.push({
      id: effect.id,
      name: effect.name,
      standalone: true,
      bodyClass: `d1 effect-page effect-${effect.slug}`,
      body: () => build({ effect: effect.slug, beam: false }),
    });
  }
})();
