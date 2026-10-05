import { readSaved, save } from './player/progress';

export type Theme = 'system' | 'light' | 'dark';
export function savedTheme(): Theme {
  const value = readSaved('theme');
  return value === 'light' || value === 'dark' ? value : 'system';
}
export function applyTheme(theme: Theme) {
  const dark = theme === 'dark' || (theme === 'system' && matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', dark ? '#101014' : '#f4f3ef');
  save('theme', theme);
}
