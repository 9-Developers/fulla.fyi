(function () {
  var switcher = document.getElementById('theme-switcher');
  var themes = {};

  switcher.querySelectorAll('[data-theme-value]').forEach(function (el) {
    var theme = el.getAttribute('data-theme-value');
    themes[theme] = [
      el.textContent.trim(),
      el.querySelector('use').getAttribute('href')
    ];
  });

  function updateSummary(theme) {
    var data = themes[theme] || themes.system;
    document.getElementById('theme-summary-icon').querySelector('use').setAttribute('href', data[1]);
    document.getElementById('theme-summary-label').textContent = data[0];
  }

  function setTheme(theme) {
    if (theme === 'system') {
      var resolved = window.matchMedia('(prefers-color-scheme: dark)').matches ? 'mocha' : 'latte';
      document.documentElement.dataset.theme = resolved;
      localStorage.removeItem('theme');
    } else {
      document.documentElement.dataset.theme = theme;
      localStorage.setItem('theme', theme);
    }

    updateSummary(theme);
    switcher.removeAttribute('open');
  }

  updateSummary(localStorage.getItem('theme') || 'system');

  switcher.querySelectorAll('[data-theme-value]').forEach(function (el) {
    el.addEventListener('click', function () {
      setTheme(this.getAttribute('data-theme-value'));
    });
  });
})();
