import { Box, FormControl, InputLabel, MenuItem, Select, TextField } from '@mui/material';
import { useTranslation } from '../../../hooks/useTranslation';
import { useEditorContext } from '../context/editorContext';

/** Output path and format settings bar */
export const FileSettingsBar = () => {
  const { t } = useTranslation();

  const [state, dispatch] = useEditorContext();
  const tab = state.tabs[state.active];

  if (!tab) return null;

  return (
    <Box
      sx={{
        display: 'flex',
        gap: 2,
        alignItems: 'center',
        px: 2,
        py: 1,
        borderBottom: '1px solid #444',
        bgcolor: '#2a2a2a',
      }}
    >
      <TextField
        label={t('output.path_label')}
        size='small'
        fullWidth
        value={tab.outputPath}
        onChange={(e) =>
          dispatch({
            type: 'UPDATE_OUTPUT',
            outputPath: e.target.value,
          })
        }
      />

      <FormControl size='small' sx={{ minWidth: 120 }}>
        <InputLabel>{t('convert.output_format_label')}</InputLabel>
        <Select
          label={t('convert.output_format_label')}
          value={tab.format}
          onChange={(e) =>
            dispatch({
              type: 'UPDATE_FORMAT',
              format: e.target.value,
            })
          }
        >
          <MenuItem value='amd64'>amd64</MenuItem>
          <MenuItem value='win32'>win32</MenuItem>
          <MenuItem value='xml'>xml</MenuItem>
        </Select>
      </FormControl>
    </Box>
  );
};
