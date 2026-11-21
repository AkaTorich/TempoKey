// Окно — программа, а не страница: без контекстного меню браузера и без его клавиш
// (перезагрузка, печать, поиск по странице, исходник, масштаб, назад/вперёд, каретка F7).

const BLOCKED_CTRL = new Set(['r', 'p', 'f', 'g', 'u', 's', 'j', 'h', 'o', 'n', 't', 'w', 'd', 'e', 'l', '+', '-', '=', '0']);
const BLOCKED_KEYS = new Set(['F1', 'F3', 'F5', 'F7', 'F11', 'F12', 'BrowserBack', 'BrowserForward', 'BrowserRefresh', 'BrowserSearch']);

export function initGuard() {
  document.addEventListener('contextmenu', (e) => e.preventDefault(), true);

  document.addEventListener('keydown', (e) => {
    const k = e.key.length === 1 ? e.key.toLowerCase() : e.key;
    const editing = e.target instanceof HTMLInputElement;
    if (BLOCKED_KEYS.has(e.key)) {
      e.preventDefault();
      return;
    }
    // в полях ввода остаются привычные Ctrl+A/C/V/X/Z
    if ((e.ctrlKey || e.metaKey) && BLOCKED_CTRL.has(k) && !(editing && 'acvxz'.includes(k))) {
      e.preventDefault();
      return;
    }
    if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Home')) e.preventDefault();
  }, true);

  // масштаб колесом с Ctrl и жестом
  window.addEventListener('wheel', (e) => { if (e.ctrlKey) e.preventDefault(); }, { passive: false, capture: true });
  // кнопки мыши «назад/вперёд»
  window.addEventListener('mouseup', (e) => { if (e.button === 3 || e.button === 4) e.preventDefault(); }, true);
  // перетаскивание картинок и выделенного текста самой страницей
  document.addEventListener('dragstart', (e) => e.preventDefault());
  document.addEventListener('selectstart', (e) => {
    if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
  });
}
