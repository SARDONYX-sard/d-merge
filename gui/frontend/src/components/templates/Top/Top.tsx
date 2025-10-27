'use client';

import LayersIcon from '@mui/icons-material/Layers';
import NotesIcon from '@mui/icons-material/Notes';
import SettingsIcon from '@mui/icons-material/Settings';
import TransformIcon from '@mui/icons-material/Transform';
import { Box, Card, CardActionArea, CardContent, Grid, SxProps, Theme, Typography } from '@mui/material';
import { useRouter } from 'next/navigation';
import { useInjectJs } from '@/components/hooks/useInjectJs';

const pages = [
  {
    path: '/convert',
    icon: <TransformIcon sx={{ fontSize: 48 }} />,
    title: 'Convert',
    desc: 'HKX ⇄ XML converter for Havok animation data.',
  },
  {
    path: '/patch',
    icon: <LayersIcon sx={{ fontSize: 48 }} />,
    title: 'Patch',
    desc: 'Nemesis/FNIS compatible patcher for HKX files.',
  },
  {
    path: '/hkanno',
    icon: <NotesIcon sx={{ fontSize: 48 }} />,
    title: 'HKAnno',
    desc: 'Annotation editor for HKX/XML structure.',
  },
  {
    path: '/settings',
    icon: <SettingsIcon sx={{ fontSize: 48 }} />,
    title: 'Settings',
    desc: 'Application preferences and configuration.',
  },
] as const;

export const WelcomePage = () => {
  const router = useRouter();
  useInjectJs();

  return (
    <Box component='main' sx={pageSx}>
      <Typography variant='h3' sx={{ mb: 1, fontWeight: 'bold', letterSpacing: 1 }}>
        Welcome to HKX Studio
      </Typography>
      <Typography variant='subtitle1' sx={{ mb: 6, color: 'text.secondary' }}>
        Select a tool to get started
      </Typography>

      <Grid container spacing={4} justifyContent='center' sx={{ maxWidth: 900 }}>
        {pages.map((p) => (
          <Grid size={6} key={p.path}>
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
              <CardActionArea onClick={() => router.push(p.path)} sx={{ height: '100%' }}>
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
                  {p.icon}
                  <Typography variant='h6' sx={{ mt: 2 }}>
                    {p.title}
                  </Typography>
                  <Typography variant='body2' sx={{ mt: 1 }}>
                    {p.desc}
                  </Typography>
                </CardContent>
              </CardActionArea>
            </Card>
          </Grid>
        ))}
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
