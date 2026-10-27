(() => {
  const { build } = window.velaMeet;
  const flows = [
    {
      id: 24,
      name: 'Ribbon',
      bodyClass: 'd1 flow-ribbon',
      options: { flow: 'ribbon', tone: 'ribbon', flowX: 64 },
    },
    {
      id: 25,
      name: 'Beam',
      bodyClass: 'd1 flow-beam',
      options: { flow: 'beam', tone: 'beam', flowX: 64 },
    },
    {
      id: 26,
      name: 'Curtain',
      bodyClass: 'd1 flow-curtain',
      options: { flow: 'curtain', tone: 'aurora', flowX: 64 },
    },
    {
      id: 27,
      name: 'Pulse',
      bodyClass: 'd1 flow-pulse',
      options: { flow: 'pulse', tone: 'amber', flowX: 64 },
    },
    {
      id: 28,
      name: 'Circuit',
      bodyClass: 'd1 flow-circuit',
      options: { flow: 'grid', tone: 'lime', flowX: 50 },
    },
  ];

  for (const flow of flows) {
    window.velaDirections.push({
      id: flow.id,
      name: flow.name,
      standalone: true,
      bodyClass: flow.bodyClass,
      body: () => build(flow.options),
    });
  }
})();
