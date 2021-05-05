import { getItem, setItem } from './local-storage';

const THEME_KEY = 'tmeme';

export const theme = {
  loadSettingTheme() {
    switch (getItem(THEME_KEY)) {
      case 'light': {
        this.useLightTheme();
        break;
      }
      case 'dark': {
        this.useDarkTheme();
        break;
      }
      default: {
        this.useSystemTheme();
        break;
      }
    }
  },
  useSystemTheme() {
    document.querySelector('html').dataset.colorMode = 'auto';
    setItem(THEME_KEY, 'auto');
  },
  useLightTheme() {
    document.querySelector('html').dataset.colorMode = 'light';
    setItem(THEME_KEY, 'light');
  },
  useDarkTheme() {
    document.querySelector('html').dataset.colorMode = 'dark';
    setItem(THEME_KEY, 'dark');
  },
};
