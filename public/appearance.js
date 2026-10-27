// Runs before first paint. Only the nonsecret preference record is read.
(() => {
  let theme = 'system';
  try {
    const value = JSON.parse(localStorage.getItem('nexus.preferences.v1') || 'null');
    if (value?.version === 1 && ['light', 'dark', 'system'].includes(value.theme)) theme = value.theme;
  } catch { /* Preferences module reports inaccessible storage after mount. */ }
  const resolved = theme === 'system' ? (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light') : theme;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
})();
