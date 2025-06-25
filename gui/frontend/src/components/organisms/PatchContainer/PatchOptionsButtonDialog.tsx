import SettingsIcon from '@mui/icons-material/Settings';
import {
  Box,
  Dialog,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  FormGroup,
  FormHelperText,
  FormLabel,
  IconButton,
  MenuItem,
  Select,
  Switch,
  Tooltip,
} from '@mui/material';
import { produce } from 'immer';
import { useState } from 'react';

import { useTranslation } from '@/components/hooks/useTranslation';
import { usePatchContext } from '@/components/providers/PatchProvider';
import type { PatchOptions } from '@/services/api/patch';

export const PatchOptionsDialog = () => {
  const [open, setOpen] = useState(false);
  const { patchOptions, setPatchOptions } = usePatchContext();
  const { t } = useTranslation();

  const apply = (recipe: (draft: PatchOptions) => void) => {
    setPatchOptions((prev) => produce(prev, recipe));
  };

  return (
    <>
      <Tooltip placement='top' title={t('patch.options_title')}>
        <IconButton aria-label={t('patch.open_options_dialog')} onClick={() => setOpen(true)}>
          <SettingsIcon />
        </IconButton>
      </Tooltip>
      <Dialog
        aria-labelledby='patch-options-dialog-title'
        fullWidth={true}
        maxWidth='sm'
        onClose={() => setOpen(false)}
        open={open}
      >
        <DialogTitle id='patch-options-dialog-title'>{t('patch.options_title')}</DialogTitle>
        <DialogContent dividers={true}>
          {/* Output Target */}
          <Box mb={3}>
            <FormLabel component='legend' sx={{ mb: 1, fontWeight: 'bold' }}>
              {t('patch.output_target_label')}
            </FormLabel>
            <FormGroup>
              <Select
                aria-label={t('patch.output_target_aria_label')}
                fullWidth={true}
                onChange={(e) =>
                  apply((draft) => {
                    draft.outputTarget = e.target.value;
                  })
                }
                value={patchOptions.outputTarget}
              >
                <MenuItem value='SkyrimSE'>{t('patch.output_targets.skyrim_se')}</MenuItem>
                <MenuItem value='SkyrimLE'>{t('patch.output_targets.skyrim_le')}</MenuItem>
              </Select>
              <FormHelperText sx={{ mt: 1 }}>{t('patch.output_target_help')}</FormHelperText>
            </FormGroup>
          </Box>

          <Box>
            <FormControlLabel
              control={
                <Switch
                  checked={patchOptions.autoRemoveMeshes}
                  name='autoRemoveMeshes'
                  onChange={(e) =>
                    apply((draft) => {
                      draft.autoRemoveMeshes = e.target.checked;
                    })
                  }
                />
              }
              label={t('patch.auto_remove_meshes_option_label')}
            />
            <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
              {t('patch.auto_remove_meshes_option_help')}
            </FormHelperText>

            <FormControlLabel
              control={
                <Switch
                  checked={patchOptions.useProgressReporter}
                  name='useProgressReporter'
                  onChange={(e) =>
                    apply((draft) => {
                      draft.useProgressReporter = e.target.checked;
                    })
                  }
                />
              }
              label={t('patch.use_progress_reporter_option_label')}
            />
            <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
              {t('patch.use_progress_reporter_option_help')}
            </FormHelperText>
          </Box>

          {/* Hack Options */}
          <Box mb={3}>
            <FormLabel component='legend' sx={{ mb: 1, fontWeight: 'bold' }}>
              {t('patch.hack_options_label')}
            </FormLabel>
            <FormGroup>
              <FormControlLabel
                control={
                  <Switch
                    checked={patchOptions.hackOptions.castRagdollEvent}
                    name='castRagdollEvent'
                    onChange={(e) =>
                      apply((draft) => {
                        draft.hackOptions.castRagdollEvent = e.target.checked;
                      })
                    }
                  />
                }
                label={t('patch.hack_options.cast_ragdoll_event')}
              />
              <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
                {t('patch.hack_options.cast_ragdoll_event_help')}
              </FormHelperText>
            </FormGroup>
          </Box>

          {/* Debug Options */}
          <Box mb={3}>
            <FormLabel component='legend' sx={{ mb: 1, fontWeight: 'bold' }}>
              {t('patch.debug_options_label')}
            </FormLabel>
            <FormGroup>
              <FormControlLabel
                control={
                  <Switch
                    checked={patchOptions.debug.outputPatchJson}
                    name='outputPatchJson'
                    onChange={(e) =>
                      apply((draft) => {
                        draft.debug.outputPatchJson = e.target.checked;
                      })
                    }
                  />
                }
                label={t('patch.debug_options.output_patch_json')}
              />
              <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
                {t('patch.debug_options.output_patch_json_help')}
              </FormHelperText>

              <FormControlLabel
                control={
                  <Switch
                    checked={patchOptions.debug.outputMergedJson}
                    name='outputMergedXml'
                    onChange={(e) =>
                      apply((draft) => {
                        draft.debug.outputMergedJson = e.target.checked;
                      })
                    }
                  />
                }
                label={t('patch.debug_options.output_merged_json')}
              />
              <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
                {' '}
                {t('patch.debug_options.output_merged_json_help')}{' '}
              </FormHelperText>

              <FormControlLabel
                control={
                  <Switch
                    checked={patchOptions.debug.outputMergedXml}
                    name='outputMergedXml'
                    onChange={(e) =>
                      apply((draft) => {
                        draft.debug.outputMergedXml = e.target.checked;
                      })
                    }
                  />
                }
                label={t('patch.debug_options.output_merged_xml')}
              />
              <FormHelperText sx={{ ml: 3, mb: 1, color: 'text.secondary' }}>
                {t('patch.debug_options.output_merged_xml_help')}
              </FormHelperText>
            </FormGroup>
          </Box>
        </DialogContent>
      </Dialog>
    </>
  );
};
