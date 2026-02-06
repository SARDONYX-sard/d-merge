'use client';

import LayersIcon from '@mui/icons-material/Layers';
import NotesIcon from '@mui/icons-material/Notes';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import { Box, Card, CardActionArea, CardContent, Grid, type SxProps, type Theme, Typography } from '@mui/material';
import { useRouter } from '@tanstack/react-router';
import { useInjectJs } from '@/components/hooks/useInjectJs';
import { useTranslation } from '@/components/hooks/useTranslation';

const PAGES_DATA = [
  { path: 'convert', icon: <TransformIcon sx={{ fontSize: 48 }} /> },
  { path: 'patch', icon: <LayersIcon sx={{ fontSize: 48 }} /> },
  { path: 'hkanno', icon: <NotesIcon sx={{ fontSize: 48 }} /> },
  { path: 'settings', icon: <SettingsIcon sx={{ fontSize: 48 }} /> },
] as const;

export const WelcomePage = () => {
  const router = useRouter();
  const { t } = useTranslation();
  useInjectJs();

  return (
    <Box component='main' sx={pageSx}>
      <Typography variant='h3' sx={{ mb: 1, fontWeight: 'bold', letterSpacing: 1 }}>
        {t('welcome.title')}
      </Typography>
      <Typography variant='subtitle1' sx={{ mb: 6, color: 'text.secondary' }}>
        {t('welcome.subtitle')}
      </Typography>

      <Grid container spacing={4} justifyContent='center' sx={{ maxWidth: 900 }}>
        {PAGES_DATA.map((page) => {
          const key = page.path;
          const title = t(`welcome.tools.${key}.title`);
          const desc = t(`welcome.tools.${key}.desc`);

          return (
            <Grid key={key} size={{ xs: 12, sm: 6, md: 6 }}>
              <Card
                elevation={4}
                sx={(theme) => ({
                  bgcolor: theme.palette.mode === 'dark' ? 'rgba(30, 30, 30, 0.6)' : 'rgba(255, 255, 255, 0.7)',
                  backdropFilter: 'blur(5px)',
                  borderRadius: 4,
                  transition: 'transform 0.2s, box-shadow 0.2s',
                  '&:hover': {
                    transform: 'translateY(-4px)',
                    boxShadow: 8,
                  },
                })}
              >
                <CardActionArea
                  onClick={() =>
                    router.navigate({
                      to: `/${page.path}`,
                    })
                  }
                  sx={{ height: '100%' }}
                >
                  <CardContent
                    sx={{
                      textAlign: 'center',
                      p: 4,
                      display: 'flex',
                      flexDirection: 'column',
                      alignItems: 'center',
                      '&:hover': {
                        color: 'var(--mui-palette-primary-main)',
                      },
                    }}
                  >
                    {page.icon}
                    <Typography variant='h6' sx={{ mt: 2 }}>
                      {title}
                    </Typography>
                    <Typography variant='body2' sx={{ mt: 1 }}>
                      {desc}
                    </Typography>
                  </CardContent>
                </CardActionArea>
              </Card>
            </Grid>
          );
        })}
      </Grid>
    </Box>
  );
};

const pageSx: SxProps<Theme> = {
  display: 'grid',
  minHeight: 'calc(100vh - 56px)',
  placeItems: 'center',
  pt: 8,
  px: 2,
  width: '100%',
};
