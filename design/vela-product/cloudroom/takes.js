(() => {
  const { build } = window.velaMeet;
  const takes = [
    {
      id: 21,
      name: 'Aurora',
      bodyClass: 'd1 take-aurora',
      options: {
        description: 'A desktop IDE for long sessions. The editor, OpenCode, terminal and live diffs stay in one cool, quiet window. Use the models you already run.',
        flow: 'ribbon',
        tone: 'ribbon',
        flowX: 76,
      },
    },
    {
      id: 22,
      name: 'Paper',
      bodyClass: 'd1 take-paper',
      options: {
        description: 'A desktop IDE for coding with an agent. The editor, OpenCode, terminal and live diffs, arranged without noise.',
        shot: 'light',
        flow: 'none',
      },
    },
    {
      id: 23,
      name: 'Studio',
      bodyClass: 'd1 take-studio',
      options: {
        description: 'A desktop IDE built like a tool. Editor, OpenCode, terminal and live diffs in one sharp, monochrome workspace.',
        flow: 'grid',
        tone: 'lime',
        flowX: 50,
      },
    },
  ];

  for (const take of takes) {
    window.velaDirections.push({
      id: take.id,
      name: take.name,
      standalone: true,
      bodyClass: take.bodyClass,
      body: () => build(take.options),
    });
  }
})();
