import { CssBaseline, createTheme, ThemeProvider } from '@mui/material';
import { enUS, jaJP } from '@mui/x-data-grid/locales';
import { type ReactNode, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

type Props = {
  children: ReactNode;
};

export const DynamicThemeProvider = ({ children }: Props) => {
  const { i18n } = useTranslation();

  const localeText = useMemo(() => {
    switch (i18n.language) {
      case 'ja':
      case 'ja-JP':
        return jaJP;
      default:
        return enUS;
    }
  }, [i18n.language]);

  const theme = useMemo(
    () =>
      createTheme(
        {
          cssVariables: true,
          palette: {
            mode: 'dark',
          },
        },
        localeText,
      ),
    [localeText],
  );

  return (
    <ThemeProvider theme={theme}>
      <CssBaseline />
      {children}
    </ThemeProvider>
  );
};
