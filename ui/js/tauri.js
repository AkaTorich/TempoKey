// Мост к Tauri. В обычном браузере (просмотр вёрстки) вызовы ничего не делают.
const T = window.__TAURI__;

export const inTauri = !!T;
export const appWindow = T ? T.window.getCurrentWindow() : null;

export function invoke(cmd, args) {
  return T ? T.core.invoke(cmd, args) : Promise.resolve(null);
}

export function listen(name, cb) {
  return T ? T.event.listen(name, cb) : Promise.resolve(() => {});
}

const AUDIO = ['flac', 'wav', 'mp3', 'aac', 'm4a', 'mp4', 'ogg', 'oga'];

const asList = (r) => (r ? (Array.isArray(r) ? r : [r]) : []);

export async function pickFiles() {
  if (!T) return [];
  return asList(await T.dialog.open({ multiple: true, title: 'Add tracks', filters: [{ name: 'Audio', extensions: AUDIO }] }));
}

export async function pickFolders() {
  if (!T) return [];
  return asList(await T.dialog.open({ directory: true, multiple: true, title: 'Add folders' }));
}

export async function pickSavePath(name) {
  if (!T) return null;
  return T.dialog.save({ title: 'Export CSV', defaultPath: name, filters: [{ name: 'CSV', extensions: ['csv'] }] });
}
