(() => {
  const { build } = window.velaMeet;
  const beams = [
    {
      id: 24,
      name: 'Mint Drop',
      options: {
        beamOrigin: 'header',
        beamColor: '#6ee7b7',
        beamNative: true,
        beamSettings: 'default',
        beamDelay: 2200,
        beamIntro: 7.6,
        beamRise: 1.4,
        beamPace: 0.9,
      },
    },
    {
      id: 25,
      name: 'Brand Mark',
      bodyClass: 'd1 beam-center',
      options: {
        beamOrigin: 'title',
        beamColor: '#5eead4',
        beamNative: true,
        beamSettings: 'soft',
        beamDelay: 2800,
        beamIntro: 10,
        beamRise: 2,
        beamPace: 0.65,
      },
    },
    {
      id: 26,
      name: 'Signal',
      options: {
        beamOrigin: 'header',
        beamColor: '#c6ff3d',
        beamNative: true,
        beamSettings: 'laser',
        beamDelay: 1200,
        beamIntro: 6,
        beamRise: 1,
        beamPace: 1.35,
      },
    },
    {
      id: 27,
      name: 'Deep Haze',
      bodyClass: 'd1 beam-center',
      options: {
        beamOrigin: 'secondary',
        beamColor: '#38bdf8',
        beamNative: true,
        beamSettings: 'haze',
        beamDelay: 3800,
        beamIntro: 11,
        beamRise: 2.4,
        beamPace: 0.55,
      },
    },
    {
      id: 28,
      name: 'CTA Bloom',
      bodyClass: 'd1 beam-center',
      options: {
        beamOrigin: 'primary',
        beamColor: '#fbbf24',
        beamNative: true,
        beamSettings: 'soft',
        beamDelay: 2600,
        beamIntro: 8.5,
        beamRise: 1.6,
        beamPace: 1,
      },
    },
  ];

  for (const beam of beams) {
    window.velaDirections.push({
      id: beam.id,
      name: beam.name,
      standalone: true,
      bodyClass: beam.bodyClass || 'd1',
      body: () => build(beam.options),
    });
  }
})();
