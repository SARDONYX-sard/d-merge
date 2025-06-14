import { createTheme, ThemeProvider, CssBaseline } from '@mui/material';
import { jaJP } from '@mui/x-data-grid/locales';
import { enUS } from '@mui/x-data-grid/locales';
import { useMemo, type ReactNode } from 'react';
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
