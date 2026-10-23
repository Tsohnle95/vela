(() => {
  // Foreground and action colors are explicit so light and vivid palettes stay readable.
  window.velaPalettes = [
    { id: 'mint', name: 'Vela mint', group: 'Standard', bg: '#080909', fg: '#f4f5f1', accent: '#b3f6d6', muted: '#a1aaa4', line: '#303a34', surface: '#131b16', button: '#10251a', shadow: '#00000055' },
    { id: 'mono', name: 'Black & white', group: 'Standard', bg: '#090909', fg: '#f5f5f5', accent: '#ffffff', muted: '#a5a5a5', line: '#343434', surface: '#181818', button: '#171717', shadow: '#00000066' },
    { id: 'graphite', name: 'Soft graphite', group: 'Standard', bg: '#212325', fg: '#f7f5ef', accent: '#d7d0c0', muted: '#b7b6b0', line: '#494b4b', surface: '#2b2e30', button: '#282821', shadow: '#00000050' },
    { id: 'navy', name: 'Midnight & ice', group: 'Standard', bg: '#09121f', fg: '#f0f5ff', accent: '#b3d7ff', muted: '#a4b4c9', line: '#304059', surface: '#111f33', button: '#102744', shadow: '#00000055' },
    { id: 'paper', name: 'Warm paper', group: 'Standard', bg: '#f4f0e7', fg: '#292c29', accent: '#375d4d', muted: '#62675f', line: '#cecfc3', surface: '#e9e6da', button: '#ffffff', shadow: '#26332922' },
    { id: 'porcelain', name: 'Porcelain & ink', group: 'Standard', bg: '#fcfcfa', fg: '#1b2429', accent: '#285ba8', muted: '#5c6970', line: '#d5dde0', surface: '#ecf0f1', button: '#ffffff', shadow: '#16253425' },
    { id: 'cobalt', name: 'Electric cobalt', group: 'Vibrant', bg: '#080d1b', fg: '#f3f6ff', accent: '#91b5ff', muted: '#a5b3cf', line: '#2d3b5c', surface: '#111d36', button: '#102b5f', shadow: '#00000066' },
    { id: 'ultraviolet', name: 'Ultraviolet', group: 'Vibrant', bg: '#110a21', fg: '#f9f3ff', accent: '#c7a1ff', muted: '#b8a6cb', line: '#49365c', surface: '#221533', button: '#31134f', shadow: '#00000066' },
    { id: 'acid', name: 'Acid green', group: 'Vibrant', bg: '#0b0e05', fg: '#f5f7e8', accent: '#c5ff00', muted: '#afb79b', line: '#3c4626', surface: '#1b2210', button: '#1a2800', shadow: '#00000066' },
    { id: 'coral', name: 'Hot coral', group: 'Vibrant', bg: '#190c0b', fg: '#fff3ed', accent: '#ff927b', muted: '#c1aaa1', line: '#593b32', surface: '#2c1714', button: '#42180e', shadow: '#00000066' },
    { id: 'rose', name: 'Electric rose', group: 'Vibrant', bg: '#1b0915', fg: '#fff1fb', accent: '#ff8ece', muted: '#c5a5b9', line: '#553345', surface: '#2e1424', button: '#481132', shadow: '#00000066' },
    { id: 'tangerine', name: 'Tangerine', group: 'Vibrant', bg: '#170e07', fg: '#fff6e9', accent: '#ffad52', muted: '#c1b29d', line: '#55422b', surface: '#2b1d11', button: '#392007', shadow: '#00000066' },
    { id: 'sunflower', name: 'Sunflower', group: 'Vibrant', bg: '#131106', fg: '#fffae8', accent: '#ffe15a', muted: '#bbb49a', line: '#514a29', surface: '#25200d', button: '#332900', shadow: '#00000066' },
    { id: 'cyan', name: 'Bright cyan', group: 'Vibrant', bg: '#051518', fg: '#ecfcff', accent: '#55e5f4', muted: '#9cbec3', line: '#2b5156', surface: '#0d252a', button: '#073239', shadow: '#00000066' },
    { id: 'aqua', name: 'Neon aqua', group: 'Vibrant', bg: '#051612', fg: '#eafff4', accent: '#4dffc7', muted: '#99bfb0', line: '#2a5141', surface: '#0e2920', button: '#073527', shadow: '#00000066' },
    { id: 'orchid', name: 'Orchid & plum', group: 'Vibrant', bg: '#1a0a24', fg: '#fff3ff', accent: '#ef9bff', muted: '#c0a6cd', line: '#50325c', surface: '#2d153a', button: '#3c1049', shadow: '#00000066' },
    { id: 'red', name: 'Signal red', group: 'Vibrant', bg: '#180909', fg: '#fff0ee', accent: '#ff9292', muted: '#c4a7a3', line: '#573535', surface: '#2a1414', button: '#4b1013', shadow: '#00000066' },
    { id: 'lime-paper', name: 'Lime on paper', group: 'Light & colorful', bg: '#efffc6', fg: '#26321a', accent: '#3f6515', muted: '#586943', line: '#bdce97', surface: '#e2efb7', button: '#ffffff', shadow: '#2c3f1625' },
    { id: 'lavender', name: 'Lavender daylight', group: 'Light & colorful', bg: '#f0e7ff', fg: '#302044', accent: '#7440ad', muted: '#6e5a84', line: '#cebbdf', surface: '#e5d7f4', button: '#ffffff', shadow: '#46266125' },
    { id: 'peach', name: 'Peach & terracotta', group: 'Light & colorful', bg: '#ffe9db', fg: '#482920', accent: '#a23e22', muted: '#7f6256', line: '#dfbfaa', surface: '#f7dac7', button: '#ffffff', shadow: '#6f382525' },
    { id: 'sky', name: 'Open sky', group: 'Light & colorful', bg: '#e0f3ff', fg: '#15364b', accent: '#17668a', muted: '#506e80', line: '#aecfdf', surface: '#cee8f4', button: '#ffffff', shadow: '#1e506825' },
    { id: 'pink-paper', name: 'Pink lemonade', group: 'Light & colorful', bg: '#ffe5ef', fg: '#462436', accent: '#a82f68', muted: '#805b70', line: '#dfb7cb', surface: '#f5d3e2', button: '#ffffff', shadow: '#66233f25' },
    { id: 'blue-field', name: 'Cobalt field', group: 'Color fields', bg: '#143cb5', fg: '#ffffff', accent: '#e3ff77', muted: '#d0dcff', line: '#6080d1', surface: '#214cc2', button: '#25300b', shadow: '#071c6955' },
    { id: 'violet-field', name: 'Violet field', group: 'Color fields', bg: '#5023a6', fg: '#fffaff', accent: '#a9ffc9', muted: '#e0cffb', line: '#926bc8', surface: '#6435ba', button: '#173c25', shadow: '#24104d66' }
  ];
})();
