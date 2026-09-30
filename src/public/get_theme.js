(function() {
  let t = localStorage.getItem('theme');

  if (!t || t === 'system') {
    t = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'mocha' : 'latte';
  }

  document.documentElement.dataset.theme = t;
})();
