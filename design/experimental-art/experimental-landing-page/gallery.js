const filters = [...document.querySelectorAll('[data-filter]')];
filters.forEach(button => button.addEventListener('click', () => {
  filters.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
  document.querySelectorAll('.card').forEach(card => {
    card.hidden = button.dataset.filter !== 'all' && card.dataset.tone !== button.dataset.filter;
  });
}));
