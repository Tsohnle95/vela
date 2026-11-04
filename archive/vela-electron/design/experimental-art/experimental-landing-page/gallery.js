const filters = [...document.querySelectorAll('[data-group]')].filter(item => item.tagName === 'BUTTON');
const cards = [...document.querySelectorAll('.card')];
const search = document.querySelector('#search');
const tone = document.querySelector('#tone');
let group = new URLSearchParams(location.search).get('group') || 'slick';
if (!['all', 'original', 'new', 'compact', 'slick'].includes(group)) group = 'slick';

function filterCards() {
  const query = search.value.trim().toLowerCase();
  const numericQuery = /^\d+$/.test(query) ? Number(query) : null;
  let shown = 0;
  for (const card of cards) {
    const searchMatch = numericQuery !== null
      ? Number(card.querySelector('.number').textContent) === numericQuery
      : card.dataset.search.includes(query);
    const visible = (group === 'all' || card.dataset.group === group)
      && (tone.value === 'all' || card.dataset.tone === tone.value) && searchMatch;
    card.hidden = !visible;
    if (visible) shown++;
  }
  filters.forEach(button => button.setAttribute('aria-pressed', String(button.dataset.group === group)));
  document.querySelector('#result-count').textContent = `Showing ${shown} of ${cards.length} concepts`;
  document.querySelector('#no-results').hidden = shown > 0;
  const url = new URL(location.href);
  url.searchParams.set('group', group);
  history.replaceState(null, '', url);
}
filters.forEach(button => button.addEventListener('click', () => {
  group = button.dataset.group;
  filterCards();
}));
if (search && tone) {
  search.addEventListener('input', filterCards);
  tone.addEventListener('change', filterCards);
  filterCards();
} else {
  const legacy = [...document.querySelectorAll('[data-filter]')];
  legacy.forEach(button => button.addEventListener('click', () => {
    legacy.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
    cards.forEach(card => { card.hidden = button.dataset.filter !== 'all' && card.dataset.tone !== button.dataset.filter; });
  }));
}
